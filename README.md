# 🍋 Mossambi — Universal File Conversion Platform

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

   ╭╮╭╮╭╮ ╭───╮ ╭────╮ ╭────╮ ╭───╮ ╭╮╭╮╭╮ ╭────╮ ╭─╮
   ││││││ │╭─╮│ │╭───╯ │╭───╯ │╭─╮│ ││││││ │╭─╮│ │ │
   ││││││ ││ ││ ╰───╮  ╰───╮  ││ ││ ││││││ │╰─╯│ │ │
   ││││││ │╰─╯│ ╭───╯  ╭───╯  │╰─╯│ ││││││ │╭─╮│ │ │
   ╰╯╰╯╰╯ ╰───╯ ╰────╯ ╰────╯ ╰─╰─╯ ╰╯╰╯╰╯ ╰───╯ ╰─╯

   Universal File Conversion Platform · Indian Exam Form Studio
   One Rust Core · Four Native Interfaces: CLI, TUI, Desktop Studio GUI & Standalone Radial Widget
```

**Mossambi** (Sweet Lime) is a high-performance native desktop file conversion suite and Indian competitive exam form optimizer written entirely in **100% pure Rust**. It combines **Apple macOS elegance** (liquid glass, specular highlights, authentic traffic lights) with **Linux power** (streaming chunk hashing, direct NVMe stream copy, and friction-free UNIX `mv`-style grammar).

---

## ⚡ Quick Install

### 🪟 Windows Binary Installers
- **Windows Installer (`.msi`)**: [`mossambi-2.1.0-x64.msi`](https://github.com/mossambi/mossambi/releases/download/v2.1.0/mossambi-2.1.0-x64.msi)
  - Full WiX Toolset standard MSI with Start Menu shortcuts, Desktop icon, and `PATH` registration.
- **Portable ZIP Bundle**: [`mossambi-v2.1.0-windows-x64.zip`](https://github.com/mossambi/mossambi/releases/download/v2.1.0/mossambi-v2.1.0-windows-x64.zip)
  - Portable bundle with all 4 binaries, `setup.bat`, `install.ps1`, and icon assets.
- **PowerShell One-Liner**:
  ```powershell
  irm https://raw.githubusercontent.com/mossambi/mossambi/main/install.ps1 | iex
  ```

### 🐧 Linux Binary Packages & One-Liner
- **Debian / Ubuntu (`.deb`)**: [`mossambi_2.1.0_amd64.deb`](https://github.com/mossambi/mossambi/releases/download/v2.1.0/mossambi_2.1.0_amd64.deb)
  ```bash
  sudo apt install ./mossambi_2.1.0_amd64.deb
  ```
- **Fedora / RHEL / openSUSE (`.rpm`)**: [`mossambi-2.1.0-1.x86_64.rpm`](https://github.com/mossambi/mossambi/releases/download/v2.1.0/mossambi-2.1.0-1.x86_64.rpm)
  ```bash
  sudo dnf install ./mossambi-2.1.0-1.x86_64.rpm
  ```
- **Universal Linux & macOS Script**:
  ```bash
  curl -fsSL https://raw.githubusercontent.com/mossambi/mossambi/main/install.sh | bash
  ```

---

## 📦 Package Managers Supported

| OS / Distro | Package Manager | Install Command | Manifest Location |
|---|---|---|---|
| **Windows** | `winget` | `winget install Mossambi.Mossambi` | [`packaging/winget/Mossambi.Mossambi.yaml`](packaging/winget/Mossambi.Mossambi.yaml) |
| **Windows** | `scoop` | `scoop install mossambi` | [`packaging/scoop/mossambi.json`](packaging/scoop/mossambi.json) |
| **Arch Linux** | `pacman` / `yay` | `yay -S mossambi-bin` | [`packaging/arch/PKGBUILD`](packaging/arch/PKGBUILD) |
| **Ubuntu / Debian** | `apt` | `sudo apt install ./mossambi_2.1.0_amd64.deb` | [`packaging/debian/`](packaging/debian/) |
| **Fedora / RHEL** | `dnf` / `rpm` | `sudo dnf install mossambi` | [`packaging/rpm/mossambi.spec`](packaging/rpm/mossambi.spec) |
| **openSUSE** | `zypper` | `sudo zypper install mossambi` | [`packaging/zypper/mossambi-opensuse.spec`](packaging/zypper/mossambi-opensuse.spec) |
| **Gentoo** | `emerge` | `emerge --ask app-misc/mossambi` | [`packaging/gentoo/mossambi-2.1.0.ebuild`](packaging/gentoo/mossambi-2.1.0.ebuild) |
| **Nix / NixOS** | `nix` | `nix profile install github:mossambi/mossambi` | [`packaging/nix/flake.nix`](packaging/nix/flake.nix) |
| **GNU Guix** | `guix` | `guix package -f packaging/guix/mossambi.scm` | [`packaging/guix/mossambi.scm`](packaging/guix/mossambi.scm) |
| **Alpine Linux** | `apk` | `apk add mossambi` | [`packaging/alpine/APKBUILD`](packaging/alpine/APKBUILD) |
| **macOS / Linux** | `brew` | `brew install packaging/homebrew/mossambi.rb` | [`packaging/homebrew/mossambi.rb`](packaging/homebrew/mossambi.rb) |

---

## 🎨 Official Sweet Lime Brand Icons (`assets/icons/`)

Mossambi includes custom-crafted, publication-grade Sweet Lime fruit icon artwork across all standard dimensions:

| Format / Dimension | Path | Resolution | Usage |
|---|---|---|---|
| **Scalable Vector** | [`assets/icons/mossambi.svg`](assets/icons/mossambi.svg) | Vector | Scalable vector displays, web banners, Linux theme icons |
| **Windows Multi-Res Icon** | [`assets/icons/mossambi.ico`](assets/icons/mossambi.ico) | 16–256 px | Windows Explorer, Taskbar, MSI Installer, Desktop Shortcut |
| **Ultra-High Resolution** | [`assets/icons/mossambi-1024x1024.png`](assets/icons/mossambi-1024x1024.png) | 1024x1024 | App Store, Retina display splashes, marketing |
| **Standard App Icon** | [`assets/icons/mossambi-512x512.png`](assets/icons/mossambi-512x512.png) | 512x512 | Linux `/usr/share/pixmaps/`, Flathub, Snap Store |
| **Desktop Frame Icon** | [`assets/icons/mossambi-256x256.png`](assets/icons/mossambi-256x256.png) | 256x256 | Embedded in `mossambi-gui` and `mossambi-widget` windows |
| **Large Shell Icon** | [`assets/icons/mossambi-128x128.png`](assets/icons/mossambi-128x128.png) | 128x128 | Application docks, modern desktop environments |
| **Medium Shell Icon** | [`assets/icons/mossambi-64x64.png`](assets/icons/mossambi-64x64.png) | 64x64 | Linux hicolor standard |
| **Small Shell Icon** | [`assets/icons/mossambi-48x48.png`](assets/icons/mossambi-48x48.png) | 48x48 | Windows Control Panel, file dialogs |
| **Taskbar / Menu Icon** | [`assets/icons/mossambi-32x32.png`](assets/icons/mossambi-32x32.png) | 32x32 | Window titlebar and alt-tab switcher |
| **System Tray Micro Icon** | [`assets/icons/mossambi-16x16.png`](assets/icons/mossambi-16x16.png) | 16x16 | Notification area, small list views |

---

## 🏛️ Architecture: One Core, Four Interfaces

```
                    ┌────────────────────────────────────────────────────────┐
                    │               MOSSAMBI SHARED CORE                     │
                    │  200+ Format Registry · Mathematical Remux Engine      │
                    │  Pantone Palette · Exam Constraint Solver · Checkers   │
                    └────────────────────────────────────────────────────────┘
                                   │            │             │
                ┌──────────────────┘            │             └──────────────────┐
                ▼                               ▼                                ▼
       ┌─────────────────┐             ┌─────────────────┐             ┌─────────────────┐
       │  mossambi (CLI) │             │  mossambi-tui   │             │   mossambi-gui  │
       │  UNIX mv Syntax │             │ Ratatui Studio  │             │ Desktop Studio  │
       │  Auto-Complete  │             │ Traffic Lights  │             │ Sidebar Layout  │
       │  Carapace Spec  │             │ 6 macOS Worksp. │             │ Native Controls │
       └─────────────────┘             └─────────────────┘             └─────────────────┘
                                                                                 ▲
                                                                                 │
                                                                       ┌─────────────────┐
                                                                       │ mossambi-widget │
                                                                       │ Pure Fruit Ring │
                                                                       │ 0 Topbars/Menus │
                                                                       │ 32-bit Transpar.│
                                                                       └─────────────────┘
