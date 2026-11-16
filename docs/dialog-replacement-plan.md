# Dialog Replacement Plan (W4b)

Pure 7-bit ASCII Engineering Specification.
Author: Mossambi Systems Team
Target: Windows 10 (1809+) & Windows 11 (x86_64 and ARM64)

Status: Option A (companion bar) is implemented in this wave, opt-in, off by
default. Option B (injection) is a plan only; no injection code exists in the
tree.

## Problem

Standard Windows Open/Save dialogs (Notepad, browsers, Office, ...) have no
quick way to jump to a Mossambi bookmark. Users must walk the dialog's own
folder tree by hand every time. Option A fixes that non-invasively; Option B
(outlined below, plan only) aims at full in-place replacement.

## Option A - companion bar (implemented)

A small Mossambi-owned popup strip docked to the edge of an active Open/Save
dialog. It lists up to 12 Mossambi bookmarks (the existing `pinned_custom`
store; no new store) as buttons; clicking one navigates the dialog to that
folder.

Detection:

- `SetWinEventHook` for `EVENT_SYSTEM_FOREGROUND` and
  `EVENT_OBJECT_LOCATIONCHANGE` with `WINEVENT_OUTOFCONTEXT`. Both hooks run
  on a dedicated Mossambi thread that also runs the `GetMessage` loop, so
  callbacks arrive on that thread without touching the GUI app.
- A top-level window is a file dialog when its class is `#32770` and its
  descendant classes contain `DirectUIHWND` (the modern IFileDialog frame)
  plus an `Edit` inside a `ComboBoxEx32` (the filename box). The
  classification is a pure function (`is_file_dialog`) unit-tested headlessly.
- The bar is a plain Win32 popup window with
  `WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE`: it floats above the
  dialog, never appears in the taskbar, and never steals keyboard focus.

Navigation:

- The bar window procedure writes the folder path into the dialog's filename
  Edit with `WM_SETTEXT`, then sends `WM_COMMAND` with `IDOK` to the dialog.
  The dialog performs its own navigation; no synthetic global input is
  generated anywhere.

Lifecycle:

- `start()` reads the bookmarks and spawns the companion thread (hook +
  bar window + message loop).
- The bar docks to the right edge of the dialog and falls back to the left
  edge when the right side does not fit on screen (pure function
  `dock_rect`). It follows the dialog on move/resize via
  `EVENT_OBJECT_LOCATIONCHANGE` and hides when the foreground is no longer a
  file dialog or the tracked dialog is destroyed.
- `stop()` posts `WM_QUIT` to the companion thread, which unhooks, destroys
  the bar window, and exits.
- Opt-in setting `dialog_companion` (default false) lives in Preferences,
  General: "File dialog companion bar". Toggling starts/stops the bar live;
  at app startup the bar starts when the setting is on.

Limitations (accepted for W4b):

- Docking math uses primary-monitor metrics (`SM_CXSCREEN`/`SM_CYSCREEN`).
- Classic `GetOpenFileName` dialogs (no `DirectUIHWND`) get a fallback via
  the 1148/1152 filename control ids but are not the primary target.
- Elevated (admin) dialogs are not reachable from a non-elevated Mossambi
  (UIPI blocks the messages); running Mossambi elevated lifts this.

## Option B - injection replacing the dialog (plan only, NOT implemented)

Opt-in, OFF by default; requires an installer-level toggle. When an
application requests an Open or Save file dialog, it would receive a full
Mossambi macOS-style Miller-column / Grid browser with zero-loss previews,
instant search, and tag filtering, completely replacing the Windows DirectUI
shell. Phases and test plan only; there is NO code for Option B.

### B.1 Windows COM File Dialog Architecture

Modern Windows applications instantiate file dialogs through COM:

- CLSID_FileOpenDialog: `{DC1C5A9C-E88A-4DDE-A5A1-60F82A20AEF7}`
- CLSID_FileSaveDialog: `{C0B4E2F3-BAFF-4921-B461-897B14035421}`
- Primary Interfaces:
  - `IFileOpenDialog` (inherits `IFileDialog` -> `IModalWindow` -> `IUnknown`)
  - `IFileSaveDialog` (inherits `IFileDialog` -> `IModalWindow` -> `IUnknown`)
  - `IFileDialogCustomize` (for apps appending custom controls)
  - `IFileDialogEvents` (for client advisory event sinks)

Lifecycle of a dialog call:
1. Client app calls `CoCreateInstance(CLSID_FileOpenDialog, NULL,
   CLSCTX_INPROC_SERVER, IID_IFileOpenDialog, &pDialog)`.
2. Client configures options: `pDialog->SetOptions(FOS_ALLOWMULTISELECT |
   FOS_FILEMUSTEXIST)`.
3. Client configures file types: `pDialog->SetFileTypes(cTypes, rgFilterSpec)`.
4. Client displays the modal dialog: `HRESULT hr = pDialog->Show(hwndOwner)`.
5. Upon user confirmation (`hr == S_OK`):
   - Client retrieves items via `pDialog->GetResults(&pItemArray)` or
     `pDialog->GetResult(&pItem)`.
   - Client calls `IShellItem::GetDisplayName(SIGDN_FILESYSPATH, &pszPath)`.
6. Client releases COM interface pointers.

### B.2 Injection and Interception Strategy

To replace the dialog seamlessly, Mossambi must intercept the instantiation
without modifying target application binaries on disk.

