# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2026-10-12

### Added
- **Windows File Explorer Context Menu Integration**: Native right-click menu entry for `.docx`, `.doc`, `.pptx`, `.ppt`, `.xlsx`, and `.xls` files.
- **Embedded Custom Icon**: Multi-resolution crimson PDF icon embedded in binary resource table and displayed in the context menu.
- **TrueType/OpenType Font Embedding**: Leverages Microsoft Office COM automation (`ExportAsFixedFormat` with `EmbedTrueTypeFonts`, `SaveSubsetFonts`, and `BitmapMissingFonts`) to embed exact document fonts without substitution.
- **Live Win32 Fluent Progress Dialog**: Hardware-accelerated native progress window displaying animated progress bar, real-time stage updates, and post-conversion quick actions ("Open PDF", "Show in Folder", 2.5s auto-dismiss).
- **Security Hardening**: Macro execution unconditionally disabled (`AutomationSecurity = 3`), modal dialog alerts suppressed (`DisplayAlerts = wdAlertsNone`), and read-only source file opening.
- **Atomic File Replacement**: Conversion writes to temporary file (`.tmp.<pid>.pdf`) before atomic rename, protecting against partial or interrupted writes.
- **Safe Collision Handling**: Automatically prevents accidental data loss if a PDF with the same name already exists by generating safe sequential copies (`document (1).pdf`).
- **Command-Line Interface**:
  - `documorph.exe "<path>"`: Converts file with live GUI.
  - `documorph.exe --register`: One-click per-user registry installation without UAC prompt.
  - `documorph.exe --unregister`: Clean context menu removal.
  - `documorph.exe --status`: System diagnostic inspection.
