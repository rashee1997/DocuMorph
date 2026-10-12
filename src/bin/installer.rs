#![windows_subsystem = "windows"]

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::Mutex;
use std::thread;

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, DeleteObject, EndPaint, FillRect, InvalidateRect, ScreenToClient,
    SetBkMode, SetTextColor, UpdateWindow, HDC, PAINTSTRUCT, TRANSPARENT,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Controls::DRAWITEMSTRUCT;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetClientRect,
    GetMessageW, GetSystemMetrics, GetWindowRect, GetWindowTextLengthW, GetWindowTextW,
    LoadIconW, MessageBoxW, PostMessageW, PostQuitMessage, RegisterClassExW, SetWindowTextW,
    ShowWindow, TranslateMessage, CS_HREDRAW, CS_VREDRAW, IDI_APPLICATION, MB_ICONERROR,
    MB_ICONINFORMATION, MB_OK, SM_CXSCREEN, SM_CYSCREEN, SW_SHOW, WM_APP, WM_CLOSE,
    WM_COMMAND, WM_CTLCOLORSTATIC, WM_DESTROY, WM_DRAWITEM, WM_ERASEBKGND, WM_PAINT,
    WNDCLASSEXW, WS_CAPTION, WS_EX_APPWINDOW, WS_EX_COMPOSITED, WS_OVERLAPPED, WS_SYSMENU,
};

use documorph::ui::fluent::{
    apply_fluent_window_frame, draw_card, draw_fluent_button, rgb, FluentTheme,
    COLOR_TEXT_MAIN_RGB,
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
    theme: FluentTheme,
    installed: bool,
}

unsafe impl Send for InstallerState {}
static INSTALLER_STATE: Mutex<Option<InstallerState>> = Mutex::new(None);
const WM_APP_INSTALL_RESULT: u32 = WM_APP + 10;

fn show_alert(hwnd: HWND, title: &str, text: &str, is_error: bool) {
    let flags = MB_OK | if is_error { MB_ICONERROR } else { MB_ICONINFORMATION };
    unsafe { MessageBoxW(hwnd, com::to_wide(text).as_ptr(), com::to_wide(title).as_ptr(), flags); }
}

fn get_edit_text(hwnd: HWND) -> String {
    unsafe {
        let len = GetWindowTextLengthW(hwnd) as usize;
        if len == 0 { return String::new(); }
        let mut buf = vec![0u16; len + 1];
        GetWindowTextW(hwnd, buf.as_mut_ptr(), (len + 1) as i32);
        String::from_utf16_lossy(&buf[..len])
    }
}