- Injection relies on two architecture-matched native DLLs:
  - `mossambi_hook64.dll` (for 64-bit target processes)
  - `mossambi_hook32.dll` (for 32-bit legacy WOW64 applications)
- Activation via `SetWindowsHookExW(WH_CBT, CbtHookProc, hinstDll, 0)`:
  the Windows hook manager injects the DLL into every GUI thread upon
  initial message dispatch; `DllMain(DLL_PROCESS_ATTACH)` verifies the
  executable against the process allowlist and denylist.
- Stage 1: detour `CoCreateInstance` / `CoCreateInstanceEx`. If `rclsid` is
  neither `CLSID_FileOpenDialog` nor `CLSID_FileSaveDialog`, pass through.
  Otherwise instantiate the real dialog via the trampoline and wrap the
  returned pointer in a Mossambi COM proxy.
- Stage 2: intercept `IModalWindow::Show` on the proxy. Complex custom
  controls (`IFileDialogCustomize`) that cannot be mapped fall through to
  the original dialog.

### B.3 Modal UI Execution and IPC

- The injected DLL disables the owner window (`EnableWindow(hwndOwner,
  FALSE)`) per the standard modal contract.
- A synchronous `SHOW_DIALOG` request goes over a named pipe
  `\\.\pipe\mossambi_dialog_ipc_<pid>` to the running Mossambi host, which
  shows a topmost modal window positioned over `hwndOwner`.
- The DLL pumps a modal message loop while the dialog is open.
- On confirmation, selected absolute paths return over IPC; the DLL builds
  standard `IShellItem`/`IShellItemArray` results via
  `SHCreateItemFromParsingName`, re-enables the owner window, and returns
  `S_OK` (or `HRESULT_FROM_WIN32(ERROR_CANCELLED)` on cancel).

### B.4 Process Filtering: Allowlist & Denylist

Strict denylist (never hook): Windows core processes (`explorer.exe`,
`dwm.exe`, `lsass.exe`, `csrss.exe`, `smss.exe`, `services.exe`,
`taskmgr.exe`); security software (Defender `MsMpEng.exe`, CrowdStrike,
SentinelOne, Symantec); anti-cheat (EasyAntiCheat, BattlEye, Vanguard);
browser sandboxes; developer runtimes (`rustc.exe`, `cargo.exe`, `git.exe`,
`devenv.exe`, `cl.exe`).

Phased allowlist deployment:
- Phase 1: opt-in per executable via Settings (`notepad.exe`, `code.exe`,
  `photoshop.exe`, `vlc.exe`).
- Phase 2: category allowlist (Productivity, Image Editors, Audio DAWs,
  Media Players).
- Phase 3: global opt-out mode for all desktop apps outside the denylist.

Sandbox awareness: UWP/AppContainer needs `ALL APPLICATION PACKAGES` DACLs
on the named pipes; Protected Processes Light refuses injection and is
skipped automatically.

### B.5 Fail-Safe, Watchdog & Emergency Unhook

- 500ms watchdog: if Mossambi crashes, hangs, or misses the IPC ping within
  500 milliseconds, the DLL aborts interception and delegates to the
  original `IFileDialog::Show(hwndOwner)`; no file operation is ever lost.
- Emergency unhook kill switch: a global named event
  (`Global\MossambiEmergencyUnhookEvent`) triggers full unpatch, detach, and
  clean unload. Triggers: `Win + Alt + Esc` hotkey,
  `mossambi --unhook-all` CLI command, and automatic signalling when the
  Mossambi primary process exits.

### B.6 Security and Code Signing Requirements

- Both DLLs must be Authenticode EV code-signed; unsigned injection triggers
  Defender SmartScreen heuristics and behavioral blocking.
- Deterministic trampoline validation: before patching any vtable or API
  prologue, validate opcode bytes; if foreign hooks (screen recorders,
  antivirus) already patched the site, yield gracefully.

### B.7 Risks

- Antivirus / EDR heuristics commonly flag DLL injection and COM hooking.
- 32-bit and 64-bit targets need two builds of the injected DLL.
- Elevated targets cannot be injected from a non-elevated process and vice
  versa.
- A crash inside the hook takes down the host app (Notepad, Word, ...), not
  Mossambi.

### B.8 Test Plan (Option B)

- Unit: COM class-id interception table; allow-list matching.
- Integration: test host opens 100 dialogs with and without the hook; assert
  identical result codes and no leaked handles.
- Soak: 24h loop over Notepad/Paint/Word; monitor host crash rate == 0.
- AV: sample submissions to major engines before any release.

## Summary Comparison

| Metric / Dimension          | Option A (Companion Bar)          | Option B (Full Replacement)       |
|-----------------------------|-----------------------------------|-----------------------------------|
| Implementation Risk         | Low (zero injection, out-of-proc) | Medium-High (DLL injection, COM)  |
| Visual Experience           | Standard dialog + docking toolbar | 100% Mossambi Miller / Grid UI    |
| Compatibility Scope         | 100% of Windows #32770 dialogs    | Filtered allowlist of desktop apps|
| Antivirus / Heuristics Risk | None                              | Mitigated by EV Signing & Denylist|
| Custom Controls Support     | Unaffected (handled by native UI) | Fallback to native on complex UI  |
| Deployment Phase            | Active in W4                      | Planned for Future Phase          |

Decision: Option A ships now. Option B stays documented here, off by default,
with no code in the repository.
