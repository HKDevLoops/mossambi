# 🍋 Mossambi — Universal File Conversion Platform & Explorer

[![CI](https://github.com/mossambi/mossambi/actions/workflows/ci.yml/badge.svg)](https://github.com/mossambi/mossambi/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)
[![Rust 1.75+](https://img.shields.io/badge/Rust-1.75%2B-orange.svg)](https://www.rust-lang.org)
[![Zero-Loss](https://img.shields.io/badge/Fidelity-0.00%25%20Zero--Loss-brightgreen.svg)](#mathematical-remuxing)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20Linux%20%7C%20macOS-lightgrey.svg)](#package-managers-supported)

```text
                       .-''''-.     _/\_
                     .'  ____  `.  (____)
                    /  .' || `.  \   ||
                   |  / \ || / \  |  ||
                   | ==  (())  == |  ||
                   |  \ / || \ /  |  ||
                    \  `. || .'  /   ||
                     `.  `--'  .'
                       `'-..-'`

   M O S S A M B I
   Zero-Loss File Conversion Platform & Universal File Explorer
   One Pure Rust Core · Three Modalities: Explorer App, Radial Widget, Interactive TUI
   v2.1.0 | 0.00% Zero-Loss Standard | Windows Fluent & macOS Finder DNA
```

**Mossambi** (Sweet Lime) is a high-performance, single-executable file conversion platform and modern universal file explorer written in **100% pure Rust**. It merges **Windows 11 Fluent Design / macOS Finder aesthetics** (acrylic/mica translucency, multi-tab navigation, certified Pantone palettes) with **high-throughput Linux power** (streaming chunk hashing, direct NVMe stream copy, and friction-free UNIX `mv`-style grammar).

---

## ⚡ Unified Binary & 3 Modalities

Mossambi compiles to a **single unified binary**: `mossambi` (`mossambi.exe` on Windows). All tools, converters, and explorer views run from this single executable:

```text
┌──────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│  Mossambi (MOSSAMBI — ZERO-LOSS CONVERSION PLATFORM & UNIVERSAL FILE EXPLORER v2.1.0)                            │
│  One Pure Rust Core · Three Operating Modalities · Apple Elegance + Linux Power                                  │
├──────────────────────────────────────────────────────────────────────────────────────────────────────────────────┤
│  Usage: mossambi [FLAGS / OPTIONS] [INPUT] [TARGET_FORMAT]                                                       │
│  UNIX 'mv' POSITIONAL GRAMMAR: mossambi <INPUT> [TARGET_FORMAT] [-a|--all] [-o <DIR>]                            │
├──────────────────────────────────┬──────────────────────────────────────────────────────┬────────────────────────┤
│  FLAG / OPTION                   │  DESCRIPTION & FUNCTION                              │  EXAMPLE USAGE         │
├──────────────────────────────────┼──────────────────────────────────────────────────────┼────────────────────────┤
│  --app, --explorer, --gui [DIR]  │  Launch macOS-styled Universal File Explorer & Studio │  mossambi --app        │
│  --widget [FILE]                 │  Launch Sweet Lime Resident Radial Widget (8 petals) │  mossambi --widget     │
│  --tui [DIR]                     │  Launch Yazi-Grade Interactive Terminal File Explorer │  mossambi --tui        │
├──────────────────────────────────┼──────────────────────────────────────────────────────┼────────────────────────┤
│  -a, --all                       │  Batch transcode to ALL valid formats in domain      │  mossambi file.wav -a  │
│  -o, --output <DIR>              │  Custom destination directory for output files       │  mossambi file.png -o .│
│  --strict                        │  Enforce strict 0.00% zero-loss mathematical standard│  mossambi in.wav flac  │
│  --json                          │  Output structured machine-parseable JSON response   │  mossambi in.wav --json│
├──────────────────────────────────┼──────────────────────────────────────────────────────┼────────────────────────┤
│  --exam <PRESET>                 │  Indian Exam Tuner (upsc-photo, ssc-photo, ibps-sign)│  mossambi in.jpg --exam│
│  --width <PX> / --height <PX>    │  Exact target width and height in pixels             │  --width 350 --height  │
│  --dpi <INT>                     │  Target resolution DPI (e.g. 200 or 300 DPI)         │  --dpi 300             │
│  --min-kb <N> / --max-kb <N>     │  Target file byte envelope range (e.g. 20KB to 50KB) │  --min-kb 20 --max-kb50│
│  --whiten                        │  Auto-whiten background of scanned signatures/photos │  --whiten              │
├──────────────────────────────────┼──────────────────────────────────────────────────────┼────────────────────────┤
│  --archive <FILES...> -o <ZIP>   │  Pack files into ZIP, 7Z, TAR, GZ, BZ2, XZ, or ZST   │  --archive a b -o c.zip│
│  --extract <ARCHIVE> [-o <DIR>]  │  Extract archive safely with Zip-Slip protection     │  --extract bundle.zip  │
├──────────────────────────────────┼──────────────────────────────────────────────────────┼────────────────────────┤
│  --check <FILE>                  │  4-dimension Quality & Container Integrity Scorecard │  mossambi --check f.png│
│  --bench <FILE> [TARGET]         │  Benchmark latency, throughput (MB/s) and speedup    │  mossambi --bench f.wav│
│  --hw                            │  Display GPU/NPU hardware acceleration engines       │  mossambi --hw         │
│  --verify <ORIG> <CONV>          │  Mathematical 0.00% zero-loss verification proof     │  --verify a.png b.png  │
│  --catalogue [QUERY]             │  Search across 200+ format registry catalog          │  --catalogue lossless  │
├──────────────────────────────────┼──────────────────────────────────────────────────────┼────────────────────────┤
│  --themes                        │  List all 20 certified eye-safe Pantone themes       │  mossambi --themes     │
│  --config [get|set|list]         │  View or edit persistent preferences (~/.mossambi)   │  --config set theme ...│
│  --daemon [start|stop|status]    │  Resident mouse gesture daemon (Win+Shift+C Hotkey) │  --daemon start        │
│  --banner                        │  Display Sweet Lime ASCII fruit art and summary      │  mossambi --banner     │
│  --manual                        │  Display Unicode box table manual                    │  mossambi --manual     │
│  -h, --help                      │  Display this complete usage manual table            │  mossambi --help       │
│  -V, --version                   │  Display version and system architecture info        │  mossambi --version    │
└──────────────────────────────────┴──────────────────────────────────────────────────────┴────────────────────────┘
```

### 1. 🪟 Universal File Explorer & Studio GUI
- Launch with `mossambi` or `mossambi --app [DIR]`.
- **Windows 11 Fluent & macOS Tab Bar**: Multi-tab navigation with tab titles, tab icons, and close buttons.
- **Home Dashboard**: Quick Access 9-card folder grid (Desktop, Downloads, Documents, Pictures, Music, Videos, between_pcs, wallpapers, Recycle Bin), Storage Space Meters for all system drives, and recent files list.
- **Embedded Agave Nerd Font**: Crisp vector icons and typography for all files and folders.
- **Zero Alternate Tint Bugs**: Eye-safe background with clean card surfaces, subtle hover elevation, and responsive flex wrapping.
- **200+ Format Transcoder**: Built-in batch conversion studio, archive manager, quality inspector, and hardware telemetry.

### 2. 🍋 Sweet Lime Resident Radial Widget
- Summon with `mossambi --widget [FILE]` or **`Win + Shift + C`** global hotkey.
- **Radial Petal Menu**: 8 contextual conversion sectors rendered around the cursor with real-time gesture tracking.
- Converts files in place with 0.00% zero-loss quality.

### 3. 🖥️ Yazi-Grade Interactive Terminal Explorer (TUI)
- Launch with `mossambi --tui [DIR]`.
- High-speed terminal file manager powered by Ratatui and Crossterm.
- Multi-column tree navigation, instant file preview, and keyboard shortcuts.

---

## ⚡ Fast-Path UNIX `mv`-Style Conversion

Convert files instantly from your terminal without opening any GUI:

```bash
# Instant format conversion
mossambi photo.png jpg
mossambi sound.wav flac
mossambi doc.txt pdf

# Transcode to ALL compatible formats in domain
mossambi sample.wav -a

# Indian Exam Form Tuning (UPSC, SSC, IBPS, GATE)
mossambi candidate.png --exam upsc-photo
mossambi signature.jpg --exam ibps-signature

# Archive Packing & Extraction
mossambi --archive doc1.pdf doc2.pdf -o bundle.zip
mossambi --extract bundle.zip -o ./extracted

# Mathematical Zero-Loss Verification Proof
mossambi --verify original.png converted.png
```

---

## 🎨 20 Certified Eye-Safe Pantone Themes

Mossambi includes **20 certified Pantone themes** configured specifically to prevent eye fatigue. Neon greens have been replaced with Pantone certified hues:

| Mode | Theme ID | Name | Accent Color | Surface |
|---|---|---|---|---|
| **Light** | `light_brilliant` | **Brilliant Lime** (Default Light) | `#0071E3` (Apple / Fluent Blue) | `#FFFFFF` |
| **Light** | `light_glacier` | **Glacier Aqua** | `#0A8F82` (Turquoise Bay) | `#FFFFFF` |
| **Light** | `light_zest` | **Sweet Zest Sunlight** | `#D97706` (Apple Amber) | `#FFFFFF` |
| **Light** | `light_blanc` | **Blanc de Blanc** | `#C48300` (Citrus Gold) | `#FFFFFF` |
| **Light** | `light_tangerine` | **Tangerine Sorbet** | `#D46124` (Tangerine Sun) | `#FFFFFF` |
| **Light** | `light_albedo` | **Albedo Pure Ivory** | `#D97706` (Amber Harvest) | `#FFFFFF` |
| **Light** | `light_quartz` | **Cool Quartz** | `#0077D4` (Quartz Blue) | `#FFFFFF` |
| **Light** | `light_pistachio` | **Pistachio Cream** | `#16A34A` (Fresh Leaf) | `#FFFFFF` |
| **Light** | `light_amber` | **Vanilla Amber** | `#B45309` (Amber Resin) | `#FFFFFF` |
| **Light** | `light_silver` | **Silver Pearl** | `#0284C7` (Sky Blue) | `#FFFFFF` |
| **Dark** | `dark_anthracite` | **Anthracite Obsidian** (Default Dark) | `#0071E3` (Apple Blue) | `#1A1D24` |
| **Dark** | `dark_forest` | **Deep Forest Emerald** | `#10B981` (Emerald Accent) | `#1B2420` |
| **Dark** | `dark_eclipse` | **Midnight Eclipse** | `#6366F1` (Indigo Neon) | `#1A1B29` |
| **Dark** | `dark_pewter` | **Pewter Slate** | `#38BDF8` (Sky Highlight) | `#20252D` |
| **Dark** | `dark_tangerine` | **Tangerine Nightfall** | `#F97316` (Citrus Glow) | `#281E19` |
| **Dark** | `dark_caviar` | **Citrus Caviar** | `#EAB308` (Gold Core) | `#1F1E19` |
| **Dark** | `dark_indigo` | **Abyssal Indigo** | `#818CF8` (Soft Violet) | `#1C1D2E` |
| **Dark** | `dark_carbon` | **Carbon Noir** | `#94A3B8` (Steel Gray) | `#1D1E22` |
| **Dark** | `dark_merlot` | **Vintage Merlot** | `#F43F5E` (Rose Blossom) | `#261B20` |
| **Dark** | `dark_gunmetal` | **Gunmetal Steel** | `#06B6D4` (Cyan Glow) | `#1B2327` |

---

## 📦 Installation & Packaging

### Windows
```powershell
# Run the automated installer
irm https://raw.githubusercontent.com/mossambi/mossambi/main/install.ps1 | iex

# Or install via package managers:
winget install Mossambi.Mossambi
scoop install mossambi
```

### Linux & macOS
```bash
# Universal Linux & macOS Script
curl -fsSL https://raw.githubusercontent.com/mossambi/mossambi/main/install.sh | bash

# Package Managers
sudo apt install ./mossambi_2.1.0_amd64.deb    # Ubuntu / Debian
sudo dnf install ./mossambi-2.1.0-1.x86_64.rpm# Fedora / RHEL
yay -S mossambi-bin                          # Arch Linux
```

---

## 🧪 Testing & Verification

Run the full integration test suite:
```bash
cargo test
```

Strict compiler and clippy gate (zero warnings enforced):
```bash
cargo clippy --all-targets -- -D warnings
```

Build production release executable:
```bash
cargo build --release
```

---

## 📜 License

Dual-licensed under the **MIT License** or **Apache License 2.0** at your option.