```

1. **`mossambi` (CLI)**: Friction-free UNIX grammar modeled after `mv` (`mossambi input.wav flac`). Supports `--all` batch transcode, shell auto-completions (`bash`, `zsh`, `fish`, `powershell`, `elvish`), context-aware Carapace bridge, and Unicode table manuals.
2. **`mossambi-tui` (TUI)**: Terminal user interface styled after macOS Sequoia, featuring authentic traffic lights, Pantone color themes, and 6 full workspaces (Transcoder & ALL, Indian Exam Studio, Archive Manager, Hardware Diagnostics, Settings & Themes, and Quality Inspector).
3. **`mossambi-gui` (Desktop Studio GUI)**: Standard application-style workspace window with native titlebar, window decorations, sidebar navigation, drag-and-drop batch queue, format catalogue modal, and settings.
4. **`mossambi-widget` (Standalone Radial Fruit Widget)**: 100% Topbarless, Menu-less, Ribbon-less, Frameless, 32-Bit Transparent Desktop Widget. Faithfully reproduces all 6 visual states from `Image reference.png`, summons directly under cursor, and converts dropped files beside their source.

---

## 🚀 Whisper Flow Resident Background Service & Global Hotkeys

Mossambi operates silently in the background with **zero idle resource load** (~5-8 MB RAM, 0% CPU):
- **`Win + Shift + C`**: Summons the Floating Radial Sweet Lime Wheel (`mossambi-widget`) directly under the user's cursor.
- **`Win + Ctrl + C`**: Launches the Full Desktop Studio GUI (`mossambi-gui`).
- **Whisper Flow Onboarding Prompt**: `install.ps1` and `install.sh` interactively ask whether you'd like Mossambi running quietly on startup.
- **CLI Daemon Management**:
  ```bash
  mossambi daemon status          # Check resident status & autostart
  mossambi daemon start | stop    # Spawn or gracefully terminate daemon
  mossambi daemon gesture-on      # Enable file drag gesture activation
  mossambi daemon gesture-off     # Disable gesture activation
  mossambi daemon install         # Register OS startup entry
  ```

---

## 🖱️ Deep System-Level File Drag & Drop Gesture Engine

A specialized system-level listener that monitors mouse motion **strictly when the user is actively dragging actual files or folders**:
1. **Zero Idle Overhead (Deep Window Gate)**:
   - On Windows: Uses a dedicated thread with a Win32 low-level mouse hook (`WH_MOUSE_LL`).
   - On `WM_LBUTTONDOWN`, verifies if the cursor is pressing on File Explorer (`CabinetWClass`), Desktop (`Progman` / `WorkerW`), or File Dialogs (`#32770`).
   - If user is clicking in Chrome, Discord, or Word, the hook passes through (`CallNextHookEx`) with **zero gesture calculations and zero CPU cycles**.
