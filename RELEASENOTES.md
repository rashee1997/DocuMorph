# Release Notes — DocuMorph v1.0.0

**DocuMorph** is a native, lightweight Windows context-menu utility written in Rust that converts Microsoft Word, PowerPoint, and Excel documents into pristine PDFs with 100% font embedding and live progress tracking.

---

### Highlights of v1.0.0

1. **Native Context Menu Seamlessness**
   Right-click any Word (`.docx`, `.doc`), PowerPoint (`.pptx`, `.ppt`), or Excel (`.xlsx`, `.xls`) file in Windows Explorer. Click **"Convert to PDF"** with the custom DocuMorph icon.

2. **100% Font Embedding Fidelity**
   Unlike third-party converters that replace document fonts with generic system fonts or fallbacks, DocuMorph uses Microsoft Office COM automation with TrueType font embedding enabled. Every font, symbol, table, and layout element is rendered exactly as intended by the author.

3. **Live Win32 Fluent Progress Dialog**
   A sleek, native progress dialog appears immediately upon launch, showing:
   - Dynamic stage indicator (Engine Init -> Loading Document -> Rendering PDF -> Finalizing).
   - Smooth animated progress bar.
   - Quick action buttons ("Open PDF", "Show in Folder") and automatic 2.5-second countdown dismissal.

4. **Zero-Dependency Lightweight Build**
   Compiled directly with the Rust GNU toolchain (`stable-x86_64-pc-windows-gnu`). No Visual Studio 2022 IDE or multi-gigabyte C++ build tools required.

5. **Security by Design**
   - Unconditionally disables VBA macro execution (`AutomationSecurity = 3`).
   - Opens files read-only and suppresses modal alerts.
   - Atomic output renaming protects against corrupted outputs.
   - Safe collision avoidance preserves existing PDFs.

---

### Installation & Usage

#### Register Context Menu (No Admin / UAC Required):
```powershell
.\documorph.exe --register
```

#### Convert from Command Line:
```powershell
.\documorph.exe "C:\Path\To\Report.docx"
```

#### Check System Status:
```powershell
.\documorph.exe --status
```

#### Unregister Context Menu:
```powershell
.\documorph.exe --unregister
```
