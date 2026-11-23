# Donut Toolbar, Widget Settings & Native Converter Engine Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build complete visual and functional parity with Filee 2.5.0 for the Mossambi Radial Donut Toolbar and Widget Settings (all 10 tabs, 20 domain profiles, 2-face Front/Back donut with Tab/wheel 180ms 3D flip, preset editor, watch folders, shortcuts) powered by a pure native Rust conversion engine delivering 0.00% zero-data-loss parity, ICC color profile preservation, EXIF retention, and zero external downloads, with intelligent auto-discovery and quality-gated shifting to existing system-installed engines (ArtCraft, LibreOffice, FFmpeg, Inkscape, Ghostscript) when native parsing cannot achieve industry grade parity.

**Architecture:** 
1. **Config & Profile Layer (`src/core/donut_config.rs`, `src/core/presets.rs`)**: 20 domain profiles with explicit Front and Back preset IDs matching Filee's `BuiltInData.cs`, customizable extension tags, and preset definitions with ICC, EXIF, DPI, and quality parameters.
2. **Radial Widget Layer (`src/widget/widget_app.rs`, `src/widget/citrus_geometry.rs`, `src/gui/donut.rs`)**: 2-face Sweet Lime geometry with central columella hub (`Profile N/12` vs `Profile Back M/12`), interactive 180ms cosine 3D flip animation triggered via `Tab` key or mouse wheel, sub-ring group fan-out, and pure-memory profile switching (zero `Box::leak`).
3. **Widget Settings Surface (`src/gui/preferences_view.rs`)**: 10-tab settings window mirroring Filee 2.5.0: Home (drop zone + recent table), General (system context menu, save location, browser drag, proxy), Shortcuts (gesture cards, Tab/wheel flip settings, app exclusion list), Donut toolbar (3-column profile picker, interactive donut preview with Edit Donut Size, available presets drawer), Presets (preset gallery + detailed property inspector for ICC, EXIF, DPI, debayer, PDF tall image), Fields (12 category toggles), Watch folders (auto-transcode directories), Theme (Filee Soft vs Material You, accent swatches), Engines (pure native Rust status + detected system engines on PC, zero downloads), About.
4. **Native Transcoder Engine & Quality-Gated Existing Engine Shift (`src/codec/mod.rs`, `src/codec/icc_meta.rs`, `src/core/existing_engines.rs`)**: Pure Rust transcoding pipeline preserving embedded ICC color profiles (PNG `iCCP`, JPEG `APP2`, WebP `ICCP`, TIFF `0x8773`), 4:4:4 full-chroma preservation, EXIF date and metadata retention, 0.00% mathematical zero-loss verification. Includes `ExistingEngineRegistry` scanning `PATH` and OS standard directories (ArtCraft, LibreOffice, FFmpeg, Inkscape, Ghostscript) so that if native transcode lacks complex script shaping or proprietary format fidelity, execution seamlessly shifts to the user's existing engine without downloading any third-party bloat.

**Tech Stack:** Rust 2021, `egui` / `eframe`, `image` crate (PNG, JPEG, WebP, TIFF, BMP, ICO, AVIF), `resvg` / `tiny-skia` (SVG), `lopdf` (PDF), `calamine` + `rust_xlsxwriter` (Excel/CSV), `zip` / `flate2` / `zstd` (Archives), `filetime` (timestamps), ArtCraft stdio bridge, system CLI auto-discovery.

**Spec:** Filee 2.5.0 Reference Architecture (`ref/Filee/src/Filee.Core/Presets/BuiltInData.cs`, `ToolbarProfile.cs`, `AppSettings.cs`), 12 user-provided interface screenshots, and Phase 23 Converter Roadmap (`.planning/phases/23-converter-engine-donut-tangerine-artcraft/PLAN.md`).

---

## Global Constraints

