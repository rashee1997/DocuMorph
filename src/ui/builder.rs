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
            let text = format!("Preview: {}", preview_name);
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
            let text = format!("Files to convert ({} queued)", count);
            let wide = to_wide(&text);
            unsafe {
                SetWindowTextW(state.hwnd_lbl_count, wide.as_ptr());
            }
        }
    }
}

pub fn build_dialog_controls(hwnd_main: HWND, initial_files: &[PathBuf]) -> UiState {
    let fonts = ControlFonts::new();
    let theme = crate::ui::fluent::FluentTheme::new();

    // Top Header
    create_label(hwnd_main, "DocuMorph", 28, 14, 150, 26, fonts.title);
    create_label(hwnd_main, "Document to PDF Converter", 160, 20, 440, 20, fonts.small);

    // Card 1: Files Queue (Card bounds: y=46..172)
    let hwnd_lbl_count = create_label(hwnd_main, "Files to convert", 38, 56, 320, 18, fonts.bold);
    let hwnd_list = create_listbox(hwnd_main, ID_LIST_FILES, 38, 80, 460, 78, fonts.normal);
    let _ = create_fluent_button(hwnd_main, "Add...", ID_BTN_ADD, 510, 80, 95, 24, fonts.normal);
    let _ = create_fluent_button(hwnd_main, "Remove", ID_BTN_REMOVE, 510, 107, 95, 24, fonts.normal);
    let _ = create_fluent_button(hwnd_main, "Clear", ID_BTN_CLEAR, 510, 134, 95, 24, fonts.normal);

    // Card 2: PDF Format Standard (Card bounds: y=182..244)
    create_label(hwnd_main, "PDF Standard", 38, 190, 200, 18, fonts.bold);
    let hwnd_radio_std = create_radio(
        hwnd_main, "Standard PDF", ID_RADIO_STD, 44, 212, 270, 22, fonts.normal, true, true,
    );
    let hwnd_radio_pdfa = create_radio(
        hwnd_main, "PDF/A-1a (archival)", ID_RADIO_PDFA, 325, 212, 270, 22, fonts.normal, false, false,
    );

    // Card 3: Output Destination (Card bounds: y=254..338)
    create_label(hwnd_main, "Output Destination", 38, 262, 200, 18, fonts.bold);
    let hwnd_radio_dest_src = create_radio(
        hwnd_main, "Same folder as source file", ID_RADIO_DEST_SRC, 44, 282, 250, 20, fonts.normal, true, true,
    );
    let hwnd_radio_dest_cust = create_radio(
        hwnd_main, "Custom folder:", ID_RADIO_DEST_CUST, 44, 304, 115, 24, fonts.normal, false, false,
    );
    let hwnd_edit_dest = create_edit(hwnd_main, "", ID_EDIT_DEST, 165, 304, 335, 24, fonts.normal, true);
    let hwnd_btn_browse_dest = create_fluent_button(hwnd_main, "Browse...", ID_BTN_BROWSE_DEST, 510, 303, 95, 26, fonts.normal);
    unsafe {
        EnableWindow(hwnd_edit_dest, 0);
        EnableWindow(hwnd_btn_browse_dest, 0);
    }

    // Card 4: Renaming & Bulk Renaming (Card bounds: y=348..464)
    create_label(hwnd_main, "Output Filename", 38, 356, 280, 18, fonts.bold);
    create_label(hwnd_main, "Pattern:", 44, 378, 55, 18, fonts.normal);
    let hwnd_edit_pattern = create_edit(hwnd_main, "{name}", ID_EDIT_PATTERN, 100, 376, 115, 24, fonts.normal, false);
    create_label(hwnd_main, "Prefix:", 230, 378, 45, 18, fonts.normal);
    let hwnd_edit_prefix = create_edit(hwnd_main, "", ID_EDIT_PREFIX, 280, 376, 95, 24, fonts.normal, false);
    create_label(hwnd_main, "Suffix:", 390, 378, 45, 18, fonts.normal);
    let hwnd_edit_suffix = create_edit(hwnd_main, "", ID_EDIT_SUFFIX, 440, 376, 165, 24, fonts.normal, false);

    let hwnd_chk_safe = create_checkbox(
        hwnd_main, "Add number if file exists (e.g. file (1).pdf)", ID_CHK_SAFE_COLLISION, 44, 406, 420, 20, fonts.normal, true,
    );
    let hwnd_lbl_preview = create_label(hwnd_main, "Preview: document.pdf", 44, 432, 550, 18, fonts.small);

    // Progress Area (hidden initially, shown during conversion)
    // Layout: status at top, progress bars below, then buttons at bottom
    let hwnd_lbl_status_main = create_label(hwnd_main, "Ready to convert", 28, 476, 580, 20, fonts.bold);
    let hwnd_progress_overall = create_progress_bar(hwnd_main, 201, 28, 498, 580, 14);
    let hwnd_lbl_status_sub = create_label(hwnd_main, "", 28, 522, 580, 18, fonts.small);
    let hwnd_progress_current = create_progress_bar(hwnd_main, 202, 28, 544, 580, 12);

    unsafe {
        ShowWindow(hwnd_progress_overall, SW_HIDE);
        ShowWindow(hwnd_progress_current, SW_HIDE);
        ShowWindow(hwnd_lbl_status_main, SW_HIDE);
        ShowWindow(hwnd_lbl_status_sub, SW_HIDE);
    }

    // Action Buttons at the very bottom with proper margin
    // Window client height ~630, buttons at y=588 with 36px = 624 bottom, 6px margin
    let hwnd_btn_start = create_fluent_button(hwnd_main, "Convert", ID_BTN_START, 350, 588, 150, 36, fonts.bold);
    let hwnd_btn_cancel = create_fluent_button(hwnd_main, "Cancel", ID_BTN_CANCEL, 510, 588, 95, 36, fonts.normal);
    let hwnd_btn_open_dir = create_fluent_button(hwnd_main, "Open Folder", ID_BTN_OPEN_DIR, 28, 588, 175, 36, fonts.normal);
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
        theme,
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
