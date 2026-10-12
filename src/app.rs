use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

use crate::com::{ComApartment, ExcelConverter, PowerPointConverter, WordConverter};
use crate::ui::window::ProgressMessage;

pub enum DocumentKind {
    Word,
    PowerPoint,
    Excel,
}

impl DocumentKind {
    pub fn from_path(path: &Path) -> Result<Self, String> {
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_lowercase())
            .ok_or_else(|| "File has no extension".to_string())?;

        match ext.as_str() {
            "docx" | "doc" => Ok(DocumentKind::Word),
            "pptx" | "ppt" => Ok(DocumentKind::PowerPoint),
            "xlsx" | "xls" => Ok(DocumentKind::Excel),
            _ => Err(format!(
                "Unsupported file format '.{}'. Supported formats: .docx, .doc, .pptx, .ppt, .xlsx, .xls",
                ext
            )),
        }
    }
}

/// Computes the destination PDF path, applying safe collision avoidance if a PDF already exists
pub fn determine_output_path(input_path: &Path, overwrite: bool) -> PathBuf {
    let parent = input_path.parent().unwrap_or_else(|| Path::new("."));
    let stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");

    let initial_pdf = parent.join(format!("{}.pdf", stem));
    if overwrite || !initial_pdf.exists() {
        return initial_pdf;
    }

    // Safe numbering: "Document (1).pdf", "Document (2).pdf", etc.
    let mut counter = 1;
    loop {
        let candidate = parent.join(format!("{} ({}).pdf", stem, counter));
        if !candidate.exists() {
            return candidate;
        }
        counter += 1;
    }
}

/// Executes the document conversion workflow with live progress messaging
pub fn process_conversion(
    input_path: PathBuf,
    output_path: PathBuf,
    tx: Sender<ProgressMessage>,
) {
    let kind = match DocumentKind::from_path(&input_path) {
        Ok(k) => k,
        Err(e) => {
            let _ = tx.send(ProgressMessage::Finished {
                success: false,
                message: e,
                output_path: None,
            });
            return;
        }
    };

    let pid = std::process::id();
    let temp_output = output_path.with_extension(format!("tmp.{}.pdf", pid));

    let _ = tx.send(ProgressMessage::Update {
        percent: 10,
        text: "Initializing COM environment...".to_string(),
    });

    let result = (|| -> Result<(), String> {
        // Initialize COM apartment for this thread
        let _apartment = ComApartment::new()?;

        let tx_clone = tx.clone();
        let progress_cb = move |percent: u32, text: &str| {
            let _ = tx_clone.send(ProgressMessage::Update {
                percent,
                text: text.to_string(),
            });
        };

        match kind {
            DocumentKind::Word => {
                WordConverter::convert(&input_path, &temp_output, progress_cb)?;
            }
            DocumentKind::PowerPoint => {
                PowerPointConverter::convert(&input_path, &temp_output, progress_cb)?;
            }
            DocumentKind::Excel => {
                ExcelConverter::convert(&input_path, &temp_output, progress_cb)?;
            }
        }

        // Verify temporary output file exists and has content
        if !temp_output.exists() {
            return Err("Conversion engine completed without producing an output PDF".to_string());
        }

        let metadata = fs::metadata(&temp_output)
            .map_err(|e| format!("Could not inspect output file metadata: {}", e))?;
        if metadata.len() == 0 {
            return Err("Generated PDF file is empty (0 bytes)".to_string());
        }

        // Atomically replace destination file
        if output_path.exists() {
            let _ = fs::remove_file(&output_path);
        }

        fs::rename(&temp_output, &output_path)
            .map_err(|e| format!("Failed to move temporary PDF to destination: {}", e))?;

        Ok(())
    })();

    // Clean up temporary file if left behind due to an error
    if temp_output.exists() {
        let _ = fs::remove_file(&temp_output);
    }

    match result {
        Ok(_) => {
            let file_name = output_path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("document.pdf");
            let _ = tx.send(ProgressMessage::Finished {
                success: true,
                message: format!("Saved copy to {}", file_name),
                output_path: Some(output_path),
            });
        }
        Err(err) => {
            let _ = tx.send(ProgressMessage::Finished {
                success: false,
                message: err,
                output_path: None,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;

    #[test]
    fn test_document_kind_detection() {
        assert!(matches!(DocumentKind::from_path(Path::new("test.docx")), Ok(DocumentKind::Word)));
        assert!(matches!(DocumentKind::from_path(Path::new("test.doc")), Ok(DocumentKind::Word)));
        assert!(matches!(DocumentKind::from_path(Path::new("test.DOCX")), Ok(DocumentKind::Word)));
        assert!(matches!(DocumentKind::from_path(Path::new("test.pptx")), Ok(DocumentKind::PowerPoint)));
        assert!(matches!(DocumentKind::from_path(Path::new("test.ppt")), Ok(DocumentKind::PowerPoint)));
        assert!(matches!(DocumentKind::from_path(Path::new("test.xlsx")), Ok(DocumentKind::Excel)));
        assert!(matches!(DocumentKind::from_path(Path::new("test.xls")), Ok(DocumentKind::Excel)));
        assert!(DocumentKind::from_path(Path::new("test.pdf")).is_err());
        assert!(DocumentKind::from_path(Path::new("test.txt")).is_err());
    }

    #[test]
    fn test_determine_output_path_non_colliding() {
        let temp_dir = std::env::temp_dir().join(format!("documorph_test_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);

        let doc = temp_dir.join("sample.docx");
        let initial_pdf = temp_dir.join("sample.pdf");
        let _ = File::create(&initial_pdf);

        // When sample.pdf already exists and overwrite is false:
        let safe_path = determine_output_path(&doc, false);
        assert_eq!(safe_path, temp_dir.join("sample (1).pdf"));

        // When sample.pdf already exists and overwrite is true:
        let overwrite_path = determine_output_path(&doc, true);
        assert_eq!(overwrite_path, initial_pdf);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_live_word_conversion() {
        let (tx, rx) = std::sync::mpsc::channel();
        let input = PathBuf::from(r"F:\DocuMorph\samples\test_sample.docx");
        let output = PathBuf::from(r"F:\DocuMorph\samples\test_sample.pdf");
        process_conversion(input, output, tx);
        while let Ok(msg) = rx.recv() {
            match msg {
                ProgressMessage::Update { percent, text } => {
                    println!("Progress {}: {}", percent, text);
                }
                ProgressMessage::Finished { success, message, .. } => {
                    println!("Finished: success={}, message={}", success, message);
                    assert!(success, "Conversion failed: {}", message);
                    break;
                }
            }
        }
    }
}

