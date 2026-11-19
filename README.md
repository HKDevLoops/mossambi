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
+------------------------------------------------------------------------------------------------------------------+
|  Mossambi (MOSSAMBI - ZERO-LOSS CONVERSION PLATFORM & UNIVERSAL FILE EXPLORER v2.1.0)                            |
|  One Pure Rust Core · Three Operating Modalities · Apple Elegance + Linux Power                                  |
+------------------------------------------------------------------------------------------------------------------+
|  Usage: mossambi [FLAGS / OPTIONS] [INPUT] [TARGET_FORMAT]                                                       |
|  UNIX 'mv' POSITIONAL GRAMMAR: mossambi <INPUT> [TARGET_FORMAT] [-a|--all] [-o <DIR>]                            |
+----------------------------------+------------------------------------------------------+------------------------+
|  FLAG / OPTION                   |  DESCRIPTION & FUNCTION                              |  EXAMPLE USAGE         |
+----------------------------------+------------------------------------------------------+------------------------+
|  --app, --explorer, --gui [DIR]  |  Launch macOS-styled Universal File Explorer & Studio|  mossambi --app        |
|  --widget [FILE]                 |  Launch Sweet Lime Resident Radial Widget (8 petals) |  mossambi --widget     |
|  --tui [DIR]                     |  Launch Yazi-Grade Interactive Terminal File Explorer|  mossambi --tui        |
+----------------------------------+------------------------------------------------------+------------------------+
|  -a, --all                       |  Batch transcode to ALL valid formats in domain      |  mossambi file.wav -a  |
|  -o, --output <DIR>              |  Custom destination directory for output files       |  mossambi file.png -o .|
|  --strict                        |  Enforce strict 0.00% zero-loss mathematical standard|  mossambi in.wav flac  |
|  --json                          |  Output structured machine-parseable JSON response   |  mossambi in.wav --json|
+----------------------------------+------------------------------------------------------+------------------------+
|  --exam <PRESET>                 |  Indian Exam Tuner (upsc-photo, ssc-photo, ibps-sign)|  mossambi in.jpg --exam|
|  --width <PX> / --height <PX>    |  Exact target width and height in pixels             |  --width 350 --height  |
|  --dpi <INT>                     |  Target resolution DPI (e.g. 200 or 300 DPI)         |  --dpi 300             |
|  --min-kb <N> / --max-kb <N>     |  Target file byte envelope range (e.g. 20KB to 50KB) |  --min-kb 20 --max-kb50|
|  --whiten                        |  Auto-whiten background of scanned signatures/photos |  --whiten              |
+----------------------------------+------------------------------------------------------+------------------------+
|  --archive <FILES...> -o <ZIP>   |  Pack files into ZIP, TAR, GZ, BZ2, XZ, or ZST       |  --archive a b -o c.zip|
|  --extract <ARCHIVE> [-o <DIR>]  |  Extract archive safely with Zip-Slip protection     |  --extract bundle.zip  |
+----------------------------------+------------------------------------------------------+------------------------+
|  --check <FILE>                  |  4-dimension Quality & Container Integrity Scorecard |  mossambi --check f.png|
|  --bench <FILE> [TARGET]         |  Benchmark latency, throughput (MB/s) and speedup    |  mossambi --bench f.wav|
|  --hw                            |  Display GPU/NPU hardware acceleration engines       |  mossambi --hw         |
|  --verify <ORIG> <CONV>          |  Mathematical 0.00% zero-loss verification proof     |  --verify a.png b.png  |
|  --catalogue [QUERY]             |  Search across 200+ format registry catalog          |  --catalogue lossless  |
+----------------------------------+------------------------------------------------------+------------------------+
|  --themes                        |  List 26 certified eye-safe Pantone themes (13D + 13L)|  mossambi --themes     |
|  --config [get|set|list]         |  View or edit persistent preferences (~/.mossambi)   |  --config set theme ...|
|  --daemon [start|stop|status]    |  Resident mouse gesture daemon (Win+Shift+C Hotkey)  |  --daemon start        |
|  --banner                        |  Display Sweet Lime ASCII fruit art and summary      |  mossambi --banner     |
|  --manual                        |  Display Unicode box table manual                    |  mossambi --manual     |
|  -h, --help                      |  Display this complete usage manual table            |  mossambi --help       |
|  -V, --version                   |  Display version and system architecture info        |  mossambi --version    |
+----------------------------------+------------------------------------------------------+------------------------+
```

### 1. 🪟 Universal File Explorer & Studio GUI
- Launch with `mossambi` or `mossambi app [DIR]`.
- **Windows 11 Fluent & macOS Tab Bar**: Multi-tab navigation with tab titles, tab icons, and close buttons.
- **Home Dashboard**: Quick Access 9-card folder grid (Desktop, Downloads, Documents, Pictures, Music, Videos, between_pcs, wallpapers, Recycle Bin), Storage Space Meters for all system drives, and recent files list.
- **Embedded Agave Nerd Font**: Crisp vector icons and typography for all files and folders.
- **Zero Alternate Tint Bugs**: Eye-safe background with clean card surfaces, subtle hover elevation, and responsive flex wrapping.
- **200+ Format Transcoder**: Built-in batch conversion studio, archive manager, quality inspector, and hardware telemetry.

### 2. 🍋 Sweet Lime Resident Radial Widget
- Summon with `mossambi widget [FILE]` or **`Win + Shift + C`** / **`Win + Ctrl + C`** global hotkeys.
- **Radial Petal Menu**: 8 contextual conversion sectors rendered around the cursor with real-time gesture tracking.
- Converts files in place with 0.00% zero-loss quality.

### 3. 🖥️ Yazi-Grade Interactive Terminal Explorer (TUI)
- Launch with `mossambi tui [DIR]`.
- High-speed terminal file manager powered by Ratatui and Crossterm with 60 FPS non-blocking event pump.
- **3 Switchable Layout Modes (`[F6]` key)**:
  1. **3-Column Miller Layout**: Parent directory context -> Active folder listing -> Rich syntax & metadata preview pane.
  2. **Full-Screen Dedicated Explorer**: Full-width table with sortable columns, file permissions, dates, and action toolbar.
  3. **Dual-Pane Commander**: Midnight Commander-style source and target panes with `[Tab]` switching.
- **In-App Keyboard & Mouse Help Modal (`[?]` or `[F1]`)**: Centered squircle cheatsheet overlay showing all navigation, selection, action, appearance, and search keybindings.
- **Interactive File Tag Modal (`[m]`)**: Toggle `#Tag` badges per file with NTFS Alternate Data Streams (ADS) & SQLite persistent storage.
- **Clipboard Operations (`[y]` copy, `[X]` cut, `[p]` paste)**: Recursive clipboard buffer with auto-colliding name resolution (`file_1.ext`) and undo journal logging.
- **Safe Archive Extraction (`[x]`)**: Background unpacking with Zip-Slip path traversal attack protection.
- **Recycle Bin Deletion (`[Delete]`) & Undo (`[u]`)**: Direct integration with OS recycle bin and persistent action reversal journal.
- **Theme Families & Set Toggling**:
  - `[t]`: Cycle across all 13 Pantone theme families sequentially.
  - `[T]`: Toggle appearance set between 13 Certified Dark Themes and 13 Eye-Care Light Themes.
  - `[g]`: Toggle Terminal Glass (compositor transparency and blur).

