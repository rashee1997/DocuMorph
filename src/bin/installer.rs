#![windows_subsystem = "windows"]

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::Mutex;

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{COLOR_BTNFACE, DeleteObject, UpdateWindow};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
    GetSystemMetrics, GetWindowTextLengthW, GetWindowTextW, LoadIconW, MessageBoxW,
    PostQuitMessage, RegisterClassExW, SetWindowTextW, ShowWindow, TranslateMessage,
    CS_HREDRAW, CS_VREDRAW, IDI_APPLICATION, MB_ICONERROR, MB_ICONINFORMATION, MB_OK,
    SM_CXSCREEN, SM_CYSCREEN, SW_SHOW, WM_CLOSE, WM_COMMAND, WM_DESTROY,
    WNDCLASSEXW, WS_CAPTION, WS_EX_APPWINDOW, WS_OVERLAPPED, WS_SYSMENU,
};

use documorph::{com, registry, ui};

const DOCUMORPH_EXE_BYTES: &[u8] = include_bytes!("../../target/release/documorph.exe");
const UNINSTALLER_EXE_BYTES: &[u8] = include_bytes!("../../target/release/uninstaller.exe");
const ICON_BYTES: &[u8] = include_bytes!("../../resources/icon.ico");

const ID_EDIT_DIR: usize = 201;
const ID_BTN_BROWSE: usize = 202;
const ID_BTN_INSTALL: usize = 203;
const ID_BTN_CANCEL: usize = 204;

struct InstallerState {
    hwnd_edit_dir: HWND,
    hwnd_lbl_status: HWND,
    hwnd_btn_install: HWND,
    #[allow(dead_code)]
    hwnd_btn_cancel: HWND,
    fonts: ui::controls::ControlFonts,
    installed: bool,
}

unsafe impl Send for InstallerState {}
static INSTALLER_STATE: Mutex<Option<InstallerState>> = Mutex::new(None);

fn show_alert(title: &str, text: &str, is_error: bool) {
    let wide_title = com::to_wide(title);
    let wide_text = com::to_wide(text);
    let flags = MB_OK | if is_error { MB_ICONERROR } else { MB_ICONINFORMATION };
    unsafe {
        MessageBoxW(ptr::null_mut(), wide_text.as_ptr(), wide_title.as_ptr(), flags);
    }
}

fn get_edit_text(hwnd: HWND) -> String {
    unsafe {
        let len = GetWindowTextLengthW(hwnd) as usize;
        if len == 0 {
            return String::new();
        }
        let mut buf = vec![0u16; len + 1];
        GetWindowTextW(hwnd, buf.as_mut_ptr(), (len + 1) as i32);
        String::from_utf16_lossy(&buf[..len])
    }
}

pub fn perform_install(target_dir: &Path) -> Result<(), String> {
    if !target_dir.exists() {
        fs::create_dir_all(target_dir)
            .map_err(|e| format!("Could not create directory {}: {}", target_dir.display(), e))?;
    }

    // 1. Write documorph.exe
    let target_documorph = target_dir.join("documorph.exe");
    fs::write(&target_documorph, DOCUMORPH_EXE_BYTES)
        .map_err(|e| format!("Failed to write documorph.exe: {}", e))?;

    // 2. Write uninstall.exe
    let target_uninstaller = target_dir.join("uninstall.exe");
    fs::write(&target_uninstaller, UNINSTALLER_EXE_BYTES)
        .map_err(|e| format!("Failed to write uninstall.exe: {}", e))?;

    // 3. Write icon.ico
    let target_icon = target_dir.join("icon.ico");
    fs::write(&target_icon, ICON_BYTES)
        .map_err(|e| format!("Failed to write icon.ico: {}", e))?;

    // 4. Register Context Menu Verbs in HKCU
    registry::register_for_dir(target_dir)?;

    // 5. Register in Windows Settings Add or Remove Programs
    registry::register_uninstaller(target_dir)?;

    Ok(())
}

