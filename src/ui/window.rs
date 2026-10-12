use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::mpsc::{channel, Sender};
use std::thread;

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    CreateFontW, DeleteObject, UpdateWindow, COLOR_BTNFACE, FW_BOLD, FW_NORMAL, HFONT,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Controls::{
    InitCommonControlsEx, INITCOMMONCONTROLSEX, ICC_PROGRESS_CLASS, PBM_SETPOS, PBM_SETRANGE32,
    PBS_SMOOTH,
};
use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
    GetSystemMetrics, KillTimer, LoadIconW, PostMessageW, PostQuitMessage, RegisterClassExW,
    SendMessageW, SetTimer, SetWindowTextW, ShowWindow, TranslateMessage,
    CS_HREDRAW, CS_VREDRAW, HMENU, IDI_APPLICATION,
    SM_CXSCREEN, SM_CYSCREEN, SW_SHOW, SW_SHOWNORMAL,
    WM_APP, WM_CLOSE, WM_COMMAND, WM_DESTROY, WM_SETFONT, WM_TIMER,
    WNDCLASSEXW, WS_CAPTION, WS_CHILD, WS_EX_APPWINDOW, WS_EX_TOPMOST, WS_OVERLAPPED,
    WS_SYSMENU, WS_TABSTOP, WS_VISIBLE,
};

use crate::com::to_wide;

const WM_APP_PROGRESS: u32 = WM_APP + 10;
const WM_APP_FINISHED: u32 = WM_APP + 11;

const ID_BTN_OPEN: usize = 1001;
const ID_BTN_FOLDER: usize = 1002;
const ID_BTN_CLOSE: usize = 1003;
const TIMER_AUTOCLOSE: usize = 2001;

pub enum ProgressMessage {
    Update { percent: u32, text: String },
    Finished {
        success: bool,
        message: String,
        #[allow(dead_code)]
        output_path: Option<PathBuf>,
    },
}

use std::sync::Mutex;

struct DialogState {
    hwnd_progress: HWND,
    #[allow(dead_code)]
    hwnd_title: HWND,
    hwnd_status: HWND,
    hwnd_btn_open: HWND,
    hwnd_btn_folder: HWND,
    hwnd_btn_close: HWND,
    output_path: Option<PathBuf>,
    font_bold: HFONT,
    font_normal: HFONT,
}

// DialogState pointers are valid for the Win32 window lifetime
unsafe impl Send for DialogState {}

