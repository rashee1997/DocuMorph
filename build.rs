fn main() {
    println!("cargo:rerun-if-changed=resources/icon.ico");
    println!("cargo:rerun-if-changed=resources/app.manifest");
    println!("cargo:rerun-if-changed=build.rs");

    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
        let res_o = out_dir.join("resource.o");
        let scoop_bin = std::path::PathBuf::from(r"C:\Users\user\scoop\apps\mingw\current\bin");
        let windres = scoop_bin.join("windres.exe");

        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap().replace('\\', "/");
        let rc_content = format!(
            r#"#pragma code_page(65001)
1 VERSIONINFO
FILEVERSION 1, 2, 1, 0
PRODUCTVERSION 1, 2, 1, 0
FILEFLAGSMASK 0x3f
FILEFLAGS 0x0
FILEOS 0x40004
FILETYPE 0x1
{{
BLOCK "StringFileInfo"
{{
BLOCK "000004b0"
{{
VALUE "FileDescription", "DocuMorph Document-to-PDF Engine"
VALUE "FileVersion", "1.2.2.0"
VALUE "InternalName", "documorph"
VALUE "LegalCopyright", "Copyright (c) 2026 Rasheedh Ahamad"
VALUE "OriginalFilename", "documorph.exe"
VALUE "ProductName", "DocuMorph"
VALUE "ProductVersion", "1.2.2.0"
}}
}}
BLOCK "VarFileInfo"
{{
VALUE "Translation", 0x0, 0x04b0
}}
}}
1 ICON "{manifest_dir}/resources/icon.ico"
1 24 "{manifest_dir}/resources/app.manifest"
"#
        );
        let rc_path = out_dir.join("app.rc");
        let _ = std::fs::write(&rc_path, rc_content);

        let windres_cmd = if windres.exists() {
            windres
        } else {
            std::path::PathBuf::from("windres")
        };

        let status = std::process::Command::new(&windres_cmd)
            .arg("-i")
            .arg(&rc_path)
            .arg("-o")
            .arg(&res_o)
            .arg("--output-format=coff")
            .status();

        if let Ok(s) = status {
            if s.success() {
                println!("cargo:rustc-link-arg={}", res_o.display());
                return;
            }
        }

        // Fallback to winres if direct windres invocation failed
        let mut res = winres::WindowsResource::new();
        res.set_icon("resources/icon.ico");
        res.set_manifest_file("resources/app.manifest");
        res.set("FileDescription", "DocuMorph Document-to-PDF Engine");
        res.set("ProductName", "DocuMorph");
        res.set("ProductVersion", "1.2.2.0");
        res.set("FileVersion", "1.2.2.0");
        res.set("LegalCopyright", "Copyright (c) 2026 Rasheedh Ahamad");
        res.set("OriginalFilename", "documorph.exe");
        let _ = res.compile();
    }
}