2. **Kinematic Gesture Recognition**:
   - **Shake**: Detects $\ge 3$ rapid horizontal direction reversals within $\le 450\text{ ms}$.
   - **Circle**: Detects a swept circular arc $\ge 300^\circ$ within $\le 500\text{ ms}$.
3. **Instant Radial Overlay**:
   - The Sweet Lime radial wheel appears centered at `(cursor.x - 180, cursor.y - 180)` in **State 3 (Active Drag & Drop Petals)** tailored to the dragged file format.
   - Releasing the mouse button over any petal converts the file and places the output directly beside the original file.

---

## 💎 Mathematical Remuxing: 0.00% Zero-Loss Standard

Mossambi guarantees the **0.00% Zero-Loss mathematical standard** across media transformations:
- **Video Stream Copy Fast-Path (`-c copy`)**: Converts containers (MKV $\leftrightarrow$ MP4 $\leftrightarrow$ MOV $\leftrightarrow$ TS) at disk NVMe read/write speed (~1-2 GB/s) in **5 to 15 seconds** for a 2-hour movie, with **0 bytes of re-encoding loss**.
- **Audio Linear PCM & Lossless Compression**: 24-bit / 48 kHz Linear PCM (`pcm_s24le`), FLAC Level 8, ALAC, and LAME 320 kbps V0 Extreme.
- **Image Raster Precision**: 300 DPI metadata injection into JFIF (`APP0`) and PNG (`pHYs`), clean alpha canvas compositing (preventing pitch black transparency artifacts), and bit-exact pixel matrix comparison.
- **Document Vector Preservation**: Text and markdown converted into vector PDF with native Type 1 PostScript font CMap embedding (zero bitmap rasterization loss).

