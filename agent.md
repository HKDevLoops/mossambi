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

## Release Policy (BETA - Mandatory)

- **Mossambi is in BETA TESTING.** It is not production-ready. Treat every release as a beta artifact.
- **NEVER publish a GitHub Release until BOTH the CI workflow and the CD workflow pass green for the exact commit being released.** The CD workflow's `verify-ci` job enforces this gate; never bypass, skip, edit around, or disable it. A red or missing CI run means no release.
- **All tests must run in both CI and CD:** workspace tests, the bug-regression suite, E2E tier suites, and the ASCII integrity gate. A release is built per target OS (Windows x86_64, Linux x86_64, macOS aarch64 + x86_64) only after those gates pass.
- **Publish releases as prerelease/beta** until the beta phase is formally closed by the user. Do not mark a release as a stable/full release.
- Do not cut releases, tag, or publish manually; only the gated CD workflow triggered by a `v*.*.*` tag may publish, and only after CI + CD are green.

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

[Donut live-apply verification: 2026-11-11, on pid 3444]

- Setup: Preferences (gear at (910,56)) -> "Donut toolbar" category
  pill at (320,182); right pane scrolled to the Appearance card; Size
  slider track y=627, x~700-864, value box right of the track.
- Baseline: config.json donut.appearance.size = 430 (mtime
  11:55:10.711).
- Change: click on the track at (715,627) -> size flipped to 340 in
  config.json, mtime 12:03:59.984. Slider changes persist to disk
  immediately - the live-apply save path works.
- Slider region (660,605 290x45) diff 289 px (handle move plus value
  text "430 px" -> "340 px"; crop_gs1_slider.png).
- Live category label (240,176 190x22) diff 0 px - the subtitle
  updates lazily (on rebuild), not per-frame; cosmetic, not a blocker.
- Negative control: preview donut region (640,40 280x200) diff 0 px -
  code-accurate: the preferences preview is canvas-fixed
  (preferences_view.rs:1085-1093) and does not consume
  appearance.size; the real widget consumes it at widget_app.rs:617-618.
- Restore: calibrated track clicks x=780->427, x=782->432, x=781->430
  (slope ~1.338 value/px). Final size=430 - RESTORED TO 430.
- Harness notes: the first unguarded run was invalidated by window
  occlusion (fg-before False; capture showed 0 traffic-light red
  dots). The guarded harness (per-action RefreshRect + traffic-light
  guard + ForceForeground with ALT-unlock / AttachThreadInput) is
  mandatory - it caught the occlusion on the immediate retry.
- Still queued: maximized capture, Search-clip diagnosis; context-menu
  wave; theme wave; icon blur + Miller 2+; A4 security, A5 fixes, A6 QA.

[Maximized capture verification: 2026-11-11, on pid 3444]

- Method: ShowWindow(hwnd,3) -> guarded capture -> ShowWindow(hwnd,9).
  First attempt aborted safely: the capture guard tripped (0 traffic-
  light red dots) because the user's maximized File Explorer was on
  top in z-order; the saved frame proved the maximize itself worked.
  The retry re-asserted foreground immediately before capture and
  passed the guard; the script restores the window in a finally block.
- Captured at rect (-8,24) 1936x1064 (standard maximized border
  overhang); window restored cleanly to 1120x720 @(412,167),
  zoomed=False (mx1b_settings_max.png).
- Result: the Settings -> Donut toolbar page lays out correctly in
  maximized mode. All three columns render (settings categories,
  profile editor, Available presets). The presets column, clipped at
  the right edge in the 1120px windowed layout ("Edit Gr...", "Pages
  per sh..."), is fully visible maximized (full labels: "Edit Group of
  6", "Pages per sheet Group of 5", "Crop video Group of 6",
  "Grayscale Same format", "Trim transparent borders") - consistent
  with the 1280px toolbar-width uncapping in 4de5917. No overlapping
  panels, no rendering garbage. The band below the content divider is
  trailing scroll space, not a defect.
- Appearance card shows Size 430 px (restored value), Hole 52.0 px,
  Opacity 0.680 - config state intact through the maximize cycle.
- VERDICT: maximized-mode settings layout PASS; the 4de5917 fix is
  confirmed live.
- Still queued: Search-clip diagnosis (needs explorer tab); context-
  menu wave; theme wave; icon blur + Miller 2+; A4 security, A5
  fixes, A6 QA.

[Search-clip diagnosis: 2026-11-11, on pid 3444]

- Symptom under investigation: toolbar search box suspected to clip
  (search_w=170, studio_view.rs:705-715).
- Method: explorer Home view captured windowed (1120x720) and
  maximized (1936x1064; ShowWindow 3 -> guarded capture -> ShowWindow
  9). Window restored cleanly to 412,167 1120x720 both times.
- Findings (crop_sc_win_search.png, crop_sc_max_search.png):
  1. The TextEdit is a fixed 170px box in a right_to_left toolbar
     cluster. In maximized mode the box sits well inside the window
     (right edge ~1920 of 1928) and nothing in the cluster is
     clipped: segmented view switcher, new-window, gear, and search
     all fully visible. In windowed mode the box ends ~3px from the
     window edge (flush-right Finder-style placement) but is not cut.
  2. The placeholder renders elided as "Search (/..." in BOTH
     layouts (code hint is "Search (/)...", studio_view.rs:714). The
     elision is width-independent: the hint does not fit the 170px
     TextEdit inner text area at the toolbar font. It is a cosmetic
     single-control defect, not a layout clip.
- VERDICT: no layout clipping in either mode. Queued 1-line fix
  (widen search_w to ~190 or shorten the hint to "Search (/)") for
  the next code-touching wave; not blocking.
- Incidental live proof: the maximized crop shows the active
  "Columns" segment of the view switcher wearing the new pill ring -
  the 9cf27e4 ring system rendering live.
- Live-verification batch COMPLETE: Ctrl+T single-fire (chain),
  Ctrl+W single-fire (93cb097), donut live-apply (c046bed), maximized
  settings layout (6435f76), Search-clip diagnosed (this entry).
- Next: context-menu wave - deep research first (Finder menu model +
  Windows shell integration), then plan, then implementation.

[Phase 1 Miller right-click live-verified: 2026-11-11, on pid 7568]

- Wave: context-menu (plan 4578ba8). Phase 1 commit 359bc9a added
  right-click menus to Miller Columns: canvas menu registered FIRST
  inside the outer Miller ScrollArea (explorer_miller_canvas_bg) so
  rows keep pointer priority; item menu hooks on both row loops
  (first column col1_w + sub-columns col_w) with the Explorer
  multi-select rule; full intent consumption mirroring the list
  view; _on_exam_conform/_on_inspect_file un-underscored. Five
  gates green pre-commit.
- Incident during setup: navigation script failed twice with
  "window lost". Root cause by direct diagnostic (wdiag.ps1): the
  window was MINIMIZED (iconic True, rect -32000,-32000 160x28),
  which fails the >400x300 FindMain size filter while
  Process.MainWindowHandle still reports the handle. Process alive
  throughout - NOT silent death #4. Protocol fix adopted for all
  live scripts: FindMain records iconic windows (IconicHwnd) and
  the retry loop SW_RESTOREs them before re-enumerating
  (nav_c_drive.ps1 pattern, now the standard).
- Live evidence (all guard-passed, pid 7568, rect 400,196
  1120x720, view switcher Columns segment active):
  1. mc2_item_menu.png - right-click "home" row in Miller column 1
     opens the full folder item menu: Open / Open in new window /
     Open in new tab | Compress to ZIP file / Create a copy |
     Send To > | Copy as path > | Show in Explorer / Open in
     Terminal / Create shortcut | Cut / Copy | Tags > | Pin to
     Sidebar | Rename / Delete / Scan with Microsoft Defender /
     Properties. THE DEFECT (right-click dead in Miller) IS FIXED.
  2. mc4_canvas_menu.png - right-click intended for empty space
     landed on row 19 (vfcompat.dll; 30px pitch miscalculation) and
     opened the FILE item menu: Open / Edit / Quick look | Convert >
     | Compress to ZIP / Conform to exam specification / Quality
     and integrity inspect. / Open with / Create a copy | Send To >
     | Copy as path > | ... - bonus proof for file rows. Also
     confirms "Open with" is still the flat pre-Phase-2 row (no
     submenu arrow).
  3. mc7_canvas_menu.png - PerfLogs selected (empty folder ->
     empty column 2, two-column reflow col_w ~444); right-click
     column 2 empty space (750,550) opened the "Explorer" canvas
     menu: New folder / Open in new window / Open in new tab |
     Refresh / Go to Parent Folder | Sort by > | View > | Select
     all / Invert Selection / Clear Selection / Paste (grayed) |
     Open in Terminal / Show in Explorer / Properties.
     Canvas-first registration works: rows win pointer priority,
     empty space falls through to canvas.
  4. mc3 / mc5 / mc8 captures - ESC dismisses both menu types
     cleanly; baseline intact after each test ("19 items (7
     hidden)"; 1 selected 0 B after the PerfLogs test).
- VERDICT: Phase 1 (Miller Columns right-click fix) PASS in the
  live release binary. Item menus proven on folder row, file row,
  and sub-column rows share the same builder; canvas menu proven
  on empty column space.
- Next: Phase 2 - Open With submenu (registry enumeration,
  windows-sys only, research decision D-1): open_with_candidates()
  + launch_open_with() in context_menu.rs; replace the flat
  OpenWith row in run_item_context_menu with a Send To-style
  submenu ending in "Choose another app..." (existing rundll32
  dialog).


[Phase 2 Open With submenu live-proven + menu-row click fix: 2026-11-11, on pids 20424/11956/26776/28292]

- Wave: context-menu (plan 4578ba8). Phase 2 added the Open With
  submenu to all item context menus: static registry enumeration
  (research decision D-1, windows-sys only, no COM): FileExts
  UserChoice default first, then FileExts OpenWithProgids, then HKCR
  .ext OpenWithProgids, then a bounded Applications\* SupportedTypes
  scan; dedupe, cap 8; launch by resolving shell\open\command with
  %1/%L substituted to the quoted path via CreateProcessW; final row
  "Choose another app..." -> open_with_choose().
