# Mossambi Agent Instructions

Status: clippy clean (all targets), 12 lib tests + all integration suites pass, release build succeeds.

## Environment rules

- **Windows gates run through PowerShell (`pwsh`), never bash.** No bash on this host; every
  environment/status check uses the `shell` tool with pwsh syntax (e.g. `$env:X`, `Get-ChildItem`,
  `Select-String`). Linux-side checks go through WSL at `/mnt/z/Projects/mossambi` (no sudo;
  WSLg provides Wayland + XWayland).
- **Temp direction:** prefer `C:\Users\haris\AppData\Local\Temp\opencode` for scratch files.

## ASCII discipline (mandatory)

- All NEW file content must be **pure ASCII**. After every authored file/edits, re-verify with a
  `[^\\x00-\\x7F]` regex sweep (write findings to a UTF-8 file when the console codepage mangles
  output - the pwsh console renders Unicode as `??` soup).
- **No emoji in GUI strings.** Emoji are removed from any string the user sees.
- Pre-existing glyphs are explicitly OFF-LIST (leave alone, do not fix, do not reformat):
  em dash / en dash, ellipsis, symlink arrow (feather-arrow), breadcrumb angle-quote (single
  guillemet), right/left/up/down arrows, plus-sign copy badge, check mark, cross mark, times sign,
  bullet, sort column triangles, horizontal rule and double-line rule box-drawing comment banners,
  vertical/box-drawing tree chars in daemon banner, window-control glyphs (restore/minimize
  squares), sort-direction..drawer glyphs (plus, minus-over-dot, info sign, degree sign,
  micro sign, implies-arrow, check, two-line arrow), and the pre-existing emoji tab/category/
  button labels that the UI overhaul replaces wholesale (home, folder, globe, image, music,
  clapper, page, package, laptop, magnifier, folder-open, monitor, download, waste basket,
  target, microscope, clipboard, floppy, plus, pencil nib, gear, warning, sun, cloud, hamburger,
  trigram, crossed swords, star, heavy check, heavy cross, right-pointing arrow, VS16 selector).

## Drag & drop status (Windows + Linux, est. 2026-11-06) - milestone CLOSED

Commits `b65ceed` (real OS drag-out, drop-in polish, platform policy) and `7452250` (cross-platform
gates clean). See git history for the full matrix; key contracts preserved:

- Windows OLE in `src/core/ole_drag_out.rs` (windows 0.59 `<Interface>_Impl` outer-type contract;
  pass data object BY REFERENCE to `DoDragDrop`; `&mut effect` coerces to a raw effect pointer).
