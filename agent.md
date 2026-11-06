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
  `[^\x00-\x7F]` regex sweep (write findings to a UTF-8 file when the console codepage mangles
  output - the pwsh console renders Unicode as `??` soup).
- **No emoji in GUI strings.** Emoji are removed from any string the user sees.
- Pre-existing glyphs are explicitly OFF-LIST (leave alone, do not fix, do not reformat):
  em dash / en dash, ellipsis, symlink arrow (feather-arrow), breadcrumb angle-quote (single
  guillemet), right/left/up/down arrows, plus-sign copy badge, check mark, cross mark, times sign,
  bullet, sort column triangles, horizontal rule and double-line rule box-drawing comment banners,
  vertical/box-drawing tree chars in daemon banner, window-control glyphs (restore/minimize
  squares), sort-direction..drawer glyphs (plus, minus-over-dot, info sign, degree sign,
  micro sign, implies-arrow, check, two-line arrow), and the pre-existing emoji tab/category/
  button labels that the next UI overhaul replaces wholesale (home, folder, globe, image, music,
  clapper, page, package, laptop, magnifier, folder-open, monitor, download, waste basket,
  target, microscope, clipboard, floppy, plus, pencil nib, gear, warning, sun, cloud, hamburger,
  trigram, crossed swords, star, heavy check, heavy cross, right-pointing arrow, VS16 selector).
- Twice agents slipped foreign text into authored files (CJK word "biaoji", the word "coeur") -
  the regex sweep is the regression guard; names in docs are ASCII phrasings only.

## Drag & drop status (Windows + Linux, est. 2026-11-06)

| Flavor | Windows | Linux X11 | Linux Wayland |
|---|---|---|---|
| External drop-in | works (winit + Studio `active_file` cursor-rescue fallback) | works (winit XDND receive) | receives existed via forced-X11 policy see below |
| Internal drag to folder | egui payload, move / Ctrl-copy, copy-only to OS | same | same |
| Drag out to other apps | real OLE `DoDragDrop` (CF_HDROP + Preferred DropEffect, COPY-only) | typed `text/uri-list` clipboard hand-off (xclip) | same via wl-copy |

Key facts to preserve when touching this code:

- Windows OLE lives in `src/core/ole_drag_out.rs` using the windows 0.59 macro contract:
  implement `<Interface>_Impl` traits on the GENERATED outer type (`X_Impl`,
  `impl windows_core::IUnknownImpl for X_Impl` with `type Impl = X`), access inner state via the
  outer's `Deref`, obtain interfaces with `value.into()`, pass to `DoDragDrop` BY REFERENCE
  (`&data_object`), and `&mut effect` coerces to a raw effect pointer.
- Linux clipboard file hand-off lives in `src/core/linux_clipboard_files.rs`. The payload builder is
  OS-neutral (testable everywhere); only the write is Linux-gated. One helper process serves ONE
  MIME type (wl-copy cannot multi-type: wl-clipboard issue #71), so the universal `text/uri-list`
  is written; receivers fall back to it as COPY (COPY-only semantics).
- Cross-OS dispatch is `src/core/drag_out.rs::begin_file_drag_out`, called by:
  - Explorer (`explorer_view.rs`): hand-off when the payload leaves the viewport bounds
    (dual-signal: `interact_pos()` outside `ctx.screen_rect()` OR `None` after `PointerGone` while
    `is_decidedly_dragging()` holds); per-press `drag_out_in_flight` latch resets on release.
  - Widget hub (`widget_app.rs`): hand-off on drag threshold when a file is loaded;
    `ViewportCommand::StartDrag` (window move) only when none.
- `src/core/dnd_backend.rs::apply_backend_policy` wires eframe/winit options at both GUI entry
  points (Studio + Widget); egui 0.29 eframe has NO `clipboard` feature; winit 0.30 XDND receive
  exists on X11 only, so Wayland sessions bridge through the forced-X11 policy. Verify the
  `Drag & drop backend policy` log line when smoke-testing Linux.
- winit 0.30 `ViewportCommand::StartDrag` only window-drags; it is NOT an OS file drag.

## Gates before any commit

1. `cargo fmt`
2. `cargo clippy --all-targets -- -D warnings`
3. `cargo test`
4. `cargo build --release`
5. ASCII sweep of every file touched (see above).
6. open-computer-use QA pass on Windows for GUI work.
7. Keep `graphify-out/` untracked. Do not stage scratch artifacts (screenshots etc.).

## Known intentional pre-existing glyphs

`studio_app.rs` and other legacy files keep their em dashes; `explorer_view.rs` keeps its category
emoji labels until the UI overhaul swaps in the real icon pipeline. Remove emoji only from touched
strings.