---

## 🇮🇳 Indian Competitive Exam & Form Studio (Upload Ready)

Constraint-based document, photo, and signature optimizer designed for strict Indian portals (**UPSC, SSC, IBPS, GATE, RRB, CAT, JEE, NEET**):
- **5-Iteration 100% Scorecard Loop**: Dimension scaling, contrast normalization, mobile scan whitening, and binary-search byte size convergence.
- **Non-Destructive Metadata Padding**: Inserts compliant JPEG comment markers (`0xFF 0xFE`) to satisfy strict portal minimum size thresholds without pixel distortion.
- **Built-in Presets**:
  - `upsc-photo`: 350x450 px, 300 DPI, 20 KB – 300 KB
  - `upsc-signature`: 350x150 px, 300 DPI, 20 KB – 300 KB
  - `ssc-photo`: 100x120 px, 200 DPI, 20 KB – 50 KB
  - `ssc-signature`: 140x60 px, 200 DPI, 10 KB – 20 KB
  - `ibps-photo`: 200x230 px, 200 DPI, 20 KB – 50 KB
  - `ibps-signature`: 140x60 px, 200 DPI, 10 KB – 20 KB

```bash
mossambi candidate.png --exam upsc-photo
mossambi signature.jpg --exam ibps-signature
```

---

## 🔬 Universal Quality Inspector & Benchmark Engine

Exhaustively checks files across 4 distinct dimensions with **zero RAM spikes** on multi-gigabyte media:
- **Streaming Hashes**: 1 MB chunk streaming for SHA-256 and CRC-32 (RAM clamped to ~1 MB on 50 GB movies).
- **Instant Header/Trailer Sampling**: 64 KB header and trailer disk seek for instant container validation.
- **Cross-Platform Compatibility**: Compliance scores for Web, Mobile, Archival, and Government portals.
- **Performance Benchmark Matrix**: Measures latency and throughput, comparing speedups against Online APIs (CloudConvert, Zamzar) and Offline CLIs (ImageMagick, FFmpeg).

```bash
mossambi check test_vault/image/sample_photo.png
mossambi bench test_vault/image/sample_photo.png webp
```

---

## 🛡️ Security & Pessimistic Engineering Audit

- **UTF-8 Character Boundary Slicing Safety**: Replaced raw byte slicing with character-aware iterators, eliminating panics on multi-byte international unicode characters or emojis.
- **Win32 Hook RAII Guard (`HookGuard`)**: Guaranteed unhooking of `WH_MOUSE_LL` on thread exit or panic, eliminating desktop mouse lag.
- **COM Initialization RAII Guard (`ComGuard` & `ComReleaseGuard`)**: Eliminates COM apartment leaks during active Explorer file detection.
- **Zip-Slip & Tar-Slip Shield**: Rejects `ParentDir` (`..`), `RootDir`, and `Prefix` paths; prevents symlink and hardlink directory traversal.
- **Decompression Bomb Ceilings**: 4 GB uncompressed byte ceiling, 20,000 archive entry cap, and 16,384 px / 512 MB image limit (`open_image_safe`).
- **Memory Ceiling Caps**: 100 MB text ceiling, 50 MB subtitle read limit with cached `OnceLock<Regex>`, 50 MB DOCX XML bomb limit, 10,000 spreadsheet preview row cap, 100 MB JPEG padding cap.
- **PDF Vector JPEG Magic Byte Verification**: Validates `[0xFF, 0xD8]` magic bytes before applying `DCTDecode`, falling back to `FlateDecode` to prevent corrupted PDF generation.
- **Adaptive Repaint**: 16ms (60 FPS) when converting; 150-200ms when idle (slashes idle CPU churn by >90%).

---

## 🧪 Testing & Verification

Run the full workspace test suite:
```bash
cargo test --workspace
```
Strict clippy zero-warning gate:
```bash
cargo clippy --workspace --all-targets -- -D warnings
```
Build release binaries:
```bash
cargo build --release
```

---

## 📜 License

Dual-licensed under the **MIT License** or **Apache License 2.0** at your option.