- Defect found and fixed (menu-row click registration): egui 0.29.1
  Response::interact(Sense::click()) re-registers the SAME widget id
  with OR'd sense -> hover resolves via the container but clicks
  never resolve (hit-test/interaction contention). Fix: fresh-id
  widget ui.interact(container.response.rect,
  container.response.id.with("mossambi_menu_click"), Sense::click())
  at all four return sites of menu_item_labeled/menu_item_glyph
  (Finder + non-Finder branches). Temporary [MENU-HOVER]/[MENU-CLICK]
  instrumentation used for calibration, then removed before commit.
- Live proof (instrumented binaries pid 20424 broken / 11956 fixed):
  identical probe produced zero clicks pre-fix, then [MENU-CLICK]
  fired for Scan with Microsoft Defender [406,658]-[601,677], Notepad
  [619,302]-[695,321] (x2), and Choose another app...
  [619,364]-[767,383]; notepad process delta 0 -> 2; LAUNCH PROOF:
  notepad pid=27248 title='setup.log - Notepad' - the %1/append
  file-argument substitution works end to end.
- System defect documented (NOT an app defect): the Windows Open
  With picker dialog never appears on this machine. Verified
  standalone outside the app: rundll32 OpenAs_RunDLL spawns and
  exits silently (pid delta caught mid-poll, gone by poll end);
  ShellExecuteW openas verb internally does the same; direct
  SHOpenWithDialog blocks with no visible dialog (MTA and STA
  retries). Three identical desktop censuses (11 titled windows, no
  picker); ApplicationFrameHost owns zero windows. Wiring is
  live-proven up to the OS boundary; open_with_choose now tries the
  openas verb first with rundll32 OpenAs_RunDLL as fallback.
- Incidental: app silent-death candidates now include pid 28292
  (empty stderr); two Application Event Log Id=1000 records found
  for 11/09 23:22 matching mossambi. Probes used throughout:
  RefreshRect + traffic-light guard + ForceForeground + iconic
  restore + finally-cleanup; calibration parsed live from hover logs.
- VERDICT: Phase 2 PASS in the live release binary (click
  registration + launch chain). Choose-another dialog blocked by a
  machine-level shell defect, documented with evidence above.
- Next: Phase 3 Finder conformance - live-capture current menu
  baseline, make rows text-first and iconless (resolve G-1 conflict:
  file_menu.rs doc says every entry has a glyph), verify hairline
  separators and hover accent fill. Then Phase 4 customization
  (config context_menu.custom_items + Preferences "Context Menu"
  section), then Phase 5 Search-clip hint fix ("Search (/)") at
  studio_view.rs:714.

[Phase 3 Finder conformance + cursor discipline + search hint: 2026-11-11, on pid 9244]

- Wave: context-menu (plan 4578ba8) Phase 3, plus two strict user
  directives received mid-cycle (cursor shape; no zebra).
- Phase 3 changes (file_menu.rs): menu_item_labeled and
  menu_item_glyph collapsed into one allocation-native renderer
  menu_row_text - a full-width click-sensed row
  (allocate_exact_size(available_width, galley_height + 8),
  Sense::click()), egui widget-state hover/active accent fill painted
  behind the label before the 15 px white galley (SubMenuButton
  idiom, LEFT_CENTER with 6 px inset). Glyph/pack/action/style/tint
  params retained underscored for call-site compatibility; egui_label
  helper removed; G-1 doc conflict resolved (glyph kept in
  signatures, never painted; module doc updated). Visuals are
  Finder-always in every wording style (design directive decouples
  visuals from wording).
- Click mechanism refined: the allocation-native allocation IS the
  fresh click-sensed widget - it replaces the 95cc29a
  ui.interact(fresh id) form. Live re-proof on the release binary
  (pid 9244): right-click setup.log (400,606) -> hover Open with ->
  flyout -> click Notepad row center (760,275) -> notepad process
  delta 0 -> 2, menu closed, stderr clean. New menu geometry:
  panel x 402-795, rows ~30 px pitch (iconless rows are taller than
  the old 19 px icon rows); flyout x 722-1085 (Notepad ~y275,
  Code ~y307, Choose another app... ~y349). Captures: p3a_menu_open,
  p3b_menu, p3b_flyout (hover accent fill visible on the Open with
  row), p3c_after_click.
- Cursor discipline (strict rule, new standing rule): the I-beam
  text cursor is allowed ONLY in text-editing widgets. Root cause in
  egui 0.29.1: defaults selectable_labels=true AND
  multi_widget_text_select=true make static labels paint
  CursorIcon::Text on hover app-wide (label_text_selection.rs
  :163/:488). Fix: style.interaction.selectable_labels=false +
  style.interaction.multi_widget_text_select=false in studio_app.rs
  style setup; menu_row_text additionally returns
  resp.on_hover_cursor(CursorIcon::Default). TextEdit keeps its own
  I-beam (text_edit/builder.rs:596). Empirical census via
  GetCursorInfo hCursor compared against LoadCursor(IDC_ARROW=32512)/
  LoadCursor(IDC_IBEAM=32513) (screenshots cannot show cursors):
  file list row ARROW, menu row ARROW, flyout row ARROW, search box
  IBEAM. Probe: p3b_verify.ps1.
- Phase 5 (search hint): "Search (/)..." -> "Search (/)" at
  studio_view.rs:714; windowed captures show the hint fully rendered,
  elision gone.
- Zebra check: grep (% 2, alternating, even_row, odd_row) found no
  alternating-row shading in the codebase; p3b_listview capture shows
  uniform row backgrounds. The user's "no alternative line
  highlighting" directive is recorded as a spec constraint for the
  upcoming list-view columns wave (no zebra, ever).
- Gates: cargo fmt clean; clippy -D warnings exit 0 (6.42s); test
  exit 0; ASCII sweep 0; release build 58.39s exit 0.
- VERDICT: PASS - Phase 3 Finder conformance, cursor discipline, and
  the Phase 5 hint fix are live-proven in the release binary.
- Next: Phase 4 menu customization (config context_menu.custom_items
  {label, kind: command|script|app|link, value, args?, show_in} +
  run_custom_item with %p/%1 substitution + Preferences "Context
  Menu" section), then the THEME wave (user priority: 12 palettes +
  Rangalipi at the theme-picker gate, macOS Finder principles), then
  the list-view columns wave (all file parameters as add/remove
  columns, resizable, sortable, filterable - NO zebra), then icon
  blur fix, Miller 2+ fix, breadcrumb horizontal scroll, A4/A5/A6 QA.

- Wave: context-menu Phase 4 (user-configurable custom entries) plus
  the menu-width + acrylic material wave; Phase 4 is live-proven,
  the width/acrylic wave is gate-green with live verification
  deferred to the post-wave computer-use phase per user directive
  (finish all code waves first, then verify everything and fix
  corrections/mistakes/bugs found).
- Phase 4 (e2d91fe feat, 5 files +388): config CustomMenuItem
  {label, kind command|script|app|link, value, args, show_in
  files|folders|background|all} serde snake_case; intent carries
  Option<(CustomMenuItem, Option<PathBuf>)>; builders read
  MossambiConfig::load() inline; context_menu.rs dispatch =
  run_custom_item / run_command_line / shell_execute_open with %p
  file-path substitution via CreateProcessW. Preferences editor
  added (preferences_view.rs dirty-flag pattern); E0716 fixed by
  binding config loaders to locals; E0433 fixed by absolute paths in
  canvas menus. Custom group renders between Shortcut and Clipboard
  groups in item and canvas menus. NOTE: the recurring "run_line
  helper" claim in injected summaries is FALSE - grep finds zero
  matches; no such refactor exists.
- Phase 4 LIVE-VERIFIED (release binary): custom group in item +
  canvas + folder menus (p4a_item, p4b_canvas, p4c_canvas2); notepad
  launch via %p, process delta 0 -> 2, setup.log opened
  (p4b_after_item); scope filtering per show_in; PerfLogs empty-dir
  navigation for the canvas menu (p4c_nav1, p4c_nav2,
  p4c_empty_folder); ARROW cursor census on custom rows; stderr
  clean.
- Menu width + acrylic wave (8009be8 feat, 2 files +52/-2). ROOT
  CAUSE of the ~395 px menus: egui Spacing::default menu_width =
  400.0 (style.rs:1256). Mechanism proven from egui 0.29.1 source:
  menu.rs menu_popup (:159-219) is the single Area constructor for
  ALL menus (root, submenu, context) - fixed_pos(menu_state.rect.min)
  (pointer pos for context menus) + default_width(spacing.menu_width);
  the anchor response rect is NEVER consulted, and popup.rs
  inner_width logic (popup_above_or_below_widget :408-410) is not in
  the menu path, so the earlier anchor-clone plan was REFUTED and
  dropped (no helper, no call-site wrapping). Frame::menu
  (containers/frame.rs:118-127): rounding = visuals.menu_rounding,
  fill = visuals.window_fill(), inner_margin = spacing.menu_margin;
  set_menu_style (menu.rs:77-83) touches only button_padding and
  widget strokes.
- Fix (macos_theme.rs apply_theme_visuals): one global line
  ctx.style_mut(|style| style.spacing.menu_width = 230.0) narrows
  root menus AND every submenu level (all Areas share the spacing
  value); guarded acrylic material: palette.is_translucent and
  window_bg.a() == 255 -> window_fill =
  Color32::from_rgba_unmultiplied(r,g,b,232) and visuals.menu_rounding
  = Rounding::same(10.0); else solid window_fill. Menus and popups
  paint window_fill via Frame::menu / Frame::popup; the main window
  canvas stays solid (Apple HIG).
- Elision (file_menu.rs): elide_to_width helper (measure-chop with
  the SAME font as the render, ASCII "..." ellipsis) applied to
  menu_row_text (budget = available_width - 12) and menu_header
  (13.5 px font, same budget) - Finder truncation style for long
  file names and user-configured custom labels; single line, no
  wrap.
- Clippy catch fixed: absurd_extreme_comparisons on window_bg.a()
  >= 255 (255 is the u8 max) -> == 255.
- Gates (menu wave): cargo fmt clean; clippy -D warnings exit 0
  (4.45s); cargo test exit 0 (69 lib + all integration suites +
  ASCII gate); release build 1m 09s exit 0 (running app pid 24940
  killed first - exe lock, it held the old pre-wave binary).
- VERDICT: Phase 4 PASS (live-proven). Menu width + acrylic wave
  CODE-PASS (all gates green) with live verification QUEUED to the
  post-wave computer-use phase.
