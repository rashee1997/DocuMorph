#![windows_subsystem = "windows"]

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;

use windows_sys::Win32::UI::WindowsAndMessaging::{
    MessageBoxW, MB_ICONINFORMATION, MB_ICONQUESTION, MB_OK, MB_YESNO, IDYES,
};

use documorph::{com, registry};

fn show_dialog(title: &str, text: &str, is_yes_no: bool) -> bool {
    let wide_title = com::to_wide(title);
    let wide_text = com::to_wide(text);
    let flags = if is_yes_no {
        MB_YESNO | MB_ICONQUESTION
    } else {
        MB_OK | MB_ICONINFORMATION
    };
    unsafe {
        let res = MessageBoxW(ptr::null_mut(), wide_text.as_ptr(), wide_title.as_ptr(), flags);
        res == IDYES
    }
}

use std::process::Stdio;

fn schedule_self_deletion(dir: &Path, _exe_path: &Path) {
    let dir_str = dir.to_string_lossy();
    let ps_script = format!(
        "for ($i=0; $i -lt 10; $i++) {{ Start-Sleep -Milliseconds 400; try {{ Remove-Item -LiteralPath '{}' -Force -Recurse -ErrorAction Stop; break }} catch {{}} }}",
        dir_str
    );

    let _ = Command::new("powershell")
        .args(&[
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-WindowStyle",
            "Hidden",
            "-Command",
            &ps_script,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let silent = args.iter().any(|a| a == "--silent" || a == "-s" || a == "/S");

    if !documorph::elevation::is_elevated() {
        if documorph::elevation::relaunch_elevated(&args[1..]) {
            std::process::exit(0);
        }
    }

    if !silent {
        let confirmed = show_dialog(
            "DocuMorph Uninstall",
            "Are you sure you want to completely uninstall DocuMorph and remove all Windows Explorer context menu shortcuts?",
            true,
        );
        if !confirmed {
            return;
        }
    }

    // 1. Unregister Windows context menu verbs (HKCU)
    let _ = registry::unregister();

    // 2. Unregister Windows Add or Remove Programs entry
    let _ = registry::unregister_uninstaller();

    // 3. Remove application files from current directory
    let current_exe = env::current_exe().unwrap_or_else(|_| PathBuf::from("uninstall.exe"));
    let install_dir = current_exe
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    let documorph_exe = install_dir.join("documorph.exe");
    if documorph_exe.exists() {
        let _ = fs::remove_file(&documorph_exe);
    }

    let icon_file = install_dir.join("icon.ico");
    if icon_file.exists() {
        let _ = fs::remove_file(&icon_file);
    }

    // 4. Schedule uninstaller deletion and folder cleanup
    schedule_self_deletion(&install_dir, &current_exe);

    if !silent {
        show_dialog(
            "DocuMorph Uninstall",
            "DocuMorph has been successfully uninstalled from your computer.",
            false,
        );
    }
}
