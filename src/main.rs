#![windows_subsystem = "windows"]

use std::env;
use std::fs;
use std::path::PathBuf;
use std::ptr;

use windows_sys::Win32::System::Console::{
    AttachConsole, GetStdHandle, SetStdHandle, ATTACH_PARENT_PROCESS, STD_ERROR_HANDLE,
    STD_OUTPUT_HANDLE,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    MessageBoxW, MB_ICONERROR, MB_ICONINFORMATION, MB_OK,
};

mod app;
mod com;
mod registry;
mod ui;

fn attach_parent_console() {
    unsafe {
        if AttachConsole(ATTACH_PARENT_PROCESS) != 0 {
            let h_out = GetStdHandle(STD_OUTPUT_HANDLE);
            if !h_out.is_null() && h_out != (!0usize) as *mut std::ffi::c_void {
                SetStdHandle(STD_OUTPUT_HANDLE, h_out);
            }
            let h_err = GetStdHandle(STD_ERROR_HANDLE);
            if !h_err.is_null() && h_err != (!0usize) as *mut std::ffi::c_void {
                SetStdHandle(STD_ERROR_HANDLE, h_err);
            }
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
    attach_parent_console();
    println!(
        r#"DocuMorph v1.0.0
High-fidelity Windows context-menu document-to-PDF engine

USAGE:
    documorph.exe [OPTIONS] [FILE_PATH]

ARGS:
    <FILE_PATH>    Path to .docx, .doc, .pptx, .ppt, .xlsx, or .xls file to convert

OPTIONS:
    --register     Register DocuMorph in the Windows File Explorer context menu (HKCU)
    --unregister   Remove DocuMorph from the Windows context menu
    --status       Check registry status and Microsoft Office availability
    --overwrite    Overwrite existing PDF instead of creating a numbered copy
    --help, -h     Show this help message
    --version, -v  Show version information

EXAMPLES:
    documorph.exe --register
    documorph.exe "C:\Users\User\Documents\Report.docx"
    documorph.exe --overwrite "C:\Users\User\Documents\Slides.pptx"
"#
    );
}

fn print_version() {
    attach_parent_console();
    println!("DocuMorph v1.0.0");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let is_terminal = unsafe { AttachConsole(ATTACH_PARENT_PROCESS) != 0 };

    // Check for CLI flags
    if args.len() > 1 {
        match args[1].as_str() {
            "--register" => {
                attach_parent_console();
                match registry::register() {
                    Ok(_) => {
                        println!("Success: DocuMorph context menu registered for Word, PowerPoint, and Excel.");
                        if !is_terminal {
                            show_alert("DocuMorph", "Context menu registered successfully!\nYou can now right-click Word, PowerPoint, and Excel files to convert them to PDF.", false);
                        }
                    }
                    Err(e) => {
                        eprintln!("Error registering context menu: {}", e);
                        if !is_terminal {
                            show_alert("DocuMorph Registration Error", &e, true);
                        }
                    }
                }
                return;
            }
            "--unregister" => {
                attach_parent_console();
                match registry::unregister() {
                    Ok(_) => {
                        println!("Success: DocuMorph context menu removed from the Windows registry.");
                        if !is_terminal {
                            show_alert("DocuMorph", "Context menu entries have been removed.", false);
                        }
                    }
                    Err(e) => {
                        eprintln!("Error unregistering context menu: {}", e);
                        if !is_terminal {
                            show_alert("DocuMorph Error", &e, true);
                        }
                    }
                }
                return;
            }
            "--status" => {
                attach_parent_console();
                match registry::status() {
                    Ok(rep) => {
                        println!("{}", rep);
                    }
                    Err(e) => {
                        eprintln!("Error inspecting status: {}", e);
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

    // Determine target file and overwrite preference
    let mut target_file: Option<PathBuf> = None;
    let mut overwrite = false;

    for arg in args.iter().skip(1) {
        if arg == "--overwrite" {
            overwrite = true;
        } else if !arg.starts_with("--") && target_file.is_none() {
            target_file = Some(PathBuf::from(arg));
        }
    }

    let input_path = match target_file {
        Some(p) => p,
        None => {
            print_help();
            return;
        }
    };

    // Canonicalize and validate source file
    let canonical_input = match fs::canonicalize(&input_path) {
        Ok(p) => {
            // Strip extended path prefix \\?\ if present for COM compatibility
            let s = p.to_string_lossy();
            if s.starts_with(r"\\?\") {
                PathBuf::from(&s[4..])
            } else {
                p
            }
        }
        Err(_) => input_path.clone(),
    };

    if !canonical_input.exists() {
        show_alert(
            "DocuMorph Error",
            &format!("Source file does not exist:\n{}", canonical_input.display()),
            true,
        );
        return;
    }

    if !canonical_input.is_file() {
        show_alert(
            "DocuMorph Error",
            &format!("Specified path is a directory, not a document:\n{}", canonical_input.display()),
            true,
        );
        return;
    }

    // Determine output destination
    let output_path = app::determine_output_path(&canonical_input, overwrite);

    let worker_input = canonical_input.clone();
    let worker_output = output_path.clone();

    // Show native live progress dialog and run conversion
    if let Err(e) = ui::show_conversion_dialog(&canonical_input, &output_path, move |tx| {
        app::process_conversion(worker_input, worker_output, tx);
    }) {
        show_alert("DocuMorph Execution Error", &e, true);
    }
}