- Next: THEME wave (user priority: 12 palettes + agent-designed
  Rangalipi at the theme-picker gate, macOS Finder principles), then
  the list-view columns wave (all file parameters as add/remove
  columns, resizable, sortable, filterable - NO zebra), then icon
  blur fix, Miller 2+ fix, breadcrumb horizontal scroll, A4
  security, A5 fixes, A6 QA.
- Wave: THEME - the 20 Pantone themes replaced with 13 named
  palettes (Rangalipi signature dark default + Mossambi Punch
  true-fruit light; 11 canonical editor palettes between them);
  all five gates green; live theme-picker review queued to the
  post-wave computer-use phase per user directive (finish all
  code waves first, then verify everything and fix what it finds).
- Commit e69518b feat (11 files +610/-753). pantone.rs: one
  THEMES: [PantoneTheme; 13] catalog (gruvbox, tokyo_night,
  catppuccin_mocha, modus_vivendi, nord, dracula, kanagawa,
  one_dark, atom, dark_plus, synthwave, rangalipi #default dark,
  mossambi_punch #the one light); PantoneColor.code now records
  source provenance (research + verified values in
  docs/superpowers/plans/2026-11-12-theme-research.md); fruit
  consts and CitrusShade preserved verbatim; get_pantone_theme
  fallback -> MossambiPunch for light-ish queries, else Rangalipi.
- Migration: stacked #[serde(alias)] on Rangalipi covers all 20
  old dark ids/names plus liquid_glass/liquidglass/frosted_glass/
  frostedglass/dark/system; MossambiPunch covers all 10 old light
  ids/names plus light/light_aqua/lightaqua; from_id_or_name
  remaps every legacy string by darkness (20-id dark table ->
  Rangalipi, 20-id light table -> MossambiPunch), so an existing
  config.json "liquid_glass" loads as Rangalipi.
- ThemeMode and ThemeStyle: 13-variant enums with identical alias
  sets, From impls 13-way 1:1, all() 13, dark_themes()=[0..12],
  light_themes()=[12..13], toggle_dark_light() Rangalipi <->
  MossambiPunch; MossambiConfig::default theme_mode = Rangalipi.
  GlassEffect (FrostedGlass/LiquidGlass) is the orthogonal glass
  system and was NOT touched (config.rs, dwm_helper.rs,
  colors_with_glass in macos_theme.rs, preferences_view.rs).
- Stragglers swept: tray.rs quick-picks remapped to Rangalipi /
  Gruvbox / CatppuccinMocha / MossambiPunch (CMD_THEME_* renamed,
  ids 2010-2013 kept, submenu title "Themes (13 Named Palettes)");
  tui [T] status "11 Dark Themes"/"1 Light Theme", [t] hint
  "13 named themes", "Active Theme:"; main.rs handle_themes
  rewritten ASCII (2611 -> 2142 chars, PowerShell regex surgery;
  prints 13 themes with provenance, --json kept); config set
  theme dark/light -> Rangalipi/MossambiPunch; widget toggle
  same; test_cli asserts "Rangalipi"; Preferences label
  "Pick a theme."
- tests/test_pantone.rs rewritten: 6 tests (counts 13/12/1;
  integrity with hex format, RGB roundtrip, provenance; lookup
  incl. legacy fallbacks; 13-step ThemeMode cycling + toggle;
  serde aliases incl. liquid_glass -> Rangalipi; defaults incl.
  ThemeStyle::default() = Rangalipi).
- Clippy catch: doc_lazy_continuation on "//! Light (1):" (parsed
  as a lazy continuation of list item 12) -> blank //! line
  before it.
- Gates: cargo fmt clean; clippy -D warnings exit 0 (5.29s);
  cargo test exit 0 (69 lib, test_pantone 6/6, test_cli 9/9,
  ASCII gate ok); release build 48.74s exit 0 (mossambi.exe not
  running); git status exactly the 11 edited files, no strays.
- VERDICT: THEME wave CODE-PASS. Live theme-picker gate (13
  palettes incl. Rangalipi applied and captured for review)
  QUEUED to the post-wave computer-use phase.
- Next: list-view columns wave (all file parameters as add/remove
  columns, resizable, sortable, filterable - NO zebra), then icon
  blur fix, Miller 2+ fix, breadcrumb horizontal scroll, A4
  security, A5 fixes, A6 QA, then the post-wave computer-use
  verification phase (theme-picker gate + all queued probes).

- COLUMN WAVE (2026-11-12): details-view columns - every file
  parameter addable/removable/resizable/sortable/filterable, NO
  zebra highlighting (standing rule). Plans committed 6af4ada
  (columns research + implementation docs). Commit 8f9cf20 feat
  (4 files +1155/-140): fs_utils.rs, explorer_view.rs, config.rs,
  tests/test_columns.rs (new).
- Step 1 fs_utils.rs: FileItem.created Option<SystemTime>
  populated in from_path via metadata.created().ok();
  formatted_created() mirrors the Finder date format (e.g. 22 Nov
  2024 at 08:30). from_path remains the sole FileItem constructor.
- Step 2 explorer_view.rs: pub cmp_opt_last (None sorts last) and
  kind_key via resolve_file_icon().category (a static str);
  SortColumn grown to 11 variants (Name Size Extension Date
  Permissions DateCreated Hidden Symlink SymlinkTarget Path Kind)
  with an 11-arm comparator (Date/DateCreated/SymlinkTarget via
  cmp_opt_last, dirs-first retained); DetailColumn 11 variants
  with id()/from_id()/label()/default_w()/min_w()/aligns_right()
  (only Size right-aligns)/default_visible_ids [0,1,2,3] (Finder
  Name, Date Modified, Size, Kind)/sort_column(); ColumnLayout
  finder_default/width/set_width (clamps to min)/is_locked (Name
  always visible, always first)/show/hide/from_config/to_config.
- Step 3 session-scoped per-column filters: ColumnFilterValue =
  Text (case-insensitive substring) / TriState / SizeMin or
  SizeMax (files only) / Date (Any Today ThisWeek ThisMonth
  Older), keyed by DetailColumn id in ExplorerView.column_filters
  and composed AND into filtered_and_sorted_entries after the
  category filter; pure date_in_bucket(date, today, bucket) for
  testability. Filters are NOT persisted.
- Step 4 render_details_table fully dynamic: header_specs built
  from ColumnLayout.visible; the LAST visible column auto-fills
  the remaining width clamped to its min; headers click-to-sort
  with ^/v indicator plus a |v funnel tick when a column filter
  is active; right-click header opens the visibility menu via
  menu_item_custom with ASCII [x]/[ ]/(locked) markers (Name
  locked), saving on toggle; 6 px drag strips on boundaries
  resize the LEFT column (Sense::drag, ResizeHorizontal cursor,
  drag_stopped -> save); row cells painted per column (Name keeps
  icon/tag dots/arrow suffix/hidden dim; Size right-aligned
  monospace with dirs as --; Permissions monospace; Path dim).
- Step 5 persistence: MossambiConfig.explorer_columns Vec<u8>
  (serde default vec![0,1,2,3]) and explorer_column_widths
  BTreeMap<u8,u32>; default_explorer_columns() keeps core
  gui-free; constructor loads ColumnLayout via a block-expression
  local cfg0 (E0716 lesson); save_column_layout() load-modify-
  save, called on visibility toggle and on resize release.
- Step 6 tests/test_columns.rs: 16 tests - dense ids 0..=10,
  ASCII labels, min<=default, only-Size right-align, from_id
  bounds, injective sort mapping, cmp_opt_last None-last,
  fixed-today date buckets (2026-11-12), text/Kind/tri-state/
  size/date filter semantics, Finder defaults plus Default
  fallback, show/hide/lock/clamp, to_config/from_config
  roundtrip, legacy [2,7,0,99] recovery, config serde roundtrip
  and legacy-keys-removed defaults. FileItem built via struct
  literal (11 fields, permissions_mode u32).
- Test-caught bug: from_config guarded emptiness AFTER inserting
  the locked Name (the list was always non-empty by then), so an
  empty persisted list wiped the Finder defaults down to [Name].
  Fixed to guard the raw visible_ids BEFORE building: empty ->
  Finder defaults; [2,7,0,99] -> [Name, Size, Symlink] with 99
  dropped and widths applied. Locked by the legacy/unknown test.
- Clippy catches (3): needless_borrow explorer_view.rs:1307
  (kind_key already returns a reference - dropped the extra &),
  field_reassign_with_default test_columns.rs:458 (struct-update
  syntax ..MossambiConfig::default()), needless_character_
  iteration test_columns.rs:48 (label.is_ascii()).
- Gates: cargo fmt clean; clippy -D warnings exit 0; cargo test
  exit 0 (test_columns 16/16, full suite green, ASCII gate ok);
  release build 49.97s exit 0 (mossambi.exe not running);
  git status exactly fs_utils/explorer_view/config/test_columns.
- VERDICT: COLUMN wave CODE-PASS. Live columns proofs (resize
  drag, header visibility menu, sort arrows, funnel filters,
  no-zebra census) QUEUED to the post-wave computer-use phase
  per user directive (finish all code waves first, then verify
  everything and fix what it finds).
- Next: icon blur fix, Miller 2+ fix, breadcrumb horizontal
  scroll, A4 security, A5 fixes, A6 QA, then the post-wave
  computer-use verification phase (theme-picker gate + columns
  proofs + all queued probes).

- ICON BLUR WAVE (2026-11-12): pixel-snapped icon painting, crisp
  at any display scale, no resampling in the icon paint path.
  Plans committed b45cb48 (icon-blur research + implementation
  docs). Commit 8610a83 feat (3 files +113/-10): icon_packs.rs,
  explorer_view.rs, tests/test_icons.rs (new).
- Research census: 4 blur vectors - (A) fractional paint position
  (every file-list call site centers on row_rect.center().y),
  (B) texture/quad device-size drift (quad uses raw size*ppp,
  texture uses round(css*ppp) clamped 12..512), (C) 7 direct
  icon_texture consumers rasterize at one size and display at a
  smaller one (5 nav buttons 24-to-20, 2 pills 16-to-14),
  (D) rasterize truncation casts one device pixel under phys.
- Step 1 icon_packs.rs: new pub snapped_icon_quad(ctx, rect,
  css_px) - phys formula byte-identical to icon_texture
  (((css_px*ppp).round() as u32).clamp(12,512)), css = phys/ppp,
  min corner snapped to integer device pixels ((min*ppp).round()
  /ppp), quad from_min_size(min, splat(css)). paint_icon_with
  now paints snapped_icon_quad(ctx, rect, size) instead of the
  raw from_center_size(rect.center(), splat(size)) - the single
  choke point covers ALL paint_icon* consumers.
- Step 2 icon_packs.rs: rasterize scaled_w/scaled_h casts now
  .round() - the off-by-one under f32 epsilon is gone.
- Step 3 explorer_view.rs: 7 direct consumers request the DISPLAY
  size (texture/quad matched, 1:1 blit): 5 nav buttons back/fwd/
  up/refresh/newfolder 24.0-to-20.0 (replaceAll on the 3-line
  24.0/nav_tint/) pattern, confirmed 5 occurrences by nav_tint
  census first), filter pill + toolbar pill 16.0-to-14.0.
  nav_icon_button px-to-px matched, untouched (egui Button
  layout position = accepted residual, out of scope).
