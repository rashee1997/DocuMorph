use std::path::Path;
use super::{ComDispatch, IDispatch, ManagedBstr, ManagedVariant};
use crate::options::PdfStandard;

pub struct WordConverter;

impl WordConverter {
    pub fn convert<F>(
        input_path: &Path,
        output_path: &Path,
        pdf_standard: PdfStandard,
        mut progress: F,
    ) -> Result<(), String>
    where
        F: FnMut(u32, &str),
    {
        progress(15, "Starting Microsoft Word background engine...");

        let app = ComDispatch::create_instance("Word.Application")
            .map_err(|e| format!("Could not initialize Word: {}", e))?;

        // Disable macros unconditionally for security (msoAutomationSecurityForceDisable = 3)
        let _ = app.set_property("AutomationSecurity", ManagedVariant::from_i4(3));
        let _ = app.set_property("Visible", ManagedVariant::from_bool(false));
        let _ = app.set_property("DisplayAlerts", ManagedVariant::from_i4(0)); // wdAlertsNone = 0

        progress(30, "Accessing document collection...");

        let docs_var = app.get_property("Documents")?;
        let docs_ptr = unsafe { docs_var.0.Anonymous.Anonymous.Anonymous.pdispVal as *mut IDispatch };
        if docs_ptr.is_null() {
            let _ = app.invoke_method("Quit", &mut []);
            return Err("Failed to retrieve Word Documents collection".to_string());
        }
        let docs = ComDispatch::from_raw(docs_ptr);

        let input_str = input_path
            .to_str()
            .ok_or_else(|| "Input path is not valid Unicode".to_string())?;
        let output_str = output_path
            .to_str()
            .ok_or_else(|| "Output path is not valid Unicode".to_string())?;

        let input_bstr = ManagedBstr::from_str(input_str);
        let output_bstr = ManagedBstr::from_str(output_str);

        progress(45, "Opening document and embedding TrueType fonts...");

        // Documents.Open(FileName, ConfirmConversions, ReadOnly, AddToRecentFiles)
        let mut open_args = [
            ManagedVariant::from_bstr(input_bstr.0),
            ManagedVariant::from_bool(false), // ConfirmConversions
            ManagedVariant::from_bool(true),  // ReadOnly
            ManagedVariant::from_bool(false), // AddToRecentFiles
        ];

        let doc_var = match docs.invoke_method("Open", &mut open_args) {
            Ok(v) => v,
            Err(e) => {
                let _ = app.invoke_method("Quit", &mut []);
                return Err(format!("Word failed to open file '{}': {}", input_str, e));
            }
        };

        let doc_ptr = unsafe { doc_var.0.Anonymous.Anonymous.Anonymous.pdispVal as *mut IDispatch };
        if doc_ptr.is_null() {
            let _ = app.invoke_method("Quit", &mut []);
            return Err("Failed to obtain Document object".to_string());
        }
        let doc = ComDispatch::from_raw(doc_ptr);

        // Enforce TrueType font embedding in the document export
        let _ = doc.set_property("EmbedTrueTypeFonts", ManagedVariant::from_bool(true));
        let _ = doc.set_property("SaveSubsetFonts", ManagedVariant::from_bool(true));

        progress(70, "Exporting fixed layout PDF copy...");

        // Document.ExportAsFixedFormat(
        //   OutputFileName, ExportFormat=17, OpenAfterExport=false,
        //   OptimizeFor=0, Range=0, From=1, To=1, Item=0, IncludeDocProps=true,
        //   KeepIRM=true, CreateBookmarks=1, DocStructureTags=true,
        //   BitmapMissingFonts=true, UseISO19005_1=false
        // )
        let use_iso = pdf_standard == PdfStandard::PdfA1a;
        let mut export_args = [
            ManagedVariant::from_bstr(output_bstr.0),
            ManagedVariant::from_i4(17),       // wdExportFormatPDF = 17
            ManagedVariant::from_bool(false),  // OpenAfterExport = false
            ManagedVariant::from_i4(0),        // OptimizeFor = wdExportOptimizeForPrint (0)
            ManagedVariant::from_i4(0),        // Range = wdExportAllDocument (0)
            ManagedVariant::from_i4(1),        // From = 1
            ManagedVariant::from_i4(1),        // To = 1
            ManagedVariant::from_i4(0),        // Item = wdExportDocumentContent (0)
            ManagedVariant::from_bool(true),   // IncludeDocProps = true
            ManagedVariant::from_bool(true),   // KeepIRM = true
            ManagedVariant::from_i4(1),        // CreateBookmarks = wdExportCreateHeadingBookmarks (1)
            ManagedVariant::from_bool(true),   // DocStructureTags = true
            ManagedVariant::from_bool(true),   // BitmapMissingFonts = true
            ManagedVariant::from_bool(use_iso), // UseISO19005_1 = true for PDF/A
        ];

        let export_result = doc.invoke_method("ExportAsFixedFormat", &mut export_args);

        progress(90, "Finalizing and closing document...");

        // Close document without saving changes: doc.Close(wdDoNotSaveChanges = 0)
        let mut close_args = [ManagedVariant::from_i4(0)];
        let _ = doc.invoke_method("Close", &mut close_args);

        // Terminate background Word application
        let _ = app.invoke_method("Quit", &mut []);

        match export_result {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Word export to PDF failed: {}", e)),
        }
    }
}
