use std::path::Path;
use super::{ComDispatch, ManagedBstr, ManagedVariant};
use crate::options::PdfStandard;

pub struct ExcelConverter;

impl ExcelConverter {
    pub fn convert<F>(
        input_path: &Path,
        output_path: &Path,
        _pdf_standard: PdfStandard,
        mut progress: F,
    ) -> Result<(), String>
    where
        F: FnMut(u32, &str),
    {
        progress(15, "Opening Excel...");

        let app = ComDispatch::create_instance("Excel.Application")
            .map_err(|e| format!("Could not initialize Excel: {}", e))?;

        // Disable macros, alerts, and background events to ensure fast, non-blocking execution
        let _ = app.set_property("AutomationSecurity", ManagedVariant::from_i4(3)); // msoAutomationSecurityForceDisable = 3
        let _ = app.set_property("Visible", ManagedVariant::from_bool(false));
        let _ = app.set_property("DisplayAlerts", ManagedVariant::from_bool(false));
        let _ = app.set_property("ScreenUpdating", ManagedVariant::from_bool(false));
        let _ = app.set_property("AskToUpdateLinks", ManagedVariant::from_bool(false));
        let _ = app.set_property("EnableEvents", ManagedVariant::from_bool(false));
        let _ = app.set_property("Calculation", ManagedVariant::from_i4(-4135)); // xlCalculationManual = -4135

        progress(30, "Opening workbook...");

        let workbooks = match app.get_dispatch_property("Workbooks") {
            Ok(wb) => wb,
            Err(e) => {
                let _ = app.invoke_method("Quit", &mut []);
                return Err(format!("Failed to retrieve Excel Workbooks collection: {}", e));
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

        progress(45, "Setting up sheets...");

        // Workbooks.Open(Filename, UpdateLinks = 0, ReadOnly = true)
        let mut open_args = [
            ManagedVariant::from_bstr(input_bstr.0),
            ManagedVariant::from_i4(0),       // UpdateLinks = 0
            ManagedVariant::from_bool(true),  // ReadOnly = true
        ];

        let wb_var = match workbooks.invoke_method("Open", &mut open_args) {
            Ok(v) => v,
            Err(e) => {
                let _ = app.invoke_method("Quit", &mut []);
                return Err(format!("Excel failed to open file '{}': {}", input_str, e));
            }
        };

        let workbook = match wb_var.to_dispatch() {
            Ok(wb) => wb,
            Err(e) => {
                let _ = app.invoke_method("Quit", &mut []);
                return Err(format!("Failed to obtain Workbook dispatch object: {}", e));
            }
        };

        // Query ActiveSheet and configure PageSetup to prevent horizontal splitting across pages
        if let Ok(sheet) = workbook.get_dispatch_property("ActiveSheet") {
            if let Ok(page_setup) = sheet.get_dispatch_property("PageSetup") {
                let _ = page_setup.set_property("Zoom", ManagedVariant::from_bool(false));
                let _ = page_setup.set_property("FitToPagesWide", ManagedVariant::from_i4(1));
                let _ = page_setup.set_property("FitToPagesTall", ManagedVariant::from_bool(false));
            }
        }

        progress(70, "Exporting PDF...");

        // Workbook.ExportAsFixedFormat(
        //   Type = xlTypePDF (0), Filename, Quality = xlQualityStandard (0),
        //   IncludeDocProperties = true, IgnorePrintAreas = false
        // )
        let mut export_args = [
            ManagedVariant::from_i4(0),              // xlTypePDF = 0
            ManagedVariant::from_bstr(output_bstr.0),
            ManagedVariant::from_i4(0),              // Quality = xlQualityStandard (0)
            ManagedVariant::from_bool(true),         // IncludeDocProperties = true
            ManagedVariant::from_bool(false),        // IgnorePrintAreas = false
        ];

        let export_result = workbook.invoke_method("ExportAsFixedFormat", &mut export_args);

        progress(90, "Closing workbook...");

        // Close workbook without saving changes
        let mut close_args = [ManagedVariant::from_bool(false)];
        let _ = workbook.invoke_method("Close", &mut close_args);
        let _ = app.invoke_method("Quit", &mut []);

        match export_result {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Excel export to PDF failed: {}", e)),
        }
    }
}