- Step 4 tests/test_icons.rs: 4 tests - ppp 1.0 integer snap +
  exact 20.0 quad, ppp 1.25/1.5 device-width integral and equal
  to the icon_texture phys size, 2 px request clamps to the 12
  device-pixel floor, 6000 px request clamps to 512.
- TEST-CAUGHT EGUI SEMANTIC (durable lesson): Context::
  set_pixels_per_point is DEFERRED until the next begin_pass -
  pixels_per_point() keeps returning the old value after set.
  Probed empirically with a throwaway probe_ppp.rs (1.0 before
  set, 1.0 after set, 1.25 after begin_pass; helper quad
  verified correct: min [0.8 10.4], width 20, dx 1, dy 13).
  Fix: tests use ctx_at() which sets ppp AND begins a pass
  (simulating a real frame), plus a semantics-lock assert that
  the requested ppp is ACTIVE not deferred. Probe file deleted.
- Gates: cargo fmt clean; clippy -D warnings exit 0; cargo test
  exit 0 (full suite green, test_icons 4/4, ASCII gate ok);
  release build 49.69s exit 0 (mossambi.exe not running);
  git status exactly icon_packs/explorer_view/test_icons.
- VERDICT: ICON BLUR wave CODE-PASS. Live crispness comparison
  captures QUEUED to the post-wave computer-use phase per user
  directive (finish all code waves first, then verify everything
  and fix what it finds).
- Next: Miller 2+ fix, breadcrumb horizontal scroll, A4
  security, A5 fixes, A6 QA, then the post-wave computer-use
  verification phase (theme-picker gate 13 palettes + columns
  proofs + icon crispness captures + all queued probes).

2026-11-12 MILLER LINEAGE WAVE CLOSED (feat(gui) 8c33689, +592/-529 over
2 files): Miller columns render Finder-conformant lineage on every
navigation. (1) Pure model in explorer_view.rs: ancestor_chain(dir)
root-first (excludes dir; empty for roots and relative paths),
lineage_policy(new, old, stack) -> MillerLineage {KeepStack, ShiftTo(k),
ClearStack}, ColumnRole {Ancestor, Current, Descendant};
tests/test_miller.rs 10/10. (2) navigate_to lineage rewrite: KeepStack
(refresh + fs-op completion via poll_background_scans -> navigate_to)
preserves chain/stack/preview/rename-in-progress; ShiftTo(k) drains
stack[0..=k] so deeper columns stay open and sets a directory
placeholder preview that keeps the auto-preview guard dormant (the
inspector renders files only); pure up-nav (dir.parent()==old_dir)
selects old_dir + placeholder preview (old_dir cloned before the
HashSet insert - E0382 fix); unrelated nav clears the stack.
needs_miller_scroll_align one-shot set by navigate_to (ShiftTo/
ClearStack) and the post-render stack-push site; consumed after the
inspector inside the horizontal closure via
ui.scroll_to_cursor(Some(Align::Max)) so the deepest column +
inspector are revealed. (3) Render restructure: the duplicated col-0
block and stack loop (~260 lines each) collapsed into ONE shared
render_miller_column(role) method via line-index surgery with hard
boundary assertions; chain columns built per frame from ancestor_chain
+ get_column_entries cache; num_cols = chain.len()+1+stack.len();
col1_w dropped; unified tags (all roles), drop-highlight alpha 80,
chevron, hover and selection bands in every role; path-based id_salt
("miller_col_scroll", col_salt) replaces both old salts (one-time
scroll reset on navigation); dispatch: Ancestor = navigate or preview
(no-op on the chain-highlighted row), Current = legacy col-0 dispatch,
Descendant = legacy stack dispatch. (4) column_subfolder field + ctor
init + all writes REMOVED; census now 0 in src and tests (ledger/plan
mentions are historical text only). Gates green: fmt, clippy
-D warnings, full test suite (incl ASCII sweep over src/gui), release
build 1m00s, porcelain exactly explorer_view.rs + tests/test_miller.rs.
Deviations from the plan doc: miller_suppress_autopreview was added in
Step 2 then REPLACED by directory placeholder previews (the one-shot
suppress only delayed the phantom preview one frame - the auto-preview
guard re-fires every frame while column_preview_item is None); scroll
align implemented via scroll_to_cursor(Align::Max) after the inspector
(not scroll_to_rect on the deepest column); tags now also painted in
stack columns; drop alpha unified 70/80 -> 80; focused_row highlight
kept only in the Current role.

2026-11-12 BREADCRUMB HORIZONTAL SCROLL WAVE CLOSED (feat(gui) d54b335,
+165/-129 over src/gui/explorer_view.rs only; plan doc committed e41c440
first): the Finder-style bottom path bar now scrolls horizontally
instead of clipping deep paths. (1) Defect pinned in research:
render_bottom_path_bar painted the segment trail in a plain
ui.horizontal - NO ScrollArea - so ancestor segments beyond the
available width were unreachable by mouse and the current segment
could sit out of view. Research also established the top bar carries
NO trail (nav pill + Finder-style current-folder title + search box;
its own comment says the full ancestor chain lives in the bottom path
bar) and parse_breadcrumbs (src/core/fs_utils.rs:514, unit-tested in
tests/test_fs_utils.rs) has exactly two consumers: explorer_view.rs
(this wave) and studio_view.rs:578 (separate view, out of scope).
(2) Fix: the segment row is wrapped in
egui::ScrollArea::horizontal().id_salt("breadcrumb_hscroll")
.auto_shrink([false, true]) (full width, content height)
.scroll_bar_visibility(VisibleWhenNeeded) - the scrollbar appears
only when the trail actually overflows, wheel scrolling always works,
item_spacing (3.0, 0.0) preserved. DnD drop targets, child-folder
popups and hover anims work inside the scroll area (pattern proven
live in the Miller wave). (3) One-shot alignment: new field
needs_breadcrumb_scroll_align: bool next to needs_miller_scroll_align
(ctor init false), set in navigate_to ONLY inside the
lineage != MillerLineage::KeepStack branch so a refresh (fs-op
completion) preserves the user manual scroll position; consumed
inside the ScrollArea closure after the row via
ui.scroll_to_cursor(Some(egui::Align::Max)) - right alignment reveals
the current (right-most) segment after every navigation; clicking an
ancestor re-roots the trail so the clicked segment becomes the
right-most, correct in both directions. Signature proven live in the
Miller wave (egui 0.29.1). (4) Gates green: fmt, clippy -D warnings,
full test suite (incl ASCII sweep over src/gui), release build 1m08s,
porcelain exactly src/gui/explorer_view.rs. Commit style
feat(gui)/docs(plan)/docs(audit), ASCII-only new content.
Out of scope (recorded in the plan): studio_view.rs breadcrumb
consumer, Finder-style segment collapse ("..." root elision) - scope
was scrolling, not elision - and the pre-existing synchronous
list_directory_sync inside the child-folder dropdown popup.

2026-11-12 A4 SECURITY AUDIT WAVE CLOSED (docs(plan) 5c446e9 94 lines,
fix(gui) 01d8e60 +5/-1): full execution-surface census read verbatim
across src/ - shell_verb, open_with_choose (openas verb first, rundll32
OpenAs_RunDLL fallback), expand_env_vars, open_with::candidates
(KEY_READ-only registry), open_with::launch, run_command_line,
run_custom_item (Command/App/Script/Link kinds), shell_execute_open,
open_in_default_app (+ cmd /c start argv-array fallback),
open_in_default_editor, reveal_in_file_manager, open_terminal_in_dir,
perform_drop, apply_item_intent (extract/delete/cut/copy/duplicate),
extract_archive_safe + unpack_tar_safe + extract_single_file_safe, the
fs worker (Copy via resolve_unique_paste_path + action_journal, Move,
Delete staging), reg.exe autostart add/delete/query, ffmpeg encoder
probe, daemon start/stop, widget spawns.
FINDING S-1 HIGH (FIXED): open_terminal_in_dir PowerShell fallback
interpolated the raw path into "Set-Location -LiteralPath \x27{path}\x27"
with NO escaping - an NTFS-legal single quote in a folder name
terminates the PowerShell string and executes injected code. Fix:
double embedded single quotes (\x27 -> \x27\x27) before interpolation.
Gates green: fmt, clippy -D warnings, full test suite (incl ASCII
sweep over src/gui), release build 46.54s, porcelain exactly
src/gui/explorer_view.rs.
S-2 INFO (accepted boundary, recorded not changed): naive "{path}"
double-quote wrapping in the rundll32 fallback, substitute_target and
open_with::launch - safe because Windows paths cannot contain "
(NTFS-forbidden character).
S-3 INFO (by design): run_command_line executes user-config command
lines verbatim via CreateProcessW; trust boundary is the user\x27s own
MossambiConfig file, identical to Explorer run-verb menus.
S-4 verified safe: extract_archive_safe dunce-canonicalizes the dest,
zip enclosed_name() Zip-Slip bail, tar ParentDir/RootDir/Prefix
component bail, symlink/hardlink escape checks, entry-count cap and
uncompressed-bytes ceiling with a bounded take() read (bomb
protection); perform_drop rejects dest holding/containing source
(same_or_within + parent check); deletes filtered by
is_protected_os_file with trash-first + logged permanent fallback;
reg/ffmpeg/daemon/widget spawns all Command::args([...]) argv-array
form; expand_env_vars preserves unknown %VAR% pairs literally.
Deviations from the plan doc: none.