- Linux hand-off in `src/core/linux_clipboard_files.rs`: OS-neutral payload builder at file root
  (testable on every platform), Linux-gated single-target `text/uri-list` write (wl-copy on
  Wayland, xclip on X11; one helper = one MIME type, wl-clipboard issue #71).
- `src/core/drag_out.rs::begin_file_drag_out` dispatches per-OS; dual-signal viewport-exit
  detection hands drags to the OS from Explorer and the hub.
- `dnd_backend.rs::apply_backend_policy` forces X11 on Wayland at both GUI entry points.
- winit `ViewportCommand::StartDrag` is a window drag only, never a file drag.

## Gates before any commit

1. `cargo fmt`
2. `cargo clippy --all-targets -- -D warnings`
3. `cargo test`
4. `cargo build --release`
5. ASCII sweep of every file touched.
6. open-computer-use QA pass on Windows for GUI work.
7. Keep `graphify-out/` untracked. Do not stage scratch artifacts (screenshots, test dirs).

## Token Economy & Heavy Action Policy (Mandatory)

- **DO NOT run `/gsd-graphify` automatically on intermediate code changes or normal commits.** It wastes tokens and AST extraction cycles. Only run `/gsd-graphify` when the user explicitly requests it or after asking user confirmation prior to milestone sign-off.
- **DO NOT perform new release packaging or git release tagging automatically.** Always ask the user for confirmation before performing actions like cutting a new release, tagging, or publishing.

## ACTIVE USER INSTRUCTIONS - UI OVERHAUL PHASE (est. 2026-11-06)

The user opened the UI overhaul. Target design language: **macOS Finder**, not Windows-style
chrome. Windows-only QA until the wave closes (Linux round parked on user request). Instructions
passed so far, verbatim-in-intent - follow all of them:

1. **Remove the file-type filter chips row** (All/Images/Audio/Video/Docs/Archives/Code with
   counts) from the top of the file area and replace it. Filtering may survive only inside a
   compact popover. The top chrome must be **clean and neat, Finder-style** (not the Windows-style
   navigation bar currently there).
2. **Window title buttons are badly designed** (Fluent dots plus a stray maximize glyph). Redesign
   minimize/maximize/close properly - Finder traffic lights with real hover glyphs, positioned
   like Finder (left), wired to Minimized/Maximized/Close viewport commands.
3. **Sidebar: remove custom pinned folders** (between_pcs, wallpapers are hard-coded today).
   Show ONLY standard folders (Desktop, Downloads, Documents, Pictures, Music, Videos, Recycle
   Bin). Provide a Preferences option to show custom pinned folders, plus the ability to
   pin custom folders (context menu on a folder row) and to create desktop shortcuts.
4. **Remove the Tags sidebar section** (hard-coded Home/Work/Photos/Important dots). It becomes
   an optional Preferences setting, and **tags must actually be implemented in the explorer**:
   assign tags via the right-click menu, show tag dots on file rows in all views, and honor the
   tag smart-list when the section is enabled.
5. **Miller columns view is unacceptable** (giant blurry blown-up folder preview, clipped vertical
   "Quick" label hanging over the separator, and clicking a folder does not expand column 2).
   Check it personally, then rebuild it Finder-like: proper column stack, chevron expansion,
   selection sync, no oversized preview card, clean meta panel for files only.
6. **The right-click menu is incomplete.** Complete it: Open, Open With, Peek, Rename, Delete,
   Cut/Copy/Paste, Copy Path, Tags submenu, Properties for items; New Folder and Paste Allowed
   on blank canvas; Pin/Unpin (folders); consistent with the existing file_menu actions.
7. **Implement an action journal for undo:** log all recent in-app actions (rename, move, copy,
   delete, create folder, tag changes) AND user keystrokes (context-tagged) so an undo method can
   be built on them. Undo must be able to reverse at least rename/move/copy/create/delete/tag.
8. **Fix found bugs while in there:**
   - Toolbar text collision ("N items enabled" overlaps "Select All" - two labels drawn on
     each other).
   - "1 items" grammar (pluralize correctly).
   - Wrong Date Modified: app showed 2026-11-24 for a folder created 2026-11-06. Find the
     broken conversion/formatting and fix.
   - Home "Quick access" dumps raw user-profile contents including hidden NT junctions
     (NetHood, PrintHood, SendTo, Start Menu, Recent, Templates, Local Settings). Only real
     library folders belong there.
   - Tile labels collide in the Home grid (row 2 renders "apple-music-tApplication D...").
   - The window background is so translucent other windows bleed through - reads as dirty.
   - "Custom Conforme" truncation in the sidebar (no ellipsis discipline).
   - Stale selection ring on Home ("temp") persisted across restarts.
   - Drive captions rendered as underlined links - off-style for Finder design.
9. **QA method note:** the OpenCode terminal window keeps grabbing foreground, so OCU screenshots
   of a covered Mossambi fail or capture the wrong window. Verify visuals with a pwsh
   PrintWindow snapshot helper (PW_RENDERFULLCONTENT) that captures Mossambi's window even when
   occluded; drive input with OCU app_post clicks (work even when covered).
10. DnD residue policy stands: delete `Desktop\\qa_dnd_scratch` when QA closes.

## Known intentional pre-existing glyphs

`studio_app.rs` and other legacy files keep their em dashes; `explorer_view.rs` keeps its category
emoji labels until this overhaul swaps them out. Any string the overhaul touches must end up
pure ASCII.

## FULL USER INSTRUCTION LEDGER (compiled from Big_session.md + this session)

Every line below is a user instruction, not an agent suggestion. All of it stands until the user
says otherwise. Ordered by the phase that must satisfy it.

### A. Product identity and focus

- The app began as a file CONVERTER plus radial widget, then became a FILE EXPLORER project.
  The explorer is now the primary surface; the widget is equally important and must stay
  visually and behaviorally unified with the explorer.
- The target is a macOS Finder CLONE in design language, carrying the FEATURES of Windows File
  Explorer, Linux file managers (Nautilus, Thunar, GNOME Files), and FilePilot. Apple design,
  everyone else's feature set.
- Home dashboard is mandatory. It must open as a clean first surface (standard folders only, no
  NT junctions, no colliding tile labels), not be removed.
- Windows first. Linux verification round is parked until the Windows wave closes.
- The converter engine must be integrated into the GUI (right-click Convert, batch actions,
  progress, quality gates), not left as a separate CLI-only product.

### B. Design direction (the governing brief)

- STRICT macOS Finder chrome: traffic lights on the LEFT with real hover glyphs, ONE merged
  toolbar, path bar at the BOTTOM, sheet-style popovers, sheet-style preferences.
- Pixel-perfect alignment, padding, margins, border presence, and padding around text. Wrapping
  must be correct (no label collisions, no overlap, no overflow into the next column).
- Responsive like a Tailwind-built app: a real spacing/type scale and consistent tokens, not
  ad-hoc pixel values scattered through 6500 lines of explorer code.
- The Settings/Preferences page is currently clutter and clumsy; it must be rebuilt for clarity
  of every setting LABEL and every property VALUE, with clear grouping and hierarchy.
- No emoji as icons anywhere in the GUI. Real vector icons from the embedded icon packs, each
  file type mapped to a proper icon.
- The user has called the current UI "clumsy and dirty", the column headers "shit", and the
  Miller column view "shit". Do not ship anything in that class.
- No default background highlight bands behind file/folder icons. Selection reads through the
  checkbox (existing checkbox-mode contract: Always / Hover / Off).
- Motion everywhere: zappy, snappy, smooth scrolling and movement across the whole program.
- Icons roughly twice the current size; app default font +4px (existing contract).
- Max-FPS support, highly responsive, multi-threaded heavy work, no algorithmic complexity above
  O(log n) in hot paths.

### C. Feature parity checklist

- Clicking a file must launch/open it (Finder parity). Ctrl+wheel zooms in/out; zoom down snaps
  to the list form, zoom up grows icons and their labels (Windows Explorer behavior).
- Preview/Peek for images, folders, PDF, Word, Excel, PPT, text and code files. Long hover shows
  the inside of a folder.
- Right-click menu must be COMPLETE and copied properly from the real thing: Open, Open With,
  Peek, Rename, Delete, Cut, Copy, Paste, Copy Path variants, Tags submenu, Properties, New
  Folder, Paste Allowed, Pin/Unpin, Convert submenu (type-aware, registry-driven), Duplicate,
  Send To, Sort By, View, Invert Selection.
- Tools and converters are reachable from the top of the file area, above the Name/Kind/Size/
  Date columns, not buried in a wall of text.
- Tags: real implementation in the explorer (assign from the context menu, tag dots on rows in
  every view, smart-list filtering). The Tags sidebar section is OFF by default and becomes a
  Preferences setting.
- Sidebar shows ONLY standard folders by default (Desktop, Downloads, Documents, Pictures,
  Music, Videos, Recycle Bin). Custom pins and desktop shortcuts are opt-in via Preferences and
  a "Pin to Sidebar" context-menu action.
- The file-type filter chip row (All/Images/Audio/Video/Docs/Archives/Code with counts) is
  removed from the top; filtering survives only inside a compact popover.
- Action and keystroke journal: log recent in-app actions (rename, move, copy, delete, create
  folder, tag change) AND context-tagged keystrokes so undo can be implemented on top of it.
- Bugs to fix: "N items enabled" overlapping "Select All"; "1 items" grammar; wrong Date
  Modified (2026-11-24 shown for a 2026-11-06 folder); Home Quick-access dumping NT junctions;
  colliding Home tile labels; see-through window background; truncated "Custom Conforme";
  stale selection ring persisting across restarts; underlined drive captions; broken Miller
  column expansion; vertical clipped "Quic" label in the column view.

### D. Workflow the user demands (repeat every session)

- Mantra: RESEARCH -> deep research (how it integrates with THIS project and its current
  state) -> PLAN (what, where, how, and which prerequisites to prepare) -> PLAN WITH GSD
  (gsd-plan-phase / gsd-autonomous) -> IMPLEMENT THOROUGHLY -> FORMAT, LINT, DEBUG, OPTIMIZE
  -> VERIFY with computer use as four roles (developer, real user, pessimistic tester, noob
  computer user), screenshotting at HOVER and at CLICK for every interaction -> VERIFIER AGENT
  checks code quality, integrity, and ticks the todo list. If the verifier did not do that, the
  task is not complete.
- Use the skills and MCP servers that fit: impeccable for design, gsd-* for planning, computer
  use / OS-level control to drive the real app, and professional sub-agents for parallel work.
- Keep every instruction in this file updated; the user treats it as the memory of record.
- Gates before landing: cargo fmt, cargo clippy --all-targets -- -D warnings, cargo test,
  cargo build --release, ASCII sweep of touched files, computer-use QA pass, graphify-out/
  untracked, no scratch artifacts staged.
- The user expects long autonomous runs with no hand-holding, and will not interrupt once the
  interview questions are answered.

### E. Phase 16 & Beyond Architectural Mandates (From 20-Question Audit 2026-11-07)

1. **Windows Explorer Shell Hijacking & Safe Rollback:**
   - Full explorer replacement option (hijacking Win+E and default folder double-clicks).
   - When enabled, automatically generates `mossambi_shell_restore.reg` and provides a single-click "Restore Windows Explorer Defaults" button in Preferences.
   - Uninstaller must cleanly release all file associations, registry hooks, and shell defaults across all OS platforms.
2. **Sweet Lime Resident Radial Widget (USP & Activation):**
   - Triple trigger surface: Global hotkeys (`Win+Shift+C`, `Win+Ctrl+Space`), screen corner hover puck, AND mouse gestures (shake / circle gesture).
   - Gesture activation blooms the radial widget centered immediately at the cursor coordinates with elastic spring physics.
3. **Local AI Engine & Recommended Models Catalog:**
   - Configurable in Settings: API endpoint (`http://localhost:11434` for Ollama, `http://localhost:1234` for LM Studio), model name, and temperature/effort.
   - Recommended Local Models Catalog (updated per version):
     - **OCR & Document Scan Cleanup:** `Qwen2.5-VL-7B-Instruct` or `Llama-3.2-11B-Vision-Instruct`.
     - **Scripting & Workflow Automation:** `Qwen2.5-Coder-7B-Instruct` or `DeepSeek-R1-Distill-Qwen-7B`.
   - Hybrid acceleration: DirectML GPU/NPU shader processing with CPU SIMD fallback when local AI server is absent.
4. **Installer / Uninstaller Suite & Multi-Platform Packaging:**
   - Native interactive Installer TUI and Uninstaller TUI for terminal users.
   - Platform installer packages: Windows `.msi` (with Winget/Scoop manifests), macOS `.dmg` (with Homebrew formula), Linux `.deb` / `.rpm` / Flatpak.
   - Background silent update notifications wired to GitHub Releases with rendered changelog display.
5. **Universal Conformer & Global Visa Library:**
   - Beyond Indian government exams (UPSC/SSC/IBPS/GATE/RRB), bundle international visa & passport envelopes (US Visa 2x2, Schengen Visa, UK, Canada PR, Indian Passport).
   - Persistent custom tags engine with Pantone Citrus color picker, dual-written to SQLite and OS extended attributes.
6. **Dual Keybinding Profiles:**
   - Switchable in Preferences: macOS Finder Mode (`Enter` to rename, `Cmd+Down` to open, `Space` for Quick Look) vs Windows Explorer Mode (`F2` to rename, `Enter` to open).
7. **Fast Hybrid Search:**
   - Instant directory filter + SQLite FTS5 index + background USN Journal crawl on Windows / Spotlight `mdfind` on macOS.
8. **Finder-Style Batch Renamer:**
   - Multi-file modal supporting Text Replacement, Prefix/Suffix, Numbering Sequences (`001`, `002`), and Date stamping.

### F. Phase 17 Architectural Mandates: Filee Parity, 255 Formats, Pure Rust Embedding, and Apple HIG

1. **Pure Rust Single-Binary Embedding (Anti-Download Doctrine):**
   - Filee (`KnifeLemon/Filee`) forces runtime downloads of 1.1GB+ external binaries (FFmpeg 100MB, LibreOffice 373MB, Calibre 240MB, Ghostscript 19MB, Pandoc 43MB, JRE 49MB).
   - In Mossambi, all conversion capabilities must be embedded directly in pure Rust in one single unified, optimized binary without requiring separate external runtime downloads.
   - Pure Rust image codecs (`image`, `resvg`, `ravif`), document engines (`calamine`, `rust_xlsxwriter`, `lopdf`, `csv`, `quick-xml`), archive engines (`zip`, `tar`, `flate2`, `zstd`, `bzip2`, `xz2`), and native OS GPU shaders (Direct3D11 / NVENC / DirectML / AVX2 SIMD / VideoToolbox / VA-API).
2. **255-Format Catalog Expansion & Multi-Hop Route Planning:**
   - Full format parity across all 18 categories from `https://filee.sh/formats/`: Camera RAW, CAD (DWG, DXF), Korean Hangul (HWP, HWPX), E-Books (EPUB, MOBI, AZW, AZW3, CBZ, CBR), 3D Models (GLB, glTF, STL, OBJ, 3MF, PLY), Geospatial Vector Data (GeoJSON, KML, KMZ, GPX), Fonts (TTF, OTF, WOFF, WOFF2, EOT), Audio, Video, Subtitles, Spreadsheets, and Documents.
   - Dijkstra weighted shortest-path route planner up to 3 hops (e.g. `HWP -> HWPX -> PDF -> PNG` or `SVG -> PDF -> PNG`).
3. **Two-Sided Sweet Lime Radial Donut Widget:**
   - Tab or mouse-wheel turns the radial menu over between Face 1 (Primary Formats: JPG, PNG, WEBP, PDF, MP4, MP3) and Face 2 (Studio Power Tools: CROP, EXAM, META, RENAME, AI OCR, PDF, ZIP, EXPLORE).
   - 0.20s stagger-fold 3D cosine flip animation (cos(pi * t) wedge width scaling).
   - Columella center hub hint: "Tab: Power Tools" / "Tab: Formats".
4. **Interactive Pre-Conversion Crop Canvas:**
   - Normalized crop box [0.0, 1.0], 8 draggable handles, rule-of-thirds grid lines during active dragging, aspect ratio presets (Free, 1:1, 4:3, 16:9, 3:2, 5:4), live target resolution pill badge, piped directly into imageops::crop_imm.
5. **Browser Drag-Out & WebDrop (text/uri-list, Virtual Files):**
   - Register and handle CF_HDROP, UniformResourceLocatorW, text/uri-list, and virtual files from Chromium, Edge, Firefox, and Safari.
   - Asynchronous download staging into downloads_dir()/.mossambi_staging/ with instant routing into radial widget and studio batch pipeline.
6. **Linux X11 and Wayland Desktop Suite:**
   - Force X11 backend binding when XWayland is available under Wayland to prevent silent XDND file drop drops.
   - GNOME Shell Unix socket pointer bridge ($XDG_RUNTIME_DIR/mossambi-gnome/pointer.sock).
   - KDE Plasma KWin script and Hyprland IPC window positioning rules for centered radial donut spawn.
7. **Apple Human Interface Guidelines (Apple HIG) Visual Standards:**
   - Strict 8pt spatial grid, SF Pro / Agave typographic scale with generous negative space, authentic traffic lights on the left with hover glyphs, clean acrylic/mica translucency without dirty pixel bleed.
   - Eliminate all label collisions and grammar defects: strictly "1 item" (never "1 items"), clean spacing between "N items enabled" and "Select All", crisp unblurred Miller column previews.
8. **Auto-Aware System Optimization & Hardware Telemetry:**
   - Auto-detection of CPU cores, physical RAM, GPU vendor (NVIDIA/AMD/Intel/Apple), and NPU engine (DirectML/CoreML).
   - Dynamic auto-suggestions displayed in mossambi hw CLI and Studio GUI Tab 4 ("Hardware & AI Telemetry") recommending concurrency, hardware encoder offload, SIMD image convolution, and local AI model sizing.
9. **Zero-Defect Quality Gates:**
   - cargo fmt
   - cargo clippy --all-targets -- -D warnings
   - cargo test (0 ignored, all pass)
   - cargo test --test gui_ascii_gate (100% pure ASCII verified)
   - cargo audit & cargo deny (0 vulnerabilities standard: lopdf >= 0.47, rust_xlsxwriter >= 0.102, zip >= 9.4, calamine >= 0.38, quick-xml >= 0.42, zstd >= 0.13.4)
   - open-computer-use desktop interactive verification with screenshots
   - Knowledge graph sync via /gsd-graphify.

### G. Phase 18 Master Architecture: Donut Toolbar Customizer, Pure-Rust Engine Suite, and Unified Settings Parity (2026-11-08)

1. **Donut Toolbar 3-Column Interactive Customizer (Apple HIG & Filee Parity):**
   - Column 1 (Choose a Profile): 19 domain profiles (`Images 6-8`, `PDF 7-8`, `Documents 7-4`, `Spreadsheets 6-5`, `Presentations 7-4`, `HWP/HWPX 7-4`, `Text & Markdown 6-5`, `E-books 6-3`, `Video 6-8`, `Audio 5-5`, `Subtitles 2-1`, `Vector graphics 5-5`, `Archives 4`, `CAD drawings 5-3`, `Fonts 4-1`, `Data 5-4`, `3D models 5-1`, `Geospatial 5-1`, `Mixed files 5-2`). Dynamic search input, `+ New profile` creator, keyboard reordering (`Alt+Up` / `Alt+Down`).
   - Column 2 (Arrange Presets & Appearance):
     - Interactive Vector Donut Canvas Preview with Front/Back toggle pills, Columella center hub hint (`Images 6/12`), interactive petal wedges with hover highlights and preset tooltips.
     - Profile metadata: Profile Name, "Use for mixed selections (files of different types)" toggle.
     - Extensions Tag Cloud: Removable tag badges (`.png x`, `.jpg x`, etc.) with inline `+ Add an extension` text box.
     - Profile action buttons: `Reset to default` and `Delete profile`.
     - Appearance Sliders: Size (`320..540px`), Hole (`30..80px`), Opacity (`0.20..1.00`).
   - Column 3 (Available Presets Catalog):
     - Action buttons: `+ New preset`, `New group`, `Copy to folder`.
     - Search input with category filter pills: `All`, `Image`, `Document`, `Video & audio`, `Edit`, `3D & CAD`.
     - Categorized preset cards and grouped action cards (`Edit Group of 6`, `Pages per sheet Group of 5`, `Flip Group of 2`, `Crop video Group of 6`, `JPG 70%`, `TIFF 300dpi`, `Merge PDF`, `Long image PNG`, `Split PDF`, `Compress PDF`, `Searchable PDF`, etc.).
2. **Pure-Rust Conversion Engine Suite (Anti-Download Doctrine):**
   - Filee's runtime forces 1.1GB+ downloads (LibreOffice 422MB, Calibre 240MB, FFmpeg 99MB, Ghostscript 19MB, Pandoc 42MB, JRE 49MB).
   - Mossambi implements pure-Rust engines in a single standalone binary:
     - `src/codec/cad.rs`: ASCII DXF parser (LINE, CIRCLE, ARC, POLYLINE, LWPOLYLINE) and vector renderer to SVG, PDF, and PNG via `resvg`.
     - `src/codec/data_exchange.rs`: Cross-conversion between JSON, CSV, JSON Lines, XML, vCard 3.0 (`.vcf`), and iCalendar RFC 5545 (`.ics`).
     - `src/codec/email.rs`: RFC 822 MIME parser for EML, MSG, and MHTML extracting clean HTML/TXT and packaging attachments into ZIP.
     - `src/codec/fonts.rs`: OpenType/TrueType SFNT parser and packager with WOFF 1.0 compression via `flate2` zlib.
     - `src/codec/geo.rs`: Geospatial vector engine for GeoJSON, KML, KMZ, GPX, and CSV (lat/lon) with WGS 84 normalization and automated ZIP packaging.
     - `src/codec/models_3d.rs`: 3D mesh engine for STL (ASCII/binary), Wavefront OBJ, PLY, and glTF/GLB, plus "Pictures to 3D relief" heightmap luminance generator with solid base.
     - `src/codec/pdf_tables.rs`: Vector PDF line grid and word position detector exporting tables to native Excel workbooks (`rust_xlsxwriter`) and CSV (`csv`).
     - `src/codec/subtitles.rs`: Bidirectional subtitle parser/renderer for SRT, WebVTT, ASS/SSA, LRC, and plain text with timestamp math and CP949 / EUC-KR encoding handling.
3. **Unified Settings Surfaces (Preferences Views):**
   - General Settings: Language selector, start with OS sign-in, close-window behavior, Explorer context menu hook, default save location ("Same folder as the source"), "Keep the original file dates for every conversion", "Save every geospatial result as a ZIP", browser drag staging directory & "Save only the converted file" radio, conversion history limit & clear history.
   - Shortcuts & Gestures Settings: Modifier + drag files (`Ctrl + Left`), keyboard shortcut (`Ctrl + Alt + Space`), Flip key (`Tab`), Mouse wheel flip toggle, app exclusion list ("Never trigger in these apps"), plus gesture physics & sensitivity controls.
   - Fields Settings: 12 toggleable domains (CAD, Fonts, 3D models, Maps/Geospatial, Games, Design & print, Science & medical, Developer, Technical writing, Video production, Pro audio, Old formats) with sub-format descriptions and "Always shown" footer note.
4. **Ascii Discipline & Zero-Defect Standard:**
   - 100% pure ASCII across all newly authored and edited source files (`gui_ascii_gate`).
   - 0 clippy warnings (`cargo clippy --all-targets -- -D warnings`).
   - 0 test failures, 0 ignored tests (`cargo test`).
   - 0 security vulnerabilities (`cargo audit`).


### H. Phase 19 Master Architecture: Pixel-Perfect Sweet Lime Radial Donut Widget & Unlimited Animation Engine (2026-11-08)

1. **Pixel-Perfect Sweet Lime Radial Donut Widget (Filee Feature Parity & Apple HIG Design):**
   - **Outer Sub-Ring Group Fan-Out:** Group wedge activation expands concentric outer sub-ring slices (`SubGap = 6px`, `SubBand = 46px`, `SubOverhang = 0.25`, `sub_inner = outer_radius + 13.0`, `sub_outer = sub_inner + 46.0`). Angular fan distribution (`parent_span * 1.5 / count`) with dedicated child actions for all 9 groups:
     - `EDIT`: Crop, Rotate, Flip, Grayscale, Annotate, Blur
     - `PDF`: Merge, Split, Compress, Searchable PDF
     - `ZIP`: Extract, Pack ZIP, Pack 7Z, Pack TAR.GZ
     - `FLIP`: Flip Horizontal, Flip Vertical
     - `CROP`: 1:1 Square, 4:3 Standard, 16:9 Widescreen, 3:2 Photo, 5:4 Print, Freeform
     - `Pages per sheet`: 2 pages, 4 pages, 6 pages, 8 pages, 9 pages
     - `CAD`: DXF to SVG, DXF to PDF, DXF to PNG, Mesh to 3D
     - `3D`: Relief Heightmap, Mesh STL, Mesh OBJ, Mesh GLB
     - `META`: Strip EXIF, View Metadata, GPS Scrub, Edit Tags
   - **Elastic Radial Spoke Hover Pop-Out:** Hovering over any slice translates the wedge 7px outward along its radial angle spoke vector (`cos(angle) * 7.0`, `sin(angle) * 7.0`) via an underdamped spring with responsive overshoot.
   - **Interactive Conversion Progress Mode:** Live animated progress ring replaces outer petals during active batch conversions (`sweep_angle = progress * 2*PI`, Sweet Lime arc stroke, bold centered percentage text, and bouncy spring checkmark pop-in badge held for 0.9s upon completion).
   - **Full Keyboard Navigation:**
     - `ArrowLeft` / `ArrowRight` / `ArrowUp` / `ArrowDown`: Step selection across radial wedges.
     - `Enter`: Execute action or expand hovered group.
     - `Tab` or `Space`: 3D cosine flip between Face 1 (Primary Formats) and Face 2 (Power Tools).
     - `Escape`: Collapse outer fan-out or dismiss widget.
   - **Dynamic Center Columella Hub Hints:** Live context-aware hub text ("Tab: Power Tools", "Tab: Formats", "Enter: Expand", "Enter: Convert", "Converting...", "Converted", with active profile name and slice count `N/12`).
   - **Live Geometry Synchronization:** Size (`320..540px`) and hole radius (`30..80px`) synchronize dynamically with `cfg.donut.appearance`.

2. **Unlimited Animation Engine Across 3 Modalities (Widget, Explorer, TUI):**
   - **8 Motion Presets:**
     - `Spring`: Real physical harmonic oscillator (`f = -k*x - c*v`, customizable stiffness, damping, mass).
     - `Snappy`: 150ms quick response curve for rapid power-user interactions.
     - `Natural`: 250ms Apple HIG deceleration curve (`1 - (1-t)^3`).
     - `Bouncy`: Underdamped spring with physical overshoot.
     - `Gentle`: 350ms fluid ease-in-out curve (`t^2 * (3 - 2t)`).
     - `Cinematic`: 600ms showcase smooth step curve.
     - `Fade`: Linear alpha transition.
     - `None`: Instantaneous zero-duration (reduced motion standard).
   - **Dedicated Preferences Surface:** "Motion & Animations" section under Appearance with interactive curve preset buttons, spring stiffness/damping/mass sliders, and Apple HIG toggle switches for `Reduced Motion`, `Micro-interactions`, `Shimmer Effects`, `Progress Ring Mode`, `Group Fan-Out`, `Elastic Hover`, and `Terminal TUI Glass`.

3. **Zero-Defect Quality Gates:**
   - `cargo fmt --check`: Clean (0 diffs).
   - `cargo clippy --all-targets -- -D warnings`: Clean (0 warnings, 0 errors).
   - `cargo test --test gui_ascii_gate`: PASSED (100% pure ASCII verified).
   - `cargo test`: 128 tests pass (66 unit + 62 integration across all 15 suites), 0 failed, 0 ignored.
   - `cargo audit`: Clean (0 security vulnerabilities across 588 dependencies).
   - Knowledge Graph sync via `/gsd-graphify`: 2,258 nodes, 4,256 edges, 115 communities in `.planning/graphs/`.

## Mandatory Quality & Engineering Discipline (Karpathy Guidelines & Strict Execution)

1. **Mandatory Post-Task Formatting, Linting, and Debugging:**
   - After EVERY task or modification, formatting (`cargo fmt`), linting (`cargo clippy --all-targets -- -D warnings`), and debugging/verifying program execution are **strictly mandatory**.
   - Never consider any task complete without passing all verification gates cleanly.

2. **Surgical Changes Only (Anti-Rewrite Doctrine):**
   - **Never rewrite entire files or replace working code blocks wholesale.**
   - Make precise, surgical modifications that touch ONLY the exact lines required to solve the problem.
   - Do not "improve" or reformat adjacent unrelated code or comments.
   - Every changed line must trace directly to the user's explicit request.

3. **Karpathy Coding Guidelines (multica-ai/andrej-karpathy-skills):**
   - **Think Before Coding:** Don't assume. Don't hide confusion. Surface tradeoffs explicitly.
   - **Simplicity First:** Minimum code that solves the problem. Nothing speculative. No unnecessary abstractions.
   - **Surgical Changes:** Touch only what you must. Clean up only your own mess. Match existing codebase style.
   - **Goal-Driven Execution:** Define measurable success criteria, write tests, and loop until verified.

## Session contract - AUDIT WAVE (est. 2026-11-10, from user interview)

1. **Ownership confirmed:** this session is the only active harness in the repo. The
   phase-21 uncommitted wave is owned here and may be fixed + committed freely.
2. **Order of work (user-mandated):** fix the clippy RED (3 unused vars `glass`/`ms`/`cm`
   in preferences_view.rs) and finish the resting Glass-effect refactor -> all five gates
   GREEN -> commit the wave in clean commits -> FULL AUDIT of every ledger item against
   real behavior -> verify with computer use (OCU + PrintWindow) -> fix UI defects and any
   red gate -> verify all phases (16-21) surfaces.
3. **Security testing** comes after the UI round: use **security-mcp** and the user's
   **Kali Linux WSL** to perform the security tests.
4. **Process level (user picked both):** gsd planning + autonomous execution for major
   work; lightweight direct execution for small fixes.
5. **QA depth:** full 4-role computer-use QA (developer, real user, pessimistic tester,
   noob user) with hover + click screenshots on every interaction.
6. The user notes "lots of updates happened to the code" - never trust checklist marks
   alone; verify each ledger item visually against the current tree.

## THEME SYSTEM OVERHAUL (est. 2026-11-10, user directive)

1. **Contrast mandate:** blue highlighting on text labels AND on hover makes text
   invisible or overpowered. Fix for ALL themes - proper alpha discipline and
   high-contrast selected/hover label colors in every surface (sidebar rows, grid tiles,
   table rows, menus, tabs, buttons).
2. **Themes are not accent swaps.** Today's themes only change the accent color - not
   accepted. Implement real premium themes as FULL palettes (background, sidebar/panel,
   cards, borders, text tiers, accent families): the best VIM themes, the best Emacs
   themes, and the best VS Code themes, faithfully ported.
3. **Signature theme: "Mossambi Punch"** - Mossambi's True Fruit colour theme (sweet
   lime / citrus punch identity), designed to Apple HIG standards.
