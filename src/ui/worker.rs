use std::sync::{mpsc::{channel, RecvTimeoutError}, Arc, Mutex};
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

            // Show progress controls, hide start button, keep cancel enabled for emergency stop
            unsafe {
                EnableWindow(state.hwnd_btn_start, 0);
                ShowWindow(state.hwnd_btn_start, SW_HIDE);
                
                // Ensure cancel button is enabled - user must be able to stop at any time
                EnableWindow(state.hwnd_btn_cancel, 1);
                
                ShowWindow(state.hwnd_progress_overall, SW_SHOW);
                ShowWindow(state.hwnd_progress_current, SW_SHOW);
                ShowWindow(state.hwnd_lbl_status_main, SW_SHOW);
                ShowWindow(state.hwnd_lbl_status_sub, SW_SHOW);
                
                // Force immediate redraw of the window to show progress UI
                windows_sys::Win32::Graphics::Gdi::InvalidateRect(state.hwnd_main, std::ptr::null(), 1);
                windows_sys::Win32::Graphics::Gdi::UpdateWindow(state.hwnd_main);
            }
        }
    }

    crate::ui::anim::start_anim(hwnd);

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
            let tx_clone = tx.clone(); // Clone for sending error messages from main thread
            let input_clone = file.clone();
            let config_clone = config.clone();

            // Track thread completion with Arc<Mutex<bool>>
            let thread_done = Arc::new(Mutex::new(false));
            let thread_done_clone = thread_done.clone();

            let conversion_handle = thread::spawn(move || {
                let result = app::process_conversion(input_clone, output_path, config_clone, tx);
                // Mark thread as done regardless of success/failure
                *thread_done_clone.lock().unwrap() = true;
                result
            });

            // Wait for conversion to complete or fail, with a safety timeout
            let mut timeout_count = 0usize;
            const MAX_TIMEOUTS: usize = 10; // 10 timeouts * 30 seconds = 5 minutes max wait
            
            loop {
                match rx.recv_timeout(std::time::Duration::from_secs(30)) {
                    Ok(msg) => {
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
                    Err(RecvTimeoutError::Timeout) => {
                        // Check if the thread has finished but didn't send a message
                        if *thread_done.lock().unwrap() {
                            // Thread finished but didn't send Finished message
                            let _ = tx_clone.send(app::ProgressMessage::Finished {
                                success: false,
                                message: "Conversion failed: no result received".to_string(),
                                output_path: None,
                            });
                            break;
                        }
                        
                        timeout_count += 1;
                        if timeout_count >= MAX_TIMEOUTS {
                            // Exceeded maximum wait time - force cancel
                            let _ = tx_clone.send(app::ProgressMessage::Finished {
                                success: false,
                                message: "Conversion timed out: operation took too long".to_string(),
                                output_path: None,
                            });
                            break;
                        }
                        // Continue waiting - COM operations can take time
                    }
                    Err(RecvTimeoutError::Disconnected) => {
                        // Sender dropped without sending Finished - conversion thread ended unexpectedly
                        let _ = tx_clone.send(app::ProgressMessage::Finished {
                            success: false,
                            message: "Conversion failed: process ended unexpectedly".to_string(),
                            output_path: None,
                        });
                        break;
                    }
                }
            }

            // Join the thread to catch any panics and clean up
            let _ = conversion_handle.join();
        }

        unsafe {
            PostMessageW(hwnd_w, WM_APP_FINISHED, success_count, total as isize);
        }
    });
}