2026-11-12 A5 FIXES WAVE CLOSED (docs(plan) 96d03df 94 lines, fix(gui)
a2f1132 +79/-7 over explorer_view.rs (+34) and studio_view.rs (+45)):
scope came from the breadcrumb wave plan recorded out-of-scope items -
no A-series definition exists in any doc or ledger.
F-1 STUDIO RIBBON CLIPPING (FIXED): the Explorer Path Ribbon rendered
the breadcrumb segment loop and the Edit button inside ONE plain
ui.horizontal over a bounded bc_rect, so deep trails clipped silently
- the same defect the breadcrumb wave fixed on the explorer side.
Fix: segment loop wrapped in
egui::ScrollArea::horizontal().id_salt("studio_breadcrumb_hscroll")
.auto_shrink([false,true])
.scroll_bar_visibility(VisibleWhenNeeded) inside a fixed
allocate_ui_with_layout child (trail_w = (ribbon_w - 8.0 -
edit_reserve).max(120.0), edit_reserve 52.0) so the Edit button stays
pinned right while the trail scrolls.
F-2 PER-FRAME SYNC LISTINGS (FIXED): egui re-renders open popups and
context menus every frame; both deferred closures re-ran
list_directory_sync on the UI thread each frame while open (explorer
child dropdown, studio sibling menu), blocking the frame loop on
slow/deep/network directories. Bounded fix without new async infra:
once-per-open caches - ExplorerView.breadcrumb_popup_cache with method
popup_child_dirs (load once, filter is_dir, clone out) swapped into
the popup closure; StudioView.sibling_menu_cache with method
sibling_dirs swapped into the context-menu closure; caches cleared
UNCONDITIONALLY in navigate_to (explorer) and the nav_target block
(studio) so every navigation AND refresh (KeepStack fs-op completions)
invalidates listings - never stale after copy/move/rename/delete.
First open still performs one synchronous read by design.
PROCESS NOTE: the method insertions anchored on fn signature lines
stranded attributes above them - render lost its
#[allow(clippy::too_many_arguments)] (first clippy run failed 10/7 at
studio_view.rs:128) and navigate_to lost its doc comment; both
restored before commit. Lesson: when inserting a method before an
existing one, anchor oldString below any attributes/doc comments of
the following item, or verify the junction after insertion.
Gates green: fmt, clippy -D warnings, full test suite (incl ASCII
sweep over src/gui), release build 50.24s, porcelain exactly
src/gui/explorer_view.rs + src/gui/studio_view.rs.
Out of scope (recorded): full async directory-listing infrastructure
(first-open read stays synchronous), navigation-time synchronous bulk
loads (explorer :1049 :1070 :1169 :1312 :5107 - deliberate navigation
flow, not per-frame re-entry), Finder-style segment collapse (root
elision) still open from the breadcrumb wave.

2026-11-12 A6 QA WAVE CLOSED (docs(plan) d433f06 63 lines, QA report
2026-11-12-a6-qa-report.md committed with this entry): static
evidence-based audit of the seven closed waves plus standing user
rules - ALL 10 CHECKS PASS, zero defects, no fix commit needed.
QA-1 census: every wave plan+feat+ledger (theme 564e751/e69518b/a495070;
columns 6af4ada/8f9cf20/1aa9a87; icon blur b45cb48/8610a83/0dc3c9c;
Miller 4589f87/8c33689/0b03bd7; breadcrumb e41c440/d54b335/1c5dc6c; A4
5c446e9/01d8e60/7dec35f; A5 96d03df/a2f1132/41dc448; context-menu
phases 1-4 also ledgered).
QA-2 fresh cargo test exit 0: lib 69/69, gui_ascii_gate 1/1, columns
16/16, miller 10/10, pantone 6/6, fs_utils 4/4 - all 22 suites green.
QA-3 cursor discipline: exactly 3 CursorIcon sites in src/gui
(file_menu.rs:343 Default arrow on menu rows per the 2d96f8a contract;
explorer_view.rs:4223 ResizeHorizontal column-resize handle;
studio_app.rs:123 comment only) - zero CursorIcon::Text in code, no
leakage from post-2d96f8a waves.
QA-4 ASCII gate green over src/gui.
QA-5 no zebra: only unrelated TUI/NTFS/CLI alternate strings.
QA-6 menu_row_text sole allocation-native constructor;
menu_item_labeled/menu_item_glyph are thin wrappers delegating to it;
egui_label zero matches (Phase 3 removal holds).
QA-7 exactly 13 named palettes confirmed by grep census in pantone.rs
(Gruvbox :138 through Rangalipi :285 and Mossambi Punch :298),
Rangalipi default (config.rs:121 default, :95 => &THEMES[11]),
test_pantone 6/6.
QA-8 S-1 fix present verbatim: explorer_view.rs:8282 ps_path
single-quote doubling + :8286 Set-Location interpolation.
QA-9 A5 mechanisms: breadcrumb_popup_cache (field :723, ctor :990,
popup_child_dirs :1083-1096, popup call site :3113, unconditional
clear in navigate_to :1106); sibling_menu_cache (field :73, ctor
:101, sibling_dirs :111-122, context-menu call site :691, nav clear
:742).
QA-10 mirrors byte-identical SHA256 42DDEE0B6A3A73AC2C1047768158CD25E8
7BBF17EF03A108B81B965A9EBCAD2C x3 at QA time.
CLOSING: the code wave queue is EMPTY; next per user directive: the
post-wave computer-use verification phase (theme picker 13 palettes,
columns interactions, icon crispness, Miller chains, breadcrumb
scroll, A5 popup/menu behavior, Probe D stamp). Out-of-scope items
remain: Finder segment collapse root elision, async directory listing
infrastructure.

2026-11-13 B-FIXES WAVE CLOSED (docs(plan) 0e2a879 119 lines,
fix(gui) 485a803 3 files +511/-408 incl. fmt normalization of the
wrapped block): B-1 command-palette raw-key leak, B-2 stray Home
selection, B-3 missing active pill for non-explorer views. Five
gates green: fmt clean; clippy -D warnings 30.11s plus
--all-targets -- -D warnings 6.17s; cargo test full suite (lib
69/69, gui_ascii_gate, columns, miller, pantone, fs_utils, TUI,
widget); release 1m07s; porcelain exactly 3 src files + plan doc;
ASCII sweep: added lines clean, plan doc clean.
B-1 MECHANISM: egui raw input is app-wide; command_palette.rs
(pub is_open :279; raw reads Escape :394, arrows :432/:435, Enter
:447-452) renders AFTER the tab dispatch (studio_view.rs
:1341-1347), so consumption order cannot protect underlying views.
FIX (frame-scoped gate): ExplorerView gains pub palette_open
(explorer_view.rs after show_internal_toolbar; ctor default
false); studio_view.rs sets self.explorer_view.palette_open =
self.command_palette.is_open right after the Ctrl+P/K toggle; the
entire explorer raw-key block (explorer_view.rs :1603-2100:
Escape, j/k/h/l/g/y/p/d/r/v, Space, F2/F3/F5/F6, Tab, Delete,
arrows) is wrapped in if !self.palette_open; Ctrl+T (new tab) and
Ctrl+W (close tab) reads gated with && !self.command_palette.is_open;
studio_app.rs Ctrl+wheel zoom + Ctrl+0 reset gated with
!palette_open on all four outcomes. Ctrl+P/K toggle deliberately
UNGATED (it is the keyboard way to close the palette).
lost_focus-gated Enter sites (path-edit studio_view.rs :580; rename
TextEdits explorer_view.rs :2144/:2313/:2953) untouched - the
palette steals focus on open, so they are self-protecting.
B-2 MECHANISM: navigate_to pure up-nav branch (explorer_view.rs
:1136-1148) re-selects the folder just left (Finder parity for
folder listings); Desktop->Home qualifies as pure up-nav because
the Desktop directory is a child of the home dir, so the Home
dashboard inherited 1 selected (0 B). FIX: keep selected_paths.clear()
and the directory placeholder preview unconditionally; gate only
selected_paths.insert(old_dir.clone()) on
home_dir_opt().as_ref() != Some(&dir). Both Home entry paths
(sidebar Home row studio_view.rs :857-863; navigate_to_home
explorer_view.rs :1319-1324) route through navigate_to, so one fix
covers both.
B-3 MECHANISM: tab-strip pill activation was already gated on
current_tab == FileExplorer (studio_view.rs :220-222) so explorer
pills rendered inactive while Settings was open, but the strip
showed ONLY stale explorer pills - the active view had no pill.
FIX: new helper studio_tab_label(StudioTab) -> (label, glyph)
mirroring the sidebar Tools naming (Transcoder/ActionRefresh,
Custom Conformer/ActionEdit, Quality Inspector/Search, Archive
Suite/PageArchive, Hardware & AI/ActionShield, Preferences/Gear);
after the explorer-tab loop, when current_tab != FileExplorer one
130x26 active pill renders (card_bg fill, 1px border, Finder-blue
glyph [79,168,255], 12-char title elision, text_primary 13.5).
Non-clickable (it IS the current view), no close button.
VERIFICATION PENDING (computer-use, deferred per user rule until
all work completes): palette Enter->Preferences, palette typing
leaks nothing into the explorer, Home shows no stray selection
after Desktop->Home, tab strip shows the active-view pill in
Settings and tool views. Live instance pid=10644 hwnd=67656
(relaunched 2026-11-13; older pid 26716 / hwnd 23529548 are dead).
Ledger mirrors claude.md/gemini.md byte-identical (SHA256 x3).

