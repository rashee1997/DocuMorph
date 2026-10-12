use std::path::PathBuf;
use std::ptr;
use std::sync::mpsc::channel;
use std::sync::Mutex;
use std::thread;

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, DeleteObject, EndPaint, FillRect, InvalidateRect, ScreenToClient, SetBkMode,
    SetTextColor, UpdateWindow, HDC, PAINTSTRUCT, TRANSPARENT,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Controls::{
    InitCommonControlsEx, DRAWITEMSTRUCT, ICC_PROGRESS_CLASS, INITCOMMONCONTROLSEX,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AdjustWindowRectEx, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
    GetClientRect, GetMessageW, GetSystemMetrics, GetWindowRect, GetWindowTextW, LoadIconW,
    PostMessageW, PostQuitMessage, RegisterClassExW, SendMessageW, SetWindowTextW, ShowWindow,
    TranslateMessage, CS_HREDRAW, CS_VREDRAW, EN_CHANGE, IDI_APPLICATION, LB_ADDSTRING,
    SM_CXSCREEN, SM_CYSCREEN, SW_SHOW, WM_APP, WM_CLOSE, WM_COMMAND, WM_CTLCOLORSTATIC,
    WM_DESTROY, WM_DRAWITEM, WM_ERASEBKGND, WM_PAINT, WM_TIMER, WNDCLASSEXW, WS_CAPTION,
    WS_EX_APPWINDOW, WS_EX_COMPOSITED, WS_OVERLAPPED, WS_SYSMENU,
};

use crate::com::to_wide;
use crate::ui::actions::*;
use crate::ui::builder::{build_dialog_controls, update_file_count, update_preview};
use crate::ui::controls::*;
use crate::ui::fluent::{
    apply_fluent_window_frame, draw_fluent_button, is_point_inside_card, paint_main_cards, rgb,
    FluentTheme, COLOR_TEXT_MAIN_RGB,
};
use crate::ui::worker::start_conversion_flow;

pub const WM_APP_ADD_FILE: u32 = WM_APP + 1;
pub const WM_APP_PROGRESS: u32 = WM_APP + 2;
pub const WM_APP_FILE_START: u32 = WM_APP + 3;
pub const WM_APP_FINISHED: u32 = WM_APP + 4;

pub const ID_LIST_FILES: usize = 101;
pub const ID_BTN_ADD: usize = 102;
pub const ID_BTN_REMOVE: usize = 103;
pub const ID_BTN_CLEAR: usize = 104;

pub const ID_RADIO_STD: usize = 110;
pub const ID_RADIO_PDFA: usize = 111;

pub const ID_RADIO_DEST_SRC: usize = 120;
pub const ID_RADIO_DEST_CUST: usize = 121;
pub const ID_EDIT_DEST: usize = 122;
pub const ID_BTN_BROWSE_DEST: usize = 123;

pub const ID_EDIT_PATTERN: usize = 130;
pub const ID_EDIT_PREFIX: usize = 131;
pub const ID_EDIT_SUFFIX: usize = 132;
pub const ID_CHK_SAFE_COLLISION: usize = 133;

pub const ID_BTN_START: usize = 140;
pub const ID_BTN_CANCEL: usize = 141;
pub const ID_BTN_OPEN_DIR: usize = 142;

pub struct UiState {
    pub hwnd_main: HWND,
    pub fonts: ControlFonts,
    pub theme: FluentTheme,
    pub queued_files: Vec<PathBuf>,
    pub custom_dest: Option<PathBuf>,
    pub last_output_dir: Option<PathBuf>,

    pub hwnd_list: HWND,
    pub hwnd_lbl_count: HWND,
    pub hwnd_radio_std: HWND,
    pub hwnd_radio_pdfa: HWND,
    pub hwnd_radio_dest_src: HWND,
    pub hwnd_radio_dest_cust: HWND,
    pub hwnd_edit_dest: HWND,
    pub hwnd_btn_browse_dest: HWND,
    pub hwnd_edit_pattern: HWND,
    pub hwnd_edit_prefix: HWND,
    pub hwnd_edit_suffix: HWND,
    pub hwnd_chk_safe: HWND,
    pub hwnd_lbl_preview: HWND,
    pub hwnd_btn_start: HWND,
    pub hwnd_btn_cancel: HWND,

    pub hwnd_progress_overall: HWND,
    pub hwnd_progress_current: HWND,
    pub hwnd_lbl_status_main: HWND,
    pub hwnd_lbl_status_sub: HWND,
    pub hwnd_btn_open_dir: HWND,
}

unsafe impl Send for UiState {}
pub static UI_STATE: Mutex<Option<UiState>> = Mutex::new(None);