- **Pure 7-Bit ASCII**: Every Rust source file, comment, and string literal must be 100% 7-bit ASCII (`0x00..=0x7F`). Zero unicode symbols or emoji in Rust code (enforced by `tests/gui_ascii_gate.rs`).
- **Zero Memory Leaks**: Absolutely no `Box::leak` in profile switching or dynamic preset allocation. All state transitions must use owned values, reference borrows, or `Arc`.
- **Zero External Download Requirement**: Core conversions must run on pure native Rust codecs bundled inside Mossambi without requiring 1.3 GB LibreOffice or external installs. Never download external bundles in the background.
- **Existing System Engine Shifting**: Detect engines *already installed on the computer*. If native processing for complex proprietary or script-shaped formats fails or does not meet industry standards, shift execution to existing system tools.
- **Industry-Grade Color Parity**: Preserve embedded ICC color profiles across raster formats so wide-gamut graphics (Display P3, Adobe RGB) do not suffer color shifts.
- **Lossless Standard (0.00% Delta)**: Enforce mathematical 0.00% zero-data-loss for supported remuxes, archives, and lossless image targets.
- **Cross-Platform & Cross-Architecture**: Build and execute cleanly on Windows (Win32), macOS (Cocoa), Linux (X11/Wayland) across `x86_64` and `aarch64`.

---

## Review Focus

1. **Donut Face Flipping**: Ensure pressing `Tab` or scrolling mouse wheel over the open Donut flips between Front and Back faces with a smooth 180ms animation without flickering or dropping input.
2. **ICC Color Profile Retention**: Transcoding a PNG or JPEG containing an embedded Display P3 ICC profile to WebP or TIFF must preserve the ICC chunk intact without color clipping.
3. **Existing Engine Discovery & Fallback Shift**: Transcoding a format requiring complex shaping or vector translation shifts to installed ArtCraft / LibreOffice / FFmpeg if present, and never initiates background network downloads.
4. **Preset Customization Persistence**: Edits made in the Presets tab (e.g. setting DPI to 600 or toggling Keep ICC Profile) must serialize cleanly to `config.json` and persist across application restarts.
5. **App Exclusion List**: Dragging files while an excluded process (e.g. Photoshop) has foreground focus must suppress the Donut trigger.

---

## Task Structure

### Task 1: Donut Two-Face Profile Data Model Parity

**Files:**
- Modify: `src/core/donut_config.rs`
- Test: `tests/test_donut_profiles.rs`

- [x] **Step 1: Write failing tests for the 20 Filee profiles and Front/Back slice counts**
  Write tests in `tests/test_donut_profiles.rs` asserting:
  - Total profile count is 20 (`images`, `pdf`, `office`, `spreadsheets`, `presentations`, `hwp`, `text`, `ebooks`, `video`, `audio`, `subtitles`, `vector`, `archives`, `folders`, `cad`, `fonts`, `data`, `models`, `geo`, `mixed`).
  - Profile slice counts match Filee 2.5.0: Images has 7 front and 8 back; PDF has 8 front and 8 back; Documents has 7 front and 4 back; Spreadsheets has 6 front and 5 back; etc.
  - Extension lookup maps `.png`, `.jpg`, `.dwg`, `.glb`, `.xlsx`, `.docx`, `.mp4` to their designated profiles.
  - Verify test fails: `cargo test --test test_donut_profiles`

- [x] **Step 2: Run test to confirm failure**
  Confirm test fails due to missing profiles or mismatched slice counts:
  `cargo test --test test_donut_profiles`

- [x] **Step 3: Update `src/core/donut_config.rs` with exact Filee 2.5.0 profile definitions**
  Implement:
  - Complete 20 profiles matching `BuiltInData.cs`.
  - Add explicit `front_preset_ids: Vec<String>` and `back_preset_ids: Vec<String>`.
  - Add helper `profile.front_count()` and `profile.back_count()` returning formatted counts (e.g. `"7·8"`).
  - Add helper `profile.contains_extension(ext: &str) -> bool`.
  - Ensure 100% 7-bit ASCII compliance.

- [x] **Step 4: Run test to verify it passes**
  `cargo test --test test_donut_profiles`

- [x] **Step 5: Commit changes**
  `git commit -m "feat(donut): implement 20 domain profiles with two-face preset counts matching Filee 2.5.0"`

---

### Task 2: Radial Donut 2-Face Geometry & 180ms 3D Flip Animation

**Files:**
- Modify: `src/widget/widget_app.rs`
- Modify: `src/widget/citrus_geometry.rs`
- Modify: `src/gui/donut.rs`
- Test: `tests/test_donut_flip.rs`