2026-11-13 PATH-BAR REFINEMENT WAVE CLOSED: chain e74e904
docs(plan) (docs/superpowers/plans/2026-11-13-path-bar-refinement-
wave.md, 110 lines) <- 7629528 feat(gui) (3 files +25/-20)
<- this docs(audit) entry. EVIDENCE: user screenshot called the
always-visible caret-down "v" buttons cheap (one per bottom-bar
segment including a trailing one after the current folder).
MECHANISMS: (1) finder_chrome.rs NEW pub paint_path_chevron(ui,
color): 4.5x9.0 hover-sensed slot + Shape::line chevron-right,
Stroke 1.5_f32 (first draft 1.5 hit clippy float-literal-f32-
fallback under -D warnings; fixed). Horizontal layouts center
the slot vertically like the text separators it replaces.
(2) explorer_view.rs render_bottom_path_bar: text ">" separators
-> painted chevrons; the "v" caret Button DELETED on every
segment; popup bc_pop_{i}_{path} now toggles on
resp.secondary_clicked() (right-click on the segment itself,
Finder-authentic, matching the studio ribbon) and
popup_below_widget re-anchored &ch_resp -> &resp; content +
CloseOnClick + F-2 popup_child_dirs cache byte-identical.
(3) studio_view.rs ribbon: text ">" separators -> same chevron;
its right-click context_menu (sibling listing, F-2
sibling_menu_cache) untouched. Both path bars now share one
interaction language. ALL FIVE GATES GREEN: fmt; clippy
--all-targets -- -D warnings 5.91s; cargo test full suite (lib
69/69, gui_ascii_gate, columns 16, miller 10, pantone 6, icons
4, TUI 3, widget 8, CLI 9, all integration suites pass);
release 7.21s (first attempt blocked: pid 24384 held the exe
lock, instance killed cleanly, rebuild passed); porcelain
exactly 3 files. NOTE: the user's status-bar screenshot shows
old carets because it was captured from the pre-rebuild
binary; carets no longer exist in source. Live verification of
this wave + B-1/B-2/B-3 deferred per user rule (work first,
tool after everything completes). NEXT: user mega-directive
opened a new wave queue (sidebar Zen/Arc pin design, icon pack
upgrade, settings/config/Lua, Tools & Converters redesign,
fast folder sizes, status bar redesign, grid-view icon bugs);
5 research agents dispatched.

MOSSAMBI GRID PREVIEW WAVE CLOSED (2026-11-13): feat d26fa33 (feat(gui)
6 files +293/-66: explorer_view.rs +108/-64, codec/mod.rs +114,
tests/test_thumbs.rs NEW +56, studio_app.rs +13, config.rs +1/-1,
preferences_view.rs +1/-1) <- plan e41644e (docs(plan) 161 lines). ALL
FIVE GATES GREEN (fmt clean; clippy --all-targets -- -D warnings 11.52s
zero findings; cargo test full suite passed: test_thumbs 5/5 NEW,
remux 4/4, widget 8/8, TUI 3/3, pantone 7/7, doc-tests ok; release
56.03s; porcelain exactly 6 expected files). MECHANISMS: (1)
proportional tile chrome - icon_sz=(icon_size*zoom).clamp(64,300);
label_font=(icon_sz*0.15).clamp(12,22); size_font=(label_font-2).clamp
(9.5,12); label_zone_h=label_font*2.6; bottom_chrome=label_zone_h+
size_font+10; top_pad=(icon_sz*0.10).clamp(8,26); tile_h=top_pad+
icon_sz+bottom_chrome; tile_w=(icon_sz+32).max(96) (180 cap removed);
icon zone height is exactly icon_sz so the painted icon equals the
requested size; name top = max(rect.bottom - size_font - 10 - name_h,
zone_bottom + 2.0) so a wrapped 2-line name never touches the icon;
max_text_w=(tile_w-16).max(60). (2) universal previews -
is_image_extension removed; pub enum PreviewKind{None,Image,Pdf,Video}
+ pub fn preview_kind(ext) case-insensitive + pub fn thumb_max_px
(icon_sz)=(ceil).clamp(128,512); grid tiles ask thumb_max_px(icon_sz),
hover card 260, inspector and Miller 320; decode_thumbnail match-
dispatches (PDF -> render_pdf_first_page_thumbnail, video ->
video_first_frame_thumbnail, image Lanczos3 resize); audio/text/code
keep vector icons. (3) video first frames - codec::
video_first_frame_thumbnail: ffmpeg -frames:v 1, scale
force_original_aspect_ratio=decrease, image2pipe PNG to stdout;
VIDEO_THUMB_INFLIGHT AtomicBool single-flight with VideoThumbGuard
Drop reset; CREATE_NO_WINDOW|BELOW_NORMAL_PRIORITY_CLASS on Windows;
stderr drained on a thread; ffmpeg missing -> bail, tile degrades to
vector icon. (4) thumbnail cache - CachedThumb gains max_px: hit
requires mtime match AND stored max_px >= requested; texture id
thumb_{path}@{max_px}; decode budget 3 -> 6 decodes per 300ms window.
(5) icon range 64..=300 - toolbar + status-bar sliders and the
Preferences grid_icon_size slider all widened; config comment 64..300;
studio_app last_grid_icon_size mirror so Preferences changes propagate
to explorer_view.icon_size without fighting the live sliders.

MOSSAMBI STATUS BAR SEAMLESS WAVE CLOSED (2026-11-13): feat 5f0596a
(feat(gui) 2 files +240/-143: explorer_view.rs +230/-139,
studio_view.rs +10/-4) <- plan b1041b1 (docs(plan) 153 lines), prior
ledger 44fc148 (grid preview wave). ALL FIVE GATES GREEN (fmt clean;
clippy --all-targets -- -D warnings 29.29s zero findings; cargo test
full suite passed: icons 4/4, miller 10/10, pantone 6/6, pdf 5/5,
registry 7/7, remux 4/4, thumbs 5/5, tui 3/3, widget 8/8; release
1m09s; porcelain exactly 2 expected files). MECHANISMS: (1) D1 DOCKED
PANELS - bottom_bars_h reservation (28/56) and the
allocate_ui_with_layout wrapper DELETED; render() order:
TopBottomPanel::bottom("explorer_status_bar") FIRST (docks lowest),
then TopBottomPanel::bottom("explorer_path_bar") added only when
!is_home_view && !parse_breadcrumbs(current_dir).is_empty() (docks
above status; no panel = no space, so early returns can never leave
reservation gaps), then CentralPanel::default().show_inside for the
file view filling the exact remainder. (2) D2 FLUSH EDGES -
studio_view.rs CentralPanel inner_margin ZERO for
StudioTab::FileExplorer only (other tabs keep symmetric(10,6));
explorer self-padding: top bars wrapped in Frame Margin{10,10,6,0};
file-view panel Margin{left 10, right 0, top 4} so the scrollbar sits
flush with the window edge; bottom panels carry their own horizontal
margins; the drop banner carries its own 10px inset. (3) D3 FINDER
SEMANTICS - left text: selection REPLACES items ("N selected, X" via
format_file_size, accent + strong) else pluralize_items(total) + ",
X available" (drive free_bytes) + " (K hidden)" when hidden entries
are filtered; pipe separators REMOVED; drive display_name and total
size dropped (Finder shows free space only). (4) D4 ACTIVE-PANE
AWARENESS - use_secondary = split_view_enabled && active_pane == 1;
the shared status bar reflects the ACTIVE pane's items, hidden,
selection bytes, and dir for the drive lookup (secondary_entries /
secondary_selected_paths / secondary_dir). (5) D5 DRIVE REFRESH
HOOK - last_drives_refresh: Option<Instant> (Default None); at the
top of render(): None or elapsed > 5s -> self.drives =
get_system_drives(); keeps free space and mount/unmount current for
the status bar AND the sidebar drive list. (6) D6 PANEL CHROME - each
bottom panel: Frame::none() fill window_bg,
show_separator_line(false), plus a manual 1px hairline painted at the
panel's top edge via ui.painter().line_segment over the response.rect
(Stroke::new(1.0, palette.border)); status bar content bar_h 26; path
bar keeps its horizontal ScrollArea chevron row. (7) D7 SPLIT SIZING
- render_dual_pane_view renders its toolbar row FIRST, then pane_h =
ui.available_height().max(120.0) from the exact remainder
(pre-emptive -32.0 guess removed), so pane frames reach the bottom
edge precisely.

MOSSAMBI SIDEBAR ZEN/ARC WAVE CLOSED (2026-11-13): feat 4c8fe72
(feat(gui) 3 files +176/-105: config.rs +14/0, finder_chrome.rs
+90/-40, studio_view.rs +72/-65) <- plan 1cb167c (docs(plan) 146
lines), prior ledger 9d55bf7 (status-bar wave). ALL FIVE GATES GREEN
(fmt clean; clippy --all-targets -- -D warnings 9.24s zero findings;
cargo test full suite passed: icons 4/4, miller 10/10, pantone 6/6,
pdf 5/5, registry 7/7, remux 4/4, thumbs 5/5, tui 3/3, widget 8/8;
release 52.66s; porcelain exactly 3 expected files). MECHANISMS: (1)
D1 ZONE STACKING WITH IDENTITY BOUNDARY - the sidebar is now four
stacked zones, each introduced by NEW render_sidebar_zone_header
(finder_chrome.rs): a faint 1px hairline across the content width
above the header (Color32::from_rgba_unmultiplied border at alpha 90,
painter line_segment at cursor.top()+0.5), then the ASCII chevron
(v / >) plus the uppercase caption label with hover color shift;
zones FAVORITES (the Home row stays ABOVE all zones as app identity),
LOCATIONS (drive rows + OneDrive row; replaces the separate Drives
and Cloud sections), TAGS (gated by show_tags_section), and TOOLS &
CONVERTERS; the EMPTY Network section is DELETED; the separate
"Pinned Folders" header is DELETED (custom pins merge into the
Favorites zone inline, still gated by show_custom_pinned). (2) D2
PERSISTED COLLAPSE FLAGS - four new config keys
sidebar_favorites_expanded / sidebar_locations_expanded /
sidebar_tags_expanded / sidebar_tools_expanded (all serde
default_true, Default true); the five in-memory StudioView flags
(drives/cloud/network/tags/tools_expanded) DELETED; a zone toggle
mutates the loaded config and calls save() (log::warn on failure);
MossambiConfig::load() stays per-frame-cheap via the existing
CONFIG_CACHE mtime stamp (config.rs file_stamp check; save()
refreshes the stamp so the next load is consistent). (3) D3 REAL PIN
HIT-REGIONS - SidebarRowSpec.show_pin now draws an INTERACTIVE strip:
render_sidebar_row allocates the row with Sense::hover() then creates
two NON-overlapping interact rects (row = full rect minus the right
26px strip; pin = the right 26px strip, egui Ids from position) so a
click can never double-fire; it returns SidebarRowClick { row, pin }
(all 6 call sites migrated); static favorites now show_pin FALSE
(decorative pins removed - built-ins are not user-pinnable); custom
pinned rows keep show_pin true and a pin click UNPINS
(pinned_custom.retain + cfg.save, mirroring the explorer intent path,
logged the same way); the pin glyph recentered on the strip
(max.x - 13.0) with a three-tier color ramp (direct pin hover
text_primary, row hover text_secondary, idle text_muted). (4) Old
helpers render_sidebar_section_header and
render_sidebar_collapsible_header DELETED (grep-verified: every
caller migrated into the zone structure).