static GLOBAL_STATE: Mutex<Option<DialogState>> = Mutex::new(None);

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_APP_PROGRESS => {
            let percent = wparam as u32;
            let text_ptr = lparam as *mut String;
            if !text_ptr.is_null() {
                let text = Box::from_raw(text_ptr);
                if let Ok(guard) = GLOBAL_STATE.lock() {
                    if let Some(state) = guard.as_ref() {
                        SendMessageW(state.hwnd_progress, PBM_SETPOS, percent as usize, 0);
                        let wide_text = to_wide(&text);
                        SetWindowTextW(state.hwnd_status, wide_text.as_ptr());
                    }
                }
            }
            0
        }
        WM_APP_FINISHED => {
            let success = wparam == 1;
            let msg_ptr = lparam as *mut String;
            if !msg_ptr.is_null() {
                let message = Box::from_raw(msg_ptr);
                if let Ok(guard) = GLOBAL_STATE.lock() {
                    if let Some(state) = guard.as_ref() {
                        if success {
                            SendMessageW(state.hwnd_progress, PBM_SETPOS, 100, 0);
                            let wide_status = to_wide(&format!("✓ {}", message));
                            SetWindowTextW(state.hwnd_status, wide_status.as_ptr());

                            // Reveal completion action buttons
                            ShowWindow(state.hwnd_btn_open, SW_SHOW);
                            ShowWindow(state.hwnd_btn_folder, SW_SHOW);
                            ShowWindow(state.hwnd_btn_close, SW_SHOW);

                            // Start 2.5-second auto-close countdown timer
                            SetTimer(hwnd, TIMER_AUTOCLOSE, 2500, None);
                        } else {
                            let wide_status = to_wide(&format!("❌ {}", message));
                            SetWindowTextW(state.hwnd_status, wide_status.as_ptr());
                            ShowWindow(state.hwnd_btn_close, SW_SHOW);
                        }
                    }
                }
            }
            0
        }
        WM_COMMAND => {
            let id = (wparam & 0xFFFF) as usize;
            match id {
                ID_BTN_OPEN => {
                    if let Ok(guard) = GLOBAL_STATE.lock() {
                        if let Some(state) = guard.as_ref() {
                            if let Some(path) = &state.output_path {
                                let wide_path = to_wide(path.to_str().unwrap_or_default());
                                let wide_op = to_wide("open");
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
                    DestroyWindow(hwnd);
                }
                ID_BTN_FOLDER => {
                    if let Ok(guard) = GLOBAL_STATE.lock() {
                        if let Some(state) = guard.as_ref() {
                            if let Some(path) = &state.output_path {
                                let arg = format!("/select,\"{}\"", path.display());
                                let wide_arg = to_wide(&arg);
                                let wide_explorer = to_wide("explorer.exe");
                                let wide_op = to_wide("open");
                                ShellExecuteW(
                                    hwnd,
                                    wide_op.as_ptr(),
                                    wide_explorer.as_ptr(),
                                    wide_arg.as_ptr(),
                                    ptr::null(),
                                    SW_SHOWNORMAL,
                                );
                            }
                        }
                    }
                    DestroyWindow(hwnd);
                }
                ID_BTN_CLOSE => {
                    DestroyWindow(hwnd);
                }
                _ => {}
            }
            0
        }
        WM_TIMER => {
            if wparam as usize == TIMER_AUTOCLOSE {
                KillTimer(hwnd, TIMER_AUTOCLOSE);
                DestroyWindow(hwnd);
            }
            0
        }
        WM_CLOSE => {
            DestroyWindow(hwnd);
            0
        }
        WM_DESTROY => {
            if let Ok(mut guard) = GLOBAL_STATE.lock() {
                if let Some(state) = guard.take() {
                    if !state.font_bold.is_null() {
                        DeleteObject(state.font_bold);
                    }
                    if !state.font_normal.is_null() {
                        DeleteObject(state.font_normal);
                    }
                }
            }
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// Creates and displays the native Win32 live progress dialog
pub fn show_conversion_dialog<F>(
    input_file: &Path,
    target_output: &Path,
    worker_fn: F,
) -> Result<(), String>
where
    F: FnOnce(Sender<ProgressMessage>) + Send + 'static,
{
    unsafe {
        // Initialize modern Common Controls (v6 progress bar, buttons)
        let mut icex: INITCOMMONCONTROLSEX = std::mem::zeroed();
        icex.dwSize = std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32;
        icex.dwICC = ICC_PROGRESS_CLASS;
        InitCommonControlsEx(&icex);

        let hinstance = GetModuleHandleW(ptr::null());
        let class_name = to_wide("DocuMorphProgressDialog");

        let mut wc: WNDCLASSEXW = std::mem::zeroed();
        wc.cbSize = std::mem::size_of::<WNDCLASSEXW>() as u32;
        wc.style = CS_HREDRAW | CS_VREDRAW;
        wc.lpfnWndProc = Some(window_proc);
        wc.hInstance = hinstance;
        // Load embedded application icon (resource #1 or default)
        wc.hIcon = LoadIconW(hinstance, 1 as *const u16);
        if wc.hIcon.is_null() {
            wc.hIcon = LoadIconW(ptr::null_mut(), IDI_APPLICATION);
        }
        wc.hCursor = ptr::null_mut();
        wc.hbrBackground = (COLOR_BTNFACE + 1) as windows_sys::Win32::Graphics::Gdi::HBRUSH;
        wc.lpszClassName = class_name.as_ptr();

        RegisterClassExW(&wc);

        let window_title = to_wide("DocuMorph — Converting to PDF");
        let win_width = 460;
        let win_height = 200;

        let screen_w = GetSystemMetrics(SM_CXSCREEN);
        let screen_h = GetSystemMetrics(SM_CYSCREEN);
        let pos_x = (screen_w - win_width) / 2;
        let pos_y = (screen_h - win_height) / 2;

        let hwnd_main = CreateWindowExW(
            WS_EX_APPWINDOW | WS_EX_TOPMOST,
            class_name.as_ptr(),
            window_title.as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU,
            pos_x,
            pos_y,
            win_width,
            win_height,
            ptr::null_mut(),
            ptr::null_mut(),
            hinstance,
            ptr::null_mut(),
        );

        if hwnd_main.is_null() {
            return Err("Failed to create Win32 dialog window".to_string());
        }

        // Create fonts: Segoe UI Semi-Bold and Regular
        let font_face = to_wide("Segoe UI");
        let font_bold = CreateFontW(
            17, 0, 0, 0, FW_BOLD as i32, 0, 0, 0,
            0, 0, 0, 5, 0, font_face.as_ptr(),
        );
        let font_normal = CreateFontW(
            15, 0, 0, 0, FW_NORMAL as i32, 0, 0, 0,
            0, 0, 0, 5, 0, font_face.as_ptr(),
        );

        // Document title label
        let file_name = input_file
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Document");
        let title_text = to_wide(&format!("File: {}", file_name));
        let static_class = to_wide("STATIC");

        let hwnd_title = CreateWindowExW(
            0,
            static_class.as_ptr(),
            title_text.as_ptr(),
            WS_CHILD | WS_VISIBLE,
            24, 20, 410, 22,
            hwnd_main,
            ptr::null_mut(),
            hinstance,
            ptr::null_mut(),
        );
        SendMessageW(hwnd_title, WM_SETFONT, font_bold as usize, 1);

        // Stage / Status label
        let status_initial = to_wide("Initializing conversion engine...");
        let hwnd_status = CreateWindowExW(
            0,
            static_class.as_ptr(),
            status_initial.as_ptr(),
            WS_CHILD | WS_VISIBLE,
            24, 46, 410, 20,
            hwnd_main,
            ptr::null_mut(),
            hinstance,
            ptr::null_mut(),
        );
        SendMessageW(hwnd_status, WM_SETFONT, font_normal as usize, 1);

        // Progress bar
        let progress_class = to_wide("msctls_progress32");
        let hwnd_progress = CreateWindowExW(
            0,
            progress_class.as_ptr(),
            ptr::null(),
            WS_CHILD | WS_VISIBLE | PBS_SMOOTH,
            24, 76, 410, 20,
            hwnd_main,
            ptr::null_mut(),
            hinstance,
            ptr::null_mut(),
        );
        SendMessageW(hwnd_progress, PBM_SETRANGE32, 0, 100);
        SendMessageW(hwnd_progress, PBM_SETPOS, 5, 0);

        // Action buttons
        let btn_class = to_wide("BUTTON");
        let txt_open = to_wide("Open PDF");
        let hwnd_btn_open = CreateWindowExW(
            0,
            btn_class.as_ptr(),
            txt_open.as_ptr(),
            WS_CHILD | WS_TABSTOP,
            24, 114, 100, 28,
            hwnd_main,
            ID_BTN_OPEN as HMENU,
            hinstance,
            ptr::null_mut(),
        );
        SendMessageW(hwnd_btn_open, WM_SETFONT, font_normal as usize, 1);

        let txt_folder = to_wide("Show in Folder");
        let hwnd_btn_folder = CreateWindowExW(
            0,
            btn_class.as_ptr(),
            txt_folder.as_ptr(),
            WS_CHILD | WS_TABSTOP,
            132, 114, 120, 28,
            hwnd_main,
            ID_BTN_FOLDER as HMENU,
            hinstance,
            ptr::null_mut(),
        );
        SendMessageW(hwnd_btn_folder, WM_SETFONT, font_normal as usize, 1);

        let txt_close = to_wide("Close");
        let hwnd_btn_close = CreateWindowExW(
            0,
            btn_class.as_ptr(),
            txt_close.as_ptr(),
            WS_CHILD | WS_TABSTOP,
            334, 114, 100, 28,
            hwnd_main,
            ID_BTN_CLOSE as HMENU,
            hinstance,
            ptr::null_mut(),
        );
        SendMessageW(hwnd_btn_close, WM_SETFONT, font_normal as usize, 1);

        if let Ok(mut guard) = GLOBAL_STATE.lock() {
            *guard = Some(DialogState {
                hwnd_progress,
                hwnd_title,
                hwnd_status,
                hwnd_btn_open,
                hwnd_btn_folder,
                hwnd_btn_close,
                output_path: Some(target_output.to_path_buf()),
                font_bold,
                font_normal,
            });
        }

        ShowWindow(hwnd_main, SW_SHOW);
        UpdateWindow(hwnd_main);

        // Spawn conversion background worker
        let (tx, rx) = channel::<ProgressMessage>();
        thread::spawn(move || {
            worker_fn(tx);
        });

        // Bridge channel messages to the Win32 message queue
        let hwnd_raw = hwnd_main as isize;
        thread::spawn(move || {
            let hwnd_target = hwnd_raw as HWND;
            while let Ok(msg) = rx.recv() {
                match msg {
                    ProgressMessage::Update { percent, text } => {
                        let text_boxed = Box::into_raw(Box::new(text));
                        PostMessageW(
                            hwnd_target,
                            WM_APP_PROGRESS,
                            percent as usize,
                            text_boxed as isize,
                        );
                    }
                    ProgressMessage::Finished { success, message, .. } => {
                        let msg_boxed = Box::into_raw(Box::new(message));
                        PostMessageW(
                            hwnd_target,
                            WM_APP_FINISHED,
                            if success { 1 } else { 0 },
                            msg_boxed as isize,
                        );
                        break;
                    }
                }
            }
        });

        // Win32 message pump
        let mut msg: windows_sys::Win32::UI::WindowsAndMessaging::MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        Ok(())
    }
}
