# Mossambi Option B Architecture Plan: Full File Dialog Replacement via Injection

Pure 7-bit ASCII Engineering Specification.
Author: Mossambi Systems Team
Target: Windows 10 (1809+) & Windows 11 (x86_64 and ARM64)

---

## 1. Executive Summary & Objective

In modern Windows desktop environments, file selection workflows occur primarily through
the Windows Common Item Dialogs (IFileOpenDialog and IFileSaveDialog). While Option A provides
a non-invasive docked companion bar attached to `#32770` dialog windows, Option B aims for
complete in-place replacement: when an application requests an Open or Save file dialog,
it receives a full Mossambi macOS-style Miller-column / Grid browser experience with zero-loss
previews, instant search, and tag filtering, completely replacing the dated Windows DirectUI shell.

This document outlines the end-to-end technical architecture, injection mechanics, COM vtable
interception, process filtering, fail-safe fallbacks, and emergency unhook safeguards for Option B.

---

## 2. Windows COM File Dialog Architecture

Modern Windows applications instantiate file dialogs through the Component Object Model (COM):

- CLSID_FileOpenDialog: `{DC1C5A9C-E88A-4DDE-A5A1-60F82A20AEF7}`
- CLSID_FileSaveDialog: `{C0B4E2F3-BAFF-4921-B461-897B14035421}`
- Primary Interfaces:
  - `IFileOpenDialog` (inherits `IFileDialog` -> `IModalWindow` -> `IUnknown`)
  - `IFileSaveDialog` (inherits `IFileDialog` -> `IModalWindow` -> `IUnknown`)
  - `IFileDialogCustomize` (for apps appending custom controls like checkboxes or encoding dropdowns)
  - `IFileDialogEvents` (for client advisory event sinks)

### Lifecycle of a Dialog Call:
1. Client app calls `CoCreateInstance(CLSID_FileOpenDialog, NULL, CLSCTX_INPROC_SERVER, IID_IFileOpenDialog, &pDialog)`.
2. Client configures options: `pDialog->SetOptions(FOS_ALLOWMULTISELECT | FOS_FILEMUSTEXIST)`.
3. Client configures file types: `pDialog->SetFileTypes(cTypes, rgFilterSpec)`.
4. Client displays the modal dialog: `HRESULT hr = pDialog->Show(hwndOwner)`.
5. Upon user confirmation (`hr == S_OK`):
   - Client retrieves items via `pDialog->GetResults(&pItemArray)` or `pDialog->GetResult(&pItem)`.
   - Client calls `IShellItem::GetDisplayName(SIGDN_FILESYSPATH, &pszPath)`.
6. Client releases COM interface pointers.

---

## 3. Injection and Interception Strategy

To replace the dialog seamlessly, Mossambi must intercept the instantiation and invocation
without modifying target application binaries on disk.

### 3.1 Injection Vector: Windows Hooking + DLL Injection
- Injection relies on two architecture-matched native DLLs:
  - `mossambi_hook64.dll` (for 64-bit target processes)
  - `mossambi_hook32.dll` (for 32-bit legacy WOW64 applications)
- Activation via `SetWindowsHookExW(WH_CBT, CbtHookProc, hinstDll, 0)`:
  - When installed on the interactive desktop, the Windows hook manager injects the DLL into
    every GUI thread upon initial message dispatch.
  - In `DllMain(DLL_PROCESS_ATTACH)`, the DLL queries the current executable name and verifies
    it against the process allowlist and denylist.

### 3.2 In-Process Hooking: COM VTable Interception
Rather than hooking raw Win32 APIs globally, Mossambi uses a dual-stage interception model:

1. **Stage 1: Detour CoCreateInstance**:
   - Apply a Microsoft Detours or MinHook inline hook to `CoCreateInstance` and `CoCreateInstanceEx`
     inside `combase.dll` / `ole32.dll`.
   - If `rclsid` is neither `CLSID_FileOpenDialog` nor `CLSID_FileSaveDialog`, pass through immediately
     to the original `CoCreateInstance`.
   - If `rclsid` matches:
     - Instantiate the real Windows dialog via the trampoline to obtain a valid underlying COM object.
     - Wrap or proxy the returned `IFileOpenDialog` / `IFileSaveDialog` pointer with a Mossambi
       COM proxy object (`MossambiFileDialogProxy`).

2. **Stage 2: VTable Interception on IModalWindow::Show**:
   - `Show(HWND hwndOwner)` is the sole entry point that blocks the host thread and displays UI.
   - When `Show(hwndOwner)` is invoked on the proxy:
     - Read the configured options, file filters, default folder, and title from the stored state.
     - Query whether the caller attached custom controls via `IFileDialogCustomize`.
     - If custom controls exist that cannot be mapped, gracefully fall through to original `pRealDialog->Show(hwndOwner)`.
     - Otherwise, summon the Mossambi Modal Dialog UI via high-speed local IPC.

---

## 4. Modal UI Execution and IPC Architecture

### 4.1 Host Thread Modality
When `Show(hwndOwner)` runs:
1. The injected DLL disables the owner window via `EnableWindow(hwndOwner, FALSE)` to prevent
   user interaction with the host app (standard Win32 modal dialog contract).
