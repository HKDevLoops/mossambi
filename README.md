# Mossambi — Native Desktop Widget

**Mossambi** is a zero-click, offline desktop file converter and modifier for Windows. Borrowing Tangerine's radial wheel shape and drag-to-convert interaction, Mossambi features an original, minimalist Apple-style interface with a restrained neutral palette and mossambi-green accents.

---

## Key Features

- **True Desktop Widget:** Not a website, web app, or browser page. A native Windows frameless, 32-bit alpha-translucent desktop widget that floats on your desktop.
- **Drag-to-Convert:** Drag any file from Windows Explorer or your desktop directly over the radial wheel. The petals immediately adapt to show valid output formats for that file.
- **Visual Parity:** Accurately implements the 6 visual states from `Image reference.png` and `Design.md`:
  1. *Idle State:* Upward arrow and clean "Drop files here" center prompt.
  2. *Hover State:* Petals illuminate with a soft mossambi-green glow (`#78A942` / `#A5CC72`) and dark contrast label/icon.
  3. *File Detected:* Identifies the dragged file and suggests available formats.
  4. *Converting State:* Smooth green circular progress ring around the center hub with percentage and status.
  5. *Success State:* Circular checkmark badge and sibling output notification.
  6. *Dark Mode:* Deep charcoal/slate surface with crisp typography and vibrant accent glow.
- **Non-Destructive Sibling Output:** Outputs are written right next to the original source file (`<stem>_converted.<ext>`). An incremental suffix (`_1`, `_2`) is automatically added if a collision occurs, so original files are never overwritten.
- **100% Local & Offline:** Powered by on-device processing via Pillow, system FFmpeg, and native parsers. Zero cloud uploads, zero tracking, and zero internet requirement.
- **System Tray Integration:** Runs in the Windows notification area / system tray with click-to-toggle, menu shortcuts, and always-on-top mode.
- **Desktop Drag & Positioning:** Click and drag the widget anywhere on your desktop; its position is preserved across restarts.
- **Settings Window:** 4 dedicated sections (General, Conversion, Appearance, About) to configure output destination, quality, theme, and widget size (Small 380px, Medium 440px, Large 500px).
- **Keyboard Navigation:** Cycle through petals using Arrow Keys, convert with Enter/Space, and cancel with Escape.

---

## Supported Conversions

| Category | Input Formats | Available Output Petals | Execution Engine |
|---|---|---|---|
| **Images** | JPG, PNG, WEBP, BMP, TIFF, ICO, GIF, HEIC, AVIF | `JPG` · `PNG` · `WEBP` · `HEIC` · `TIFF` · `AVIF` · `BMP` · `PDF` | Pillow / Native |
| **Audio** | MP3, WAV, FLAC, M4A, OGG, AAC, OPUS, AIFF | `MP3` · `WAV` · `FLAC` · `M4A` · `OGG` · `AAC` · `OPUS` | FFmpeg |
| **Video** | MP4, MOV, MKV, WEBM, AVI, WMV, FLV | `MP4` · `MOV` · `MKV` · `WEBM` · `GIF` · `MP3` · `AVI` | FFmpeg |
| **Documents** | PDF, DOCX, TXT, MD, HTML, RTF | `PDF` · `DOCX` · `TXT` · `HTML` · `JPG` · `PNG` | pypdf / docx / Pillow |
| **Spreadsheets** | XLSX, XLS, CSV, TSV, ODS | `XLSX` · `CSV` · `TSV` · `PDF` | openpyxl / csv |
| **Archives** | ZIP, TAR, GZ, 7Z | `ZIP` · `TAR` · `EXTRACT` (with Zip-Slip protection) | zipfile / tarfile |
| **Subtitles** | SRT, VTT | `VTT` · `SRT` · `TXT` | Native parser |

---

## Quick Start

### 1. Launch directly with Python
```powershell
python run_widget.py
```

### 2. Launch with the Windows batch script
Double-click `run_widget.bat` to launch the widget silently.

### 3. Build a standalone Windows executable (.exe)
```powershell
python build_exe.py
```
This produces `dist/Mossambi/Mossambi.exe`.

---

## Usage Guide

1. **Convert by Drag & Drop:**
   - Grab any file from Windows Explorer or Desktop.
   - Drag it over the widget. The petals will automatically update to valid formats.
   - Release the mouse button over any petal (e.g. `JPG` or `PDF`) to start conversion.
2. **Convert by Clicking:**
   - Click the center hub of the widget to open a file dialog.
   - Select your file, then click any format petal to convert.
3. **Move Widget:**
   - Click and drag the widget on your desktop to position it wherever you like.
4. **Context Menu:**
   - Right-click anywhere on the widget to switch Theme (Light / Dark), toggle Always on Top, adjust Widget Size, open Settings, or reveal the output folder in Explorer.
5. **System Tray:**
   - Left-click the Mossambi icon in the Windows taskbar system tray to toggle widget visibility.

---

## Testing

Run the automated test suite verifying geometry, theme tokens, registry classifiers, and file conversions:
```powershell
python -m unittest tests/test_suite.py
```
