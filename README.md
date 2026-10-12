<div align="center">

  <img src="resources/logo.png" alt="DocuMorph Logo" width="128" height="128" />

  # DocuMorph

  Windows context-menu utility for converting Word, PowerPoint, and Excel files to PDF with embedded fonts.

  <br />

  [![Release](https://img.shields.io/github/v/release/rashee1997/DocuMorph?style=flat-square)](https://github.com/rashee1997/DocuMorph/releases)
  [![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg?style=flat-square)](LICENSE)

</div>

---

## Overview

**DocuMorph** by **Rasheedh Ahamad** is a native Windows utility written in Rust. It adds a "Convert to PDF with DocuMorph" action to the File Explorer right-click menu for Microsoft Word (`.docx`, `.doc`), PowerPoint (`.pptx`, `.ppt`), and Excel (`.xlsx`, `.xls`) files.

Conversions run directly through Microsoft Office automation, preserving TrueType document fonts, page layouts, and table formatting without virtual print driver rasterization.

---

## Features

- **File Explorer integration**: Right-click `.docx`, `.doc`, `.pptx`, `.ppt`, `.xlsx`, or `.xls` files to convert.
- **Batch queueing**: Select multiple files in File Explorer to process them together in one window.
- **Font embedding**: Embeds TrueType fonts directly in output PDFs.
- **PDF/A support**: Option to generate PDF/A-1a archival files (`ISO 19005-1`).
- **Excel layout protection**: Fits worksheets to page width to prevent split tables.
- **Output control**: Save to the source folder or a custom destination, with optional renaming patterns (`{name}`, `{date}`, `{num}`).
- **Self-contained installer**: Standalone installer (`DocuMorph-Setup.exe`) configures the context menu and uninstaller.
- **Zero runtime dependencies**: Pure native Win32/Rust binary with no extra runtime requirements.

---

## Installation

### Installer
Download `DocuMorph-Setup.exe` from [Releases](https://github.com/rashee1997/DocuMorph/releases) and run it. The installer requests Administrator privileges to place files in `C:\Program Files\DocuMorph` and register the context menu.

### Silent install
Run the installer from an elevated prompt:
```powershell
.\DocuMorph-Setup.exe --silent "C:\Program Files\DocuMorph"
```

---

## Usage

### 1. File Explorer
1. Select one or more Word, PowerPoint, or Excel files in File Explorer.
2. Right-click and select **"Convert to PDF with DocuMorph"**.
3. Choose your output format (Standard PDF or PDF/A-1a) and destination folder.
4. Click **Convert**.
5. When finished, click **Open folder** to view the generated PDFs.

### 2. Command Line

```
DocuMorph v1.2.1
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

EXAMPLES:
    documorph.exe --register
    documorph.exe "C:\Documents\QuarterlyReport.docx"
    documorph.exe "Doc1.docx" "Slides.pptx" "Sheet.xlsx"
```

---

## Building from Source

Prerequisites: Rust 1.80+ on Windows.

```powershell
# Clone the repository
git clone https://github.com/rashee1997/DocuMorph.git
cd DocuMorph

# Run tests
cargo test --lib

# Build binaries
cargo build --release --bin documorph --bin uninstaller
cargo build --release --bin installer

# Copy installer to root
Copy-Item "target/release/installer.exe" "DocuMorph-Setup.exe"
```

Outputs in `target/release/`:
- `documorph.exe`: Conversion engine
- `installer.exe`: Standalone installer
- `uninstaller.exe`: Uninstaller

---

## License & Author

Released under the [MIT License](LICENSE).

Author: **Rasheedh Ahamad** (`rasheedhmech03@gmail.com`).
