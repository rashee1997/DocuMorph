# Release Notes — DocuMorph v1.2.0

**DocuMorph** by **Rasheedh Ahamad** is a native, lightweight Windows context-menu utility written in Rust that converts Microsoft Word, PowerPoint, and Excel documents into pristine PDFs with 100% TrueType font embedding, conversion options, bulk queueing, WinUI 3 Fluent modern UI, and a standalone GUI installer/uninstaller.

---

### Highlights of v1.2.0

1. **WinUI 3 & Windows 11 Fluent Design System**
   - **Windows 11 DWM Rounded Window Corners**: Applied via `DwmSetWindowAttribute` (`DWMWCP_ROUND`).
   - **Hardware-Accelerated Double Buffering**: `WS_EX_COMPOSITED` on all windows completely eliminates Win32 redraw flicker.
   - **Rounded Fluent Cards**: Clean `#FFFFFF` containers with 1px `#E2E8F0` border resting on a `#F8FAFC` soft slate canvas.
   - **Custom Owner-Drawn Crimson Pill Buttons**: Brand crimson accent (`#BE123C`) buttons with active pressed and hover feedback.
   - **Seamless Label Blending**: Transparent static text eliminates 90s grey boxes behind labels.
   - **Zero-Cutoff Dynamic Sizing**: Sizing computed via `AdjustWindowRectEx` ensures no buttons or elements are clipped across display scaling modes (100%, 125%, 150%).

2. **Office COM Pipeline Refactoring & Font/Layout Preservation**
   - **Word Layout & Font Fidelity**:
     - Pre-configures `ActivePrinter` to `"Microsoft Print to PDF"`, avoiding slow network printer timeouts.
     - Sets `UpdateStylesOnOpen = false` to preserve document templates without distortion.
     - Forces `View.Type = wdPrintView` (3) for exact print-layout pagination.
     - Disables `BitmapMissingFonts` (`false`) to ensure TrueType vector fonts remain crisp vectors.
   - **Excel Sheet Pagination & Table Boundaries**:
     - Sets `PageSetup` (`Zoom = false`, `FitToPagesWide = 1`, `FitToPagesTall = false`) to keep wide tables on a single page width without splitting columns.
     - Silences link alerts and manual calculation mode to prevent hangs on workbook open.
   - **PowerPoint Native Fixed Format Export**:
     - Upgraded to native `ExportAsFixedFormat` with all 15 parameters, preserving layout and font fidelity.
     - Silenced alert dialogs (`DisplayAlerts = ppAlertsNone`).

3. **UI Responsiveness & Anti-Hang Optimization**
   - Instant real-time 60 FPS status updates via explicit `InvalidateRect`.
   - Cancel / Close button remains active throughout conversions.
   - Office engines terminate with `wdDoNotSaveChanges = 0`, eliminating prompts to save `Normal.dotm`.

4. **Pre-Conversion Options & Bulk GUI Dialog**
   - Multi-file conversion queue with `[+ Add...]`, `[- Remove]`, and `[Clear]`.
   - Dynamic IPC Aggregation: Multiple files selected simultaneously in File Explorer automatically merge into one unified conversion batch.
   - Choice between **Standard PDF** and **PDF/A-1a Archival** (`ISO 19005-1`).
   - Destination directory routing and bulk renaming template engine with live preview.

5. **Automated GUI Setup Wizard (`DocuMorph-Setup.exe`) & Clean Uninstaller**
   - Self-contained ~1 MB installer embedding `documorph.exe`, `uninstall.exe`, and `icon.ico`.
   - Interactive directory selection and full registration in Windows Settings "Installed Apps".
   - Clean uninstaller (`uninstall.exe`) with autonomous self-deletion.

---

### Installation & Usage

#### Quick Setup:
Double-click `DocuMorph-Setup.exe` and click **Install Now**.

#### Command Line / Silent Install:
```powershell
.\DocuMorph-Setup.exe --silent "C:\Program Files\DocuMorph"
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
