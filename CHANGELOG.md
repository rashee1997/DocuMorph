# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.2.1] - 2026-10-12

### Fixed
- **Installer UAC Elevation**: Automatically requests Administrator privileges on startup via `ShellExecuteW(runas)` so installation to `C:\Program Files\DocuMorph` succeeds without permission errors.
- **Installer Non-Blocking Execution**: Moved file copying and registry updates to a background thread to prevent UI freezing.
- **Conversion Responsiveness & Progress Animation**: Added a 35ms timer to maintain continuous Win32 message queue activity during document export, preventing Windows from marking the window as unresponsive.
- **Copy & Interface Antislop**: Removed AI buzzwords, redundant badges, and celebratory copy across the interface and documentation.

## [1.2.0] - 2026-10-12

### Added
- **Unique Brand Identity & Multi-Resolution Windows Icons**:
  - Designed custom modern "Morphic Prism D" brand logo via logo design system, replacing generic PDF sheet clichés.
  - Multi-resolution Windows application icon (`resources/icon.ico`: 16, 24, 32, 48, 64, 128, 256 px).
  - Embedded Windows PE icon resources linked directly into `documorph.exe`, `installer.exe`, and `uninstaller.exe`.
  - Default installation target updated to standard `C:\Program Files\DocuMorph` with elevated permission handling.
- **WinUI 3 & Windows 11 Fluent Design System**:
  - Windows 11 DWM rounded window corners (`DWMWCP_ROUND`) applied to top-level dialogs.
  - Hardware-accelerated double buffering (`WS_EX_COMPOSITED`) eliminating Win32 redraw flicker.
  - Rounded Fluent Cards (`RoundRect` with 1px `#E2E8F0` border on `#F8FAFC` canvas).
  - Custom owner-drawn Crimson pill action buttons (`BS_OWNERDRAW`) with hover and pressed visual feedback.
  - Transparent static label backgrounds matching card and canvas brushes without grey bounding rectangles.
  - Dynamic `AdjustWindowRectEx` window sizing calculation, guaranteeing zero button clipping or cutoff at any DPI scale.

### Changed & Fixed
- **Office COM Pipeline Refactoring & Layout Preservation**:
  - **Microsoft Word**: Set `ActivePrinter` to `"Microsoft Print to PDF"` upfront, bypassing offline/network printer timeouts; disabled `UpdateStylesOnOpen` to stop template style distortion; set `View.Type = wdPrintView` (3) for exact pagination; set `BitmapMissingFonts = false` to preserve scalable vector TrueType fonts.
  - **Microsoft Excel**: Configured `PageSetup` (`Zoom = false`, `FitToPagesWide = 1`, `FitToPagesTall = false`) to prevent wide tables from being split across columns; disabled `AskToUpdateLinks`, `EnableEvents`, `DisplayAlerts`, and formula recalculation on open for instant load.
  - **Microsoft PowerPoint**: Migrated to native `ExportAsFixedFormat` with all 15 parameters including `BitmapMissingFonts = false` and `UseISO19005_1`; silenced alerts with `DisplayAlerts = ppAlertsNone` (1).
- **UI Responsiveness & Anti-Hang Optimization**:
  - Added explicit `InvalidateRect` on status labels upon receiving progress events, ensuring real-time 60 FPS progress updates under `WS_EX_COMPOSITED`.
  - Maintained active Cancel / Close button during conversions so the UI never feels locked.
  - Ensured Office application processes terminate with `wdDoNotSaveChanges`, preventing background prompts to save `Normal.dotm`.
- **Authorship**:
  - Updated author and publisher metadata to **Rasheedh Ahamad**.

## [1.1.0] - 2026-10-12

### Added
- **Options & Bulk Conversion GUI Dialog**:
  - Interactive pre-conversion control dialog showing queued files, output options, and live preview.
  - Multi-file queue with `[+ Add Files...]`, `[- Remove]`, and `[Clear]` controls using native Win32 dialogs.
  - Single-Instance Named Pipe IPC: Multiple files selected simultaneously in File Explorer seamlessly aggregate into a unified conversion batch.
- **PDF Archival Standards**:
  - Toggle between **Standard PDF** and **PDF/A-1a Archival** (`ISO 19005-1`) for enterprise compliance.
- **Flexible Destination Folders**:
  - Choice between source document directory or custom destination via native Windows folder browser (`SHBrowseForFolderW`).
- **Bulk Renaming & Filename Patterns**:
  - Pattern formatting supporting `{name}`, `{date}`, and `{num}` tokens, with optional prefix and suffix.
  - Live interactive filename preview updating on every keystroke.
  - Safe collision prevention with sequential indexing (`(1)`, `(2)`).
- **Automated Double-Click Setup Wizard (`DocuMorph-Setup.exe`)**:
  - Standalone, self-contained installer (~1 MB) embedding `documorph.exe`, `uninstall.exe`, and `icon.ico`.
  - User can select target directory on C: disk or any drive with native folder picker.
  - Automatic per-user context menu integration in `HKCU\Software\Classes\SystemFileAssociations`.
  - Native integration with Windows Settings "Installed Apps" / "Add or Remove Programs".
  - Supports `--silent [PATH]` for automated rollouts.
- **Standalone GUI Uninstaller (`uninstall.exe`)**:
  - Removes all registry verbs and associations cleanly.
  - Autonomous self-deletion and folder cleanup.
- **Explorer Context Menu & Title Bar Icon Display**:
  - Unquoted `.ico` registration ensures custom crimson icon renders reliably in Windows File Explorer context menus.
  - Dual `WM_SETICON` loading ensures application icon displays in the window title bar and taskbar.

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
