use std::path::Path;
use super::{ComDispatch, ManagedBstr, ManagedVariant};
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
        let _ = app.set_property("ScreenUpdating", ManagedVariant::from_bool(false));
        let _ = app.set_property("PrintBackground", ManagedVariant::from_bool(false));

        // Set ActivePrinter to "Microsoft Print to PDF" to prevent layout reflow from missing/slow physical printers
        let printer_bstr = ManagedBstr::from_str("Microsoft Print to PDF");
        let _ = app.set_property("ActivePrinter", ManagedVariant::from_bstr(printer_bstr.0));

        progress(30, "Accessing document collection...");

        let docs = match app.get_dispatch_property("Documents") {
            Ok(d) => d,
            Err(e) => {
                let _ = app.invoke_method("Quit", &mut [ManagedVariant::from_i4(0)]);
                return Err(format!("Failed to retrieve Word Documents collection: {}", e));
            }
        };

        let input_str = input_path
            .to_str()
            .ok_or_else(|| "Input path is not valid Unicode".to_string())?;
        let output_str = output_path
            .to_str()
            .ok_or_else(|| "Output path is not valid Unicode".to_string())?;

        let input_bstr = ManagedBstr::from_str(input_str);
        let output_bstr = ManagedBstr::from_str(output_str);

        progress(45, "Opening document and embedding TrueType fonts...");

        // Documents.Open(FileName, ConfirmConversions=false, ReadOnly=true, AddToRecentFiles=false)
        let mut open_args = [
            ManagedVariant::from_bstr(input_bstr.0),
            ManagedVariant::from_bool(false), // ConfirmConversions
            ManagedVariant::from_bool(true),  // ReadOnly
            ManagedVariant::from_bool(false), // AddToRecentFiles
        ];

        let doc_var = match docs.invoke_method("Open", &mut open_args) {
            Ok(v) => v,
            Err(e) => {
                let _ = app.invoke_method("Quit", &mut [ManagedVariant::from_i4(0)]);
                return Err(format!("Word failed to open file '{}': {}", input_str, e));
            }
        };

        let doc = match doc_var.to_dispatch() {
            Ok(d) => d,
            Err(e) => {
                let _ = app.invoke_method("Quit", &mut [ManagedVariant::from_i4(0)]);
                return Err(format!("Failed to obtain Document dispatch object: {}", e));
            }
        };

        // Prevent template style overrides and enforce TrueType font embedding
        let _ = doc.set_property("UpdateStylesOnOpen", ManagedVariant::from_bool(false));
        let _ = doc.set_property("EmbedTrueTypeFonts", ManagedVariant::from_bool(true));
        let _ = doc.set_property("SaveSubsetFonts", ManagedVariant::from_bool(true));

        // Switch to Print Layout view (wdPrintView = 3) to ensure exact pagination boundaries
        if let Ok(active_window) = doc.get_dispatch_property("ActiveWindow") {
            if let Ok(view) = active_window.get_dispatch_property("View") {
                let _ = view.set_property("Type", ManagedVariant::from_i4(3)); // wdPrintView = 3
            }
        }

        progress(70, "Exporting fixed layout PDF copy...");

        // Document.ExportAsFixedFormat(
        //   OutputFileName, ExportFormat=17, OpenAfterExport=false,
        //   OptimizeFor=0, Range=0, From=1, To=1, Item=0, IncludeDocProps=true,
        //   KeepIRM=true, CreateBookmarks=1, DocStructureTags=true,
        //   BitmapMissingFonts=false, UseISO19005_1=use_iso
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
            ManagedVariant::from_bool(false),  // BitmapMissingFonts = false (vector TrueType fonts preserved)
            ManagedVariant::from_bool(use_iso), // UseISO19005_1 = true for PDF/A
        ];

        let export_result = doc.invoke_method("ExportAsFixedFormat", &mut export_args);

        progress(90, "Finalizing and closing document...");

        // Close document without saving changes: doc.Close(wdDoNotSaveChanges = 0)
        let mut close_args = [ManagedVariant::from_i4(0)];
        let _ = doc.invoke_method("Close", &mut close_args);

        // Terminate background Word application cleanly without saving Normal.dotm
        let _ = app.invoke_method("Quit", &mut [ManagedVariant::from_i4(0)]);

        match export_result {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Word export to PDF failed: {}", e)),
        }
    }
}