MOSSAMBI FOLDER SIZES WAVE CLOSED (2026-11-13): feat 00c302c
(feat(gui) 3 files +392/-14: fs_utils.rs +106/0, explorer_view.rs
+179/-14, tests/test_dir_sizes.rs NEW +107) <- plan d70d304
(docs(plan) 152 lines), prior ledger a6dcd82 (sidebar Zen/Arc wave).
ALL FIVE GATES GREEN (fmt clean; clippy --all-targets -- -D warnings
6.86s zero findings after one int_plus_one fix; cargo test full suite
passed + NEW test_dir_sizes 5/5: empty_dir_is_zero,
sums_two_files_exactly, disk_cache_roundtrip, depth_cap_limits_descent,
entry_cap_bounds_work [unix-only symlinks_are_skipped compiles out on
Windows]; release 48.89s; porcelain exactly 3 expected files).
MECHANISMS: (1) D1 WORKER MODEL - ExplorerView gains dir_sizes
HashMap<PathBuf,(Option<SystemTime>,u64)> (computed total + root mtime
at compute time = the freshness key), dir_size_queued HashSet dedupe,
dir_size_job_tx/dir_size_rx channel pair, dir_size_last_request
throttle stamp, dir_size_cache_dirty counter + dir_size_last_flush;
TWO persistent worker threads spawned in Default sharing one
Arc<Mutex<Receiver<PathBuf>>> - each blocks on recv() (the receiver
lock is released before computing so bursts parallelize; all senders
dropping ends the loop), captures the root mtime via std::fs::metadata,
computes, and sends DirSizeResult { path, mtime, total }; threads
named mossambi-dir-size-{0,1}, spawn failure logged via log::warn.
(2) D2 COMPUTE KERNEL - NEW pub compute_dir_size(root, max_depth,
max_entries) in fs_utils.rs: iterative BFS with an explicit stack (no
recursion overflow), depth cap 8, entry cap 100_000, NO symlink
descent (entry.file_type() is lstat-like and never resolves links or
junctions - is_symlink() skips both descent and link length
accounting); IO errors skip the offending entry so results are partial
sums (Finder reports partial totals the same way); DirSizeResult
struct added alongside. (3) D3 REQUEST SCOPING - throttled 200ms pass
inside poll_background_scans: candidates = primary selected folders
(path-sorted) first, then the primary visible listing's folders in
filtered_and_sorted_entries order, then (when split_view_enabled) the
secondary selection + secondary listing; skips already-queued and
already-fresh (stored mtime == FileItem.modified) candidates; caps 64
sends per tick; queued entries cleared as results drain. (4) D4
STALENESS - no notify crate exists and none was added: a stored size
is fresh iff item.modified == stored root mtime; changed folders
re-enqueue on the next throttle tick; DirWatcher rescans naturally
refresh mtimes; sort-by-size stays keyed on size_bytes (out of scope,
unchanged). (5) D5 DISPLAY - NEW dir_size_text helper: files keep
formatted_size(); directories show format_file_size(total) when a
fresh computed size exists else "--"; wired into the Details Size
match arm and the Grid tile caption (both previous "--" sites);
status-bar selection totals in BOTH branches (primary + secondary) now
add fresh folder totals (0 when unknown) instead of filtering dirs
out. (6) D6 DISK CACHE WRITE-BEHIND - ~/.mossambi/cache/dir_sizes.json
via serde_json (path keys as strings, mtime as epoch secs, 0 = None;
BTreeMap for stable output); load_dir_size_cache_from /
save_dir_size_cache_to take explicit paths (testable; missing or
corrupt files load empty); loaded once in Default; new results bump
dir_size_cache_dirty; flush when dirty >= 100 OR 30s elapsed since the
last flush (synchronous write on the UI thread at these sizes).

