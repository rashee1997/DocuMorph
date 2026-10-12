#![windows_subsystem = "windows"]

use std::env;
use std::fs;
use std::path::PathBuf;
use std::ptr;

use windows_sys::Win32::System::Console::{
    AttachConsole, GetStdHandle, ATTACH_PARENT_PROCESS, STD_ERROR_HANDLE,
    STD_OUTPUT_HANDLE,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    MessageBoxW, MB_ICONERROR, MB_ICONINFORMATION, MB_OK,
};

use documorph::{com, ipc, registry, ui};

use windows_sys::Win32::Storage::FileSystem::WriteFile;

fn console_print(msg: &str) {
    unsafe {
        let _ = AttachConsole(ATTACH_PARENT_PROCESS);
        let h_out = GetStdHandle(STD_OUTPUT_HANDLE);
        if !h_out.is_null() && h_out != (!0usize) as *mut std::ffi::c_void {
            let bytes = format!("{}\r\n", msg);
            let mut written = 0u32;
            WriteFile(
                h_out,
                bytes.as_ptr(),
                bytes.len() as u32,
                &mut written,
                ptr::null_mut(),
            );
        }
    }
}

fn console_eprint(msg: &str) {
    unsafe {
        let _ = AttachConsole(ATTACH_PARENT_PROCESS);
        let h_err = GetStdHandle(STD_ERROR_HANDLE);
        if !h_err.is_null() && h_err != (!0usize) as *mut std::ffi::c_void {
            let bytes = format!("{}\r\n", msg);
            let mut written = 0u32;
            WriteFile(
                h_err,
                bytes.as_ptr(),
                bytes.len() as u32,
                &mut written,
                ptr::null_mut(),
            );
        }
    }
}

fn show_alert(title: &str, message: &str, is_error: bool) {
    let wide_title = com::to_wide(title);
    let wide_msg = com::to_wide(message);
    let flags = MB_OK | if is_error { MB_ICONERROR } else { MB_ICONINFORMATION };
    unsafe {
        MessageBoxW(ptr::null_mut(), wide_msg.as_ptr(), wide_title.as_ptr(), flags);
    }
}

fn print_help() {
    console_print(
        r#"DocuMorph v1.2.0
High-fidelity Windows context-menu document-to-PDF engine with TrueType font embedding

USAGE:
    documorph.exe [OPTIONS] [FILE_PATHS...]

ARGS:
    <FILE_PATHS...> Documents (.docx, .doc, .pptx, .ppt, .xlsx, .xls) to convert

OPTIONS:
    --register       Register DocuMorph in the Windows File Explorer context menu (HKCU)
    --unregister     Remove DocuMorph from the Windows context menu
    --status         Check registry status and Microsoft Office availability
    --help, -h       Show this help message
    --version, -v    Show version information

EXAMPLES:
    documorph.exe --register
    documorph.exe "C:\Users\User\Documents\Report.docx"
    documorph.exe "Doc1.docx" "Doc2.pptx" "Doc3.xlsx""#,
    );
}

fn print_version() {
    console_print("DocuMorph v1.2.0");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let is_terminal = unsafe { AttachConsole(ATTACH_PARENT_PROCESS) != 0 };

    // Process CLI maintenance flags
    if args.len() > 1 {
        match args[1].as_str() {
            "--register" => {
                match registry::register() {
                    Ok(_) => {
                        console_print("Success: DocuMorph context menu registered for Word, PowerPoint, and Excel.");
                        if !is_terminal {
                            show_alert(
                                "DocuMorph",
                                "Context menu registered successfully!\nYou can now right-click Word, PowerPoint, and Excel files to convert them to PDF.",
                                false,
                            );
                        }
                    }
                    Err(e) => {
                        console_eprint(&format!("Error registering context menu: {}", e));
                        if !is_terminal {
                            show_alert("DocuMorph Registration Error", &e, true);
                        }
                    }
                }
                return;
            }
            "--unregister" => {
                match registry::unregister() {
                    Ok(_) => {
                        console_print("Success: DocuMorph context menu removed from the Windows registry.");
                        if !is_terminal {
                            show_alert("DocuMorph", "Context menu entries have been removed.", false);
                        }
                    }
                    Err(e) => {
                        console_eprint(&format!("Error unregistering context menu: {}", e));
                        if !is_terminal {
                            show_alert("DocuMorph Error", &e, true);
                        }
                    }
                }
                return;
            }
            "--status" => {
                match registry::status() {
                    Ok(rep) => {
                        console_print(&rep);
                    }
                    Err(e) => {
                        console_eprint(&format!("Error inspecting status: {}", e));
                    }
                }
                return;
            }
            "--help" | "-h" => {
                print_help();
                return;
            }
            "--version" | "-v" => {
                print_version();
                return;
            }
            _ => {}
        }
    }

    // Collect all input files passed as arguments
    let mut input_files: Vec<PathBuf> = Vec::new();
    for arg in args.iter().skip(1) {
        if !arg.starts_with("--") {
            let p = PathBuf::from(arg);
            let canonical = fs::canonicalize(&p).unwrap_or(p);
            let s = canonical.to_string_lossy().to_string();
            let clean = if s.starts_with(r"\\?\") {
                PathBuf::from(&s[4..])
            } else {
                canonical
            };
            if clean.exists() && clean.is_file() {
                input_files.push(clean);
            }
        }
    }

    // Single-instance check:
    // If another DocuMorph window is already open, pipe the first file to it and exit immediately!
    match ipc::check_single_instance(input_files.first()) {
        ipc::IpcMode::Secondary => {
            return;
        }
        ipc::IpcMode::Primary(_mutex) => {
            // We are the primary instance! Launch the GUI options dialog
            if let Err(e) = ui::show_main_dialog(input_files) {
                show_alert("DocuMorph Execution Error", &e, true);
            }
        }
    }
}
