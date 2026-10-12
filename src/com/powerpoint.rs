use std::path::Path;
use super::{ComDispatch, ManagedBstr, ManagedVariant};
use crate::options::PdfStandard;

pub struct PowerPointConverter;

impl PowerPointConverter {
    pub fn convert<F>(
        input_path: &Path,
        output_path: &Path,
        pdf_standard: PdfStandard,
        mut progress: F,
    ) -> Result<(), String>
    where
        F: FnMut(u32, &str),
    {
        progress(15, "Starting Microsoft PowerPoint background engine...");

        let app = ComDispatch::create_instance("PowerPoint.Application")
            .map_err(|e| format!("Could not initialize PowerPoint: {}", e))?;

        // Disable macros and alert dialogs (msoAutomationSecurityForceDisable = 3, ppAlertsNone = 1)
        let _ = app.set_property("AutomationSecurity", ManagedVariant::from_i4(3));
        let _ = app.set_property("DisplayAlerts", ManagedVariant::from_i4(1)); // ppAlertsNone = 1

        progress(30, "Accessing presentations collection...");

        let presentations = match app.get_dispatch_property("Presentations") {
            Ok(p) => p,
            Err(e) => {
                let _ = app.invoke_method("Quit", &mut []);
                return Err(format!("Failed to retrieve PowerPoint Presentations collection: {}", e));
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

        progress(45, "Opening presentation slides...");

        // Presentations.Open(FileName, ReadOnly = -1, Untitled = 0, WithWindow = 0)
        let mut open_args = [
            ManagedVariant::from_bstr(input_bstr.0),
            ManagedVariant::from_i4(-1), // ReadOnly = msoTrue (-1)
            ManagedVariant::from_i4(0),  // Untitled = msoFalse (0)
            ManagedVariant::from_i4(0),  // WithWindow = msoFalse (0)
        ];

        let pres_obj_var = match presentations.invoke_method("Open", &mut open_args) {
            Ok(v) => v,
            Err(e) => {
                let _ = app.invoke_method("Quit", &mut []);
                return Err(format!("PowerPoint failed to open file '{}': {}", input_str, e));
            }
        };

        let presentation = match pres_obj_var.to_dispatch() {
            Ok(p) => p,
            Err(e) => {
                let _ = app.invoke_method("Quit", &mut []);
                return Err(format!("Failed to obtain Presentation dispatch object: {}", e));
            }
        };

        progress(70, "Exporting slide deck to fixed layout PDF...");

        let use_iso = pdf_standard == PdfStandard::PdfA1a;
        let empty_bstr = ManagedBstr::from_str("");

        // Presentation.ExportAsFixedFormat(
        //   Path, FixedFormatType=2, Intent=1, FrameSlides=0, HandoutOrder=1, OutputType=1,
        //   PrintHiddenSlides=0, PrintRange=0, RangeType=1, SlideShowName="",
        //   IncludeDocProperties=true, KeepIRM=true, DocStructureTags=true,
        //   BitmapMissingFonts=false, UseISO19005_1=use_iso
        // )
        let mut export_args = [
            ManagedVariant::from_bstr(output_bstr.0), // Path
            ManagedVariant::from_i4(2),              // FixedFormatType = ppFixedFormatTypePDF (2)
            ManagedVariant::from_i4(1),              // Intent = ppFixedFormatIntentPrint (1)
            ManagedVariant::from_i4(0),              // FrameSlides = msoFalse (0)
            ManagedVariant::from_i4(1),              // HandoutOrder = ppPrintHandoutVerticalFirst (1)
            ManagedVariant::from_i4(1),              // OutputType = ppPrintOutputSlides (1)
            ManagedVariant::from_i4(0),              // PrintHiddenSlides = msoFalse (0)
            ManagedVariant::from_i4(0),              // PrintRange = 0
            ManagedVariant::from_i4(1),              // RangeType = ppPrintAll (1)
            ManagedVariant::from_bstr(empty_bstr.0),  // SlideShowName = ""
            ManagedVariant::from_bool(true),         // IncludeDocProperties = true
            ManagedVariant::from_bool(true),         // KeepIRM = true
            ManagedVariant::from_bool(true),         // DocStructureTags = true
            ManagedVariant::from_bool(false),        // BitmapMissingFonts = false
            ManagedVariant::from_bool(use_iso),      // UseISO19005_1 = true for PDF/A
        ];

        let export_result = presentation.invoke_method("ExportAsFixedFormat", &mut export_args);

        progress(90, "Finalizing presentation export...");

        let _ = presentation.invoke_method("Close", &mut []);
        let _ = app.invoke_method("Quit", &mut []);

        match export_result {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("PowerPoint export to PDF failed: {}", e)),
        }
    }
}