2. The DLL sends a synchronous `SHOW_DIALOG` request over a dedicated local named pipe
   `\\.\pipe\mossambi_dialog_ipc_<pid>` to the running Mossambi host process.
3. The Mossambi host process displays a dedicated topmost modal window positioned over `hwndOwner`.
4. While the dialog is open, the injected DLL pumps a modal message loop:
   ```c
   MSG msg;
   while (bDialogActive && GetMessageW(&msg, NULL, 0, 0)) {
       TranslateMessage(&msg);
       DispatchMessageW(&msg);
   }
   ```
5. When the user confirms file selection:
   - Mossambi IPC transmits the selected absolute paths back to the injected DLL.
   - The DLL instantiates standard Windows `IShellItem` and `IShellItemArray` instances via
     `SHCreateItemFromParsingName`.
   - The proxy stores these results in its internal buffer.
6. `EnableWindow(hwndOwner, TRUE)` re-enables the host window and brings it to the foreground.
7. `Show()` returns `S_OK` (or `HRESULT_FROM_WIN32(ERROR_CANCELLED)` if the user closed/cancelled).

---

## 5. Process Filtering: Allowlist & Denylist

Injection must be conservative to prevent application crashes or security tool interference.

### 5.1 Strict Denylist (Never Hook)
The following executables and categories are unconditionally bypassed:
- Windows core system processes: `explorer.exe`, `dwm.exe`, `lsass.exe`, `csrss.exe`, `smss.exe`, `services.exe`, `taskmgr.exe`.
- Security and Antivirus software: Windows Defender (`MsMpEng.exe`), CrowdStrike, SentinelOne, Symantec.
- Anti-cheat gaming software: EasyAntiCheat, BattlEye, Vanguard.
- Web browsers with strict sandboxing: Chrome/Edge renderer processes (broker process only if allowed).
- Critical developer runtimes: `rustc.exe`, `cargo.exe`, `git.exe`, `devenv.exe`, `cl.exe`.

### 5.2 Allowlist Strategy
Mossambi adopts a phased allowlist deployment:
- Phase 1 (Initial Release): Opt-in per executable via Settings (e.g. `notepad.exe`, `code.exe`, `photoshop.exe`, `vlc.exe`).
- Phase 2: Category allowlist (Productivity, Image Editors, Audio DAWs, Media Players).
- Phase 3: Global opt-out mode for all desktop applications outside the denylist.

### 5.3 Sandbox Awareness
- UWP / AppContainer applications: Injected DLLs cannot establish arbitrary named pipe IPC across
  container boundaries without `ALL APPLICATION PACKAGES` ACLs. Mossambi sets appropriate DACLs
  on its named pipes to permit AppContainer clients.
- Protected Processes Light (PPL): Windows prevents any non-signed DLL injection into PPL binaries;
  `SetWindowsHookEx` automatically skips these processes.

---

## 6. Fail-Safe, Watchdog & Emergency Unhook

### 6.1 500ms Watchdog Fallback
If Mossambi crashes, hangs, or fails to respond to the IPC `SHOW_DIALOG` ping within 500 milliseconds:
- The injected DLL aborts Option B interception.
- It immediately delegates to the original Windows `IFileDialog::Show(hwndOwner)`.
- The user experiences at most a half-second pause before the standard Windows dialog appears; no file operation is ever lost.

### 6.2 Emergency "Unhook All" Kill Switch
To guarantee safety in case of unexpected software conflicts:
1. **Global Named Event**: Mossambi creates a named event `Global\MossambiEmergencyUnhookEvent`.
2. Any thread detecting this event immediately unpatches all Detours trampolines, detaches hooks,
   and unloads DLLs cleanly.
3. Trigger Mechanisms:
   - Global emergency hotkey: `Win + Alt + Esc`.
   - CLI emergency command: `mossambi --unhook-all`.
   - Automatic trigger: When Mossambi primary process exits or terminates, the event is signalled automatically.

---

## 7. Security and Code Signing Requirements

1. **Authenticode EV Code Signing**:
   Both `mossambi_hook64.dll` and `mossambi_hook32.dll` must be digitally signed with an
   Extended Validation (EV) certificate trusted by the Windows Root CA. Unsigned DLL injection
   triggers Microsoft Defender SmartScreen heuristics and behavioral blocking.
2. **Deterministic Trampoline Validation**:
   Before modifying any vtable or API prologue, the injection engine validates opcode bytes
   to ensure third-party tools (e.g. screen recorders, antivirus hooks) haven't already patched
   the same instruction. If foreign hooks are detected, Mossambi gracefully yields.

---

## 8. Summary Comparison

| Metric / Dimension          | Option A (Companion Bar)          | Option B (Full Replacement)       |
|-----------------------------|-----------------------------------|-----------------------------------|
| Implementation Risk         | Low (zero injection, out-of-proc) | Medium-High (DLL injection, COM)  |
| Visual Experience           | Standard dialog + docking toolbar | 100% Mossambi Miller / Grid UI    |
| Compatibility Scope         | 100% of Windows #32770 dialogs    | Filtered allowlist of desktop apps|
| Antivirus / Heuristics Risk | None                              | Mitigated by EV Signing & Denylist|
| Custom Controls Support     | Unaffected (handled by native UI) | Fallback to native on complex UI  |
| Deployment Phase            | Active in W4                      | Planned for Future Phase          |
