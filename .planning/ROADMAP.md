# ROADMAP.md — Explorer Parity Milestone

## Phase 1 — Visual foundation (icons, fonts, zoom) — IN PROGRESS
- [x] Sidebar: vector icons 54px, 70px rows, proportional(19.5) labels, no highlight, vector pin, pack passed as arg.
- [x] Details/Grid: no hover/selection bands; recent-files rows clean; thumbnail placeholder fill gone.
- [x] Drive + pinned folders: bare tiles, icons 76/104, labels +4.
- [x] Ctrl+wheel ramp: level 0 snaps Details, 1–4 Grid icon sizes w/ growing labels, wheel directions correct.
- [ ] App-wide font sweep: every `FontId::proportional(N)` → N+4 (studio_view, explorer_view, macos_theme, widget_app, citrus_geometry, quick_settings, popover, canvas/status/columns painters).
- [ ] Icon ×2 sweep remaining files: citrus_geometry.rs, widget_app.rs, quick_settings.rs, popover.rs + verify explorer surfaces meet 2× (drives/dashboards/peek/properties/columns/tab-header/status-bar).
- [ ] Settings cards polish: icons/fonts previews, sectioning, no wall of text.

## Phase 2 — Interaction parity — IN PROGRESS
- [x] Checkbox modes Always/Hover/Off with animated reveal.
- [x] Context menus (Win/Finder/Linux styles) across details rows, grid tiles, canvas.
- [x] vsync:false + continuous repaint + `max_fps` config + toggle in Performance card.
- [ ] Wire `cfg.animations` into widget + toolbar/peek hover springs.
- [ ] Mouse-nav (X1/X2/swipe) runtime verification with screenshots.