4. **Cross-check** the whole wave with the **impeccable** skill and UI/frontend design
   guidance, and verify visually with **computer use** (OCU + PrintWindow).
5. Everything integrates with the **macOS Finder explorer design**; follow macOS design
   principles throughout.

### Alpha-discipline wave - live sweep (complete, est. 2026-11-10)

All queued surfaces verified with OCU + PrintWindow scrub-hover harness (teleport
cursor moves never register in egui/winit; scrub motion does - harness gained
shover/fhover/probe/fore modes plus region-diff and hash checks):

- [verified] row hover band R38 G38 B40 with text R229 readable; row selection band
  R22 G53 B99, icon zone R20 G72 B138, neighbor rows idle R24 G24 B26 - unmul
  arithmetic live (the old premul path would flood (16,138,255)).
- [verified] sidebar, Quick Access + drive cards hover lift R24 -> R38 with a shared
  accent hairline top edge (consistent design); capacity track lifts R40 -> R47.
- [verified-fixed] "System Disk (C:)" midline overlay: not reproducible - uniform
  bounded accent band, text readable; the wave cleared it.
- [verified-fixed] F: card "Capacity unknown" fallback; Recent Files mtime-desc +
  hidden-file filtering hold.
- [style note] toolbar icon hover disc R101: brighter than the R38 row family, still
  neutral + glyph readable; retune during the theme wave.

