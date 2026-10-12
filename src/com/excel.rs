use std::path::Path;
use super::{ComDispatch, IDispatch, ManagedBstr, ManagedVariant};

pub struct ExcelConverter;

impl ExcelConverter {
    pub fn convert<F>(
        input_path: &Path,
        output_path: &Path,
        mut progress: F,
    ) -> Result<(), String>
    where
        F: FnMut(u32, &str),
    {
        progress(15, "Starting Microsoft Excel background engine...");

        let app = ComDispatch::create_instance("Excel.Application")
            .map_err(|e| format!("Could not initialize Excel: {}", e))?;

        // Disable macros unconditionally (msoAutomationSecurityForceDisable = 3)
        let _ = app.set_property("AutomationSecurity", ManagedVariant::from_i4(3));
        let _ = app.set_property("Visible", ManagedVariant::from_bool(false));
        let _ = app.set_property("DisplayAlerts", ManagedVariant::from_bool(false));
        let _ = app.set_property("ScreenUpdating", ManagedVariant::from_bool(false));

        progress(30, "Accessing workbooks collection...");

        let wb_collection_var = app.get_property("Workbooks")?;
        let wb_collection_ptr = unsafe {
            wb_collection_var.0.Anonymous.Anonymous.Anonymous.pdispVal as *mut IDispatch
        };
        if wb_collection_ptr.is_null() {
            let _ = app.invoke_method("Quit", &mut []);
            return Err("Failed to retrieve Excel Workbooks collection".to_string());
        }
        let workbooks = ComDispatch::from_raw(wb_collection_ptr);

        let input_str = input_path
            .to_str()
            .ok_or_else(|| "Input path is not valid Unicode".to_string())?;
        let output_str = output_path
            .to_str()
            .ok_or_else(|| "Output path is not valid Unicode".to_string())?;

        let input_bstr = ManagedBstr::from_str(input_str);
        let output_bstr = ManagedBstr::from_str(output_str);

        progress(45, "Opening workbook sheets...");

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

        let wb_ptr = unsafe { wb_var.0.Anonymous.Anonymous.Anonymous.pdispVal as *mut IDispatch };
        if wb_ptr.is_null() {
            let _ = app.invoke_method("Quit", &mut []);
            return Err("Failed to obtain Workbook object".to_string());
        }
        let workbook = ComDispatch::from_raw(wb_ptr);

        progress(70, "Exporting spreadsheet to PDF copy...");

        // Workbook.ExportAsFixedFormat(
        //   Type = xlTypePDF (0), Filename, Quality = 0,
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

        progress(90, "Finalizing spreadsheet export...");

        // Close workbook: SaveChanges = false
        let mut close_args = [ManagedVariant::from_bool(false)];
        let _ = workbook.invoke_method("Close", &mut close_args);
        let _ = app.invoke_method("Quit", &mut []);

        match export_result {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Excel export to PDF failed: {}", e)),
        }
    }
}
