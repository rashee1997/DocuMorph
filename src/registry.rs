use std::env;
use std::path::PathBuf;
use std::ptr;
use windows_sys::Win32::Foundation::{ERROR_SUCCESS, S_OK};
use windows_sys::Win32::System::Com::CLSIDFromProgID;
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteTreeW, RegOpenKeyExW,
    RegSetValueExW, HKEY, HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_OPTION_NON_VOLATILE,
    REG_SZ,
};

use crate::com::to_wide;

pub const SUPPORTED_EXTENSIONS: &[&str] = &[".docx", ".doc", ".pptx", ".ppt", ".xlsx", ".xls"];
pub const VERB_KEY: &str = "DocuMorph.ConvertToPdf";
pub const VERB_MENU_TEXT: &str = "Convert to PDF with DocuMorph";

fn set_registry_string(hkey: HKEY, value_name: Option<&str>, data: &str) -> Result<(), String> {
    unsafe {
        let wide_val = value_name.map(to_wide);
        let val_ptr = match &wide_val {
            Some(v) => v.as_ptr(),
            None => ptr::null(),
        };

        let wide_data = to_wide(data);
        let byte_len = (wide_data.len() * std::mem::size_of::<u16>()) as u32;

        let status = RegSetValueExW(
            hkey,
            val_ptr,
            0,
            REG_SZ,
            wide_data.as_ptr() as *const u8,
            byte_len,
        );

        if status != ERROR_SUCCESS {
            return Err(format!("RegSetValueExW failed with code {}", status));
        }
        Ok(())
    }
}

/// Registers DocuMorph in the Windows Explorer context menu for all supported Office formats
pub fn register() -> Result<(), String> {
    let exe_path = env::current_exe()
        .map_err(|e| format!("Could not determine executable path: {}", e))?;
    let exe_str = exe_path
        .to_str()
        .ok_or_else(|| "Executable path contains invalid characters".to_string())?;

    let icon_str = format!("\"{}\",0", exe_str);
    let command_str = format!("\"{}\" \"%1\"", exe_str);

    for ext in SUPPORTED_EXTENSIONS {
        let base_sub_key = format!(r"Software\Classes\SystemFileAssociations\{}\shell\{}", ext, VERB_KEY);
        let wide_base = to_wide(&base_sub_key);

        unsafe {
            let mut hkey_base: HKEY = ptr::null_mut();
            let status = RegCreateKeyExW(
                HKEY_CURRENT_USER,
                wide_base.as_ptr(),
                0,
                ptr::null_mut(),
                REG_OPTION_NON_VOLATILE,
                KEY_WRITE,
                ptr::null_mut(),
                &mut hkey_base,
                ptr::null_mut(),
            );

            if status != ERROR_SUCCESS {
                return Err(format!("Failed to create registry key for {}: code {}", ext, status));
            }

            // Set menu item display name: (Default) = "Convert to PDF with DocuMorph"
            set_registry_string(hkey_base, None, VERB_MENU_TEXT)?;
            // Set menu icon: Icon = "<exe_path>,0"
            set_registry_string(hkey_base, Some("Icon"), &icon_str)?;

            // Create command subkey
            let cmd_sub_key = format!(r"{}\command", base_sub_key);
            let wide_cmd = to_wide(&cmd_sub_key);
            let mut hkey_cmd: HKEY = ptr::null_mut();

            let status_cmd = RegCreateKeyExW(
                HKEY_CURRENT_USER,
                wide_cmd.as_ptr(),
                0,
                ptr::null_mut(),
                REG_OPTION_NON_VOLATILE,
                KEY_WRITE,
                ptr::null_mut(),
                &mut hkey_cmd,
                ptr::null_mut(),
            );

            if status_cmd != ERROR_SUCCESS {
                RegCloseKey(hkey_base);
                return Err(format!("Failed to create command key for {}: code {}", ext, status_cmd));
            }

            // Set command action: (Default) = "<exe_path>" "%1"
            set_registry_string(hkey_cmd, None, &command_str)?;

            RegCloseKey(hkey_cmd);
            RegCloseKey(hkey_base);
        }
    }

    Ok(())
}

/// Removes DocuMorph context menu entries from the Windows Registry
pub fn unregister() -> Result<(), String> {
    for ext in SUPPORTED_EXTENSIONS {
        let base_sub_key = format!(r"Software\Classes\SystemFileAssociations\{}\shell\{}", ext, VERB_KEY);
        let wide_base = to_wide(&base_sub_key);

        unsafe {
            let _ = RegDeleteTreeW(HKEY_CURRENT_USER, wide_base.as_ptr());
        }
    }
    Ok(())
}

fn check_progid(prog_id: &str) -> bool {
    unsafe {
        let wide = to_wide(prog_id);
        let mut clsid = std::mem::zeroed();
        CLSIDFromProgID(wide.as_ptr(), &mut clsid) == S_OK
    }
}

/// Checks context menu registration status and Microsoft Office availability
pub fn status() -> Result<String, String> {
    let mut report = String::new();
    report.push_str("DocuMorph System Diagnostics\n");
    report.push_str("============================\n\n");

    // Check executable
    let current_exe = env::current_exe().unwrap_or_else(|_| PathBuf::from("documorph.exe"));
    report.push_str(&format!("Binary Path : {}\n", current_exe.display()));

    // Check Office COM engines
    let word_ok = check_progid("Word.Application");
    let ppt_ok = check_progid("PowerPoint.Application");
    let excel_ok = check_progid("Excel.Application");

    report.push_str(&format!(
        "Word Engine : {}\n",
        if word_ok { "Available (Word.Application)" } else { "Not Found" }
    ));
    report.push_str(&format!(
        "PPT Engine  : {}\n",
        if ppt_ok { "Available (PowerPoint.Application)" } else { "Not Found" }
    ));
    report.push_str(&format!(
        "Excel Engine: {}\n\n",
        if excel_ok { "Available (Excel.Application)" } else { "Not Found" }
    ));

    // Check registry integration
    report.push_str("Context Menu Associations (HKCU):\n");
    let mut all_registered = true;

    for ext in SUPPORTED_EXTENSIONS {
        let base_sub_key = format!(r"Software\Classes\SystemFileAssociations\{}\shell\{}\command", ext, VERB_KEY);
        let wide_base = to_wide(&base_sub_key);

        unsafe {
            let mut hkey: HKEY = ptr::null_mut();
            let res = RegOpenKeyExW(HKEY_CURRENT_USER, wide_base.as_ptr(), 0, KEY_READ, &mut hkey);
            if res == ERROR_SUCCESS {
                report.push_str(&format!("  {} -> Registered\n", ext));
                RegCloseKey(hkey);
            } else {
                report.push_str(&format!("  {} -> Not Registered\n", ext));
                all_registered = false;
            }
        }
    }

    report.push_str(&format!(
        "\nOverall Shell Status: {}\n",
        if all_registered {
            "Active (Ready for right-click conversion)"
        } else {
            "Incomplete or Not Installed (Run with --register to enable)"
        }
    ));

    Ok(report)
}