unsafe extern "system" fn installer_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_COMMAND => {
            let id = (wparam & 0xFFFF) as usize;
            match id {
                ID_BTN_BROWSE => {
                    if let Some(picked) = ui::dialogs::pick_folder(hwnd) {
                        if let Ok(guard) = INSTALLER_STATE.lock() {
                            if let Some(state) = guard.as_ref() {
                                let wide_path = com::to_wide(&picked.to_string_lossy());
                                SetWindowTextW(state.hwnd_edit_dir, wide_path.as_ptr());
                            }
                        }
                    }
                }
                ID_BTN_INSTALL => {
                    let mut is_installed = false;
                    if let Ok(guard) = INSTALLER_STATE.lock() {
                        if let Some(state) = guard.as_ref() {
                            is_installed = state.installed;
                        }
                    }

                    if is_installed {
                        // "Finish" clicked
                        DestroyWindow(hwnd);
                        return 0;
                    }

                    let target_str = if let Ok(guard) = INSTALLER_STATE.lock() {
                        if let Some(state) = guard.as_ref() {
                            get_edit_text(state.hwnd_edit_dir)
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    };

                    let target_path = PathBuf::from(target_str.trim());
                    if target_path.as_os_str().is_empty() {
                        show_alert("DocuMorph Setup", "Please select a valid installation directory.", true);
                        return 0;
                    }

                    match perform_install(&target_path) {
                        Ok(_) => {
                            if let Ok(mut guard) = INSTALLER_STATE.lock() {
                                if let Some(state) = guard.as_mut() {
                                    state.installed = true;
                                    let success_text = format!(
                                        "✓ Installed to {}!\nExplorer context menu registered.",
                                        target_path.display()
                                    );
                                    SetWindowTextW(state.hwnd_lbl_status, com::to_wide(&success_text).as_ptr());
                                    SetWindowTextW(state.hwnd_btn_install, com::to_wide("Finish").as_ptr());
                                }
                            }
                            show_alert(
                                "DocuMorph Setup",
                                &format!(
                                    "Installation Successful!\n\nDocuMorph has been installed to:\n{}\n\nYou can now right-click any Word, PowerPoint, or Excel file in Windows Explorer to convert it to PDF with 100% font fidelity.",
                                    target_path.display()
                                ),
                                false,
                            );
                        }
                        Err(e) => {
                            show_alert("DocuMorph Setup Error", &e, true);
                        }
                    }
                }
                ID_BTN_CANCEL => {
                    DestroyWindow(hwnd);
                }
                _ => {}
            }
            0
        }
        WM_CLOSE => {
            DestroyWindow(hwnd);
            0
        }
        WM_DESTROY => {
            if let Ok(mut guard) = INSTALLER_STATE.lock() {
                if let Some(state) = guard.take() {
                    DeleteObject(state.fonts.title);
                    DeleteObject(state.fonts.bold);
                    DeleteObject(state.fonts.normal);
                    DeleteObject(state.fonts.small);
                }
            }
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let is_silent = args.iter().any(|a| a == "--silent" || a == "-s" || a == "/S");

    let default_dest = PathBuf::from(r"C:\DocuMorph");

    if is_silent {
        let custom_dir = args.iter().skip(1).find(|a| !a.starts_with("-")).map(PathBuf::from);
        let dest = custom_dir.unwrap_or(default_dest);
        match perform_install(&dest) {
            Ok(_) => std::process::exit(0),
            Err(e) => {
                eprintln!("Silent install failed: {}", e);
                std::process::exit(1);
            }
        }
    }

    unsafe {
        let hinstance = GetModuleHandleW(ptr::null());
        let class_name = com::to_wide("DocuMorphInstallerWindow");

        let mut wc: WNDCLASSEXW = std::mem::zeroed();
        wc.cbSize = std::mem::size_of::<WNDCLASSEXW>() as u32;
        wc.style = CS_HREDRAW | CS_VREDRAW;
        wc.lpfnWndProc = Some(installer_wndproc);
        wc.hInstance = hinstance;
        wc.hIcon = LoadIconW(ptr::null_mut(), IDI_APPLICATION);
        wc.hCursor = ptr::null_mut();
        wc.hbrBackground = (COLOR_BTNFACE + 1) as windows_sys::Win32::Graphics::Gdi::HBRUSH;
        wc.lpszClassName = class_name.as_ptr();

        RegisterClassExW(&wc);

        let win_w = 540;
        let win_h = 360;
        let sx = GetSystemMetrics(SM_CXSCREEN);
        let sy = GetSystemMetrics(SM_CYSCREEN);
        let px = (sx - win_w) / 2;
        let py = (sy - win_h) / 2;

        let title = com::to_wide("DocuMorph Setup (v1.1.0)");
        let hwnd_main = CreateWindowExW(
            WS_EX_APPWINDOW,
            class_name.as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU,
            px, py, win_w, win_h,
            ptr::null_mut(),
            ptr::null_mut(),
            hinstance,
            ptr::null_mut(),
        );

        if hwnd_main.is_null() {
            return;
        }

        ui::controls::apply_window_icon(hwnd_main);
        let fonts = ui::controls::ControlFonts::new();

        // Banner Header
        ui::controls::create_label(hwnd_main, "Install DocuMorph", 24, 20, 260, 28, fonts.title);
        ui::controls::create_label(
            hwnd_main,
            "High-Fidelity Document to PDF Converter with TrueType Font Embedding",
            24, 52, 490, 20, fonts.normal,
        );

        // Groupbox Destination
        ui::controls::create_groupbox(hwnd_main, " Installation Destination ", 24, 85, 480, 85, fonts.bold);
        let default_dir_str = default_dest.to_string_lossy().to_string();
        let hwnd_edit_dir = ui::controls::create_edit(
            hwnd_main, &default_dir_str, ID_EDIT_DIR, 38, 120, 345, 24, fonts.normal, false,
        );
        let _ = ui::controls::create_button(
            hwnd_main, "Browse...", ID_BTN_BROWSE, 395, 120, 95, 24, fonts.normal,
        );

        // Feature highlights
        ui::controls::create_label(
            hwnd_main,
            "• Adds 'Convert to PDF with DocuMorph' to File Explorer right-click menu\n• Supports Word (.docx, .doc), PowerPoint (.pptx, .ppt), Excel (.xlsx, .xls)\n• Embeds genuine document fonts (zero fallback font substitution)\n• Adds entry to Windows Settings Add or Remove Programs with clean uninstaller",
            24, 185, 480, 75, fonts.normal,
        );

        // Status label
        let hwnd_lbl_status = ui::controls::create_label(
            hwnd_main, "Ready to install.", 24, 275, 480, 22, fonts.bold,
        );

        // Action Buttons
        let hwnd_btn_install = ui::controls::create_button(
            hwnd_main, "Install", ID_BTN_INSTALL, 290, 310, 105, 30, fonts.bold,
        );
        let hwnd_btn_cancel = ui::controls::create_button(
            hwnd_main, "Cancel", ID_BTN_CANCEL, 405, 310, 100, 30, fonts.normal,
        );

        if let Ok(mut guard) = INSTALLER_STATE.lock() {
            *guard = Some(InstallerState {
                hwnd_edit_dir,
                hwnd_lbl_status,
                hwnd_btn_install,
                hwnd_btn_cancel,
                fonts,
                installed: false,
            });
        }

        ShowWindow(hwnd_main, SW_SHOW);
        UpdateWindow(hwnd_main);

        let mut msg: windows_sys::Win32::UI::WindowsAndMessaging::MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
