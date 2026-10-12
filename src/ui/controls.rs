use std::env;
use std::fs;
use std::path::PathBuf;
use std::ptr;

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::Graphics::Gdi::{CreateFontW, FW_BOLD, FW_NORMAL, HFONT};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Controls::{PBM_SETRANGE32, PBS_SMOOTH};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, LoadImageW, SendMessageW, BM_SETCHECK, ES_AUTOHSCROLL, ES_READONLY,
    HICON, HMENU, IMAGE_ICON, LR_LOADFROMFILE, LBS_NOTIFY, WS_BORDER, WS_CHILD, WS_GROUP,
    WS_TABSTOP, WS_VISIBLE, WS_VSCROLL, BS_AUTOCHECKBOX, BS_AUTORADIOBUTTON, BS_GROUPBOX,
    BS_PUSHBUTTON, BS_OWNERDRAW, WM_SETFONT, WM_SETICON, ICON_BIG, ICON_SMALL,
};

pub const BST_CHECKED: usize = 1;
pub const BST_UNCHECKED: usize = 0;
pub const SS_LEFT: u32 = 0;

use crate::com::to_wide;
use crate::registry::ICON_RAW_BYTES;

pub struct ControlFonts {
    pub title: HFONT,
    pub bold: HFONT,
    pub normal: HFONT,
    pub small: HFONT,
}

impl ControlFonts {
    pub fn new() -> Self {
        unsafe {
            let font_face = to_wide("Segoe UI");
            let title = CreateFontW(
                20, 0, 0, 0, FW_BOLD as i32, 0, 0, 0,
                0, 0, 0, 5, 0, font_face.as_ptr(),
            );
            let bold = CreateFontW(
                15, 0, 0, 0, FW_BOLD as i32, 0, 0, 0,
                0, 0, 0, 5, 0, font_face.as_ptr(),
            );
            let normal = CreateFontW(
                14, 0, 0, 0, FW_NORMAL as i32, 0, 0, 0,
                0, 0, 0, 5, 0, font_face.as_ptr(),
            );
            let small = CreateFontW(
                12, 0, 0, 0, FW_NORMAL as i32, 0, 0, 0,
                0, 0, 0, 5, 0, font_face.as_ptr(),
            );
            ControlFonts { title, bold, normal, small }
        }
    }
}

pub fn create_label(parent: HWND, text: &str, x: i32, y: i32, w: i32, h: i32, font: HFONT) -> HWND {
    unsafe {
        let class = to_wide("STATIC");
        let wide_text = to_wide(text);
        let hinstance = GetModuleHandleW(ptr::null());
        let hwnd = CreateWindowExW(
            0,
            class.as_ptr(),
            wide_text.as_ptr(),
            WS_CHILD | WS_VISIBLE | (SS_LEFT as u32),
            x, y, w, h,
            parent,
            ptr::null_mut(),
            hinstance,
            ptr::null_mut(),
        );
        SendMessageW(hwnd, WM_SETFONT, font as usize, 1);
        hwnd
    }
}

pub fn create_groupbox(parent: HWND, text: &str, x: i32, y: i32, w: i32, h: i32, font: HFONT) -> HWND {
    unsafe {
        let class = to_wide("BUTTON");
        let wide_text = to_wide(text);
        let hinstance = GetModuleHandleW(ptr::null());
        let hwnd = CreateWindowExW(
            0,
            class.as_ptr(),
            wide_text.as_ptr(),
            WS_CHILD | WS_VISIBLE | (BS_GROUPBOX as u32),
            x, y, w, h,
            parent,
            ptr::null_mut(),
            hinstance,
            ptr::null_mut(),
        );
        SendMessageW(hwnd, WM_SETFONT, font as usize, 1);
        hwnd
    }
}

pub fn create_button(
    parent: HWND,
    text: &str,
    id: usize,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    font: HFONT,
) -> HWND {
    unsafe {
        let class = to_wide("BUTTON");
        let wide_text = to_wide(text);
        let hinstance = GetModuleHandleW(ptr::null());
        let hwnd = CreateWindowExW(
            0,
            class.as_ptr(),
            wide_text.as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | (BS_PUSHBUTTON as u32),
            x, y, w, h,
            parent,
            id as HMENU,
            hinstance,
            ptr::null_mut(),
        );
        SendMessageW(hwnd, WM_SETFONT, font as usize, 1);
        hwnd
    }
}

pub fn create_fluent_button(
    parent: HWND,
    text: &str,
    id: usize,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    font: HFONT,
) -> HWND {
    unsafe {
        let class = to_wide("BUTTON");
        let wide_text = to_wide(text);
        let hinstance = GetModuleHandleW(ptr::null());
        let hwnd = CreateWindowExW(
            0,
            class.as_ptr(),
            wide_text.as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | (BS_OWNERDRAW as u32),
            x, y, w, h,
            parent,
            id as HMENU,
            hinstance,
            ptr::null_mut(),
        );
        SendMessageW(hwnd, WM_SETFONT, font as usize, 1);
        hwnd
    }
}