- [x] **Step 1: Write failing tests for Donut face toggle state and flip animation calculation**
  Write tests in `tests/test_donut_flip.rs` checking:
  - `DonutFace` enum (`Front`, `Back`) with `toggle()`.
  - Flip animation progress `0.0..=1.0` calculating horizontal scale factor `scale_x = cos(progress * PI).abs()`.
  - Face transitions at `progress == 0.5`.
  - Face switch on `Tab` key press and on mouse wheel scroll event.
  - Slices served correspond to active face (Front vs Back).

- [x] **Step 2: Run test to confirm failure**
  `cargo test --test test_donut_flip`

- [x] **Step 3: Implement 3D flip animation and face state in radial widget**
  - Add `active_face: DonutFace` and `flip_anim_start: Option<Instant>` in `WidgetApp`.
  - In `widget_app.rs` event handling:
    - On `Key::Tab` or mouse wheel up/down over the widget: trigger `toggle_face()` with 180ms duration.
  - In `citrus_geometry.rs` / `donut.rs`:
    - Apply `scale_x` transform to petal polygon generation during flip transition.
    - Render center columella text: Front shows `"<Profile> <N>/12"`, Back shows `"<Profile> Back · <M>/12"`.
    - Render back-face tool labels (`Crop`, `Rotate`, `Compress`, `Half size`, `Cut out`, `Merge`, etc.).
  - Ensure 100% 7-bit ASCII compliance.

- [x] **Step 4: Run test to verify it passes**
  `cargo test --test test_donut_flip`

- [x] **Step 5: Commit changes**
  `git commit -m "feat(widget): add two-face Donut geometry with Tab and mouse wheel 180ms 3D flip animation"`

---

### Task 3: Comprehensive 10-Tab Widget Settings Surface Overhaul

**Files:**
- Modify: `src/gui/preferences_view.rs`
- Modify: `src/core/config.rs`
- Test: `tests/test_preferences_categories.rs`

- [x] **Step 1: Write failing test for 10 Filee settings categories and default configs**
  Write test in `tests/test_preferences_categories.rs`:
  - Categories list: `["Home", "General", "Shortcuts", "Donut toolbar", "Presets", "Fields", "Watch folders", "Theme", "Engines", "About"]`.
  - Config fields: `autostart`, `close_action`, `keep_file_dates`, `collect_dropped_files`, `save_geospatial_as_zip`, `browser_drag_save_dir`, `network_proxy`, `history_limit` (default 200), `watch_folders`.

- [x] **Step 2: Run test to confirm failure**
  `cargo test --test test_preferences_categories`

- [x] **Step 3: Implement the 10-tab structure in `preferences_view.rs`**
  - Update `CATEGORIES` array to match the 10 tabs exactly.
  - Implement `render_home()`: Large drag-and-drop landing box with dotted/rounded border, cursor shortcut hint, recent conversions table with `Clear` button.
  - Implement `render_general()`: Language, autostart toggle, close behavior dropdown, Windows context menu button, default save directory dropdown, keep file dates toggle, collect dropped files toggle, browser drag config, proxy input, conversion history limit, settings folder open button.
  - Implement `render_shortcuts()`: Active gesture cards with toggle switch, modifier pills (`Ctrl`, `Alt`, `Shift`, `Win`), mouse button dropdown, opens on face dropdown, drag distance slider (45px); Flip the donut card (`Tab` record, mouse wheel toggle); App exclusion list with process picker.
  - Implement `render_watch_folders()`: `+ Add folder` button, table of watched folders with format target and pause/remove controls, CLI hint.
  - Implement `render_theme()`: Filee Soft vs Material You selector, Light/Dark/System mode, accent color chips, softness slider, opacity slider.
  - Implement `render_engines()`: Pure Rust Native engine status cards (Image, PDF, Document, Spreadsheet, Archive, Audio, Video), Detected System Engines on PC (ArtCraft, FFmpeg, LibreOffice, Inkscape, Ghostscript) with path and version badges, zero download prompts.
  - Implement `render_about()`: App title, version, architecture, links to GitHub & Sponsors.
  - Ensure 100% 7-bit ASCII compliance.

- [x] **Step 4: Run test to verify it passes**
  `cargo test --test test_preferences_categories`

- [x] **Step 5: Commit changes**
  `git commit -m "feat(gui): overhaul preferences view with 10-tab Filee 2.5.0 layout and setting cards"`

---

### Task 4: Donut Toolbar Tab 3-Column Interactive Profile & Preset Editor

