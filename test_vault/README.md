# Mossambi Test Vault & Universal Verification Suite

Welcome to the **Mossambi Test Vault**! This dedicated folder is designed for comprehensive testing across all four interfaces:
- **CLI (`mossambi`)**
- **TUI (`mossambi tui`)**
- **GUI Desktop Studio (`mossambi gui`)**
- **Radial Citrus Widget (`mossambi widget`)**

---

## 📂 Directory Layout

| Folder | Recommended Files | Test Scenarios |
|---|---|---|
| `audio/` | `.wav`, `.flac`, `.mp3`, `.ogg`, `.alac` | Bit-exact stream copy, audiophile 0.00% lossless transcoding, `ALL` batch transcode |
| `video/` | `.mp4`, `.mkv`, `.webm`, `.avi` | Zero-loss container remuxing, hardware acceleration (NVENC, AMF, QSV) |
| `image/` | `.png`, `.jpg`, `.webp`, `.tiff`, `.bmp` | Lossless raster conversion, DPI preservation, ICC color matrix validation |
| `document/` | `.txt`, `.md`, `.pdf` | Scalable vector typography PDF generation, TrueType font embedding, vector layout |
| `spreadsheet/` | `.csv`, `.tsv`, `.xlsx` | Clean tabular data transformation, Calamine spreadsheet parsing |
| `archive/` | `.zip`, `.tar`, `.tar.gz`, `.7z` | High-compression packaging, Zip-Slip directory traversal guard |
| `exam/` | `.png`, `.jpg` | 5-iteration Indian Government Exam optimizer (UPSC, SSC, IBPS 500x300, 200 DPI, 100-500 KB) |

---

## ⚡ Quick Testing Instructions

### 1. Universal Quality & Integrity Checker (CLI)
Inspect any file's magic bytes, container integrity, parser status, and compatibility scorecard:
```bash
mossambi check test_vault/image/sample_photo.png
mossambi check test_vault/audio/sample_audio.wav
mossambi check test_vault/document/sample_notes.txt
```
To output machine-readable JSON for CI/automation:
```bash
mossambi check test_vault/document/sample_notes.txt --json
```

### 2. High-Performance Benchmarking (vs Online & CLI Tools)
Measure latency (µs/ms), throughput (MB/s), and speedup vs CloudConvert, FFmpeg CLI, and GitHub Rust:
```bash
mossambi bench test_vault/audio/sample_audio.wav
mossambi bench test_vault/image/sample_photo.png
```

### 3. Unix `mv`-Style Drag & Drop Conversion
Convert files and place them cleanly beside the source file (with automatic collision protection):
```bash
# Convert WAV to FLAC
mossambi test_vault/audio/sample_audio.wav flac

# Transcode to ALL compatible formats simultaneously
mossambi test_vault/audio/sample_audio.wav --all

# Convert text/markdown to high-quality vector PDF
mossambi test_vault/document/sample_notes.txt pdf
```

### 4. Interactive Terminal User Interface (TUI)
Launch the 100% Apple macOS-styled terminal dashboard:
```bash
mossambi tui
```
- Press **[1-6]** or **[Tab]** to switch between tabs:
  - `[1]` Transcoder & Golden ALL
  - `[2]` Indian Exam Studio
  - `[3]` Archive Suite
  - `[4]` Hardware & AI Telemetry
  - `[5]` Appearance & Themes
  - `[6]` Quality & Benchmarks
- Press **[t]** to cycle macOS themes (Liquid Glass, Frosted Glass, Dark Anthracite, Light Aqua).
- In Quality tab: press **[Enter]** to inspect, or **[b]** to benchmark conversion speed.

### 5. macOS Desktop Studio & Radial Citrus Widget
Launch the full desktop workspace:
```bash
mossambi gui
```
- Click **"Quality Inspector"** in the sidebar to view live 4-dimension scorecard cards.
- Click **"Radial Citrus Wheel"** to summon the floating widget overlay.
- Drag any file over the widget: sectors auto-configure to the detected file type in real time. Drop directly onto any petal (e.g. FLAC) to immediately convert and place `file.flac` beside `file.wav`!