MOSSAMBI ICON PACK UPGRADE WAVE CLOSED (2026-11-13): feat 1d9e0e0
(feat(gui) 3 files +136/-22: icon_packs.rs +98/-18, finder_chrome.rs
+7/-3, tests/test_icons.rs +31/-1) <- plan 45d60ac (docs(plan) 132
lines), prior ledger 2d1efbb (folder-sizes wave). ALL FIVE GATES GREEN
(fmt clean; clippy --all-targets -- -D warnings 6.38s zero findings
first try; cargo test full suite passed + test_icons 7/7 incl. 3 NEW:
shade_hex_scales_channels, mix_hex_blends_channels,
finder_folder_svg_has_glass_anatomy; release 52.13s; porcelain exactly
3 expected files). MECHANISMS: (1) D1 GLASSY
THREE-LAYER FOLDER - folder_shapes rewritten per TabStyle with anchors
UNCHANGED: defs block emits <linearGradient id='ggfront' x1='0' y1='0'
x2='0' y2='1'> (vertical, objectBoundingBox) with stops offset 0 =
mix_hex(front,#FFFFFF,0.38) light source, 0.5 = front, 1 =
shade_hex(front,0.86) depth; layer 1 back wall rect in st.back, layer
2 front flap fill url(#ggfront) OPAQUE (glass sold by gradient +
specular, not translucency - rows/thumbnails behind can never wash
out), layer 3 specular top edge (rect inset 3 from front edges, h 2.2,
rx 1.1, white 0.35) + bottom inner-edge catch (h 1.5, rx 0.75, white
0.14) via NEW glass_layers(fx,fy,fw,fh); anchors: Flush back 8,20,84x16
+ front 8,30,84x52; Tab tab 8,12,44x16 rx=(r*0.7).max(3) + back
8,22,84x60 + front 8,34,84x48; Notch notch 18,13,40x10 rx=5 + back
8,20,84x60 + front 8,33,84x47; old flat white edge lines (0.18/0.10)
REMOVED - replaced by the specular; st.outline strokes retained on
both walls via paint-order; Tab/Notch tab rects unchanged; svg_for
assembly {head}{shapes}{gloss}{emblem} untouched so defs slot in
cleanly and emblems/paper-peek keep coordinates. (2) D2 COLOR HELPERS -
NEW pub mix_hex(a,b,t) per-channel linear blend + pub shade_hex(a,f)
per-channel scale (both round half-away-from-zero + clamp; malformed
#RRGGBB input falls back to the input unchanged) + private parse_hex
(strip #, len 6, ascii-hexdigit check, from_str_radix); NO PackStyle
field changes, no new config; derived stops keep all 14 packs
recognizable in their own hues. (3) GLOSS TUNE - gloss_overlay band
opacity 0.22 -> 0.12 so it reads as a sheen layer over the gradient
instead of a flat stripe. (4) D3 CHEVRON CRAFT - paint_path_chevron
slot 4.5x9.0 -> 5.5x10.0, stroke 1.5 -> 1.8, points inset 0.9
horizontally / 1.1 vertically from the slot edges (left_top+0.9,1.1 /
right_center-0.9,0 / left_bottom+0.9,-1.1, expressed as
rect-corner+Vec2 so no new imports) so strokes never clip and the
vertical center is preserved; both call sites (explorer_view.rs 3296,
studio_view.rs 672) absorb the +1.0 slot width via the item_spacing
3.0 horizontal layout. (5) D4 TESTS - test_icons.rs imports extended
to mix_hex/shade_hex/svg_for/IconGlyph/IconPack + 3 NEW tests:
shade_hex_scales_channels (#FFFFFF x0.5 -> #808080, x1.0 identity,
x0.0 -> #000000, malformed passthrough), mix_hex_blends_channels
(#FF0000+#FFFFFF t0.5 -> #FF8080, t0 -> a, t1 -> b, malformed
passthrough), finder_folder_svg_has_glass_anatomy
(svg_for(MacosFinder, Folder, [75,75,75]) contains <linearGradient,
ggfront id, url(#ggfront) front fill, specular 0.35, bottom catch
0.14). The proven snapped_icon_quad/icon_texture pipeline is untouched.

MOSSAMBI SETTINGS/CONFIG/LUA WAVE CLOSED (2026-11-13): feat ab8fab2 (feat(gui) settings/config/Lua wave - persistent conversion defaults, rebindable shortcuts, scripts manager + console, mlua engine; 9 files +1215/-395: Cargo.toml, Cargo.lock, src/core/config.rs, src/core/scripting.rs, src/gui/command_palette.rs, src/gui/preferences_view.rs, src/gui/quick_settings.rs, src/gui/studio_app.rs, src/gui/studio_view.rs) + plan 1eec768 (docs(plan) docs/superpowers/plans/2026-11-13-settings-lua-wave.md 104 lines). ALL FIVE GATES GREEN (fmt; clippy --all-targets -D warnings clean; test 73/73 lib + ASCII gate + all integration suites; release 1m 21s with first vendored lua54 C compile; porcelain exactly the 9 expected files). MLUA 0.10.5 API FACTS (lua54 + vendored, 8 crates locked): StdLib::SAFE_ALL does not exist; mlua's safe subset is StdLib::ALL_SAFE but REJECTED for the sandbox because it loads os and io; engine uses the explicit set COROUTINE|TABLE|STRING|UTF8|MATH; HookTriggers::every_nth_instruction (the with_ prefix is gone); Lua::set_hook returns () so no ?; Lua::new_with(StdLib, LuaOptions) unchanged. LUA SANDBOX: 16MB memory cap; instruction hook fires every 100k ops and aborts after 20 fires with RuntimeError "Too many operations"; captured print pushes tab-joined args into the logs buffer; same mossambi primitives as Rhai; sha256_of_path extracted as a shared fn; ScriptLang {Rhai,Lua} with as_str/from_str_opt/label; ScriptCommand {id,title,source ScriptSource::{Disk(PathBuf),Database},body,shortcut,lang}; ScriptConsoleEntry {title,outcome,time}. CONFIG: 10 conv_* persistent conversion defaults (image_quality, image_lossless, image_dpi, audio_bitrate, audio_strict_lossless, video_crf, video_preset, video_hw_accel, archive_compression, exam_mode_enabled); dead local_ai_* keys removed entirely (MossambiConfig has no deny_unknown_fields so old config.json still parses); shortcuts BTreeMap<String,String> ids palette/new_tab/close_tab with DEFAULT_SHORTCUTS Ctrl+P/Ctrl+T/Ctrl+W and MossambiConfig::shortcut_for falling back to defaults. UI: Preferences gains 11th category Scripts (index 9) before About (10); conversion rows dual-write BOTH quick settings and cfg.conv_* per the Finder System-Settings model (quick_settings::from_config seeds session state at startup; popover tweaks stay session-local); Shortcuts page rewritten on REAL data: rebind capture (click Rebind, next key press binds, Esc cancels; studio dispatch suppressed while capturing via prefs_view.capturing_action gate) + gesture card with six real cfg keys (gesture_type ComboBox shake/circle/both, shake_sensitivity_px slider 5..100, gesture_timeout_ms slider 150..1000, deep_drag_detection_only, enable_gesture_wakeup, enable_hotkey_wakeup); all four fake gesture cards and the exclusion_input field DELETED; key_binding_display free fn renders Event::Key as the stored display form. SCRIPTS PAGE render_scripts: manager card (disk scripts read-only with path caption; DB scripts Run/Edit/Delete via state_db delete_command; load_all_user_scripts each frame), editor card (title TextEdit + Rhai/Lua selectable pills + multiline code_editor; New clears; Save gated on non-empty title+body -> state_db::save_command CustomCommandRecord id cmd-{nanos:x} script_lang as_str shortcut None), console card (ScriptConsole mpsc drain cap 50, chrono timestamps, newest-first, OK/FAILED + returned value + error + logs, Clear). PALETTE: PaletteAction::RunUserScript gains lang: ScriptLang; studio handler clones console.sender() and sends (title, outcome) into the console; search_preferences adds 2 static Scripts hits (setting_label is &'static str so dynamic script titles cannot be entries). TESTS: 4 new Lua tests (test_script_lang_roundtrip, test_lua_sandboxed_math_and_print, test_lua_infinite_loop_is_bounded, test_lua_sandbox_denies_unsafe_libs extended to assert io denial alongside os).

## 2026-11-14 - W-7 Batch A: Conversion Queue Core (feat 43edeb5)
- Plan: docs(plan) 118dee8 (tools/converters redesign wave).
- Scope: per-item conversion queue with live per-item progress,
  cooperative two-tier cancel (stdin 'q', 2s grace, hard kill),
  retry/clear/reveal actions, ffmpeg `-progress pipe:1` parsing, ffprobe
  duration probing, multi-hop route cancellation, watchdog-stall accounting.
- Files (9): src/core/conversion_queue.rs (new, 311 lines), src/core/mod.rs,
  src/core/async_worker.rs, src/codec/mod.rs, src/gui/studio_app.rs,
  src/gui/studio_view.rs, src/tui/mod.rs, src/widget/widget_app.rs,
  tests/test_queue.rs (new, 220 lines).
- Gates: fmt OK; clippy --all-targets -D warnings OK (fixed 7 issues:
  staged tuple arity E0308, missing id field E0027, id_source->id_salt
  deprecation, summary move E0382, type annotation E0282, 2x
  type_complexity via ProgressCallback alias, new_without_default via
  Default impl); cargo test OK (all suites green incl. new 12-test
  test_queue); cargo build --release OK (0.64s, exit 0); tree: staged 9
  expected files only.
- Notes: ffmpeg out_time_us AND out_time_ms are both microseconds; parser
  tests placed in tests/test_queue.rs (not test_codec.rs); PROGRESS.md was
  modified concurrently by the cron session mid-wave and excluded from
  staging (cron owns it); stray untracked package-lock.json remains
  excluded per standing decision; egui renamed id_source to id_salt in the
  vendored version (contradicts older notes); Batch B next (async exam,
  inspector/bench on plain threads, telemetry cache + real thread count,
  rfd dialogs on background thread).
- HEAD after feat: 43edeb5. Ledger hash recorded in Honcho (not here).

## 2026-11-14 - W-7 Batch B: Async Tool Ops (feat 58b5681)
- Plan: docs(plan) 118dee8 (tools/converters redesign wave).
- Scope: UI-freezing synchronous tool calls moved off the render thread.
- Files (6): src/core/tool_console.rs (new, 101 lines), src/core/mod.rs,
  src/core/conversion_queue.rs, src/exam/mod.rs, src/gui/studio_app.rs,
  src/gui/studio_view.rs.
- Gates: fmt OK; clippy --all-targets -D warnings OK (fixed 2 issues: 2x
  collapsible_if in bench/inspect buttons - click conditions merged with
  &&); cargo test OK (all suites green incl. 12-test test_queue and 73
  lib tests); cargo build --release OK (50.23s, exit 0); tree: staged 6
  expected files only.
- Notes: new ToolConsole/ToolEvent (ScriptConsole pattern, plain threads,
  boxed report payloads keep the enum small); exam pipeline gained ctx
  variants (cancel + progress checkpoints: entry bail, 0.05 decode,
  0.10 decoded, per-pass 0.15-0.75, 0.95 write, 1.0 done) with
  None-wrappers keeping tui/main callers working; exam jobs flow through
  the conversion queue via QueueAction::RunExam -> start_conversion_batch
  ("EXAM") with ItemResult mirroring to exam_status/last_exam_output;
  exam output renamed {stem}_conformed.jpg; hardware telemetry detected
  once into a StudioView cache (one-shot trigger at render top, Refresh
  row, "Detecting..." early return); "16 Threads Ready" replaced with
  real optimal_thread_count; rfd file picker moved to a background
  thread (picker_sender/receiver/in_flight, drained per frame).
- HEAD after feat: 58b5681. Ledger hash recorded in Honcho (not here).

## 2026-11-14 - W-7 Batch C: Tools/Converters UX Polish (feat 636c371)
- Plan: docs(plan) 118dee8 (tools/converters redesign wave).
- Scope: real format catalogue, honest archive labels, per-tab drop
  zones, route chips, monospace numerals.
- Files (2): src/gui/studio_view.rs, src/gui/studio_app.rs.
- Gates: fmt OK; clippy --all-targets -D warnings OK first pass; cargo
  test OK (73 lib + 12 queue + all suites, exit 0); cargo build
  --release OK (48.63s, exit 0); tree: staged 2 expected files only.
- Notes: catalogue chips replaced the hardcoded 24-format sample list
  with UniversalRegistry::search_catalogue (200+ formats, uppercase
  contains filter over extension/display_name/container/description,
  48-chip cap with a "+ N more formats - refine the filter" hint); the
  fake "Pack to 7Z" button was replaced with real TAR.ZST
  (ArchiveType::from_extension has no 7Z arm, so the dispatcher's
  unwrap_or(Zip) silently packed a ZIP named *.7Z; TAR.ZST maps to the
  real TarZst writer); new render_tab_drop_banner helper shows an
  accent hover banner on the Transcoder/ExamStudio/QualityInspector/
  ArchiveSuite tabs; drops are now tab-aware (ArchiveSuite packs ZIP
  or safe-extracts via a looks_like_archive extension check,
  QualityInspector runs background inspect with a busy guard,
  ExamStudio pushes QueueAction::RunExam); enqueue stamps Dijkstra
  route chips via a process-wide OnceLock RoutePlanner
  (new_with_standard_matrix, find_route 3 hops,
  ConversionRoute::display_path -> set_planned_route); the queue
  summary counts line is monospace.
- HEAD after feat: 636c371. Ledger hash recorded in Honcho (not here).

## 2026-11-14 - Directives Wave: Chooser Accent + Chip Budget + Zoom Ladder (feat cbe33ce)
- Plan: docs(plan) 7fb3072 (directives wave - accent chooser, chip overlap, zoom ladder).
- Scope: three live-round directives in one wave - column chooser accent
  highlight (Image 1 looked bad; accent-colour highlight method), toolbar
  selection-chip overlap fix (chip painted over Delete), Ctrl+Scroll
  view-mode zoom ladder (four user-confirmed decisions).
- Files (4): docs/superpowers/plans/2026-11-14-directives-wave.md (new),
  src/gui/file_menu.rs, src/gui/explorer_view.rs, src/gui/studio_app.rs.
- Gates: fmt OK; clippy --all-targets -D warnings OK (first pass exit 101,
  fixed 5: Rect min/max are fields not methods (2, chip-budget code),
  Stroke::new width literals need f32 suffix (3, menu_item_toggle));
  cargo test OK (exit 0, re-run after the zoom edits); cargo build
  --release OK (52.98s, exit 0); ASCII 0 in src/gui; tree: feat staged 3
  expected files only after docs(plan) staged 1.
- Notes: menu_item_toggle carries chooser state by accent colour, not
  [x]/[ ] text glyphs - shown rows paint the accent selection fill (45
  alpha dark / 26 light, hover 65/42) with a painted check square +
  2-segment check, hidden rows flat + outlined empty square, locked rows
  pinned non-interactive chip, text_primary over fills / text_muted flat
  (2026-11-10 legibility rule), 3px row gaps, default arrow cursor; the
  toolbar RTL group has no width arbitration vs the left pills, so the
  chip budget = clear_left - buttons_right - 12 with elide "N selected |
  size" -> "N selected" -> hide (status bar already reports the same);
  zoom ladder per user decisions: five multiplier levels [0.60, 0.85,
  1.0, 1.30, 1.70] kept, one notch below the smallest grid size crosses
  into Details and one notch up from Details returns to the smallest
  grid size, Columns excluded, view-mode auto-switch session-only while
  explorer_zoom stays persisted in config.json; zoom_ladder_step +
  zoom_cross_gate (ZOOM_CROSS_DEBOUNCE 150ms) guard touchpad wiggle at
  the boundary; level 0 relabelled honestly "Extra small icons" (the
  old "Details" label never switched modes); studio_app Ctrl+wheel
  handler delegated to zoom_ladder_step, Ctrl+0 reset and
  Ctrl+Shift+wheel UI scale unchanged, command palette keeps owning the
  wheel while open (B-1); stray untracked package-lock.json deleted per
  user decision (no package.json exists anywhere).
- HEAD after feat: cbe33ce. Ledger hash recorded in Honcho (not here).