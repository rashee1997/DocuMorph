use std::ptr;
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    SendMessageW, SetWindowTextW, LB_ADDSTRING, LB_DELETESTRING, LB_ERR, LB_GETCURSEL,
    LB_RESETCONTENT, SW_SHOWNORMAL,
};

use crate::com::to_wide;
use crate::ui::builder::{update_file_count, update_preview};
use crate::ui::dialogs::{pick_documents, pick_folder};
use crate::ui::window::UI_STATE;

pub fn add_files_from_dialog(hwnd: HWND) {
    let files = pick_documents(hwnd);
    if !files.is_empty() {
        if let Ok(mut guard) = UI_STATE.lock() {
            if let Some(state) = guard.as_mut() {
                for f in files {
                    if !state.queued_files.contains(&f) {
                        let name = f.file_name().and_then(|n| n.to_str()).unwrap_or("file");
                        let wide = to_wide(name);
                        unsafe {
                            SendMessageW(state.hwnd_list, LB_ADDSTRING, 0, wide.as_ptr() as isize);
                        }
                        state.queued_files.push(f);
                    }
                }
            }
        }
        update_file_count();
        update_preview();
    }
}

pub fn remove_selected_file() {
    if let Ok(mut guard) = UI_STATE.lock() {
        if let Some(state) = guard.as_mut() {
            unsafe {
                let sel = SendMessageW(state.hwnd_list, LB_GETCURSEL, 0, 0);
                if sel != LB_ERR as isize {
                    SendMessageW(state.hwnd_list, LB_DELETESTRING, sel as usize, 0);
                    if (sel as usize) < state.queued_files.len() {
                        state.queued_files.remove(sel as usize);
                    }
                }
            }
        }
    }
    update_file_count();
    update_preview();
}

pub fn clear_all_files() {
    if let Ok(mut guard) = UI_STATE.lock() {
        if let Some(state) = guard.as_mut() {
            unsafe {
                SendMessageW(state.hwnd_list, LB_RESETCONTENT, 0, 0);
            }
            state.queued_files.clear();
        }
    }
    update_file_count();
    update_preview();
}

pub fn browse_custom_dest(hwnd: HWND) {
    if let Some(folder) = pick_folder(hwnd) {
        let wide = to_wide(&folder.to_string_lossy());
        if let Ok(mut guard) = UI_STATE.lock() {
            if let Some(state) = guard.as_mut() {
                unsafe {
                    SetWindowTextW(state.hwnd_edit_dest, wide.as_ptr());
                }
                state.custom_dest = Some(folder);
            }
        }
    }
}

pub fn set_dest_mode(is_custom: bool) {
    if let Ok(guard) = UI_STATE.lock() {
        if let Some(state) = guard.as_ref() {
            unsafe {
                EnableWindow(state.hwnd_edit_dest, if is_custom { 1 } else { 0 });
                EnableWindow(state.hwnd_btn_browse_dest, if is_custom { 1 } else { 0 });
            }
        }
    }
}

pub fn open_output_folder(hwnd: HWND) {
    if let Ok(guard) = UI_STATE.lock() {
        if let Some(state) = guard.as_ref() {
            if let Some(dir) = &state.last_output_dir {
                let wide_op = to_wide("open");
                let wide_path = to_wide(&dir.to_string_lossy());
                unsafe {
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
}