---

## 🛠️ Complete CLI Subcommands Matrix

Mossambi features an integrated suite of subcommands designed for CLI power users, shell scripting, and automation:

| Command | Arguments | Purpose & Description |
|---|---|---|
| `app` / `explorer` | `[DIR]` | Launch the Universal File Explorer & Conversion Studio GUI |
| `widget` | `[--pos X,Y] [FILE]` | Launch the Sweet Lime Resident Radial Widget overlay |
| `tui` | `[DIR]` | Launch the Yazi-grade Terminal File Explorer |
| `convert` | `<INPUT> <TARGET> [-o DIR] [-a]` | Convert a file to target format or ALL formats in domain |
| `archive` | `<INPUTS...> -o <ARCHIVE> [-t TYPE]` | Pack files into ZIP, TAR, GZ, BZ2, XZ, or ZST archive |
| `extract` | `<ARCHIVE> [-o DIR]` | Extract archive safely with Zip-Slip traversal protection |
| `info` | `<FILE>` | Inspect metadata, category, and target format possibilities |
| `check` | `<FILE>` | 4-dimension quality, integrity, parseability scorecard |
| `bench` | `<FILE> [TARGET]` | Measure transcoding latency, throughput (MB/s), and speedup |
| `hw` | none | Display detected GPU, NPU, and SIMD hardware acceleration |
| `verify` | `<SOURCE> <TARGET>` | Mathematical 0.00% zero-loss stream copy verification proof |
| `catalogue` | `[QUERY]` | Search across the 200+ format registry catalog |
| `upload-ready` | `<INPUT> [--preset P] [--width W] ...`| Prepare photos/signatures for Indian competitive exams |
| `themes` | `[--json]` | List 26 Pantone themes (13 Dark & 13 Eye-Care Light) |
| `fs copy` | `<SOURCES...> <DEST>` | Copy files/folders recursively with collision avoidance |
| `fs move` | `<SOURCES...> <DEST>` | Move files/folders with action journal recording |
| `fs rename` | `<PATH> <NEW_NAME>` | Rename file or folder with path component safety validation |
| `fs trash` | `<PATHS...>` | Safely move files to native OS Recycle Bin |
| `tags list` | `[--json]` | List all available preset and custom file tags |
| `tags add` | `<PATH> <TAG>` | Assign a tag to a file (NTFS ADS on Windows, xattr on Unix)|
| `tags remove` | `<PATH> <TAG>` | Remove a tag from a file |
| `tags find` | `<TAG> [--root DIR]` | Find all files matching a specific tag under directory |
| `queue add` | `<FILE> <TARGET> [-o DIR]` | Enqueue a file conversion task |
| `queue list` | `[--json]` | List pending and completed jobs in conversion queue |
| `queue clear`| none | Clear all pending jobs from conversion queue |
| `queue run`  | none | Execute all queued conversion jobs sequentially |
| `preview`    | `<FILE> [--json]` | Print a syntax-highlighted text preview and metadata |
| `crop`       | `<FILE> [--x X] [--y Y] [--aspect A]`| Crop image to normalized coordinates or aspect ratio |
| `route`      | `<FROM> <TO>` | Calculate shortest Dijkstra format conversion pathway |
| `search`     | `<QUERY> [-r ROOT] [--limit N]`| Fast recursive fuzzy search across directory trees |
| `batch-rename`| `<FILES...> --find X --replace Y`| Pattern-based batch rename with `%N` counter and dry-run |
| `undo`       | `[--json]` | Undo last journaled action (move, copy, trash, rename) |
| `open-with`  | `list <FILE>` | Query registered system application handlers for a file |
| `scripts`    | `list` | Enumerate custom user automation scripts (Rhai and Lua) |
| `settings`   | `get <KEY>`, `set <KEY> <VAL>`, `list`| Generic persistent configuration editor |
| `config`     | `get <KEY>`, `set <KEY> <VAL>`, `list`| View or edit preferences in `~/.mossambi/config.json` |
| `daemon`     | `start`, `stop`, `status`, `run` | Manage background resident mouse gesture listener |
| `banner`     | none | Display Sweet Lime fruit ASCII art and access summary |
| `help-manual`| none | Display complete ASCII box manual table |

