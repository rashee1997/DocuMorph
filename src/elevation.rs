use std::env;
use std::ptr;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
use windows_sys::Win32::Security::{
    GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

use crate::com::to_wide;

/// Checks whether the current process is running with elevated (Administrator) privileges
pub fn is_elevated() -> bool {
    unsafe {
        let mut handle: HANDLE = ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut handle) != 0 {
            let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
            let mut size = std::mem::size_of::<TOKEN_ELEVATION>() as u32;
            let res = GetTokenInformation(
                handle,
                TokenElevation,
                &mut elevation as *mut _ as *mut _,
                size,
                &mut size,
            );
            CloseHandle(handle);
            res != 0 && elevation.TokenIsElevated != 0
        } else {
            false
        }
    }
}

/// Relaunches the current executable with administrative privileges via Windows UAC
pub fn relaunch_elevated(extra_args: &[String]) -> bool {
    if let Ok(exe_path) = env::current_exe() {
        let wide_exe = to_wide(&exe_path.to_string_lossy());
        let wide_verb = to_wide("runas");
        let args_str = extra_args.join(" ");
        let wide_args = to_wide(&args_str);

        unsafe {
            let res = ShellExecuteW(
                ptr::null_mut(),
                wide_verb.as_ptr(),
                wide_exe.as_ptr(),
                if extra_args.is_empty() { ptr::null() } else { wide_args.as_ptr() },
                ptr::null(),
                SW_SHOWNORMAL,
            );
            res as usize > 32
        }
    } else {
        false
    }
}