**Files:**
- Modify: `src/gui/preferences_view.rs`
- Test: `tests/test_donut_toolbar_view.rs`

- [x] **Step 1: Write failing test for Donut toolbar tab state and operations**
  Write tests in `tests/test_donut_toolbar_view.rs`:
  - Profile selection updates selected index.
  - Front/Back pill toggle switches view face.
  - Extension add/remove updates profile's extension list.
  - Preset assignment adds slice to active face up to maximum limit (12).

- [x] **Step 2: Run test to confirm failure**
  `cargo test --test test_donut_toolbar_view`

- [x] **Step 3: Implement 3-column Donut toolbar editor**
  - **Column 1 (`render_donut_col1`)**:
    - Search text edit for filtering profiles.
    - Profile list showing name, extension preview, and exact Front·Back count badge (e.g. `Images 7·8`).
    - `+ New profile` button at the bottom.
  - **Column 2 (`render_donut_col2`)**:
    - Front/Back pill switcher (`[ Front 7 ]` `[ Back 8 ]`) with caption `"Flip the open donut: Tab / mouse wheel"`.
    - Live interactive Sweet Lime Donut preview with carpel petals and central columella hub (`Images 7/12` vs `Images Back · 8/12`).
    - `Edit Donut Size` button (toggles diameter and hole sliders).
    - Profile details card: Name input, `Use for mixed selections` toggle, `Used for these extensions` tag cloud with individual `x` remove badges and `+ Add` input field.
  - **Column 3 (`render_donut_col3`)**:
    - Header actions: `+ New preset`, `New group`, `Copy to folder`.
    - Search input.
    - Category filter chips: `[ All ]` `[ Image ]` `[ Document ]` `[ Video & audio ]` `[ Edit ]` `[ 3D & CAD ]`.
    - Categorized available presets list with click-to-assign action.
  - Ensure 100% 7-bit ASCII compliance.

- [x] **Step 4: Run test to verify it passes**
  `cargo test --test test_donut_toolbar_view`

- [x] **Step 5: Commit changes**
  `git commit -m "feat(gui): implement 3-column Donut toolbar editor with interactive preview and extension tags"`

---

### Task 5: Dedicated Presets Editor Tab with ICC Profile & Fine-Tuning Controls

**Files:**
- Create: `src/core/preset_model.rs`
- Modify: `src/core/presets.rs`
- Modify: `src/gui/preferences_view.rs`
- Test: `tests/test_preset_editor.rs`

- [x] **Step 1: Write failing tests for Preset model serialization and parameters**
  Write tests in `tests/test_preset_editor.rs`:
  - Preset properties: `name`, `convert_to`, `category`, `icon`, `short_name`, `show_on_slice`, `text_direction`.
  - Image tuning: `resize_mode`, `rotate_deg`, `flip_mode`, `dpi` (Option<u32>), `keep_metadata_exif` (bool), `keep_icc_profile` (bool), `grayscale` (bool), `remove_background` (bool), `bg_color` (String), `debayer_fits` (bool), `save_video_frames` (bool).
  - PDF tuning: `stack_pages_tall_image` (bool).
  - Round-trip JSON serialization and persistence to `presets.json`.

- [x] **Step 2: Run test to confirm failure**
  `cargo test --test test_preset_editor`

- [x] **Step 3: Implement Preset data model and GUI editor pane**
  - Create `src/core/preset_model.rs` with `PresetDefinition` and default built-in presets (PNG, JPG, JPG 70%, WEBP, TIFF, BMP, ICO, GIF, AVIF, HEIC, Half size, Grayscale, Remove background, Trim transparent borders, etc.).
  - In `preferences_view.rs`, implement `render_presets()`:
    - Left column: `+ New`, duplicate, delete, search box, list of presets with target format badges.
    - Right column: Property inspector with:
      - Name, Convert to format dropdown, Category dropdown.
      - Icon, Short name, Show on slice, Text direction.
      - Image section: Resize dropdown, Rotate/Flip dropdowns, DPI checkbox + input, Keep metadata (EXIF) checkbox, Keep colour profile (ICC) checkbox, Grayscale checkbox, Remove background checkbox + hex color input, Debayer FITS checkbox, Save video frames checkbox.
      - PDF section: Stack pages into tall image checkbox.
      - Action buttons: `Revert`, `Save`.
  - Ensure 100% 7-bit ASCII compliance.

