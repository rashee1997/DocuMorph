# DocuMorph

> **Lightning-fast, high-fidelity Windows context-menu document-to-PDF engine with TrueType font embedding, WinUI 3 Fluent modern UI, bulk queueing, and an automated standalone installer.**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Platform: Windows](https://img.shields.io/badge/Platform-Windows%2010%20%7C%2011-0078D6.svg)](https://microsoft.com/windows)
[![Rust: 1.80+](https://img.shields.io/badge/Rust-1.80%2B-DEA584.svg)](https://www.rust-lang.org/)
[![Version: 1.2.0](https://img.shields.io/badge/Version-1.2.0-brightgreen.svg)](CHANGELOG.md)
[![CI/CD: Release](https://github.com/rashee1997/DocuMorph/actions/workflows/release.yml/badge.svg)](https://github.com/rashee1997/DocuMorph/actions)

---

## Overview

**DocuMorph** by **Rasheedh Ahamad** is a native Windows desktop utility written in Rust. It enables 1-click document-to-PDF conversion directly from the Windows File Explorer context menu for Microsoft Word (`.docx`, `.doc`), PowerPoint (`.pptx`, `.ppt`), and Excel (`.xlsx`, `.xls`) documents.

Unlike virtual PDF printer drivers or third-party format converters that degrade vector shapes and substitute missing fonts, DocuMorph uses deep native Office COM automation to preserve 100% TrueType document fonts, layout pagination, and table boundaries without font rasterization.

---

## Key Features

- 🖱️ **Windows Explorer Context Menu**: Right-click any `.docx`, `.doc`, `.pptx`, `.ppt`, `.xlsx`, or `.xls` file and choose **"Convert to PDF with DocuMorph"**.
- 🎨 **WinUI 3 & Windows 11 Fluent Design System**:
  - Native Windows 11 DWM rounded window corners (`DWMWCP_ROUND`).
  - Hardware-accelerated double buffering (`WS_EX_COMPOSITED`) with zero Win32 redraw flicker.
  - Soft rounded cards with crisp 1px borders on a light slate canvas.
  - Custom owner-drawn Crimson pill action buttons (`BS_OWNERDRAW`) with active hover and press feedback.
  - Seamless transparent label blending without grey bounding boxes.
- 📐 **Zero-Cutoff Dynamic Sizing**: Computes window geometry dynamically via `AdjustWindowRectEx`, guaranteeing generous bottom margins and zero button truncation across all display scaling levels (100%, 125%, 150%).
- 🔤 **100% TrueType Vector Font Embedding**: Preserves scalable vector glyphs without rasterizing missing fonts into low-resolution bitmaps (`BitmapMissingFonts = false`).
- 🏛️ **ISO 19005-1 PDF/A-1a Archival Compliance**: Toggle between Standard PDF and enterprise-grade PDF/A-1a archival mode.
- 📑 **Excel Sheet Boundary Control**: Automatically configures `PageSetup` (`Zoom = false`, `FitToPagesWide = 1`, `FitToPagesTall = false`) to prevent wide tables from being split across multiple columns.
- 🖼️ **PowerPoint Fixed Format Export**: Uses native `ExportAsFixedFormat` with print intent and suppressed alert popups for rapid, non-blocking slide export.
- ⚙️ **Interactive Options & Queue Dialog**:
  - Add, remove, or clear queued documents.
  - Route output to the source directory or any custom folder.
  - Bulk renaming templates with `{name}`, `{date}`, and `{num}` tokens and live real-time preview.
  - Safe collision prevention: auto-numbers duplicate files (`Document (1).pdf`).
- 🔄 **Single-Instance Named Pipe IPC Queue**: Selecting multiple files simultaneously in Windows Explorer automatically aggregates all documents into a unified conversion batch queue via Named Pipes.
- 📦 **Automated Setup Wizard (`DocuMorph-Setup.exe`)**: All-in-one standalone installer (~1 MB) bundling all binaries and icon, with custom install path selection and full registration in Windows Settings **Installed Apps**.
- 🗑️ **Clean Standalone Uninstaller (`uninstall.exe`)**: Removes context menus, registry entries, and cleans installation files.
- ⚡ **Lightweight & Fast**: Pure native Rust binary with 0 external runtime DLL dependencies (~395 KB engine, ~1 MB setup wizard).

---

## Installation

### Method 1: Download from GitHub Releases (Recommended)
Download the latest `DocuMorph-Setup.exe` from the [GitHub Releases Tab](https://github.com/rashee1997/DocuMorph/releases) and double-click to install.

### Method 2: Silent / Automated CLI Install
Deploy silently to `C:\DocuMorph` (or any custom location) using PowerShell:
```powershell
.\DocuMorph-Setup.exe --silent "C:\DocuMorph"
```

---

## How to Use

### 1. File Explorer Context Menu
1. In Windows File Explorer, select one or more Word, PowerPoint, or Excel files.
2. Right-click and choose **"Convert to PDF with DocuMorph"**.
3. In the options dialog, configure your target settings:
   - **PDF Standard**: Standard PDF or PDF/A-1a Archival (`ISO 19005-1`).
   - **Output Destination**: Source folder or custom folder.
   - **Renaming Pattern**: `{name}`, `Prefix_{name}_{date}`, etc.
4. Click **Start Conversion**.
5. Once complete, click **Open Output Folder** to view your converted PDFs.

### 2. Command Line Interface

```
DocuMorph v1.2.0
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

## Build from Source

DocuMorph compiles with either the MSVC or lightweight GNU Windows toolchain:

```powershell
# Clone the repository
git clone https://github.com/rashee1997/DocuMorph.git
cd DocuMorph

# Run tests
cargo test --lib

# Build all release binaries
cargo build --release --bin documorph --bin uninstaller
cargo build --release --bin installer

# The standalone installer bundles the engine and uninstaller
Copy-Item "target/release/installer.exe" "DocuMorph-Setup.exe"
```

Binaries generated in `target/release/`:
- `documorph.exe`: High-fidelity conversion engine (~395 KB)
- `installer.exe`: Standalone setup wizard (~1 MB)
- `uninstaller.exe`: Autonomous uninstaller (~324 KB)

---

## Automated GitHub Actions CI/CD

DocuMorph includes a fully automated GitHub Actions workflow (`.github/workflows/release.yml`):
- **On Push to Master / Main**: Automatically compiles release binaries, tests the codebase, and packages versioned artifacts available in the GitHub Actions **Artifacts** tab.
- **On Release Tag (`v*`)**: Automatically builds, computes SHA256 checksums, and uploads `DocuMorph-Setup.exe`, `DocuMorph-Setup-vX.Y.Z.exe`, and portable release `.zip` archives directly to the **GitHub Releases** tab.

---

## License & Authorship

Released under the [MIT License](LICENSE).

Copyright (c) 2026 **Rasheedh Ahamad** (`rasheedhmech03@gmail.com`).