---

## 🎨 26 Certified Eye-Safe Pantone Themes (13 Dark & 13 Eye-Care Light)

Mossambi comes equipped with **26 certified Pantone themes** engineered with strict WCAG AA/AAA contrast ratios (>= 4.5:1 text-to-surface contrast). Every theme family provides both a certified dark variant and an eye-care light counterpart:

### 13 Certified Dark Themes
| No. | Theme ID | Name | Background | Surface | Accent | Provenance |
|---|---|---|---|---|---|---|
| 1 | `gruvbox` | **Gruvbox** | `#282828` | `#3c3836` | `#83a598` | Classic retro groove |
| 2 | `tokyo_night` | **Tokyo Night** | `#1a1b26` | `#24283b` | `#7aa2f7` | Tokyo city neon nightfall |
| 3 | `catppuccin_mocha` | **Catppuccin Mocha** | `#1e1e2e` | `#313244` | `#89b4fa` | Soothing pastel palette |
| 4 | `modus_vivendi` | **Modus Vivendi** | `#000000` | `#1e1e1e` | `#2fafff` | High-contrast accessibility |
| 5 | `nord` | **Nord** | `#2e3440` | `#3b4252` | `#88c0d0` | Arctic blue cold stillness |
| 6 | `dracula` | **Dracula** | `#282a36` | `#44475a` | `#bd93f9` | Legendary vampire dark theme |
| 7 | `kanagawa` | **Kanagawa** | `#1f1f28` | `#2a2a37` | `#7e9cd8` | Japanese ukiyo-e woodblock |
| 8 | `one_dark` | **One Dark** | `#282c34` | `#2c313a` | `#61afef` | Atom / Pro One Dark |
| 9 | `atom` | **Atom** | `#21252b` | `#282c34` | `#528bff` | Original Atom chrome |
| 10 | `dark_plus` | **VS Code Dark+** | `#1e1e1e` | `#2d2d2d` | `#569cd6` | Visual Studio standard |
| 11 | `synthwave` | **Synthwave '84** | `#262335` | `#2e2942` | `#ff7edb` | 80s retro neon outrun |
| 12 | `rangalipi` | **Rangalipi** *(Default Dark)* | `#342b3f` | `#3a2e4b` | `#e4584f` | Mossambi Bengali Alpona signature |
| 13 | `mossambi_punch_night`| **Mossambi Punch Night** | `#1b2116` | `#242c1d` | `#8fc24f` | Nightfall sweet lime orchard |

