# Mossambi Development Worklog & Verification Ledger

**Date**: 2026-11-15  
**Author**: Antigravity Assistant & Engineering Team  
**Scope**: Verification, Design Audit, UI Contrast Rules, Phase F1 Fluent & Shell Icons, Disputed Claims Audit

---

## 1. Phase F1 Strict Verification Ledger

| F1 Item | Status | Evidence (File:Line & Tests) | Strict Findings & Gaps |
| :--- | :---: | :--- | :--- |
| **a. Official Fluent UI Icons** | **DONE** | `assets/icons/fluent/` (114 SVGs)<br>`assets/icons/fluent/LICENSE` (MIT, Microsoft Corp)<br>`assets/icons/SOURCES.md`<br>`tests/test_f1_fluent_and_shell_icons.rs:20` | 114 official Microsoft Fluent vector SVGs (57 regular + 57 filled 20px) present with LICENSE and SOURCES.md attribution. Verified by test suite. |
| **b. Remove Custom Icons & `[LOCK]`** | **PARTIAL** | `assets/icons/presets/` deleted (0 files)<br>`src/core/folder_icons.rs:37` | Old custom SVG presets in `assets/icons/presets/` are 100% deleted. However, grep for `\[LOCK\]` finds 1 hit in `src/core/folder_icons.rs:37` (`emoji_glyph: "[LOCK]"`). |
| **c. Windows Native Shell Icons** | **DONE** | `src/core/shell_icons.rs:40-62` (bg worker)<br>`src/core/shell_icons.rs:135-240` (SHGetFileInfoW)<br>`src/core/shell_icons.rs:104-131` (LRU cache)<br>`src/core/shell_icons.rs:355-430` (AssocQueryStringW) | Background worker thread `mossambi_shell_icons` with `CoInitializeEx(APARTMENTTHREADED)`. Native GDI 32-bit DIB extraction. LRU cache off UI thread. 4 unit tests pass in `core::shell_icons`. |
| **d. File Info Redesign** | **DONE** | `src/gui/explorer_view.rs:10348` (min 460px, def 500px)<br>`src/gui/explorer_view.rs:10363-10450` (64x64 icon + rename)<br>`src/gui/explorer_view.rs:9992` (size split card)<br>`src/gui/explorer_view.rs:10076` (parent size bar)<br>`src/gui/explorer_view.rs:10117` (timeline dots)<br>`src/gui/explorer_view.rs:10188` (permission chips)<br>`src/gui/explorer_view.rs:10250` (open with app) | Modeled after macOS Finder Get Info + GNOME Files Properties. Hero 64x64 icon with inline rename, 4 collapsible sections (General, More Info, Permissions, Open With), physical 4KB cluster allocation card, parent folder ratio bar, created/modified/accessed timeline dots, permission chips, full detail hover tooltips, SHA-256 on demand. No 380px clipping (width 500px). |
| **e. Text Contrast & Menu Shortcuts** | **DONE** | `src/gui/design.rs:487-494` (`text_on_accent`)<br>`src/gui/file_menu.rs:412-420` (right-aligned shortcut)<br>`src/gui/explorer_view.rs:12140, 12368` (min_width 240px)<br>`src/gui/design.rs:215-260` (`segmented_pill`) | Contrast math guarantees >= 3.0:1 on all 13 themes (Rangalipi coral red `#e4584f` gets pure white `#FFFFFF` text; Gruvbox `#83a598` gets `#121212` with 6.64:1 contrast). Menu shortcuts strictly right-aligned with 240px min menu width. Segmented controls unified across Preferences. |

---

## 2. Disputed Verifier Claims Re-Check