## Phase 3 — Widget unification
- [ ] `apply_font_face` in widget, active IconPack for widget glyphs.
- [ ] Replace all widget emoji (☀️/🌙/🚀/🖥️/❌" context menu, ⭐/🔍/✓/▶/🍋 hub in citrus) with `paint_icon_with` vector glyphs.
- [ ] `std::env::current_exe()` for Open Studio spawn (fix hardcoded "mossambi.exe").
- [ ] Reuse `file_menu` visual language (menu_header/menu_item) in widget menu.
- [ ] Widget anims honor `AnimationSettings`.

## Phase 4 — Performance & threading
- [ ] Worker pool honoring `worker_threads` (0 = available_parallelism) for dir scans, hashing, image decode prewarm.
- [ ] Icon prewarm off UI thread.
- [ ] O(n)/O(log n): sorted-index search reuse, no quadratic re-sorts per frame; CONFIG_CACHE already in place.

## Phase 5 — Converter engine integration into GUI
- [ ] Context menu "Convert…" routes selected file(s) into Transcoder tab queue.
- [ ] Batch actions: multi-select → transcode pipeline, one-click ZIP via archive module.
- [ ] Progress: background threads + status/progress UI, checksums from quality gates.
- [ ] Widget file-drop conversions reuse the same pipeline.

## Phase 6 — Rigorous QA + polish + verification
- [ ] `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo build --release`.
- [ ] Computer-use QA: dashboard/details/grid(zoom 0–4)/columns/peek/properties/settings cards/sidebar packs/context menus per file type/checkbox hover reveal/zoom steps/mouse side buttons/widget — screenshot at hover AND click at every step.
- [ ] impeccable audit: alignment, padding, color, contrast, overlay issues — fix until clean.
- [ ] Verifier subagent gap report vs. instruction list; final commit.

## Phase 7 — Drag & Drop (Explorer/Finder/Nautilus parity) — NEW (researched)
- [x] `FileDragPayload { paths, }` + `Sense::click_and_drag()` on details rows (~2313), grid tiles (~3237), columns rows (~3582/3728); drag from a selected item drags the whole selection (Explorer rule).
- [x] Drop targets with animated highlight (hover_anims spring): folder rows/tiles/columns → drop into dir; canvas → current_dir; breadcrumb segments → ancestor dirs.
- [x] Move vs copy semantics: Ctrl/Cmd-drag = copy; plain cross-dir drop = move; drop onto own ancestor/no-op dirs rejected; `+` badge follows pointer during copy.
- [x] Ops via fs_utils (`move_entry`/`copy_entry_recursive` + `resolve_unique_paste_path`) with `log_result` + refresh of affected dirs.
- [x] Marquee rubber-band select on canvas empty-space drag (live union into selected_paths).
- [ ] OS clipboard priming on drag start (CF_HDROP copy effect) + restore-on-abort/intra-commit. — DEFERRED: intra-app egui drag ≠ OLE drag; priming would clobber the user clipboard every frame-drag. Cross-app transfer covered by copy/paste HDROP sync + new Phase 11 Drop Overlay.
- [x] Grid parity fixes while there: symlink " ↗" suffix + hidden-file alpha (details have them, grid doesn't).

## Phase 8 — Context-menu completion (full big_session coverage) — NEW (audited)
- [x] Intents contract enforced: move inline-bound verbs (play/print/defender/wallpaper/rotate/shortcut) into `ItemMenuIntents` fields, applied by caller.
- [x] New rows: Open With (`open_with_choose`, zero callsites today), Duplicate (sibling "name copy"), Send To submenu (Windows SendTo folder enumeration; hidden elsewhere), copy-path variants submenu (name / full / quoted / forward-slash / parent).
- [x] Canvas menu: Sort by submenu (Name/Kind/Size/Date, click-again reverses), View submenu (Details/Grid/Columns), Invert selection.
- [x] Multi-select semantics: right-click a selected item operates on ALL selected (delete/cut/copy/pack/duplicate/inspect/transcode take Vec; header "N items").
- [x] Delete to Recycle Bin: Windows SHFileOperation FOF_ALLOWUNDO; Linux `gio trash` probe; permanent remove as logged fallback (labels already say "Move to Trash" on Finder/Linux).
- [x] Renderer hygiene: OpenWith/SortBy/ViewMode bound. (Compress label dedupe → Phase 10.)

## Phase 9 — Converter resilience & threading (CLI + GUI + widget + TUI) — DONE (7305db6, 79987f8)
- [x] AsyncWorkerPool: AtomicUsize counter, progress-aware watchdog (30 s WITHOUT progress), spawn-failure logging.
- [x] Studio: pool-driven conversions, double-start guard, full channel drain, start_conversion_batch(Vec<PathBuf>), log+status surfacing.
- [x] Widget: failure surfaced in hub (6 s auto-clear) + log; env_logger at widget/TUI/CLI entrypoints; cancel token checked between TUI files (full codec cancel hook = TODO deferral).
- [x] Codec: central atomic .part→rename (+ stale-scratch purge, sanity len>64), honest bail! for DOCX/PDF/spreadsheet/presentation garbage paths, ffmpeg stderr capture (last 2000 chars) + explicit -f muxers, subtitle regex fallbacks, bench TempDirGuard, logging with durations.
- [x] Config: load() write side-effect removed, corrupt-config backup, worker_threads clamp ≤64, knobs wired (init_multi_threading, vsync/max_fps, background_work gate).
- [ ] Explorer: blocking work off-frame (unpack_archive :2579, rotate :5133, inspect studio_view :871) — carried into Phase 14 (async UI ops).
- [x] ffmpeg stderr in error strings (preflight check folded into run_ffmpeg failure paths).

## Phase 10 — Apple-HIG right-click redesign + type-aware Convert submenu (left-column parity) — NEW (2026-11-04)
- [ ] Apple menu principles on Finder style: text-only rows (no glyph chips), 15 px type, accent-hover rounded rows, group separators, Title Case.
- [ ] Item menu regrouped into HIG sections: Open/Edit · Convert & Tools · Share (Send to) · Clipboard · Danger · Info.
- [ ] Type-aware Convert submenu via `UniversalRegistry::get_sectors_for_file` (same engine as radial widget): image→PNG/JPG/WEBP/TIFF/BMP/PDF, video→MP4/MKV/WEBM, audio→FLAC/WAV/MP3/AAC/OGG, docs→PDF/DOCX/TXT/HTML/MD/EPUB, sheets→XLSX/CSV/TSV/PDF/ODS, slides→PDF/PPTX, archives→ZIP/EXTRACT, subtitles→SRT/VTT/ASS, + "All formats (batch)"; filter the input's own ext out except zero-loss rebuild.
- [ ] Left-column parity: Custom Conformer, Quality Inspector, Archive Suite (pack/extract), Transcoder — all reachable from right-click.
- [ ] Multi-select batch wiring: studio_view accumulates Vec<(PathBuf,String)> → start_conversion_batch (fixes "only last file converts"); Compress label de-duped here.

## Phase 11 — Gesture gating + global drop-to-convert overlay — NEW (2026-11-04)
- [ ] DragGestureListener: arm ONLY when a file sits under the press (CF_HDROP clipboard / Explorer COM / active-file probe); never emit empty-file events; kill stale-clipboard false positives (browser text drags, window moves).
- [ ] Windows OLE Drop Overlay in daemon: always-on-top lime puck, RegisterDragDrop + hand-rolled IDropTarget vtable; files dropped from ANY app (Explorer, browsers, editors, other managers) convert via pool with type-aware default (AUTO per category); tray balloon + logs on completion.
- [ ] Config: drop_target_default ("AUTO"|fmt); single-instance named-mutex for `daemon run`.
- [ ] Linux `daemon run`: real resident loop (no busy spin, graceful shutdown) for backend-service parity.

## Phase 12 — Default file manager takeover — NEW (2026-11-04)
- [ ] src/core/default_explorer.rs take_over/restore/status: HKCU Software\Classes\Directory|Drive\shell (open-with-Mossambi default verb + command "%V"), Directory.Background context verb; Linux .desktop + xdg-mime default inode/directory; macOS = instructions logged (Finder not replaceable).
- [ ] CLI: mossambi daemon takeover|release|default-status. GUI: Preferences buttons + daemon start/stop toggle.
- [ ] Cargo: Win32_System_Registry feature.

## Phase 13 — Hygiene debt — carried
- [ ] AtomicUsize joint swap: fs_utils spawn_async_dir_scan + explorer scan_gen + TUI scan_gen (32-bit RMW safety).
- [ ] Warning-free build (unused imports: mouse_nav, tray, drag_listener, context_menu).
- [ ] Gates: fmt, clippy -D warnings, test, Linux+Windows release, cross-target checks.

## Phase 14 — UI OVERHAUL (Finder redesign wave) — ACTIVE (wave plan in STATE.md)
- [x] config: show_tags_section / show_custom_pinned / pinned_custom (default-off).
- [x] src/core/tags.rs: NTFS ADS tags (`file:tags`), parse/join helpers tested, non-Windows no-op.
- [x] src/core/action_journal.rs: typed journal, JSONL log, undo_last; wired into delete/rename/
      move/copy/new-folder/tag-set/keystrokes.
- [x] studio_view: traffic lights left of tabs w/ real hover glyphs, fluent controls off the
      explorer title row, sidebar standard-pins only, tags/custom pins behind config, ellipsis.
- [x] explorer_view: Finder toolbar (back/fwd, title, view segmented, sort, filter popover,
      action overflow, search), bottom path bar, tag dots, date format, "1 item", junction
      filtering, tile label elision, solid bg.
- [x] render_columns_view rebuilt: multi-column stack, chevron subfolder column, inspector card.
- [x] Right-click completion: Pin to Sidebar (folders), Tags submenu, journal hooks.
- [x] Gates green 2026-11-07 (check, clippy -D warnings, 31 tests, fmt, release). Desktop
      qa_dnd_scratch deleted.
- [x] Command Palette (src/gui/command_palette.rs) 28 actions wired to Ctrl+P / Cmd+K.
- [x] Pure ASCII discipline enforced across all GUI and theme files.

## Phase 15 — Cross-Platform Universal Shell, Miller Inspector & 4-Surface Convergence — DONE (85097b6)
- [x] Wave 1: Miller Columns Right-Side Preview Inspector (media scrubber, EXIF, dimensions, quick actions) + Dual-Pane Split View (F3). (2ccd6ce)
- [x] Wave 2: Centralized SQLite Database (%LOCALAPPDATA%/mossambi/mossambi_state.db) for Tags, Keystrokes, and Multi-Session Undo Journal + Shift-Delete prompt. (fd5b109)
- [x] Wave 3: Dedicated STA Background Thread for Windows OLE DoDragDrop (60 FPS non-blocking UI during external drag) + universal edge drop targets. (7c337e4)
- [x] Wave 4: Universal Exam Conformer & Generic Parameter Studio (remove institution names, generic sliders px/cm/mm/DPI/KB + SQLite custom presets + real-time pre-scan). (40e5bcf)
- [x] Wave 5: Sandboxed Rhai Scripting Engine for Command Palette extensions and workflow pipelines. (41e0b4e)
- [x] Wave 6: Cross-platform macOS AppKit & Linux FreeDesktop portal bridges. (82647c1)
- [x] Wave 7: Per-Monitor V2 DPI awareness, Windows 11 rounded corners, pure-ASCII verification gate (`tests/gui_ascii_gate.rs`). (85097b6)
- [x] Full Quality Gates Verified: `cargo check --lib`, `cargo clippy --all-targets -- -D warnings`, `cargo test --tests -j 2`, and `cargo build --release` producing release binary.

## Phase 16 — Full Shell Replacement, Local AI Engine, Installer TUIs & Multi-Platform Packaging — IN PROGRESS (39322b1 2026-11-07 23:14 + live session). Cron evidence 23:31, on-disk (part uncommitted): batch renamer modal with Replace Text / Add Prefix-Suffix / Format with Numbers modes (no Date mode found yet), visa presets US 2x2 / Schengen 35x45 / Canada PR 50x70 (no UK found), default_explorer take_over/restore + Preferences takeover & local-AI (Ollama/LM Studio) rows; mossambi_shell_restore.reg backup and Win+E hijack NOT found.
- [ ] Windows Shell Takeover: Hijack Win+E and default folder associations with automated registry backup (`mossambi_shell_restore.reg`) and single-click restore in Preferences.
- [ ] Installer & Uninstaller Suite:
  - Interactive Installer TUI and Uninstaller TUI.
  - Multi-platform packaging: Windows `.msi` / Winget / Scoop, macOS `.dmg` / Homebrew, Linux `.deb` / `.rpm` / Flatpak.
  - Clean uninstaller logic that releases all associations, shell hooks, and shortcuts.
- [ ] Local AI & OCR Engine:
  - Integration with Ollama and LM Studio via configurable HTTP API endpoints (model, temperature, effort in Settings).
  - Recommended local models catalog: Qwen2.5-VL / Llama-3.2-Vision for scan cleanup/OCR, Qwen2.5-Coder for scripts.
- [ ] Global Search & Indexing Engine:
  - Fast hybrid search: Directory filter + SQLite FTS5 index + background USN Journal (Win) / Spotlight mdfind (macOS).
- [ ] Finder-Style Batch Renamer:
  - Multi-file modal with Text Replacement, Prefix/Suffix, Numbering Sequences (001, 002), and Date formatting.
- [ ] Dual Keybinding Profiles:
  - Toggle between macOS Finder Mode (`Enter` to rename, `Cmd+Down` to open) and Windows Explorer Mode (`F2` to rename, `Enter` to open).
- [ ] Expanded Visa & Custom Tagging Engine:
  - Global visa presets (US Visa 2x2, Schengen, UK, Canada PR) and persistent user-defined color tags.

## Phase 17 — Filee Convergence, 255-Format Engine, and Apple HIG Polish — LANDED (2713f93 2026-11-08 01:23 IST + codec-expansion follow-on in 59dfac9 02:10). Commit-message scope: filee parity, 255-format engine, two-sided donut flip, crop canvas, webdrop, Apple HIG polish. Cron note: gates GREEN 02:33-02:40 IST on head 8e33262 (release current 02:31:46; 66 tests / 0 failed); wave-7 OCU/visual QA unevidenced from cron.
- [ ] Wave 1: Two-Sided Radial Donut with 3D Flip (<kbd>Tab</kbd> / Mouse Wheel between Primary Formats and Studio Power Tools: Crop, Exam Conformer, Metadata, Rename, AI/OCR, PDF Tools).
- [ ] Wave 2: Interactive Pre-Conversion Crop Slice (`CropCanvas` with 8 drag handles, aspect ratio dials 1:1, 4:3, 16:9, Free, and rule-of-thirds fading guides).
- [ ] Wave 3: Browser Drag-Out & WebDrop Integration (`text/uri-list`, browser virtual files, HTML `<img>` parsing, auto-staging downloads).
- [ ] Wave 4: Comprehensive 255-Format Engine Expansion across 18 categories (`filee.sh/formats`) and Dijkstra multi-hop conversion routing (`RoutePlanner`).
- [ ] Wave 5: Linux X11 and Wayland Desktop Integration Suite (forced X11 backend on Wayland with XWayland, GNOME pointer socket bridge, KDE KWin script, Hyprland IPC, FreeDesktop portals).
- [ ] Wave 6: Apple Human Interface Guidelines (Apple HIG) Redesign & Defect Elimination (status bar "1 item" grammar, toolbar label overlap fix, crisp Miller column preview scaling, clean acrylic materials).
- [ ] Wave 7: Open-Computer-Use Desktop QA, Security Hardening (`cargo audit`, `cargo deny`), and Knowledge Graph Sync (`/gsd-graphify`).

## Phase 18 — Pure-Rust Filee Engine Suite, 3-Column Donut Toolbar Customizer, Unified Preferences — LANDED (59dfac9 2026-11-08 02:10 IST + ef1042c viewport visibility 02:25 + 8e33262 apple hig switch tracks/donut palette 02:32). Commit-message scope only; wave breakdown owned by the interactive session. Cron note: all five gates GREEN 02:33-02:40 IST on head 8e33262 (66 tests / 0 failed / 0 ignored, release current 02:31:46, --version 0). Visual QA for these surfaces still open (interactive-only).

## Phase 19 — Pixel-Perfect Sweet Lime Radial Widget, Group Fan-Out & Unlimited Animation Engine — LANDED (59d723d 2026-11-08 10:27 IST). Commit-message scope only; wave breakdown owned by the interactive session. Cron note: all five gates GREEN 13:52 IST on head 7725a58 (67 tests / 0 failed / 0 ignored, release current 13:31:23, --version 0); surfaces (sweet-lime widget revamp, group fan-out, animation engine preferences) have had NO visual QA - interactive-only.

## Phase 20 — Multi-Window Explorer Support, Real-Time Dir Watcher Sync & New-Window Shortcuts — LANDED (94b76d2 2026-11-08 12:21 IST + 7725a58 dir-watcher startup-deadlock fix 13:34). Commit-message scope only. NEW src/core/dir_watcher.rs (249 lines; final form uses non-blocking FindFirstChangeNotificationW after 7725a58 replaced blocking ReadDirectoryChangesW). Cron note: all five gates GREEN 13:52 IST on head 7725a58; surfaces (second-window explorer, watcher-driven sync, tray/file-menu new-window entries) have had NO visual QA - interactive-only.

## Phase 21 — PDF-to-Image Rasterization, Komorebi/DWM Visibility & TUI Resilience — LANDED (ce12bfc 2026-11-10 12:50 IST: pdf-image rasterizer, glass/window preference cards, theme-chrome polish; gates re-GREEN 23:13/01:52/02:33/03:35 Nov 9 and 23:24-23:55 Nov 10 on the wave content before commit)
- On disk, NOT committed (cron diff-stat 22:36, 14 files 308+/129-): NEW
  src/codec/pdf_image.rs (pure-Rust PDF -> JPG/PNG/WEBP/TIFF/BMP multipage renderer
  via lopdf/resvg/tiny-skia/image; fixes PDF->JPG routing into the document-text
  fallback that wrote raw text bytes into .jpg) + src/codec/mod.rs wiring +
  Cargo.toml deps; dwm_helper (DwmExtendFrameIntoClientArea removed), tray
  (WS_EX_TOOLWINDOW/WS_POPUP message window for Komorebi), tui TerminalGuard +
  Ctrl+C/Ctrl+Q, fs_utils DRIVE_CDROM/SEM_FAILCRITICALERRORS, mouse_nav,
  drag_listener, registry, route_planner, explorer_view, studio_app,
  preferences_view; komorebi.json float_rules per the 19:42 PROGRESS summary.
- Session claims (19:42 summary): 69 tests pass, clippy clean, release synced to
  .cargo\bin + scoop. NOT independently verified from cron; release linked
  20:47:35 postdates the summary.
- mossambi.exe LIVE (PID 8272 at 22:36). Gate cutoff 13:31:23 does NOT cover this
  wave; next quiet run reruns the five gates once (PROGRESS.md decision tree
  case 2).
- Cron note 23:13 IST (Nov 8): mossambi.exe gone, quiet; ALL FIVE GATES GREEN on
  this uncommitted content with NO fmt needed - fmt --check OK, check --lib
  clean, clippy -D warnings clean, test --lib 70 passed / 0 failed / 0 ignored
  (incl. test_winrt_pdf_render + test_gate_pdf_multipage_jpeg_rendering),
  release current by cargo no-op (binary 20:47:35 = the session's own link),
  `--version` exits 0. The 19:42 session claims (69 tests, clippy clean) are
  independently CONFIRMED (70 tests). Still session-owned and UNCOMMITTED (cron
  never git-adds src). Visual QA for PDF multipage split output and the
  Komorebi/DWM/TUI fixes remains interactive-only.
- Cron note 00:22 IST (Nov 9): content UNCHANGED since 23:13 (zero src/test/
  Cargo mtimes newer than 23:14, newest src save 20:46:18, no processes,
  release binary still 20:47:35) - the 23:13 GREEN verification stands on
  byte-identical content; NO rerun needed. The 22:36 "next quiet run reruns"
  expectation above is SATISFIED by the 23:13 run.
- Cron note 00:52 IST (Nov 9): wave GREW to 20 porcelain paths (19 modified src
  + Cargo.toml + untracked pdf_image.rs, 673+/385-) with a NEW theme/chrome
  polish batch saved 00:42-00:50 (pantone, design, finder_chrome, macos_theme,
  popover, studio_view) while mossambi.exe stays LIVE (PID 8272). The 23:13
  GREEN verification is STALE for the 00:42-00:50 saves; session active ->
  cron stood down (case 1). Next quiet run reruns the five gates once (case 2).
- Cron note 01:52 IST (Nov 9): the wave grew again (session saved through
  01:32:56, release relinked 01:34:23, then quiet) - now 22 paths: 21 modified
  (1602+/930-, incl. Cargo.toml) + untracked pdf_image.rs at head 722c466.
  Quiet 10 min at the 01:42 scan -> ALL FIVE GATES RERUN GREEN with NO fmt
  needed: fmt --check OK, check --lib clean, clippy -D warnings clean,
  test --lib 70 passed / 0 failed / 0 ignored, release current by cargo no-op
  (binary 01:34:23 = the session's own link), `--version` exits 0. Gate cutoff
  01:34:23 IST covers the whole 00:42-01:32 batch. Still session-owned and
  UNCOMMITTED. The 23:13 expectation above is satisfied by the 01:52 run.
- Cron note 02:33 IST (Nov 9): five gates rerun GREEN on the 00:42-02:23
  batch (22 paths, head 8b72e90) after cron fmt'd fs_utils/explorer_view
  (left uncommitted); release relinked 02:35:42; cutoff 02:35:42 IST.
- Cron note 03:35 IST (Nov 9): after the 02:51-03:20 active-edit stretch
  (saves through 03:29:44, two app relaunches in between) the session went
  quiet and the 03:33-03:35 run reran the five gates GREEN on the grown
  23-path wave: fmt --check OK (NO fmt needed), check/clippy clean, test
  --lib 66 passed / 0 failed / 0 ignored, release current by cargo no-op
  (binary 03:30:45 = the session's own link), --version exits 0. Cutoff
  03:30:45 IST covers the whole 02:46-03:29 batch (supersedes 02:35:42).
  Suite count 70 -> 66 explained by the session's own edits (one src test
  moved to tests/test_pdf.rs as test_pdf_to_jpg_transcode). Still
  session-owned and UNCOMMITTED.
  - Cron note 00:26 IST (Nov 10): gate attempt INTERRUPTED - the session saved
  preferences_view.rs 00:30:45 mid-run (live Glass-effect settings-row
  refactor). clippy -D warnings RED on that in-flight content: 3 unused vars
  (preferences_view.rs 1743 glass, 1805 ms, 1848 cm) - not a stable failure.
  Cron stood down (no test/release, no src touches); fmt-only edits left
  uncommitted. Cutoff still 03:30:45 IST, stale for the 03:40-00:30 batch;
  next quiet run reruns all five gates once.
- Cron note 01:16 IST (Nov 10): gate attempt COMPLETED on resting content
  (session quiet 45+ min). fmt --check OK; check --lib exit 0; clippy
  -D warnings RED exit 101 - the same 3 unused variables in
  preferences_view.rs (1743 glass, 1805 ms, 1848 cm), now a STABLE blocker
  left by the session's unfinished Glass-effect refactor; test --lib 66
  passed / 0 failed / 0 ignored (suite healthy); release left stale
  (23:24:57, not relinked on RED content). Cron cannot fix src
  (session-owned). Cutoff stays 03:30:45 IST (Nov 9). Rerun only when src
  content changes after this attempt.

## Audit wave — cron notes (session-driven; live-proof record in agent.md/claude.md/gemini.md)

- 00:15 IST Nov 12 (cron): since the 23:47 Nov 10 gate run the session landed
  15 commits (95cc29a..00acb34) executing the audit wave with live proofs -
  Miller-columns right-click menus (359bc9a/195988c), Open With submenu with
  registry enumeration (95cc29a/c28bf36), Finder-conformance iconless menu
  rows + arrow cursor (66c9786), app-wide I-beam cursor restriction (2d96f8a),
  search hint 'Search (/)' (63fe4f0/00acb34), pill ring wave
  (9cf27e4/20e00d3/459c3e8), Ctrl+W close (93cb097), donut live-apply
  (c046bed), maximized capture (6435f76), search-clip diagnosis (f652483),
  context-menu wave plan (4578ba8). All five gates re-GREEN 00:15 on head
  00acb34 + the 5-file uncommitted wave (388+/2-, saves 17:15-17:20): fmt OK,
  check/clippy clean, 69 tests / 0 failed / 0 ignored, release current by
  cargo no-op (binary 17:23:57, the session's own link), --version 0.
  Gate cutoff 17:23:57 IST Nov 11. A1-A6 remain session-owned.
- 17:01 IST Nov 13 (cron): five gates re-GREEN cold on head ab8fab2 (clean
  tree; target/ had been cleaned - cron rebuilt release from scratch 4m28s):
  fmt --check OK, check/clippy clean, test --lib 73 passed / 0 failed /
  0 ignored, --version 0 (v2.1.0). Gate cutoff 17:01:30 IST Nov 13; covers
  the whole Nov 12-13 wave line (status-bar seamless, sidebar Zen/Arc,
  folder sizes, icon pack upgrade, settings/config/Lua mlua). Surfaces
  visual-QA-open (interactive-only).
- 03:31 IST Nov 15 (cron): five gates re-GREEN on head 431935a (clean tree):
  fmt --check OK with no reformat, check/clippy clean, test --lib 128
  passed / 0 failed / 0 ignored (up from 73), release current by cargo
  no-op (binary 03:18:44, the session's own link), --version 0 (v2.1.0).
  Gate cutoff 03:18:44 IST Nov 15; covers b2da8d6..431935a (keybinding
  profile, interface size, UI font, real glass + window opacity, look
  presets, DnD action menu, job3 3a topmost/pins/tabs, 3b real glass,
  3c special folder icons in all packs, 3d donut pie geometry). New
  surfaces visual-QA-open (interactive-only).

## Design System & Motion Wave (Approved 2026-11-15)
- [x] Phase D1: Semantic Palette Expansion & Design Style Presets (16 roles, 13 dark + 13 light, contrast floors, DesignTokens, Preferences picker) — DONE (commit aae2610)
- [x] Phase D2: Responsive Bento Dashboard & Max-Width Container (4 tiers, centered 1440px container, style-specific layouts, storage rings, pure layout funcs) — DONE (commit c73aca4)
- [ ] Phase D3: Multi-Column Recent Files Table & View Zoom (bounded line lengths, Details/Grid/Miller independent zoom, shortcuts, reflow) — ACTIVE
- [ ] Phase D4: Arc-Style Sidebar Spaces & Floating Pill Nav (Spaces, minimal dock, library tree)
- [ ] Phase D5: Context Menu Windows 3rd-Party Shell Verbs & Apple-HIG 7-Dot Tag Picker
- [ ] Job B: Window Level Animations (Genie effect, minimize/maximize, alt-tab) + Component Polish

## Phase 23 — Universal Converter Engine, FileType-Aware Donut, Tangerine Dialog & Artcraft Bridge
- [x] Wave 1: Dijkstra RoutePlanner Engine (weighted multigraph, format sniffer, zero-loss routing, ICC profile preservation) — DONE
- [x] Wave 2: FileType-Aware Donut Toolbar in Mossambi Shape (20 domain profiles, adaptive petals per category, 180ms 3D flip) — DONE
- [x] Wave 3: Tangerine-Inspired Floating Tweak Dialog & Widget Settings (10-tab settings window, live preview, quality/DPI sliders) — DONE
- [x] Wave 4: Artcraft Rust CLI & Existing Engine Bridge (zero-download discovery of ArtCraft, LibreOffice, FFmpeg, Pandoc, Inkscape, Ghostscript) — DONE
- [x] Wave 5: Extensible Preset Architecture (JSON schema, import/export API, ICC profile retention, EXIF preservation, custom preset gallery) — DONE
- [x] Wave 6: Cross-OS & Architecture Native Gates (Win/Mac/Linux, x86_64/ARM64, pure ASCII contract, 0.00% zero-loss tests) — DONE