unsafe extern "system" fn window_proc(
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
            if let Ok(guard) = UI_STATE.lock() {
                if let Some(state) = guard.as_ref() {
                    FillRect(hdc, &rc, state.theme.canvas_brush);
                }
            }
            1
        }
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);
            if let Ok(guard) = UI_STATE.lock() {
                if let Some(state) = guard.as_ref() {
                    paint_main_cards(hdc, &state.theme);
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
            if let Ok(guard) = UI_STATE.lock() {
                if let Some(state) = guard.as_ref() {
                    let mut rect: RECT = std::mem::zeroed();
                    GetWindowRect(lparam as HWND, &mut rect);
                    let mut pt = POINT { x: rect.left, y: rect.top };
                    ScreenToClient(hwnd, &mut pt);
                    return if is_point_inside_card(pt.y) {
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
            if let Ok(guard) = UI_STATE.lock() {
                if let Some(state) = guard.as_ref() {
                    let mut text_buf = [0u16; 64];
                    let len = GetWindowTextW(dis.hwndItem, text_buf.as_mut_ptr(), 64) as usize;
                    let text = String::from_utf16_lossy(&text_buf[..len]);
                    let is_primary = dis.CtlID as usize == ID_BTN_START;
                    draw_fluent_button(dis, &state.theme, &text, is_primary);
                }
            }
            1
        }
        WM_APP_ADD_FILE => {
            let path_ptr = lparam as *mut PathBuf;
            if !path_ptr.is_null() {
                let path = *Box::from_raw(path_ptr);
                if let Ok(mut guard) = UI_STATE.lock() {
                    if let Some(state) = guard.as_mut() {
                        if !state.queued_files.contains(&path) {
                            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
                            let wide_name = to_wide(name);
                            SendMessageW(state.hwnd_list, LB_ADDSTRING, 0, wide_name.as_ptr() as isize);
                            state.queued_files.push(path);
                        }
                    }
                }
                update_file_count();
                update_preview();
            }
            0
        }
        WM_TIMER => {
            if wparam == crate::ui::anim::ID_TIMER_PROGRESS {
                if let Ok(guard) = UI_STATE.lock() {
                    if let Some(state) = guard.as_ref() {
                        crate::ui::anim::on_timer_tick(
                            state.hwnd_progress_current,
                            state.hwnd_progress_overall,
                        );
                    }
                }
            }
            0
        }
        WM_APP_FILE_START => {
            let idx = (wparam & 0xFFFF) as usize;
            let total = ((wparam >> 16) & 0xFFFF) as usize;
            let filename_ptr = lparam as *mut String;
            if !filename_ptr.is_null() {
                let filename = *Box::from_raw(filename_ptr);
                if let Ok(guard) = UI_STATE.lock() {
                    if let Some(state) = guard.as_ref() {
                        let text = format!("Converting file {} of {}: {}", idx + 1, total, filename);
                        SetWindowTextW(state.hwnd_lbl_status_main, to_wide(&text).as_ptr());
                        let overall_pct = ((idx as f32 / total as f32) * 100.0) as usize;
                        crate::ui::anim::set_overall_target(overall_pct);
                        crate::ui::anim::set_target_percent(10);
                        InvalidateRect(state.hwnd_lbl_status_main, ptr::null(), 1);
                    }
                }
            }
            0
        }
        WM_APP_PROGRESS => {
            let pct = wparam as usize;
            let text_ptr = lparam as *mut String;
            if !text_ptr.is_null() {
                let text = *Box::from_raw(text_ptr);
                crate::ui::anim::set_target_percent(pct);
                if let Ok(guard) = UI_STATE.lock() {
                    if let Some(state) = guard.as_ref() {
                        SetWindowTextW(state.hwnd_lbl_status_sub, to_wide(&text).as_ptr());
                        InvalidateRect(state.hwnd_lbl_status_sub, ptr::null(), 1);
                    }
                }
            }
            0
        }
        WM_APP_FINISHED => {
            let success_count = wparam as usize;
            let total = lparam as usize;
            if let Ok(guard) = UI_STATE.lock() {
                if let Some(state) = guard.as_ref() {
                    // Stop animation and reset progress bars to 100%
                    crate::ui::anim::stop_anim(hwnd, state.hwnd_progress_current, state.hwnd_progress_overall);
                    
                    // Restore UI state
                    let result_str = format!("Converted {} of {} files.", success_count, total);
                    SetWindowTextW(state.hwnd_lbl_status_main, to_wide(&result_str).as_ptr());
                    SetWindowTextW(state.hwnd_lbl_status_sub, to_wide("").as_ptr());
                    
                    // Show open folder button, restore cancel button
                    ShowWindow(state.hwnd_btn_open_dir, SW_SHOW);
                    SetWindowTextW(state.hwnd_btn_cancel, to_wide("Close").as_ptr());
                    EnableWindow(state.hwnd_btn_cancel, 1);
                    
                    // Force UI update
                    InvalidateRect(hwnd, ptr::null(), 1);
                    UpdateWindow(hwnd);
                }
            }
            0
        }
        WM_COMMAND => {
            let id = (wparam & 0xFFFF) as usize;
            let code = ((wparam >> 16) & 0xFFFF) as u32;

            if code == EN_CHANGE as u32 {
                update_preview();
                return 0;
            }

            match id {
                ID_BTN_ADD => add_files_from_dialog(hwnd),
                ID_BTN_REMOVE => remove_selected_file(),
                ID_BTN_CLEAR => clear_all_files(),
                ID_RADIO_DEST_SRC => set_dest_mode(false),
                ID_RADIO_DEST_CUST => set_dest_mode(true),
                ID_BTN_BROWSE_DEST => browse_custom_dest(hwnd),
                ID_BTN_START => start_conversion_flow(hwnd),
                ID_BTN_OPEN_DIR => open_output_folder(hwnd),
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
            if let Ok(mut guard) = UI_STATE.lock() {
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

pub fn show_main_dialog(initial_files: Vec<PathBuf>) -> Result<(), String> {
    unsafe {
        let mut icex: INITCOMMONCONTROLSEX = std::mem::zeroed();
        icex.dwSize = std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32;
        icex.dwICC = ICC_PROGRESS_CLASS;
        InitCommonControlsEx(&icex);

        let hinstance = GetModuleHandleW(ptr::null());
        let class_name = to_wide("DocuMorphMainWindow");

        let mut wc: WNDCLASSEXW = std::mem::zeroed();
        wc.cbSize = std::mem::size_of::<WNDCLASSEXW>() as u32;
        wc.style = CS_HREDRAW | CS_VREDRAW;
        wc.lpfnWndProc = Some(window_proc);
        wc.hInstance = hinstance;
        wc.hIcon = LoadIconW(ptr::null_mut(), IDI_APPLICATION);
        wc.hCursor = ptr::null_mut();
        wc.hbrBackground = ptr::null_mut();
        wc.lpszClassName = class_name.as_ptr();

        RegisterClassExW(&wc);

        let mut client_rect = RECT {
            left: 0,
            top: 0,
            right: 640,
            bottom: 630,
        };
        let style = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU;
        let ex_style = WS_EX_APPWINDOW | WS_EX_COMPOSITED;
        AdjustWindowRectEx(&mut client_rect, style, 0, ex_style);

        let win_w = client_rect.right - client_rect.left;
        let win_h = client_rect.bottom - client_rect.top;
        let sx = GetSystemMetrics(SM_CXSCREEN);
        let sy = GetSystemMetrics(SM_CYSCREEN);
        let px = (sx - win_w) / 2;
        let py = (sy - win_h) / 2;

        let title = to_wide("DocuMorph — Document to PDF Converter (v1.2.2)");
        let hwnd_main = CreateWindowExW(
            ex_style,
            class_name.as_ptr(),
            title.as_ptr(),
            style,
            px, py, win_w, win_h,
            ptr::null_mut(),
            ptr::null_mut(),
            hinstance,
            ptr::null_mut(),
        );

        if hwnd_main.is_null() {
            return Err("Failed to create main window".into());
        }

        apply_window_icon(hwnd_main);
        apply_fluent_window_frame(hwnd_main);

        // Build controls and populate initial files
        let state = build_dialog_controls(hwnd_main, &initial_files);
        if let Ok(mut guard) = UI_STATE.lock() {
            *guard = Some(state);
        }

        update_file_count();
        update_preview();

        ShowWindow(hwnd_main, SW_SHOW);
        UpdateWindow(hwnd_main);

        // Start Named Pipe listener for external files
        let (pipe_tx, pipe_rx) = channel::<PathBuf>();
        crate::ipc::start_pipe_server(pipe_tx);

        let hwnd_w = hwnd_main as isize;
        thread::spawn(move || {
            let hwnd_t = hwnd_w as HWND;
            while let Ok(path) = pipe_rx.recv() {
                let boxed = Box::into_raw(Box::new(path));
                PostMessageW(hwnd_t, WM_APP_ADD_FILE, 0, boxed as isize);
            }
        });

        // Win32 Message Loop
        let mut msg: windows_sys::Win32::UI::WindowsAndMessaging::MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        Ok(())
    }
}