pub fn perform_install(target_dir: &Path) -> Result<(), String> {
    if !target_dir.exists() {
        fs::create_dir_all(target_dir).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                "Access denied. Run installer as Administrator to install to Program Files, or select another folder.".to_string()
            } else {
                format!("Could not create dir: {}", e)
            }
        })?;
    }
    fs::write(target_dir.join("documorph.exe"), DOCUMORPH_EXE_BYTES).map_err(|e| format!("Write failed: {}", e))?;
    fs::write(target_dir.join("uninstall.exe"), UNINSTALLER_EXE_BYTES).map_err(|e| format!("Write failed: {}", e))?;
    fs::write(target_dir.join("icon.ico"), ICON_BYTES).map_err(|e| format!("Write failed: {}", e))?;
    registry::register_for_dir(target_dir)?;
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
        WM_ERASEBKGND => {
            let hdc = wparam as HDC;
            let mut rc: RECT = std::mem::zeroed();
            GetClientRect(hwnd, &mut rc);
            if let Ok(guard) = INSTALLER_STATE.lock() {
                if let Some(state) = guard.as_ref() {
                    FillRect(hdc, &rc, state.theme.canvas_brush);
                }
            }
            1
        }
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);
            if let Ok(guard) = INSTALLER_STATE.lock() {
                if let Some(state) = guard.as_ref() {
                    draw_card(hdc, &RECT { left: 24, top: 78, right: 580, bottom: 165 }, &state.theme);
                    draw_card(hdc, &RECT { left: 24, top: 178, right: 580, bottom: 350 }, &state.theme);
                }
            }
            EndPaint(hwnd, &ps);
            0
        }
        WM_CTLCOLORSTATIC => {
            let hdc = wparam as HDC;
            SetBkMode(hdc, TRANSPARENT as i32);
            SetTextColor(hdc, rgb(COLOR_TEXT_MAIN_RGB.0, COLOR_TEXT_MAIN_RGB.1, COLOR_TEXT_MAIN_RGB.2));
            if let Ok(guard) = INSTALLER_STATE.lock() {
                if let Some(state) = guard.as_ref() {
                    let mut rect: RECT = std::mem::zeroed();
                    GetWindowRect(lparam as HWND, &mut rect);
                    let mut pt = POINT { x: rect.left, y: rect.top };
                    ScreenToClient(hwnd, &mut pt);
                    return if pt.y >= 70 && pt.y <= 360 { state.theme.card_brush as isize } else { state.theme.canvas_brush as isize };
                }
            }
            0
        }
        WM_DRAWITEM => {
            let dis = &*(lparam as *const DRAWITEMSTRUCT);
            if let Ok(guard) = INSTALLER_STATE.lock() {
                if let Some(state) = guard.as_ref() {
                    match dis.CtlID as usize {
                        ID_BTN_INSTALL => draw_fluent_button(dis, &state.theme, if state.installed { "Close" } else { "Install" }, true),
                        ID_BTN_CANCEL => draw_fluent_button(dis, &state.theme, "Cancel", false),
                        ID_BTN_BROWSE => draw_fluent_button(dis, &state.theme, "Browse...", false),
                        _ => {}
                    }
                }
            }
            1
        }
        WM_APP_INSTALL_RESULT => {
            let success = wparam == 1;
            if success {
                let target_str = if let Ok(mut guard) = INSTALLER_STATE.lock() {
                    if let Some(state) = guard.as_mut() {
                        state.installed = true;
                        let dir = get_edit_text(state.hwnd_edit_dir);
                        let success_text = format!("Installed to {}.", dir);
                        SetWindowTextW(state.hwnd_lbl_status, com::to_wide(&success_text).as_ptr());
                        EnableWindow(state.hwnd_btn_install, 1);
                        InvalidateRect(state.hwnd_btn_install, ptr::null(), 1);
                        dir
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                };
                show_alert(
                    hwnd,
                    "DocuMorph Setup",
                    &format!(
                        "Installed to:\n{}\n\nRight-click any Word, PowerPoint, or Excel file in File Explorer to convert to PDF.",
                        target_str
                    ),
                    false,
                );
            } else {
                let err_ptr = lparam as *mut String;
                let err_msg = if !err_ptr.is_null() {
                    *unsafe { Box::from_raw(err_ptr) }
                } else {
                    "Installation failed.".to_string()
                };
                if let Ok(guard) = INSTALLER_STATE.lock() {
                    if let Some(state) = guard.as_ref() {
                        SetWindowTextW(state.hwnd_lbl_status, com::to_wide("Installation failed.").as_ptr());
                        EnableWindow(state.hwnd_btn_install, 1);
                        InvalidateRect(state.hwnd_btn_install, ptr::null(), 1);
                    }
                }
                show_alert(hwnd, "DocuMorph Setup Error", &err_msg, true);
            }
            0
        }
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
                    let is_installed = INSTALLER_STATE
                        .lock()
                        .ok()
                        .and_then(|g| g.as_ref().map(|s| s.installed))
                        .unwrap_or(false);

                    if is_installed {
                        DestroyWindow(hwnd);
                        return 0;
                    }

                    let target_str = INSTALLER_STATE
                        .lock()
                        .ok()
                        .and_then(|g| g.as_ref().map(|s| get_edit_text(s.hwnd_edit_dir)))
                        .unwrap_or_default();

                    let target_path = PathBuf::from(target_str.trim());
                    if target_path.as_os_str().is_empty() {
                        show_alert(hwnd, "DocuMorph Setup", "Please select a valid installation directory.", true);
                        return 0;
                    }

                    if let Ok(guard) = INSTALLER_STATE.lock() {
                        if let Some(state) = guard.as_ref() {
                            EnableWindow(state.hwnd_btn_install, 0);
                            SetWindowTextW(state.hwnd_lbl_status, com::to_wide("Installing DocuMorph...").as_ptr());
                        }
                    }

                    let hwnd_target = hwnd as isize;
                    thread::spawn(move || {
                        let res = perform_install(&target_path);
                        let (success, err_ptr) = match res {
                            Ok(_) => (1, 0isize),
                            Err(e) => (0, Box::into_raw(Box::new(e)) as isize),
                        };
                        unsafe {
                            PostMessageW(hwnd_target as HWND, WM_APP_INSTALL_RESULT, success, err_ptr);
                        }
                    });
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

    // Request administrative elevation if running unelevated
    if !documorph::elevation::is_elevated() {
        if documorph::elevation::relaunch_elevated(&args[1..]) {
            std::process::exit(0);
        }
    }

    let default_dest = env::var("ProgramFiles")
        .map(|p| PathBuf::from(p).join("DocuMorph"))
        .unwrap_or_else(|_| PathBuf::from(r"C:\Program Files\DocuMorph"));

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
        wc.hbrBackground = ptr::null_mut();
        wc.lpszClassName = class_name.as_ptr();

        RegisterClassExW(&wc);

        let win_w = 620;
        let win_h = 490;
        let sx = GetSystemMetrics(SM_CXSCREEN);
        let sy = GetSystemMetrics(SM_CYSCREEN);
        let px = (sx - win_w) / 2;
        let py = (sy - win_h) / 2;

        let title = com::to_wide("DocuMorph Setup (v1.2.2)");
        let hwnd_main = CreateWindowExW(
            WS_EX_APPWINDOW | WS_EX_COMPOSITED, class_name.as_ptr(), title.as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU, px, py, win_w, win_h,
            ptr::null_mut(), ptr::null_mut(), hinstance, ptr::null_mut(),
        );

        if hwnd_main.is_null() {
            return;
        }

        ui::controls::apply_window_icon(hwnd_main);
        apply_fluent_window_frame(hwnd_main);
        let fonts = ui::controls::ControlFonts::new();
        let theme = FluentTheme::new();

        ui::controls::create_label(hwnd_main, "DocuMorph Setup", 28, 18, 280, 26, fonts.title);
        ui::controls::create_label(hwnd_main, "Version 1.2.2", 28, 48, 550, 20, fonts.small);

        ui::controls::create_label(hwnd_main, "Installation Folder", 40, 92, 200, 18, fonts.bold);
        let default_dir_str = default_dest.to_string_lossy().to_string();
        let hwnd_edit_dir = ui::controls::create_edit(hwnd_main, &default_dir_str, ID_EDIT_DIR, 40, 116, 415, 26, fonts.normal, false);
        let _ = ui::controls::create_fluent_button(hwnd_main, "Browse...", ID_BTN_BROWSE, 465, 115, 95, 28, fonts.normal);

        ui::controls::create_label(hwnd_main, "Features", 40, 192, 250, 18, fonts.bold);
        let desc = "• Adds 'Convert to PDF with DocuMorph' to File Explorer right-click menu\n\n• Converts Word (.docx, .doc), PowerPoint (.pptx, .ppt), and Excel (.xlsx, .xls)\n\n• Embeds TrueType fonts directly in exported PDFs\n\n• Includes uninstaller in Windows Settings > Installed Apps";
        ui::controls::create_label(hwnd_main, desc, 40, 218, 520, 115, fonts.normal);

        let hwnd_lbl_status = ui::controls::create_label(hwnd_main, "Ready to install.", 28, 402, 310, 22, fonts.normal);
        let hwnd_btn_install = ui::controls::create_fluent_button(hwnd_main, "Install", ID_BTN_INSTALL, 350, 394, 125, 36, fonts.bold);
        let hwnd_btn_cancel = ui::controls::create_fluent_button(hwnd_main, "Cancel", ID_BTN_CANCEL, 485, 394, 95, 36, fonts.normal);

        if let Ok(mut guard) = INSTALLER_STATE.lock() {
            *guard = Some(InstallerState {
                hwnd_edit_dir,
                hwnd_lbl_status,
                hwnd_btn_install,
                hwnd_btn_cancel,
                fonts,
                theme,
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
