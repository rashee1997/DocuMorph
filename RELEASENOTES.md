# Release Notes — DocuMorph v1.1.0

**DocuMorph** is a native, lightweight Windows context-menu utility written in Rust that converts Microsoft Word, PowerPoint, and Excel documents into pristine PDFs with 100% TrueType font embedding, conversion options, bulk queueing, and a standalone GUI installer/uninstaller.

---

### Highlights of v1.1.0

1. **Pre-Conversion Options & Bulk GUI Dialog**
   Right-clicking a document or opening DocuMorph now presents a full graphical options dialog:
   - **Queue Management**: Multi-file conversion queue with `[+ Add...]`, `[- Remove]`, and `[Clear]`.
   - **Dynamic IPC Aggregation**: Selecting multiple files in Windows Explorer and clicking "Convert to PDF with DocuMorph" automatically merges all files into a single batch queue via Named Pipe IPC (`\\.\pipe\DocuMorph_Queue_Pipe`).
   - **Dual Progress Views**: Progress displays both overall queue progress and per-file step progression.

2. **PDF Archival Standards (Standard vs PDF/A-1a)**
   - **Standard PDF**: Optimized for screen reading, sharing, and standard printing.
   - **PDF/A-1a Archival**: Adheres to `ISO 19005-1` for strict long-term document preservation and compliance archiving.

3. **Destination Directory Routing**
   - Output directly to the source document's directory.
   - Or route to any custom folder with native Win32 directory browsing (`SHBrowseForFolderW`).

4. **Bulk Renaming Engine with Live Preview**
   - Flexible template syntax supporting `{name}`, `{date}`, and `{num}` tokens.
   - Customizable prefixes and suffixes.
   - Safe collision avoidance: automatically creates `Document (1).pdf` instead of overwriting existing files.
   - Live preview updating in real-time as you type.

5. **Automated GUI Setup Wizard (`DocuMorph-Setup.exe`)**
   - Self-contained ~1 MB installer embedding `documorph.exe`, `uninstall.exe`, and `icon.ico`.
   - Interactive directory selection (defaults to `C:\DocuMorph` or custom user location).
   - Automatically registers HKCU context menu verbs for `.docx`, `.doc`, `.pptx`, `.ppt`, `.xlsx`, and `.xls`.
   - Native integration with Windows Settings "Installed Apps" / "Add or Remove Programs".
   - Supports automated deployment via `DocuMorph-Setup.exe --silent [TARGET_DIR]`.

6. **Clean Standalone Uninstaller (`uninstall.exe`)**
   - Interactive confirmation dialog or silent removal (`--silent`).
   - Cleanly deletes context menu registry keys and Windows uninstaller registry keys.
   - Automatically cleans files and removes the installation folder.

7. **Reliable Explorer Icon Visibility**
   - Context menu registry icon paths are unquoted, resolving Windows Explorer `.ico` rendering.
   - Window titlebar, taskbar, and Alt+Tab icons are dynamically loaded and assigned via Win32 `WM_SETICON`.

8. **100% Font Embedding Fidelity**
   - TrueType/OpenType document fonts are embedded directly via Microsoft Office 16 COM automation (`EmbedTrueTypeFonts = true`, `BitmapMissingFonts = true`), eliminating font substitution.

---

### Installation & Usage

#### Quick Setup:
Double-click `DocuMorph-Setup.exe` and click **Install**.

#### Command Line / Silent Install:
```powershell
.\DocuMorph-Setup.exe --silent "C:\DocuMorph"
```

#### Converting Documents:
- Right-click any `.docx`, `.doc`, `.pptx`, `.ppt`, `.xlsx`, or `.xls` file in Windows Explorer and select **"Convert to PDF with DocuMorph"**.
- Or run:
```powershell
C:\DocuMorph\documorph.exe "C:\Path\To\Document.docx"
```

#### Uninstalling:
- From Windows Settings: Open **Apps > Installed Apps**, locate **DocuMorph**, and click **Uninstall**.
- Or run `C:\DocuMorph\uninstall.exe`.
