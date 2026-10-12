fn main() {
    // Only embed Windows resources when compiling for Windows target
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        let mut res = winres::WindowsResource::new();
        res.set_icon("resources/icon.ico");
        res.set_manifest_file("resources/app.manifest");
        res.set("FileDescription", "DocuMorph Document-to-PDF Engine");
        res.set("ProductName", "DocuMorph");
        res.set("ProductVersion", "1.0.0.0");
        res.set("FileVersion", "1.0.0.0");
        res.set("LegalCopyright", "Copyright (c) 2026 Rasheed Hameed");
        res.set("OriginalFilename", "documorph.exe");
        if let Err(e) = res.compile() {
            eprintln!("Warning: failed to compile Windows resources: {}", e);
        }
    }
}
