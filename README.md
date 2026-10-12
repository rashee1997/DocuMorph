# DocuMorph

> **Lightning-fast, high-fidelity Windows context-menu document-to-PDF engine with TrueType font embedding, options dialog, bulk queueing, and automated installer.**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Platform: Windows](https://img.shields.io/badge/Platform-Windows%2010%20%7C%2011-0078D6.svg)](https://microsoft.com/windows)
[![Rust: 1.80+](https://img.shields.io/badge/Rust-1.80%2B-DEA584.svg)](https://www.rust-lang.org/)
[![Version: 1.1.0](https://img.shields.io/badge/Version-1.1.0-brightgreen.svg)](CHANGELOG.md)

---

## Features

- 🖱️ **Windows Explorer Context Menu**: Right-click `.docx`, `.doc`, `.pptx`, `.ppt`, `.xlsx`, or `.xls` to convert.
- 🎨 **Crimson Icon**: Custom icon displayed directly in the Explorer context menu and window title bar.
- ⚙️ **Options & Bulk Dialog**: Interactive dialog with queue management, PDF standard selection, folder routing, and renaming templates.
- 🏛️ **PDF/A-1a Archival Compliance**: Full support for ISO 19005-1 compliant PDF/A archival documents.
- 🔤 **100% TrueType Font Embedding**: Leverages native Office COM automation to embed genuine document fonts with zero fallback substitution.
- 📦 **Automated Setup Wizard**: Double-click `DocuMorph-Setup.exe` to install to `C:\DocuMorph` or a custom directory with 1-click uninstaller.
- 🗑️ **Clean Uninstaller**: Easily uninstalls via Windows Settings "Installed Apps" or `uninstall.exe`.
- 🔄 **Single-Instance IPC Queue**: Multiple files selected in File Explorer automatically consolidate into one unified batch queue via Named Pipes.
- ⚡ **Lightweight & Fast**: Built in pure Rust (~390 KB engine, ~1 MB all-in-one setup wizard).

---

## Quick Start

### 1. Automated Installation
Double-click `DocuMorph-Setup.exe` and click **Install**.

Or install silently via command line:
```powershell
.\DocuMorph-Setup.exe --silent "C:\DocuMorph"
```

### 2. Convert Documents
Right-click any Word, PowerPoint, or Excel document in Windows File Explorer and select:
**"Convert to PDF with DocuMorph"**.

Configure options (Standard PDF vs PDF/A-1a, destination, renaming pattern) and click **"Start Conversion"**.

---

## Build from Source

Built with the lightweight GNU toolchain (`stable-x86_64-pc-windows-gnu`):

```powershell
cd F:\DocuMorph
cargo build --release --bins
```

Produces:
- `target/release/documorph.exe`: Conversion engine
- `target/release/installer.exe`: Setup wizard
- `target/release/uninstaller.exe`: Clean uninstaller

---

## Command Line Usage

```
DocuMorph v1.1.0
USAGE:
    documorph.exe [OPTIONS] [FILE_PATHS...]

ARGS:
    <FILE_PATHS...> Documents (.docx, .doc, .pptx, .ppt, .xlsx, .xls) to convert

OPTIONS:
    --register       Register DocuMorph in the Windows File Explorer context menu (HKCU)
    --unregister     Remove DocuMorph from the Windows context menu
    --status         Check registry status and Microsoft Office availability
    --help, -h       Show this help message
    --version, -v    Show version information
```

---

## License

MIT License. Copyright (c) 2026 Rasheed Hameed.
