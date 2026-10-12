#![windows_subsystem = "windows"]

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::Mutex;

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, DeleteObject, EndPaint, FillRect, InvalidateRect, ScreenToClient,
    SetBkMode, SetTextColor, UpdateWindow, HDC, PAINTSTRUCT, TRANSPARENT,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Controls::DRAWITEMSTRUCT;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetClientRect,
    GetMessageW, GetSystemMetrics, GetWindowRect, GetWindowTextLengthW, GetWindowTextW,
    LoadIconW, MessageBoxW, PostQuitMessage, RegisterClassExW, SetWindowTextW, ShowWindow,
    TranslateMessage, CS_HREDRAW, CS_VREDRAW, IDI_APPLICATION, MB_ICONERROR,
    MB_ICONINFORMATION, MB_OK, SM_CXSCREEN, SM_CYSCREEN, SW_SHOW, WM_CLOSE, WM_COMMAND,
    WM_CTLCOLORSTATIC, WM_DESTROY, WM_DRAWITEM, WM_ERASEBKGND, WM_PAINT, WNDCLASSEXW,
    WS_CAPTION, WS_EX_APPWINDOW, WS_EX_COMPOSITED, WS_OVERLAPPED, WS_SYSMENU,
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

fn show_alert(title: &str, text: &str, is_error: bool) {
    let flags = MB_OK | if is_error { MB_ICONERROR } else { MB_ICONINFORMATION };
    unsafe { MessageBoxW(ptr::null_mut(), com::to_wide(text).as_ptr(), com::to_wide(title).as_ptr(), flags); }
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
        fs::create_dir_all(target_dir).map_err(|e| format!("Could not create dir: {}", e))?;
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
                    // Card 1: Destination Card
                    let rc_card1 = RECT { left: 24, top: 78, right: 580, bottom: 165 };
                    draw_card(hdc, &rc_card1, &state.theme);

                    // Card 2: Feature Integration Card
                    let rc_card2 = RECT { left: 24, top: 178, right: 580, bottom: 350 };
                    draw_card(hdc, &rc_card2, &state.theme);
                }
            }
            EndPaint(hwnd, &ps);
            0
        }
        WM_CTLCOLORSTATIC => {
            let hdc = wparam as HDC;
            SetBkMode(hdc, TRANSPARENT as i32);
            SetTextColor(
                hdc,
                rgb(
                    COLOR_TEXT_MAIN_RGB.0,
                    COLOR_TEXT_MAIN_RGB.1,
                    COLOR_TEXT_MAIN_RGB.2,
                ),
            );
            if let Ok(guard) = INSTALLER_STATE.lock() {
                if let Some(state) = guard.as_ref() {
                    let mut rect: RECT = std::mem::zeroed();
                    GetWindowRect(lparam as HWND, &mut rect);
                    let mut pt = POINT { x: rect.left, y: rect.top };
                    ScreenToClient(hwnd, &mut pt);
                    return if pt.y >= 70 && pt.y <= 360 {
                        state.theme.card_brush as isize
                    } else {
                        state.theme.canvas_brush as isize
                    };
                }
            }
            0
        }
        WM_DRAWITEM => {
            let dis = &*(lparam as *const DRAWITEMSTRUCT);
            if let Ok(guard) = INSTALLER_STATE.lock() {
                if let Some(state) = guard.as_ref() {
                    match dis.CtlID as usize {
                        ID_BTN_INSTALL => {
                            let text = if state.installed { "Finish" } else { "Install Now" };
                            draw_fluent_button(dis, &state.theme, text, true);
                        }
                        ID_BTN_CANCEL => {
                            draw_fluent_button(dis, &state.theme, "Cancel", false);
                        }
                        ID_BTN_BROWSE => {
                            draw_fluent_button(dis, &state.theme, "Browse...", false);
                        }
                        _ => {}
                    }
                }
            }
            1
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
                    let mut is_installed = false;
                    if let Ok(guard) = INSTALLER_STATE.lock() {
                        if let Some(state) = guard.as_ref() {
                            is_installed = state.installed;
                        }
                    }

                    if is_installed {
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
                                        "✓ Installed to {}! Explorer context menu is ready.",
                                        target_path.display()
                                    );
                                    SetWindowTextW(state.hwnd_lbl_status, com::to_wide(&success_text).as_ptr());
                                    InvalidateRect(state.hwnd_btn_install, ptr::null(), 1);
                                }
                            }
                            show_alert(
                                "DocuMorph Setup",
                                &format!(
                                    "Installation Successful!\n\nDocuMorph has been installed to:\n{}\n\nRight-click any Word, PowerPoint, or Excel document in File Explorer to convert to PDF with 100% font fidelity.",
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
        wc.hbrBackground = ptr::null_mut();
        wc.lpszClassName = class_name.as_ptr();

        RegisterClassExW(&wc);

        let win_w = 620;
        let win_h = 490;
        let sx = GetSystemMetrics(SM_CXSCREEN);
        let sy = GetSystemMetrics(SM_CYSCREEN);
        let px = (sx - win_w) / 2;
        let py = (sy - win_h) / 2;

        let title = com::to_wide("DocuMorph Setup (v1.2.0)");
        let hwnd_main = CreateWindowExW(
            WS_EX_APPWINDOW | WS_EX_COMPOSITED,
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
        apply_fluent_window_frame(hwnd_main);
        let fonts = ui::controls::ControlFonts::new();
        let theme = FluentTheme::new();

        // Top Brand Header
        ui::controls::create_label(hwnd_main, "DocuMorph Setup", 28, 18, 280, 26, fonts.title);
        ui::controls::create_label(
            hwnd_main,
            "v1.2.0 • High-Fidelity Document to PDF Converter with TrueType Font Embedding",
            28, 48, 550, 20, fonts.small,
        );

        // Card 1 Content (Installation Destination)
        ui::controls::create_label(hwnd_main, "Installation Folder", 40, 92, 200, 18, fonts.bold);
        let default_dir_str = default_dest.to_string_lossy().to_string();
        let hwnd_edit_dir = ui::controls::create_edit(
            hwnd_main, &default_dir_str, ID_EDIT_DIR, 40, 116, 415, 26, fonts.normal, false,
        );
        let _ = ui::controls::create_fluent_button(
            hwnd_main, "Browse...", ID_BTN_BROWSE, 465, 115, 95, 28, fonts.normal,
        );

        // Card 2 Content (Feature Highlights)
        ui::controls::create_label(hwnd_main, "Features & Integration", 40, 192, 250, 18, fonts.bold);
        ui::controls::create_label(
            hwnd_main,
            "• Adds 'Convert to PDF with DocuMorph' right-click menu in File Explorer\n\n• Supports Word (.docx, .doc), PowerPoint (.pptx, .ppt), Excel (.xlsx, .xls)\n\n• 100% TrueType document font embedding (zero fallback font substitution)\n\n• Automatically registered in Windows Settings Installed Apps with clean uninstaller",
            40, 218, 520, 115, fonts.normal,
        );

        // Bottom Action Bar (Generously positioned at y = 390 with zero cutoff!)
        let hwnd_lbl_status = ui::controls::create_label(
            hwnd_main, "Ready to install.", 28, 402, 310, 22, fonts.normal,
        );

        let hwnd_btn_install = ui::controls::create_fluent_button(
            hwnd_main, "Install Now", ID_BTN_INSTALL, 350, 394, 125, 36, fonts.bold,
        );
        let hwnd_btn_cancel = ui::controls::create_fluent_button(
            hwnd_main, "Cancel", ID_BTN_CANCEL, 485, 394, 95, 36, fonts.normal,
        );

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
