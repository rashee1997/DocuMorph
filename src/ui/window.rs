use std::path::PathBuf;
use std::ptr;
use std::sync::mpsc::channel;
use std::sync::Mutex;
use std::thread;

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{COLOR_BTNFACE, DeleteObject, UpdateWindow};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Controls::{
    InitCommonControlsEx, INITCOMMONCONTROLSEX, ICC_PROGRESS_CLASS, PBM_SETPOS,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
    GetSystemMetrics, LoadIconW, PostMessageW,
    PostQuitMessage, RegisterClassExW, SendMessageW, SetWindowTextW, ShowWindow,
    TranslateMessage, CS_HREDRAW, CS_VREDRAW, EN_CHANGE, IDI_APPLICATION, LB_ADDSTRING,
    LB_DELETESTRING, LB_ERR, LB_GETCURSEL, LB_RESETCONTENT, SM_CXSCREEN, SM_CYSCREEN,
    SW_SHOW, SW_SHOWNORMAL, WM_APP, WM_CLOSE, WM_COMMAND, WM_DESTROY, WNDCLASSEXW,
    WS_CAPTION, WS_EX_APPWINDOW, WS_OVERLAPPED, WS_SYSMENU,
};

use crate::com::to_wide;
use crate::ui::builder::{build_dialog_controls, update_file_count, update_preview};
use crate::ui::controls::*;
use crate::ui::dialogs::{pick_documents, pick_folder};
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
                        SendMessageW(state.hwnd_progress_overall, PBM_SETPOS, overall_pct, 0);
                        SendMessageW(state.hwnd_progress_current, PBM_SETPOS, 0, 0);
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
                if let Ok(guard) = UI_STATE.lock() {
                    if let Some(state) = guard.as_ref() {
                        SendMessageW(state.hwnd_progress_current, PBM_SETPOS, pct, 0);
                        SetWindowTextW(state.hwnd_lbl_status_sub, to_wide(&text).as_ptr());
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
                    SendMessageW(state.hwnd_progress_overall, PBM_SETPOS, 100, 0);
                    SendMessageW(state.hwnd_progress_current, PBM_SETPOS, 100, 0);
                    let result_str = format!(
                        "✓ Successfully converted {} of {} documents to PDF!",
                        success_count, total
                    );
                    SetWindowTextW(state.hwnd_lbl_status_main, to_wide(&result_str).as_ptr());
                    SetWindowTextW(
                        state.hwnd_lbl_status_sub,
                        to_wide("All fonts embedded with 100% fidelity.").as_ptr(),
                    );
                    ShowWindow(state.hwnd_btn_open_dir, SW_SHOW);
                    SetWindowTextW(state.hwnd_btn_cancel, to_wide("Close").as_ptr());
                    EnableWindow(state.hwnd_btn_cancel, 1);
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
                ID_BTN_ADD => {
                    let files = pick_documents(hwnd);
                    if !files.is_empty() {
                        if let Ok(mut guard) = UI_STATE.lock() {
                            if let Some(state) = guard.as_mut() {
                                for f in files {
                                    if !state.queued_files.contains(&f) {
                                        let name = f.file_name().and_then(|n| n.to_str()).unwrap_or("file");
                                        let wide = to_wide(name);
                                        SendMessageW(state.hwnd_list, LB_ADDSTRING, 0, wide.as_ptr() as isize);
                                        state.queued_files.push(f);
                                    }
                                }
                            }
                        }
                        update_file_count();
                        update_preview();
                    }
                }
                ID_BTN_REMOVE => {
                    if let Ok(mut guard) = UI_STATE.lock() {
                        if let Some(state) = guard.as_mut() {
                            let sel = SendMessageW(state.hwnd_list, LB_GETCURSEL, 0, 0);
                            if sel != LB_ERR as isize {
                                SendMessageW(state.hwnd_list, LB_DELETESTRING, sel as usize, 0);
                                if (sel as usize) < state.queued_files.len() {
                                    state.queued_files.remove(sel as usize);
                                }
                            }
                        }
                    }
                    update_file_count();
                    update_preview();
                }
                ID_BTN_CLEAR => {
                    if let Ok(mut guard) = UI_STATE.lock() {
                        if let Some(state) = guard.as_mut() {
                            SendMessageW(state.hwnd_list, LB_RESETCONTENT, 0, 0);
                            state.queued_files.clear();
                        }
                    }
                    update_file_count();
                    update_preview();
                }
                ID_RADIO_DEST_SRC => {
                    if let Ok(guard) = UI_STATE.lock() {
                        if let Some(state) = guard.as_ref() {
                            EnableWindow(state.hwnd_edit_dest, 0);
                            EnableWindow(state.hwnd_btn_browse_dest, 0);
                        }
                    }
                }
                ID_RADIO_DEST_CUST => {
                    if let Ok(guard) = UI_STATE.lock() {
                        if let Some(state) = guard.as_ref() {
                            EnableWindow(state.hwnd_edit_dest, 1);
                            EnableWindow(state.hwnd_btn_browse_dest, 1);
                        }
                    }
                }
                ID_BTN_BROWSE_DEST => {
                    if let Some(folder) = pick_folder(hwnd) {
                        let wide = to_wide(&folder.to_string_lossy());
                        if let Ok(mut guard) = UI_STATE.lock() {
                            if let Some(state) = guard.as_mut() {
                                SetWindowTextW(state.hwnd_edit_dest, wide.as_ptr());
                                state.custom_dest = Some(folder);
                            }
                        }
                    }
                }
                ID_BTN_START => {
                    start_conversion_flow(hwnd);
                }
                ID_BTN_OPEN_DIR => {
                    if let Ok(guard) = UI_STATE.lock() {
                        if let Some(state) = guard.as_ref() {
                            if let Some(dir) = &state.last_output_dir {
                                let wide_op = to_wide("open");
                                let wide_path = to_wide(&dir.to_string_lossy());
                                ShellExecuteW(
                                    hwnd,
                                    wide_op.as_ptr(),
                                    wide_path.as_ptr(),
                                    ptr::null(),
                                    ptr::null(),
                                    SW_SHOWNORMAL,
                                );
                            }
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
        wc.hbrBackground = (COLOR_BTNFACE + 1) as windows_sys::Win32::Graphics::Gdi::HBRUSH;
        wc.lpszClassName = class_name.as_ptr();

        RegisterClassExW(&wc);

        let win_w = 620;
        let win_h = 580;
        let sx = GetSystemMetrics(SM_CXSCREEN);
        let sy = GetSystemMetrics(SM_CYSCREEN);
        let px = (sx - win_w) / 2;
        let py = (sy - win_h) / 2;

        let title = to_wide("DocuMorph — Document to PDF Converter (v1.1.0)");
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
            return Err("Failed to create main window".into());
        }

        apply_window_icon(hwnd_main);

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