### 13 Eye-Care Light Themes
| No. | Theme ID | Name | Background | Surface | Accent | Provenance |
|---|---|---|---|---|---|---|
| 1 | `mossambi_punch` | **Mossambi Punch** *(Default Light)* | `#e6e0cf` | `#efe9dc` | `#3d7a4f` | True sweet-lime fruit palette |
| 2 | `gruvbox_light` | **Gruvbox Light** | `#fbf1c7` | `#f7ecc5` | `#076678` | Warm parchment paper |
| 3 | `tokyo_night_day` | **Tokyo Night Day** | `#e1e2e7` | `#e2e6ee` | `#2e7de9` | Daytime Tokyo daylight |
| 4 | `catppuccin_latte` | **Catppuccin Latte** | `#eff1f5` | `#e6e9ef` | `#1e66f5` | Creamy coffee latte |
| 5 | `modus_operandi` | **Modus Operandi** | `#f4f1ea` | `#ece7df` | `#0031a9` | High-contrast WCAG AAA light |
| 6 | `nord_light` | **Nord Light** | `#eceff4` | `#e2e6ee` | `#5e81ac` | Snow storm daylight |
| 7 | `alucard` | **Alucard** | `#f7f2dc` | `#eee6d2` | `#644ac9` | Dracula light counterpart |
| 8 | `kanagawa_lotus` | **Kanagawa Lotus** | `#f2ecbc` | `#eae3b2` | `#4d699b` | Morning lotus flower |
| 9 | `one_light` | **One Light** | `#eeebe6` | `#e6e2dc` | `#4078f2` | Clean minimal editor light |
| 10 | `atom_light` | **Atom Light** | `#e2dfdb` | `#dad7d2` | `#526fff` | Soft alabaster paper |
| 11 | `light_plus` | **VS Code Light+** | `#f3efe6` | `#ebe6dc` | `#005fb8` | Warm Visual Studio light |
| 12 | `synthwave_day` | **Synthwave '84 Day** | `#f8f1ff` | `#efe6fa` | `#d13fb0` | Daylight neon vaporwave |
| 13 | `rangalipi_day` | **Rangalipi Day** | `#fbf4ee` | `#f2e7de` | `#c8423a` | Morning terracotta & kumkum |

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