- [x] **Step 4: Run test to verify it passes**
  `cargo test --test test_preset_editor`

- [x] **Step 5: Commit changes**
  `git commit -m "feat(presets): create dedicated Presets editor tab with ICC profile, EXIF, and dimension controls"`

---

### Task 6: Native Converter Engine, ICC Parity & Existing Engine Quality-Gated Shift

**Files:**
- Create: `src/core/existing_engines.rs`
- Create: `src/codec/icc_meta.rs`
- Modify: `src/codec/mod.rs`
- Modify: `src/codec/dpi_meta.rs`
- Modify: `src/core/route_planner.rs`
- Test: `tests/test_icc_preservation.rs`
- Test: `tests/test_existing_engines.rs`

- [x] **Step 1: Write failing tests for ICC preservation and existing engine discovery**
  Write tests in `tests/test_icc_preservation.rs` and `tests/test_existing_engines.rs`:
  - Create synthetic PNG with embedded `iCCP` profile chunk; transcode to JPEG, WebP, and TIFF with `keep_icc = true`; verify exact ICC chunk preserved.
  - Test `ExistingEngineRegistry::detect()` finding installed tools on `PATH` or standard folders without initiating network calls.
  - Test route planner prioritizing Native Rust (cost 10) -> ArtCraft on PC (cost 15) -> Existing CLI on PC (cost 25).
  - Test quality fallback: if native transcode lacks complex script shaping or proprietary format support, transcode shifts to the discovered existing engine.

- [x] **Step 2: Run tests to confirm failure**
  `cargo test --test test_icc_preservation`
  `cargo test --test test_existing_engines`

- [x] **Step 3: Implement ICC extraction/injection, ExistingEngineRegistry, and adaptive shift**
  - Implement `src/core/existing_engines.rs`:
    - Discovers existing `ffmpeg`, `soffice`, `pandoc`, `gswin64c`/`gs`, `tesseract`, `inkscape`, `ebook-convert`, and ArtCraft binaries on the machine.
    - Zero background downloads. Pure inspection of `PATH` and standard installation directories.
  - Implement `src/codec/icc_meta.rs`:
    - `extract_png_icc`, `inject_png_icc`, `extract_jpeg_icc`, `inject_jpeg_icc`, `extract_webp_icc`, `inject_webp_icc`.
  - Wire into `src/codec/mod.rs` and `src/core/route_planner.rs`:
    - Check native transcode viability.
    - If format requires complex script shaping or proprietary structures, shift to detected existing engine.
    - Preserve ICC profiles and file timestamps on all image transcode operations.
  - Ensure 100% 7-bit ASCII compliance.

- [x] **Step 4: Run tests to verify they pass**
  `cargo test --test test_icc_preservation`
  `cargo test --test test_existing_engines`

- [x] **Step 5: Commit changes**
  `git commit -m "feat(codec): implement ICC color profile preservation and adaptive shift to existing system engines"`

---

### Task 7: Comprehensive Test Suite, ASCII Gate & Release Binary Build

**Files:**
- Modify: `tests/gui_ascii_gate.rs`
- Run: Full test suite (`cargo test`)
- Run: Release build (`cargo build --release`)

- [x] **Step 1: Run GUI pure ASCII gate check**
  Run: `cargo test --test gui_ascii_gate`
  Verify all files pass the strict 7-bit ASCII gate without any unicode or emoji violations.

- [x] **Step 2: Run full unit and integration test suite**
  Run: `cargo test --lib`
  Run: `cargo test --test test_donut_profiles`
  Run: `cargo test --test test_donut_flip`
  Run: `cargo test --test test_preferences_categories`
  Run: `cargo test --test test_preset_editor`
  Run: `cargo test --test test_icc_preservation`
  Run: `cargo test --test test_existing_engines`
  Ensure 100% tests pass.

- [x] **Step 3: Build release binary and update installation**
  Run: `cargo build --release`
  Deploy release executable to `C:\Users\haris\.cargo\bin\mossambi.exe`.
  Verify `mossambi.exe --version` outputs clean status.

- [x] **Step 4: Commit release gate and update roadmap/state**
  Update `.planning/ROADMAP.md` and `.planning/phases/23-converter-engine-donut-tangerine-artcraft/PLAN.md`.
  `git commit -m "chore: complete phase 23 donut widget settings and native converter engine parity"`
