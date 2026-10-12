use std::sync::mpsc::channel;
use std::thread;
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    PostMessageW, SendMessageW, BM_GETCHECK, SW_HIDE, SW_SHOW, ShowWindow,
};

use crate::app;
use crate::options::{
    CaseTransform, ConversionConfig, NamingOptions, OutputDestination, PdfStandard,
};
use crate::ui::builder::get_edit_text;
use crate::ui::controls::BST_CHECKED;
use crate::ui::window::*;

pub fn start_conversion_flow(hwnd: HWND) {
    let (files, config) = match UI_STATE.lock() {
        Ok(guard) => {
            if let Some(state) = guard.as_ref() {
                if state.queued_files.is_empty() {
                    return;
                }

                let pdf_std = if unsafe { SendMessageW(state.hwnd_radio_pdfa, BM_GETCHECK, 0, 0) } == BST_CHECKED as isize {
                    PdfStandard::PdfA1a
                } else {
                    PdfStandard::Standard
                };

                let is_custom = unsafe { SendMessageW(state.hwnd_radio_dest_cust, BM_GETCHECK, 0, 0) } == BST_CHECKED as isize;
                let destination = if is_custom && state.custom_dest.is_some() {
                    OutputDestination::Custom(state.custom_dest.clone().unwrap())
                } else {
                    OutputDestination::SourceDirectory
                };

                let pattern = get_edit_text(state.hwnd_edit_pattern);
                let prefix = get_edit_text(state.hwnd_edit_prefix);
                let suffix = get_edit_text(state.hwnd_edit_suffix);
                let safe_collision = unsafe { SendMessageW(state.hwnd_chk_safe, BM_GETCHECK, 0, 0) } == BST_CHECKED as isize;

                let naming = NamingOptions {
                    pattern: if pattern.is_empty() { "{name}".into() } else { pattern },
                    prefix,
                    suffix,
                    case_transform: CaseTransform::Original,
                    safe_collision,
                };

                let cfg = ConversionConfig { format: pdf_std, destination, naming };
                (state.queued_files.clone(), cfg)
            } else {
                return;
            }
        }
        Err(_) => return,
    };

    // Transition UI to progress view
    if let Ok(mut guard) = UI_STATE.lock() {
        if let Some(state) = guard.as_mut() {
            state.last_output_dir = match &config.destination {
                OutputDestination::Custom(p) => Some(p.clone()),
                OutputDestination::SourceDirectory => files.first().and_then(|f| f.parent()).map(|p| p.to_path_buf()),
            };

            // Disable options and show progress controls
            unsafe {
                EnableWindow(state.hwnd_btn_start, 0);
                EnableWindow(state.hwnd_btn_cancel, 0);
                ShowWindow(state.hwnd_btn_start, SW_HIDE);
                ShowWindow(state.hwnd_progress_overall, SW_SHOW);
                ShowWindow(state.hwnd_progress_current, SW_SHOW);
                ShowWindow(state.hwnd_lbl_status_main, SW_SHOW);
                ShowWindow(state.hwnd_lbl_status_sub, SW_SHOW);
            }
        }
    }

    let total = files.len();
    let hwnd_target = hwnd as isize;

    thread::spawn(move || {
        let hwnd_w = hwnd_target as HWND;
        let mut success_count = 0;

        for (idx, file) in files.iter().enumerate() {
            let filename = file.file_name().and_then(|n| n.to_str()).unwrap_or("file").to_string();
            let packed_wparam = (idx & 0xFFFF) | ((total & 0xFFFF) << 16);
            let name_boxed = Box::into_raw(Box::new(filename));
            unsafe {
                PostMessageW(hwnd_w, WM_APP_FILE_START, packed_wparam, name_boxed as isize);
            }

            let output_path = crate::options::resolve_target_path(file, idx, total, &config);
            let (tx, rx) = channel::<app::ProgressMessage>();
            let input_clone = file.clone();
            let config_clone = config.clone();

            thread::spawn(move || {
                app::process_conversion(input_clone, output_path, config_clone, tx);
            });

            while let Ok(msg) = rx.recv() {
                match msg {
                    app::ProgressMessage::Update { percent, text } => {
                        let text_boxed = Box::into_raw(Box::new(text));
                        unsafe {
                            PostMessageW(hwnd_w, WM_APP_PROGRESS, percent as usize, text_boxed as isize);
                        }
                    }
                    app::ProgressMessage::Finished { success, .. } => {
                        if success {
                            success_count += 1;
                        }
                        break;
                    }
                }
            }
        }

        unsafe {
            PostMessageW(hwnd_w, WM_APP_FINISHED, success_count, total as isize);
        }
    });
}
