use std::path::PathBuf;
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetWindowTextLengthW, GetWindowTextW, SendMessageW, SetWindowTextW, LB_ADDSTRING, SW_HIDE, ShowWindow,
};

use crate::com::to_wide;
use crate::options::{render_filename, CaseTransform, NamingOptions};
use crate::ui::controls::*;
use crate::ui::window::*;

pub fn get_edit_text(hwnd: HWND) -> String {
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

pub fn update_preview() {
    if let Ok(guard) = UI_STATE.lock() {
        if let Some(state) = guard.as_ref() {
            let sample_stem = state
                .queued_files
                .first()
                .and_then(|p| p.file_stem())
                .and_then(|s| s.to_str())
                .unwrap_or("document");

            let pattern = get_edit_text(state.hwnd_edit_pattern);
            let prefix = get_edit_text(state.hwnd_edit_prefix);
            let suffix = get_edit_text(state.hwnd_edit_suffix);

            let opts = NamingOptions {
                pattern: if pattern.is_empty() { "{name}".into() } else { pattern },
                prefix,
                suffix,
                case_transform: CaseTransform::Original,
                safe_collision: true,
            };

            let preview_name = render_filename(sample_stem, 0, state.queued_files.len(), &opts);
            let text = format!("Output Preview: {}", preview_name);
            let wide = to_wide(&text);
            unsafe {
                SetWindowTextW(state.hwnd_lbl_preview, wide.as_ptr());
            }
        }
    }
}

pub fn update_file_count() {
    if let Ok(guard) = UI_STATE.lock() {
        if let Some(state) = guard.as_ref() {
            let count = state.queued_files.len();
            let text = format!("Documents to Convert ({} queued):", count);
            let wide = to_wide(&text);
            unsafe {
                SetWindowTextW(state.hwnd_lbl_count, wide.as_ptr());
            }
        }
    }
}

pub fn build_dialog_controls(hwnd_main: HWND, initial_files: &[PathBuf]) -> UiState {
    let fonts = ControlFonts::new();

    // Banner Title
    create_label(hwnd_main, "DocuMorph", 20, 14, 150, 26, fonts.title);
    create_label(
        hwnd_main,
        "High-Fidelity Document to PDF Engine (Embedded Fonts)",
        160, 20, 420, 20, fonts.normal,
    );

    // Section 1: Files Queue
    let hwnd_lbl_count = create_label(hwnd_main, "Documents to Convert:", 20, 48, 400, 18, fonts.bold);
    let hwnd_list = create_listbox(hwnd_main, ID_LIST_FILES, 20, 70, 460, 85, fonts.normal);
    let _ = create_button(hwnd_main, "+ Add...", ID_BTN_ADD, 490, 70, 105, 26, fonts.normal);
    let _ = create_button(hwnd_main, "- Remove", ID_BTN_REMOVE, 490, 100, 105, 25, fonts.normal);
    let _ = create_button(hwnd_main, "Clear", ID_BTN_CLEAR, 490, 129, 105, 25, fonts.normal);

    // Section 2: PDF Format Standard
    let _ = create_groupbox(hwnd_main, " PDF Standard ", 20, 165, 575, 55, fonts.bold);
    let hwnd_radio_std = create_radio(
        hwnd_main, "Standard PDF (Optimized for Screen & Print)", ID_RADIO_STD, 32, 186, 275, 22, fonts.normal, true, true,
    );
    let hwnd_radio_pdfa = create_radio(
        hwnd_main, "PDF/A-1a Archival (ISO 19005-1 Compliance)", ID_RADIO_PDFA, 315, 186, 270, 22, fonts.normal, false, false,
    );

    // Section 3: Output Destination
    let _ = create_groupbox(hwnd_main, " Output Destination ", 20, 228, 575, 75, fonts.bold);
    let hwnd_radio_dest_src = create_radio(
        hwnd_main, "Same folder as source document", ID_RADIO_DEST_SRC, 32, 248, 250, 20, fonts.normal, true, true,
    );
    let hwnd_radio_dest_cust = create_radio(
        hwnd_main, "Custom destination:", ID_RADIO_DEST_CUST, 32, 273, 140, 22, fonts.normal, false, false,
    );
    let hwnd_edit_dest = create_edit(hwnd_main, "", ID_EDIT_DEST, 175, 273, 305, 22, fonts.normal, true);
    let hwnd_btn_browse_dest = create_button(hwnd_main, "Browse...", ID_BTN_BROWSE_DEST, 490, 272, 95, 24, fonts.normal);
    unsafe {
        EnableWindow(hwnd_edit_dest, 0);
        EnableWindow(hwnd_btn_browse_dest, 0);
    }

    // Section 4: Renaming & Bulk Renaming
    let _ = create_groupbox(hwnd_main, " Output Filename & Renaming ", 20, 312, 575, 105, fonts.bold);
    create_label(hwnd_main, "Pattern:", 32, 334, 55, 18, fonts.normal);
    let hwnd_edit_pattern = create_edit(hwnd_main, "{name}", ID_EDIT_PATTERN, 90, 332, 120, 22, fonts.normal, false);
    create_label(hwnd_main, "Prefix:", 225, 334, 45, 18, fonts.normal);
    let hwnd_edit_prefix = create_edit(hwnd_main, "", ID_EDIT_PREFIX, 272, 332, 100, 22, fonts.normal, false);
    create_label(hwnd_main, "Suffix:", 385, 334, 45, 18, fonts.normal);
    let hwnd_edit_suffix = create_edit(hwnd_main, "", ID_EDIT_SUFFIX, 432, 332, 150, 22, fonts.normal, false);

    let hwnd_chk_safe = create_checkbox(
        hwnd_main, "Append sequential number (1), (2) if file already exists", ID_CHK_SAFE_COLLISION, 32, 360, 420, 22, fonts.normal, true,
    );
    let hwnd_lbl_preview = create_label(hwnd_main, "Output Preview: document.pdf", 32, 388, 550, 20, fonts.small);

    // Progress Area (hidden initially)
    let hwnd_lbl_status_main = create_label(hwnd_main, "Ready to convert.", 20, 425, 575, 22, fonts.bold);
    let hwnd_progress_overall = create_progress_bar(hwnd_main, 201, 20, 450, 575, 16);
    let hwnd_lbl_status_sub = create_label(hwnd_main, "", 20, 470, 575, 18, fonts.small);
    let hwnd_progress_current = create_progress_bar(hwnd_main, 202, 20, 490, 575, 14);

    unsafe {
        ShowWindow(hwnd_progress_overall, SW_HIDE);
        ShowWindow(hwnd_progress_current, SW_HIDE);
        ShowWindow(hwnd_lbl_status_main, SW_HIDE);
        ShowWindow(hwnd_lbl_status_sub, SW_HIDE);
    }

    // Action Buttons
    let hwnd_btn_start = create_button(hwnd_main, "Start Conversion", ID_BTN_START, 350, 515, 135, 30, fonts.bold);
    let hwnd_btn_cancel = create_button(hwnd_main, "Cancel", ID_BTN_CANCEL, 495, 515, 100, 30, fonts.normal);
    let hwnd_btn_open_dir = create_button(hwnd_main, "Open Destination Folder", ID_BTN_OPEN_DIR, 20, 515, 180, 30, fonts.normal);
    unsafe {
        ShowWindow(hwnd_btn_open_dir, SW_HIDE);
    }

    // Populate initial files
    let mut queued_list = Vec::new();
    for f in initial_files {
        if f.exists() {
            let name = f.file_name().and_then(|n| n.to_str()).unwrap_or("file");
            let wide = to_wide(name);
            unsafe {
                SendMessageW(hwnd_list, LB_ADDSTRING, 0, wide.as_ptr() as isize);
            }
            queued_list.push(f.clone());
        }
    }

    UiState {
        hwnd_main,
        fonts,
        queued_files: queued_list,
        custom_dest: None,
        last_output_dir: None,
        hwnd_list,
        hwnd_lbl_count,
        hwnd_radio_std,
        hwnd_radio_pdfa,
        hwnd_radio_dest_src,
        hwnd_radio_dest_cust,
        hwnd_edit_dest,
        hwnd_btn_browse_dest,
        hwnd_edit_pattern,
        hwnd_edit_prefix,
        hwnd_edit_suffix,
        hwnd_chk_safe,
        hwnd_lbl_preview,
        hwnd_btn_start,
        hwnd_btn_cancel,
        hwnd_progress_overall,
        hwnd_progress_current,
        hwnd_lbl_status_main,
        hwnd_lbl_status_sub,
        hwnd_btn_open_dir,
    }
}
