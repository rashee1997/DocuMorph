fn main() {
    println!("cargo:rerun-if-changed=resources/icon.ico");
    println!("cargo:rerun-if-changed=resources/app.manifest");
    println!("cargo:rerun-if-changed=build.rs");

    // Read version from Cargo.toml
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap().replace('\\', "/");
    let cargo_toml_path = std::path::Path::new(&manifest_dir).join("Cargo.toml");
    let cargo_toml_content = std::fs::read_to_string(&cargo_toml_path).unwrap_or_default();
    
    // Extract version from Cargo.toml (format: version = "1.2.2")
    let version = cargo_toml_content
        .lines()
        .find_map(|line| {
            let line = line.trim();
            if line.starts_with("version") {
                Some(line.split('=').nth(1)?.trim().trim_matches('"').to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "1.2.2".to_string());
    
    let version_parts: Vec<&str> = version.split('.').collect();
    let major = version_parts.first().unwrap_or(&"1");
    let minor = version_parts.get(1).unwrap_or(&"0");
    let patch = version_parts.get(2).unwrap_or(&"0");
    let build = "0"; // Build number
    
    // Version string for resources (e.g., "1.2.2.0")
    let version_full = format!("{}.{}.{}.{}", major, minor, patch, build);

    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
        let res_o = out_dir.join("resource.o");
        let scoop_bin = std::path::PathBuf::from(r"C:\Users\user\scoop\apps\mingw\current\bin");
        let windres = scoop_bin.join("windres.exe");

        let rc_content = format!(
            r#"#pragma code_page(65001)
1 VERSIONINFO
FILEVERSION {major}, {minor}, {patch}, {build}
PRODUCTVERSION {major}, {minor}, {patch}, {build}
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
VALUE "FileVersion", "{version_full}"
VALUE "InternalName", "documorph"
VALUE "LegalCopyright", "Copyright (c) 2026 Rasheedh Ahamad"
VALUE "OriginalFilename", "documorph.exe"
VALUE "ProductName", "DocuMorph"
VALUE "ProductVersion", "{version_full}"
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
        , major = major, minor = minor, patch = patch, build = build, version_full = version_full, manifest_dir = manifest_dir);
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
        res.set("ProductVersion", &version_full);
        res.set("FileVersion", &version_full);
        res.set("LegalCopyright", "Copyright (c) 2026 Rasheedh Ahamad");
        res.set("OriginalFilename", "documorph.exe");
        let _ = res.compile();
    }
}