[new defects from the live audit]
- D-G1 grid view renders 0 tiles for the populated Desktop folder (list shows ~19);
  tile hover verification blocked until fixed.
- D-S2 status bar "0 items" while rows render; Home counts correctly ("101 items
  (111 hidden)") - Desktop-folder-specific, likely shared root with D-G1.
- D-P3 Desktop browse resolves the wrong path: shell Desktop is OneDrive-redirected
  (C:/Users/haris/OneDrive/Desktop, 32 real items; raw profile Desktop is empty);
  app renders Miniconda-root contents under a "C: > Users > haris > Desktop"
  breadcrumb and the render pane + status bar read different sources. Fix via
  SHGetKnownFolderPath + a single browser-model source of truth. Column-view
  dead-zone left half noted for the A2/A3 layout audit.

[D-P3 wave: fixed-code 2026-11-10, commit 1c59776]
- Root cause confirmed: all known-folder navigation was built from raw
  USERPROFILE joins that bypassed platform_paths (8 sites in explorer_view:
  constructor, navigate_to, add_tab, is_home_directory, navigate_to_home,
  dashboard base; 2 sites in studio_view: sidebar Home row + the Favorites
  pinned pointers that the Desktop row resolves through).
- Fix: every navigation target now routes through the SHGetKnownFolderPath
  helpers in platform_paths (desktop_dir, downloads_dir, documents_dir,
  pictures_dir, music_dir, videos_dir); added home_dir_opt() for Option-shaped
  call sites plus a helper-agreement test. The command palette was already
  routed through platform_paths and needed no change. OneDrive row and
  config/journal storage paths left raw by design (not known folders).
- Gates GREEN: cargo fmt, clippy -D warnings (26s), cargo test (67 lib tests
  0 failed 0 ignored + full integration suites), ASCII sweep (74 added lines
  all pure ASCII), release build (1m00s).
- VERIFIED LIVE on the rebuilt release: Desktop lists the 32 OneDrive items,
  status count honest ("32 items (1 hidden)"), grid tiles render with hover
  lift, tooltip, and selection (accent card + checkbox + "1 selected
  (2.3 KB)").
- Conda-root mechanism SOLVED: the phantom column was the columns auto-preview
  dir auto-push firing against stale entries (see Miller wave below).

[Miller wave: fixed-code 2026-11-10, commit 9d078ca]
- User-reported defect: Miller columns 3rd/4th keep stale content after
  navigating upward. Root cause: go_back/go_forward set current_dir directly
  and spawned the async scan without the column-state resets navigate_to
  performs (column_stack, column_subfolder, column_preview_item,
  focused_row, is_home_view, tab metadata, dir watcher) - column 1
  re-rendered while the old subfolder columns stayed on screen.
- Fix: go_back/go_forward route through navigate_to; its history-push guard
  is a no-op when the target sits at history_idx, so back/forward positions
  are preserved. Same wave: the columns auto-preview dir auto-push branch
  removed (it pushed entries.first() while the async scan was in flight,
  pushing the anaconda3 root as a phantom second column on first navigation
  after launch) and the file auto-preview guarded by a parent-match check.
- Gates GREEN: fmt, clippy -D warnings, cargo test (19 suites 0 failed 0
  ignored), ASCII sweep (17 added lines), release build (46.98s).
- Live verification (pid 4952): first navigation into columns view shows the
  real Desktop listing + inspector with NO phantom column; 3-column stack
  (C: > Users > haris) then Back wiped the whole stack to the single Desktop
  listing; Forward landed clean on the C:\ single column.
- Follow-up defect noted for the audit wave: Miller columns 2+ render raw
  unfiltered listings via get_column_entries (desktop.ini, hidden junctions
  like "All Users ->", dot-dirs) while column 1 renders the filtered view -
  hidden-item visibility inconsistent across columns.

[Settings spacing wave: fixed-code 2026-11-10, commit 4255481]
- User-reported defect: settings page pills/toggles/switches "sticking to
  each other" with no differentiation gaps; full audit of margins, padding,
  layout, borders, radius requested via the impeccable skill + computer-use
  MCP.
- Root cause: preferences_view.rs:157 set item_spacing = Vec2::ZERO across
  the whole sidebar|divider|pane partition, so egui inserted no widget gaps;
  all separation was hand-typed add_space(XS) = 4px (monotone, cramped).
  Pills used radius::SM (5px) not PILL; toggle_row's On/Off label sat flush
  against the switch; extension pills were bare default-styled buttons.
- Assessment: mechanical detector (impeccable detect --scope layout) zero
  findings pre- and post-fix; rendered evidence from live captures of the
  General and Donut tabs identified the cramming set.
- Fix (9 edits, +30/-8): automatic rhythm restored inside both partitions
  (item_spacing 8x6, outer partition stays flush); section headings more
  space above than below (LG/XS); card inner margin 16x12; Front/Back +
  category pills true radius::PILL; extension pills themed (card fill,
  hairline border, PILL radius); donut narrow fallback reserves the two
  8px inter-column gaps (165/284/215).
- Gates GREEN: fmt, clippy -D warnings, cargo test (19 suites 0 failed),
  ASCII sweep (31 added lines), release build (49.31s).
- Live verification (pid 27548): General tab On/Off labels separated from
  switches, rows breathing inside cards, sidebar rows separated; Donut tab
  extension pills true-pill themed with 8px gaps, Front/Back + category
  pills separated, columns gap-reserved.

[Live settings save wave: fixed-code 2026-11-10, commit 352ae82]
- User mandate: "saving new changes to settings completes without any
  errors" - live persistence proven with computer use.
- Defects: preferences_view.rs discarded the frame-local dirty flag (let _
  = dirty) so the save never fired from the Preferences page; the save
  site swallowed all errors (let _ = self.cfg.save()); studio_app.rs had
  three more silent saves (theme change, autostart, explorer/quick prefs).
- Fix: dirty propagates into self.dirty; save errors surface in a pinned
  danger banner at the top of the pane (visible regardless of scroll) and
  keep the flag set so the next frame retries transient locks; new
  save_error field; banner uses a theme-independent danger red because
  macos_theme::ThemePalette carries no error role (clippy caught 4x E0609
  when first written against design.rs's separate palette); studio_app
  saves converted to the house log::warn! precedent (explorer_view.rs).
- Gates GREEN: fmt, clippy -D warnings, cargo test (19 suites 0 failed),
  ASCII sweep (66 added lines), release build (49.87s).
- Live verification: toggling "Start Mossambi when I sign in" flipped
  launch_at_login false->true on disk within a frame (config.json mtime
  16:34:27.921 -> 16:50:33.046; JSON diff shows only that key plus the
  theme_mode render-sync); the value survived an app restart; success-path
  capture shows the switch On with no banner.
- New defects discovered during verification (queued for next waves):
  (1) Settings do not APPLY to the running app: PreferencesView owns a
  detached MossambiConfig copy (loads its own at construction,
  studio_view.rs:91) while the live app reads studio_app.config; nothing
  syncs them, so every cfg-backed change persists to disk but takes effect
  only on restart - the user-reported "nothing I change works".
  (2) Theme round-trip clobber: ThemeMode::LiquidGlass maps to
  ThemeStyle::DarkAnthracite in both From impls (macos_theme.rs:332-396),
  so the preferences render-sync (preferences_view.rs:152) rewrites the
  persisted theme to dark_anthracite on first save.
  (3) Maximized-mode layout: donut toolbar column 3 ("Available presets")
  clips at the window edge and pane content is offset ~370px from the
  sidebar (user screenshot); the toolbar Search box clips too.
  (4) App died spontaneously once after a settings save (no Windows Error
  Reporting record; monitoring for recurrence).
- Protocol note: an earlier attempt clicked against a window whose
  geometry had shifted mid-test (capture ~1920x1050 vs launch 1120x720
  @(400,196)) and hit the wrong control; resolved by relaunching to
  standard geometry and confirming the tab via capture before the decisive
  click; keep a config baseline snapshot and Compare-Object the JSON for
  exact key-level proof.

[Settings-apply wave: fixed-code 2026-11-10, commit 4de5917]

- Wave: settings now APPLY to the running app (queued defect #1 from the
  live-save wave) plus the maximized donut layout fix (queued defect #3).
- Root cause (settings-apply): PreferencesView owned a detached
  MossambiConfig copy (loaded its own at construction, studio_view.rs:91)
  while the live app reads studio_app.config; render_preferences_tab
  passed no config, so every cfg-backed change persisted to disk but took
  effect only on restart.
- Fix (settings-apply): single source of truth. StudioView::render now
  receives the live &mut MossambiConfig (studio_app.rs call site passes
  &mut self.config; disjoint field borrows) and threads it through the
  tab dispatcher into render_preferences_tab, which swaps the live config
  into prefs_view.cfg for the frame (std::mem::swap before/after
  PreferencesView::render). The stale construction-time snapshot is
  discarded in both directions. Safe because PreferencesView::render has
  no early returns (verified 2026-11-10) and its only caller is
  render_preferences_tab; search_preferences and setting_value_label are
  free functions taking cfg as a parameter with no external callers, so
  no staleness vector remains.
- Root cause (maximized): ui.set_width(donut_w) only raises the minimum
  rect, so render_donut_toolbar computed its 3 columns from the full pane
  available_width; in maximized mode the columns ran past the right
  window edge after the centering pad_x (col3 "Available presets"
  clipped) and the 680px donut_w cap left a large centering void.
- Fix (maximized): donut_w uncapped 680 -> 1280 (still centered) and
  ui.set_max_width(donut_w) added so the inner 3-column layout sees
  exactly donut_w (preferences_view.rs donut branch). Preset buttons
  needed no change: card_w = (col3_w - 16).max(180) is always <= col3_w
  because col3_w >= 215 in every width branch.
- Gates GREEN: fmt; clippy -D warnings (zero warnings); cargo test (19
  suites, 67 lib tests, 0 failed, gui_ascii_gate included); ASCII sweep
  on the diff (clean); release build (51.98s). Diff +29/-3 across
  studio_view.rs, studio_app.rs, preferences_view.rs only.
- Still queued: theme round-trip clobber (ThemeMode::LiquidGlass maps to
  DarkAnthracite in both From impls, macos_theme.rs:332-396; theme wave);
  toolbar Search box clip (mechanism unexplained, diagnose during the
  maximized live capture); Miller 2+ unfiltered listings; explorer icon
  blur; app-death anomaly monitoring; context-menu wave (Miller
  right-click broken, unified Finder design, menu customization in
  Preferences) is the user-prioritized next wave.

[Keybinding double-fire wave: fixed-code 2026-11-10, commit 2c2828d]

- Wave: user-reported live defect - "pressing any keybinding performs the
  same action 2 times".
- Root cause: 4 duplicate handler sites. Two cross-layer (Ctrl+T / Ctrl+W
  handled globally in StudioView::render where they also switch to the
  explorer tab, AND again inside ExplorerView::render's single ui.input
  closure); two verbatim duplicates inside that same closure (F5, F2).
- Fix: deleted all 4 duplicate sites in explorer_view.rs, keeping the more
  capable/guarded site each time. Kept: studio_view Ctrl+T/W (also
  switches tabs), split-view-aware F5, is_typing-guarded F2, the Ctrl+Tab
  cycler (NOT a duplicate - no other handler exists), and Ctrl+L.
  Replaced the Ctrl+T/W block with a comment documenting the single
  global handling.
- Visible effect per duplicate: F5 rescanned the directory twice per
  press; Ctrl+T opened 2 tabs per press; Ctrl+W closed 2 tabs per press;
  the second F2 site fired even while typing (unguarded).
- Gates GREEN: fmt; clippy -D warnings (zero warnings); cargo test (19
  suites, 67 lib tests, 0 failed, gui_ascii_gate included); ASCII sweep
  on the diff (clean); release build (50.56s; app kill first - the app
  had already exited, so Stop-Process was a no-op).
- Still queued: theme round-trip clobber (ThemeMode::LiquidGlass maps to
  DarkAnthracite in both From impls, macos_theme.rs:332-396; theme wave);
  toolbar Search box clip (diagnose during the maximized live capture);
  Miller 2+ unfiltered listings; explorer icon blur; app-death anomaly
  monitoring; live verification of keybinding single-fire + settings-
  apply + maximized layout next; context-menu wave then theme wave.

[Pill ring highlight wave: fixed-code 2026-11-11, commit 9cf27e4]

- Wave: user-directed pill-ring system - highlight the ring of every
  pill-shaped control on hover, active, and buffer states; idle keeps
  the regular border. Scope confirmed by interview: ALL pill controls
  app-wide; buffer = pressed OR keyboard focus, the strongest ring
  (Apple focus-ring style).
- Implementation: shared ring system in design.rs - PillState
  (Idle/Hover/Active/Buffer), pill_state_from (pure priority ladder,
  unit-tested: buffer > active > hover > idle), pill_ring (Idle=none,
  Hover=1.0px accent at low alpha, Active=1.5px full accent,
  Buffer=2.0px strong accent; every ring drawn expanded concentrically
  so it stays visible over accent fills and hairline borders),
  paint_pill_ring (grows corner radii with the expansion).
- Applied at 14 call sites: finder_chrome segmented view switcher;
  macos_theme sidebar pill items + segmented switch; preferences_view
  Front/Back pills, extension removal pills, category filter pills,
  Apple HIG toggle switch; explorer_view Back/Forward nav pill,
  Home Dashboard/Folder Files pills, Dual/Preview pills, ghost-pill
  action row (rings suppressed on disabled pills).
- Gates GREEN: fmt; clippy -D warnings (zero warnings after suffixing
  3 Stroke float literals _f32); cargo test (all suites pass; a
  transient usvg/rusqlite rlib clash right after clippy resolved on
  re-run); ASCII sweep on the diff (clean); release build (9.66s
  incremental; app killed first). Diff +303/-42 across design.rs,
  finder_chrome.rs, macos_theme.rs, preferences_view.rs,
  explorer_view.rs.
- Still queued: live ring verification (atomic hover/click captures);
  Ctrl+W close single-fire, donut live-apply, maximized capture,
  Search-clip diagnosis; then context-menu wave, theme wave, icon
  blur + Miller 2+, A4 security, A5 fixes, A6 QA.

[Pill ring live verification: 2026-11-11, post-commit 9cf27e4]

- Method: atomic pwsh (foreground -> CopyFromScreen idle baseline ->
  SetCursorPos hover -> capture -> revert -> mouse_event hold-press ->
  release -> active capture -> restore click), region-scoped pixel diffs
  (tolerance 14) around the switcher (738,38 132x44) and nav (8,42 76x32)
  in window coordinates.
- Coordinate correction: the legacy verified coordinate map is SCREEN
  coords (window 1120x720 @(400,196)); window-relative = map minus origin.
  An earlier pass treated the map as window-relative and hovered/clicked
  ~400px right of the window - harmless, the app stayed in its mode.
- Results (window-relative): baseline 0 px in both regions; hover(List)
  890 px bbox (784,42)-(817,70); revert 0 px (idle untouched);
  hover(Back) 798 px; pressed(List) 1007 px bbox (782,40)-(819,72) -
  exactly +/-2px beyond the hover bbox on all four sides, matching the
  concentric buffer ring design (expand 2.0 vs 1.0); active(List) 1864 px
  (accent fill + ring, mode switch); restore click returned the original
  mode (1902 px across the switcher).
- Visual: 4x nearest-neighbor crops of idle/hover/pressed/active
  switcher and idle/hover nav confirm the thin accent ring on hover, a
  stronger ring while pressed, accent fill when active, and nothing at
  idle.
- App-death anomaly: pid 28976 died silently between runs (window gone;
  no Event Log Id-1000 record surfaced in the 3h window queried);
  relaunched as pid 27548 for the successful pass. Monitoring continues.
- Still queued: Ctrl+W close single-fire, donut live-apply, maximized
  capture, Search-clip diagnosis; context-menu wave; theme wave; icon
  blur + Miller 2+; A4 security, A5 fixes, A6 QA.

[Ctrl+W close single-fire verification: 2026-11-11, on pid 27548]

- Route findings: PostMessage WM_KEYDOWN/WM_KEYUP chords are no-ops on
  this app instance even with system foreground verified True
  (GetForegroundWindow()==hwnd; all four posts returned TRUE; tab-strip
  diff 0 for Ctrl+T and Ctrl+W alike). PostMessage had worked on an
  earlier instance, so it is instance-dependent - do not trust it alone.
  The reliable route is keybd_event real-input synthesis (system input
  queue) with foreground acquired and re-verified immediately before
  every injection (ALT-unlock + AttachThreadInput fallback; abort if
  lost) so synthetic keys can never land in another app.
- Proof: with 2 tabs, one real Ctrl+T added exactly one tab (2->3,
  866 px changed in tab region 0,0 440x36); one real Ctrl+W closed
  exactly one tab (3->2, 866 px); a second Ctrl+W closed exactly one
  more (2->1, 1094 px). 2x crops count the tabs visually per frame
  (3/2/1). The close fires exactly once per press - no double-fire.
  Last-tab guard not exercised (2 tabs existed at each close).
- "+" coordinate dependency: the + button sits AFTER the last tab, so
  its position depends on tab count: ~(405,18) with 2 tabs, ~(255,18)
  with 1 tab. The legacy map's (255,16) is the 1-tab layout; clicking
  it with 2 tabs hits the active tab (no visual change, diff 0).
  Always re-locate "+" from a fresh capture.
- State hygiene: the proof consumed the tab count to 1; one real
  Ctrl+T restored the baseline (1->2, 1094 px, crop-verified). App
  left as found: 2 "Music" tabs, second active, window 1120x720
  @(323,124), pid 27548.
- Still queued: donut live-apply, maximized capture, Search-clip
  diagnosis; context-menu wave; theme wave; icon blur + Miller 2+;
  A4 security, A5 fixes, A6 QA.