| Disputed Claim | Status | File:Line Evidence | Exact Architectural Reality |
| :--- | :---: | :--- | :--- |
| **Job B Genie: Real mesh deformation & minimize animation** | **CONFIRMED** | `src/core/window_anim.rs:73-108`<br>`src/gui/studio_app.rs:1392-1403`<br>`src/gui/studio_app.rs:1664-1716` | **Real Mesh Deformation**: `genie_mesh_transform` computes non-linear polynomial funnel wave with bottom slice lead (0.35) and quadratic suction curve. `render_window_animation_overlay` subdivides the window into 24 vertical slices and renders 24 deformed convex polygon bands and funnel contour stroke.<br>**Minimize Intercept**: When minimize is clicked, `request_minimize` is set; `studio_app.rs:1392-1403` intercepts it, animates over ~280ms (`dt / 0.28`), and calls `ctx.send_viewport_cmd(Minimized(true))` only after `progress >= 1.0`. (Note: Renders solid background alpha quads rather than live GPU framebuffer texture). |
| **D5 Right-Click: 3rd-party shell verbs from registry** | **PARTIAL** | `src/gui/context_menu.rs:150-250`<br>`src/gui/context_menu.rs:339-449`<br>`src/gui/explorer_view.rs:12623` | **What it does**: Reads `HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\<ext>\UserChoice` and `OpenWithProgids`, `HKCR\<ext>\OpenWithProgids`, and `HKCR\Applications\<app>\SupportedTypes` to resolve 3rd-party application commands for the **"Open with >"** submenu.<br>**What it does NOT do**: It does NOT enumerate COM `IContextMenu` shell extensions (e.g., 7-Zip, Git GUI, WinRAR under `*\shellex\ContextMenuHandlers`) or arbitrary registry verbs on the root context menu. |
| **D3 Per-Mode Zoom: Persistence & keyboard shortcuts** | **CONFIRMED** | `src/core/config.rs:1137-1147`<br>`src/core/config.rs:1520-1540`<br>`src/gui/studio_app.rs:680-746`<br>`src/gui/studio_app.rs:1807-1825`<br>`tests/test_view_zoom.rs` | **Shortcuts Active**: `Ctrl+wheel` (`studio_app.rs:708`), `Ctrl+=` / `Ctrl++` (`studio_app.rs:695`), `Ctrl+-` (`studio_app.rs:696`), `Ctrl+0` reset (`studio_app.rs:713`).<br>**Per-Mode Persistence**: `config.rs:1137-1147` stores independent `list_zoom`, `grid_zoom`, and `miller_zoom` (0..=4). `studio_app.rs:730-745` steps/resets active mode zoom and updates config; `studio_app.rs:1807-1825` detects deltas and writes to config JSON. Fully verified by `test_view_zoom.rs`. |

---

## 3. Foreground Check Results

All 5 required check commands were executed cleanly:

1. **`cargo fmt --check`**: **PASSED** (exit code 0, 0 diffs).
2. **`cargo clippy --all-targets -- -D warnings`**: **PASSED** (exit code 0, 0 warnings across lib, bins, tests, examples).
3. **`cargo test --test gui_ascii_gate`**: **PASSED** (1 passed, 100% 7-bit ASCII verified across all source files).
4. **`cargo test --tests`**: **PASSED** (Grand Total: **359 tests passed**, 0 failed, 0 ignored):
   - 157 unit tests in `mossambi` library (`src/lib.rs`).
   - 202 integration tests across 32 test binaries in `tests/`.
5. **`cargo build --release`**: **PASSED** (exit code 0, optimized release binary produced in 1m 09s).

---

## 4. Git Ledger Today

- **`bd77656`** - `wip(f1): official fluent system icons, native shell icon cache, and properties modal revamp`  
  Committed all uncommitted Phase F1 assets, native shell icon background engine, contrast luminance rules, and file info inspector modal.
- **`38ac7e7`** - `docs: add development worklog and verification ledger`
- **`ab2a25c`** - `feat: enforce ui contrast rules, standardize segmented pills, and revamp file info inspector`
- **`3dba6ea`** - `test: improve dir watcher lifecycle test retry stability`
- **`ea32b9f`** - `feat: classical cpu partition, navigation button micro-animations, hover previews, and settings modernization`
- **`43102e9`** - `feat: standard shift-click range selection, compound ctrl-shift multi-selection, and shift-arrow keyboard navigation`
- **`1985d61`** - `feat: bento dashboard, DWM acrylic blur, genie animation, hybrid CPU QoS, and folder icons`
