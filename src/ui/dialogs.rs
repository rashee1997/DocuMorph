use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::System::Com::CoTaskMemFree;
use windows_sys::Win32::UI::Controls::Dialogs::{
    GetOpenFileNameW, OFN_ALLOWMULTISELECT, OFN_EXPLORER, OFN_FILEMUSTEXIST, OPENFILENAMEW,
};
use windows_sys::Win32::UI::Shell::{
    SHBrowseForFolderW, SHGetPathFromIDListW, BIF_NEWDIALOGSTYLE, BIF_RETURNONLYFSDIRS, BROWSEINFOW,
};

use crate::com::to_wide;

/// Opens the native Windows folder picker dialog
pub fn pick_folder(parent: HWND) -> Option<PathBuf> {
    unsafe {
        let title = to_wide("Select Output Folder for Converted PDFs");
        let mut bi: BROWSEINFOW = std::mem::zeroed();
        bi.hwndOwner = parent;
        bi.lpszTitle = title.as_ptr();
        bi.ulFlags = BIF_RETURNONLYFSDIRS | BIF_NEWDIALOGSTYLE;

        let pidl = SHBrowseForFolderW(&bi);
        if !pidl.is_null() {
            let mut path_buf = [0u16; 260];
            let ok = SHGetPathFromIDListW(pidl, path_buf.as_mut_ptr());
            CoTaskMemFree(pidl as _);
            if ok != 0 {
                let len = path_buf.iter().position(|&c| c == 0).unwrap_or(path_buf.len());
                let os_str = OsString::from_wide(&path_buf[..len]);
                return Some(PathBuf::from(os_str));
            }
        }
        None
    }
}

/// Parses the multi-string buffer returned by GetOpenFileNameW with OFN_ALLOWMULTISELECT
pub fn parse_open_file_name(buffer: &[u16]) -> Vec<PathBuf> {
    let mut parts: Vec<String> = Vec::new();
    let mut start = 0;

    for i in 0..buffer.len() {
        if buffer[i] == 0 {
            if start < i {
                let s = String::from_utf16_lossy(&buffer[start..i]);
                parts.push(s);
            }
            start = i + 1;
            if i + 1 < buffer.len() && buffer[i + 1] == 0 {
                break;
            }
        }
    }

    if parts.is_empty() {
        return Vec::new();
    }

    if parts.len() == 1 {
        // Single file selected
        return vec![PathBuf::from(&parts[0])];
    }

    // First part is the directory, remaining parts are filenames
    let dir = PathBuf::from(&parts[0]);
    parts[1..]
        .iter()
        .map(|name| dir.join(name))
        .collect()
}

/// Opens the native Windows multi-file picker for supported document formats
pub fn pick_documents(parent: HWND) -> Vec<PathBuf> {
    unsafe {
        let filter = to_wide(
            "Supported Documents (*.docx, *.doc, *.pptx, *.ppt, *.xlsx, *.xls)\0*.docx;*.doc;*.pptx;*.ppt;*.xlsx;*.xls\0Word Documents (*.docx, *.doc)\0*.docx;*.doc\0PowerPoint (*.pptx, *.ppt)\0*.pptx;*.ppt\0Excel (*.xlsx, *.xls)\0*.xlsx;*.xls\0All Files (*.*)\0*.*\0\0",
        );
        let mut buffer = vec![0u16; 32768];
        let mut ofn: OPENFILENAMEW = std::mem::zeroed();
        ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
        ofn.hwndOwner = parent;
        ofn.lpstrFilter = filter.as_ptr();
        ofn.lpstrFile = buffer.as_mut_ptr();
        ofn.nMaxFile = buffer.len() as u32;
        ofn.Flags = OFN_ALLOWMULTISELECT | OFN_EXPLORER | OFN_FILEMUSTEXIST;

        if GetOpenFileNameW(&mut ofn) != 0 {
            parse_open_file_name(&buffer)
        } else {
            Vec::new()
        }
    }
}
