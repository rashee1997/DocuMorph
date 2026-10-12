# DocuMorph

> **Lightning-fast, high-fidelity Windows context-menu document-to-PDF engine written in Rust.**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Platform: Windows](https://img.shields.io/badge/Platform-Windows%2010%20%7C%2011-0078D6.svg)](https://microsoft.com/windows)
[![Rust: 1.80+](https://img.shields.io/badge/Rust-1.80%2B-DEA584.svg)](https://www.rust-lang.org/)

---

## Features

- 🖱️ **Context Menu Native**: Right-click `.docx`, `.doc`, `.pptx`, `.ppt`, `.xlsx`, or `.xls` to convert.
- 🎨 **Custom Icon**: Elegant crimson PDF icon displayed directly inside File Explorer context menus.
- 🔤 **Document Font Embedding**: Preserves and embeds the document's true fonts (no ugly substitutions).
- ⚡ **Live Progress UI**: Hardware-accelerated Win32 Fluent progress dialog with stage tracking.
- 🛡️ **Hardened Security**: Unconditionally forces macro execution off; suppresses blocking modal dialogs.
- 🔒 **Atomic Output**: Writes to temporary buffer before atomic rename; protects existing files with safe copy numbering.
- 🪶 **Lightweight Toolchain**: Built with the Rust GNU toolchain (`x86_64-pc-windows-gnu`) — zero Visual Studio 2022 bloat.

---

## Quick Start

### 1. Build the Binary
```powershell
cd F:\DocuMorph
cargo build --release
```

### 2. Register Context Menu
```powershell
.\target\release\documorph.exe --register
```

### 3. Convert Documents
Right-click any Word, PowerPoint, or Excel file in Windows File Explorer and select **"Convert to PDF"**.

---

## CLI Reference

```
DocuMorph v1.0.0
High-fidelity Windows context-menu document-to-PDF engine

USAGE:
    documorph.exe [OPTIONS] [FILE_PATH]

ARGS:
    <FILE_PATH>    Path to .docx, .doc, .pptx, .ppt, .xlsx, or .xls file to convert

OPTIONS:
    --register     Register DocuMorph in the Windows File Explorer context menu (HKCU)
    --unregister   Remove DocuMorph from the Windows context menu
    --status       Check registry status and Microsoft Office availability
    --overwrite    Overwrite existing PDF instead of creating a numbered copy
    --help, -h     Show this help message
    --version, -v  Show version information
```

---

## Architecture

```
User Right-Click in Explorer
          │
          ▼
   documorph.exe "%1"
          │
    ┌─────┴────────────────────────┐
    ▼                              ▼
UI Thread (Win32)          Worker Thread (STA COM)
- Centered Dialog          - Word/Excel/PowerPoint Automation
- Progress Bar (0-100%)    - Disable Macros (AutomationSecurity=3)
- Stage Status Text        - Embed TrueType/OpenType Fonts
- "Open PDF" Action        - Export to .tmp.<pid>.pdf
                           - Atomic Move to Output PDF
```

---

## License

This project is licensed under the [MIT License](LICENSE).