pub fn create_radio(
    parent: HWND,
    text: &str,
    id: usize,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    font: HFONT,
    checked: bool,
    is_group_start: bool,
) -> HWND {
    unsafe {
        let class = to_wide("BUTTON");
        let wide_text = to_wide(text);
        let hinstance = GetModuleHandleW(ptr::null());
        let mut style = WS_CHILD | WS_VISIBLE | WS_TABSTOP | (BS_AUTORADIOBUTTON as u32);
        if is_group_start {
            style |= WS_GROUP;
        }
        let hwnd = CreateWindowExW(
            0,
            class.as_ptr(),
            wide_text.as_ptr(),
            style,
            x, y, w, h,
            parent,
            id as HMENU,
            hinstance,
            ptr::null_mut(),
        );
        SendMessageW(hwnd, WM_SETFONT, font as usize, 1);
        SendMessageW(
            hwnd,
            BM_SETCHECK,
            if checked { BST_CHECKED as usize } else { BST_UNCHECKED as usize },
            0,
        );
        hwnd
    }
}

pub fn create_checkbox(
    parent: HWND,
    text: &str,
    id: usize,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    font: HFONT,
    checked: bool,
) -> HWND {
    unsafe {
        let class = to_wide("BUTTON");
        let wide_text = to_wide(text);
        let hinstance = GetModuleHandleW(ptr::null());
        let hwnd = CreateWindowExW(
            0,
            class.as_ptr(),
            wide_text.as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | (BS_AUTOCHECKBOX as u32),
            x, y, w, h,
            parent,
            id as HMENU,
            hinstance,
            ptr::null_mut(),
        );
        SendMessageW(hwnd, WM_SETFONT, font as usize, 1);
        SendMessageW(
            hwnd,
            BM_SETCHECK,
            if checked { BST_CHECKED as usize } else { BST_UNCHECKED as usize },
            0,
        );
        hwnd
    }
}

pub fn create_edit(
    parent: HWND,
    text: &str,
    id: usize,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    font: HFONT,
    readonly: bool,
) -> HWND {
    unsafe {
        let class = to_wide("EDIT");
        let wide_text = to_wide(text);
        let hinstance = GetModuleHandleW(ptr::null());
        let mut style = WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_BORDER | (ES_AUTOHSCROLL as u32);
        if readonly {
            style |= ES_READONLY as u32;
        }
        let hwnd = CreateWindowExW(
            0,
            class.as_ptr(),
            wide_text.as_ptr(),
            style,
            x, y, w, h,
            parent,
            id as HMENU,
            hinstance,
            ptr::null_mut(),
        );
        SendMessageW(hwnd, WM_SETFONT, font as usize, 1);
        hwnd
    }
}

pub fn create_listbox(
    parent: HWND,
    id: usize,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    font: HFONT,
) -> HWND {
    unsafe {
        let class = to_wide("LISTBOX");
        let hinstance = GetModuleHandleW(ptr::null());
        let style = WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_BORDER | WS_VSCROLL | (LBS_NOTIFY as u32);
        let hwnd = CreateWindowExW(
            0,
            class.as_ptr(),
            ptr::null(),
            style,
            x, y, w, h,
            parent,
            id as HMENU,
            hinstance,
            ptr::null_mut(),
        );
        SendMessageW(hwnd, WM_SETFONT, font as usize, 1);
        hwnd
    }
}

pub fn create_progress_bar(parent: HWND, id: usize, x: i32, y: i32, w: i32, h: i32) -> HWND {
    unsafe {
        let class = to_wide("msctls_progress32");
        let hinstance = GetModuleHandleW(ptr::null());
        let hwnd = CreateWindowExW(
            0,
            class.as_ptr(),
            ptr::null(),
            WS_CHILD | WS_VISIBLE | PBS_SMOOTH,
            x, y, w, h,
            parent,
            id as HMENU,
            hinstance,
            ptr::null_mut(),
        );
        SendMessageW(hwnd, PBM_SETRANGE32, 0, 100);
        hwnd
    }
}

/// Discovers icon.ico on disk or extracts embedded icon, and applies it to the window
pub fn apply_window_icon(hwnd: HWND) {
    let mut icon_path = env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.join("icon.ico")))
        .unwrap_or_else(|| PathBuf::from("icon.ico"));

    if !icon_path.exists() {
        let dev_icon = PathBuf::from("resources/icon.ico");
        if dev_icon.exists() {
            icon_path = dev_icon;
        } else {
            let temp_icon = env::temp_dir().join("documorph_icon.ico");
            let _ = fs::write(&temp_icon, ICON_RAW_BYTES);
            icon_path = temp_icon;
        }
    }

    unsafe {
        let wide_path = to_wide(&icon_path.to_string_lossy());
        let hicon_big: HICON = LoadImageW(
            ptr::null_mut(),
            wide_path.as_ptr(),
            IMAGE_ICON,
            32,
            32,
            LR_LOADFROMFILE,
        );
        let hicon_sm: HICON = LoadImageW(
            ptr::null_mut(),
            wide_path.as_ptr(),
            IMAGE_ICON,
            16,
            16,
            LR_LOADFROMFILE,
        );

        if !hicon_big.is_null() {
            SendMessageW(hwnd, WM_SETICON, ICON_BIG as usize, hicon_big as isize);
        }
        if !hicon_sm.is_null() {
            SendMessageW(hwnd, WM_SETICON, ICON_SMALL as usize, hicon_sm as isize);
        }
    }
}
