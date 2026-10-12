use std::path::Path;
use super::{ComDispatch, IDispatch, ManagedBstr, ManagedVariant};

pub struct PowerPointConverter;

impl PowerPointConverter {
    pub fn convert<F>(
        input_path: &Path,
        output_path: &Path,
        mut progress: F,
    ) -> Result<(), String>
    where
        F: FnMut(u32, &str),
    {
        progress(15, "Starting Microsoft PowerPoint background engine...");

        let app = ComDispatch::create_instance("PowerPoint.Application")
            .map_err(|e| format!("Could not initialize PowerPoint: {}", e))?;

        // Disable macros unconditionally (msoAutomationSecurityForceDisable = 3)
        let _ = app.set_property("AutomationSecurity", ManagedVariant::from_i4(3));

        progress(30, "Accessing presentations collection...");

        let pres_var = app.get_property("Presentations")?;
        let pres_ptr = unsafe { pres_var.0.Anonymous.Anonymous.Anonymous.pdispVal as *mut IDispatch };
        if pres_ptr.is_null() {
            let _ = app.invoke_method("Quit", &mut []);
            return Err("Failed to retrieve PowerPoint Presentations collection".to_string());
        }
        let presentations = ComDispatch::from_raw(pres_ptr);

        let input_str = input_path
            .to_str()
            .ok_or_else(|| "Input path is not valid Unicode".to_string())?;
        let output_str = output_path
            .to_str()
            .ok_or_else(|| "Output path is not valid Unicode".to_string())?;

        let input_bstr = ManagedBstr::from_str(input_str);
        let output_bstr = ManagedBstr::from_str(output_str);

        progress(45, "Opening presentation slides...");

        // Presentations.Open(FileName, ReadOnly, Untitled, WithWindow)
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

        let pres_obj_ptr = unsafe { pres_obj_var.0.Anonymous.Anonymous.Anonymous.pdispVal as *mut IDispatch };
        if pres_obj_ptr.is_null() {
            let _ = app.invoke_method("Quit", &mut []);
            return Err("Failed to obtain Presentation object".to_string());
        }
        let presentation = ComDispatch::from_raw(pres_obj_ptr);

        progress(70, "Exporting slide deck to PDF copy...");

        // SaveAs(FileName, FileFormat = ppSaveAsPDF (32))
        let mut save_args = [
            ManagedVariant::from_bstr(output_bstr.0),
            ManagedVariant::from_i4(32), // ppSaveAsPDF = 32
        ];

        let save_result = presentation.invoke_method("SaveAs", &mut save_args);

        progress(90, "Finalizing presentation export...");

        let _ = presentation.invoke_method("Close", &mut []);
        let _ = app.invoke_method("Quit", &mut []);

        match save_result {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("PowerPoint export to PDF failed: {}", e)),
        }
    }
}
