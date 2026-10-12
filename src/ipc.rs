use std::path::PathBuf;
use std::ptr;
use std::sync::mpsc::Sender;
use std::thread;

use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_ALREADY_EXISTS, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, ReadFile, WriteFile, FILE_ATTRIBUTE_NORMAL, OPEN_EXISTING,
};
use windows_sys::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_READMODE_MESSAGE,
    PIPE_TYPE_MESSAGE, PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
};
use windows_sys::Win32::System::Threading::CreateMutexW;

use crate::com::to_wide;

const MUTEX_NAME: &str = r"Local\DocuMorph_SingleInstance_Mutex";
const PIPE_NAME: &str = r"\\.\pipe\DocuMorph_Queue_Pipe";
const PIPE_ACCESS_INBOUND: u32 = 1;

pub enum IpcMode {
    Primary(HANDLE),
    Secondary,
}

/// Checks whether an existing DocuMorph window is running
pub fn check_single_instance(file_arg: Option<&PathBuf>) -> IpcMode {
    unsafe {
        let wide_mutex = to_wide(MUTEX_NAME);
        let h_mutex = CreateMutexW(ptr::null(), 0, wide_mutex.as_ptr());
        let last_err = windows_sys::Win32::Foundation::GetLastError();

        if last_err == ERROR_ALREADY_EXISTS {
            // Another instance is already running! Send the file argument to it via Named Pipe
            if let Some(path) = file_arg {
                send_file_to_primary(path);
            }
            if !h_mutex.is_null() {
                CloseHandle(h_mutex);
            }
            return IpcMode::Secondary;
        }

        IpcMode::Primary(h_mutex)
    }
}

/// Sends a file path to the running primary instance through the named pipe
fn send_file_to_primary(path: &PathBuf) {
    unsafe {
        let wide_pipe = to_wide(PIPE_NAME);
        let h_pipe = CreateFileW(
            wide_pipe.as_ptr(),
            GENERIC_WRITE,
            0,
            ptr::null(),
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            ptr::null_mut(),
        );

        if h_pipe != INVALID_HANDLE_VALUE {
            let path_str = path.to_string_lossy().to_string();
            let bytes = path_str.as_bytes();
            let mut written = 0u32;
            WriteFile(
                h_pipe,
                bytes.as_ptr(),
                bytes.len() as u32,
                &mut written,
                ptr::null_mut(),
            );
            CloseHandle(h_pipe);
        }
    }
}

/// Starts the Named Pipe server thread on the primary instance to listen for newly queued files
pub fn start_pipe_server(sender: Sender<PathBuf>) {
    thread::spawn(move || {
        let wide_pipe = to_wide(PIPE_NAME);
        unsafe {
            loop {
                let h_pipe = CreateNamedPipeW(
                    wide_pipe.as_ptr(),
                    PIPE_ACCESS_INBOUND,
                    PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE | PIPE_WAIT,
                    PIPE_UNLIMITED_INSTANCES,
                    4096,
                    4096,
                    0,
                    ptr::null(),
                );

                if h_pipe == INVALID_HANDLE_VALUE {
                    break;
                }

                let connected = ConnectNamedPipe(h_pipe, ptr::null_mut());
                if connected != 0 || windows_sys::Win32::Foundation::GetLastError() == 535 {
                    // ERROR_PIPE_CONNECTED
                    let mut buffer = [0u8; 4096];
                    let mut bytes_read = 0u32;
                    let success = ReadFile(
                        h_pipe,
                        buffer.as_mut_ptr(),
                        buffer.len() as u32,
                        &mut bytes_read,
                        ptr::null_mut(),
                    );

                    if success != 0 && bytes_read > 0 {
                        if let Ok(path_str) = std::str::from_utf8(&buffer[..bytes_read as usize]) {
                            let path = PathBuf::from(path_str.trim());
                            if path.exists() {
                                let _ = sender.send(path);
                            }
                        }
                    }
                }

                DisconnectNamedPipe(h_pipe);
                CloseHandle(h_pipe);
            }
        }
    });
}
