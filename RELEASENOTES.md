# Release Notes — DocuMorph v1.2.2

**DocuMorph** by **Rasheedh Ahamad** is a native Windows desktop utility written in Rust. It enables 1-click document-to-PDF conversion directly from the Windows File Explorer context menu for Microsoft Word (`.docx`, `.doc`), PowerPoint (`.pptx`, `.ppt`), and Excel (`.xlsx`, `.xls`) files.

---

### What's New in v1.2.2

1. **UAC Elevation for Installer & Uninstaller**
   - Automatically prompts for Administrator privileges via Windows UAC when run, ensuring file installation to `C:\Program Files\DocuMorph` and registry entry creation succeed without permission errors.
   - The uninstaller also checks elevation to cleanly remove installed files and registry entries.

2. **Non-Blocking Background Installation**
   - File copying and registry configuration run in a dedicated background worker thread, keeping the installer interface responsive.

3. **Smooth Progress Animation & Anti-Hang Heartbeat**
   - Added a 35ms animation timer to keep the Win32 message queue active during document conversions.
   - Eliminates Windows Desktop Window Manager (DWM) "Not Responding" false flags during lengthy COM export calls.
   - Progress bar smoothly advances and creeps during document conversion, providing continuous visual feedback.

4. **Copy & Text Cleanup**
   - Applied antislop copywriting principles across all dialogs, buttons, and documentation.
   - Removed AI buzzwords, redundant badge shields, and robotic confirmation phrasing.

---

### Installation

Download `DocuMorph-Setup.exe` from [Releases](https://github.com/rashee1997/DocuMorph/releases) and double-click to install.

For silent automated deployment:
```powershell
.\DocuMorph-Setup.exe --silent "C:\Program Files\DocuMorph"
```

Author: **Rasheedh Ahamad** (`rasheedhmech03@gmail.com`)  
License: [MIT](LICENSE)
