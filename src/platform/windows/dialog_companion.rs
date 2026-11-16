//! W4b Dialog Companion Bar for standard Windows Open/Save dialogs.
//!
//! Pure 7-bit ASCII. Opt-in via the `dialog_companion` setting (default
//! false). A companion thread owns a plain popup bar window (topmost, tool
//! window, no-activate) holding one button per Mossambi bookmark (max 12,
//! read from the existing pinned-folder store). WinEvent hooks track the
//! active common file dialog (class "#32770" hosting DirectUIHWND plus an
//! Edit inside ComboBoxEx32) and the bar docks beside it, follows it, and
//! hides when it goes away. Clicking a button writes the folder into the
//! dialog's filename Edit (WM_SETTEXT) and sends IDOK (WM_COMMAND) to the
//! dialog itself - the dialog does its own navigation and no synthetic
//! global input is ever generated.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, Ordering};
use std::sync::Mutex;

#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
#[cfg(windows)]
use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM, LRESULT, RECT, WPARAM};
#[cfg(windows)]
use windows_sys::Win32::Graphics::Gdi::{GetStockObject, DEFAULT_GUI_FONT, HBRUSH};
#[cfg(windows)]
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
#[cfg(windows)]
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
#[cfg(windows)]
#[cfg(windows)]
use windows_sys::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK};
#[cfg(windows)]
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, EnumChildWindows,
    FindWindowExW, GetClassNameW, GetDlgItem, GetMessageW, GetSystemMetrics, GetWindowRect,
    IsWindow, LoadCursorW, MoveWindow, PeekMessageW, PostMessageW, PostThreadMessageW,
    RegisterClassW, SendMessageW, SetWindowPos, TranslateMessage, EVENT_OBJECT_LOCATIONCHANGE,
    EVENT_SYSTEM_FOREGROUND, HMENU, HWND_TOPMOST, IDC_ARROW, IDOK, MSG, SM_CXSCREEN, SM_CYSCREEN,
    SWP_HIDEWINDOW, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW,
    WINEVENT_OUTOFCONTEXT, WM_COMMAND, WM_DESTROY, WM_QUIT, WM_SETFONT, WM_SETTEXT, WM_SIZE,
    WNDCLASSW, WS_CHILD, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP, WS_VISIBLE,
};

/// Maximum number of bookmark buttons on the bar.
const MAX_FOLDERS: usize = 12;
/// First child-control id used for folder buttons (ids are sequential).
const BUTTON_ID_BASE: usize = 1000;
/// Bar width in pixels (dock_rect input for the live bar).
const BAR_WIDTH: i32 = 180;
/// Companion bar window class name.
#[cfg(windows)]
const CLASS_NAME: &str = "MossambiDialogCompanionBar";

// ---------------------------------------------------------------------------
// Pure functions (no Win32) - unit-tested headlessly in tests/test_dialog_companion.rs
// ---------------------------------------------------------------------------

/// Classifies a top-level window as a modern Windows common file dialog.
///
/// The top-level class must be "#32770" and the collected descendant classes
/// must contain `DirectUIHWND` (the modern IFileDialog frame) plus the
/// filename box signature: an `Edit` inside a `ComboBoxEx32`. Comparison of
/// descendant classes is case-insensitive; the dialog class must match
/// exactly as reported by Win32.
pub fn is_file_dialog(class: &str, descendant_classes: &[&str]) -> bool {
    if class != "#32770" {
        return false;
    }
    let mut has_directui = false;
    let mut has_combo = false;
    let mut has_edit = false;
    for d in descendant_classes {
        if d.eq_ignore_ascii_case("DirectUIHWND") {
            has_directui = true;
        } else if d.eq_ignore_ascii_case("ComboBoxEx32") {
            has_combo = true;
        } else if d.eq_ignore_ascii_case("Edit") {
            has_edit = true;
        }
    }
    has_directui && has_combo && has_edit
}

/// Computes the bar rectangle for a dialog rectangle.
///
/// Tuples are `(x, y, width, height)`. The bar (width `bar_width`, spanning
/// the dialog's full height) docks to the right of the dialog; when it would
/// not fit inside the screen's right edge it docks to the left of the dialog.
pub fn dock_rect(
    dialog: (i32, i32, i32, i32),
    bar_width: i32,
    screen: (i32, i32, i32, i32),
) -> (i32, i32, i32, i32) {
    let (dx, dy, dw, dh) = dialog;
    let (sx, _sy, sw, _sh) = screen;
    let fits_right = dx + dw + bar_width <= sx + sw;
    let x = if fits_right { dx + dw } else { dx - bar_width };
    (x, dy, bar_width, dh)
}

// ---------------------------------------------------------------------------
// Live companion thread (Win32). Started only when the setting is enabled.
// ---------------------------------------------------------------------------

#[cfg(windows)]
static RUNNING: AtomicBool = AtomicBool::new(false);
#[cfg(windows)]
static THREAD_ID: AtomicU32 = AtomicU32::new(0);
#[cfg(windows)]
static BAR_HWND: AtomicIsize = AtomicIsize::new(0);
#[cfg(windows)]
static CURRENT_DIALOG: AtomicIsize = AtomicIsize::new(0);
/// Button id -> folder path. Filled by start(), consumed by the wndproc.
#[cfg(windows)]
static FOLDERS: Mutex<Vec<(usize, PathBuf)>> = Mutex::new(Vec::new());
/// Child button hwnds in creation order, for WM_SIZE layout.
#[cfg(windows)]
static BUTTON_HWND: Mutex<Vec<isize>> = Mutex::new(Vec::new());

#[cfg(windows)]
fn folders_lock() -> std::sync::MutexGuard<'static, Vec<(usize, PathBuf)>> {
    FOLDERS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(windows)]
fn buttons_lock() -> std::sync::MutexGuard<'static, Vec<isize>> {
    BUTTON_HWND
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(windows)]
fn to_wide(s: &str) -> Vec<u16> {
    std::ffi::OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

/// Starts the companion thread. Reads folders from the existing bookmark
/// store (`pinned_custom`); no new store is created. No-op when already
/// running.
pub fn start() {
    #[cfg(windows)]
    {
        if RUNNING.swap(true, Ordering::SeqCst) {
            return;
        }
        let mut paths: Vec<PathBuf> = crate::core::MossambiConfig::load()
            .pinned_custom
            .iter()
            .map(PathBuf::from)
            .filter(|p| p.is_dir())
            .collect();
        paths.truncate(MAX_FOLDERS);
        *folders_lock() = (0..paths.len())
            .map(|i| (BUTTON_ID_BASE + i, paths[i].clone()))
            .collect();
        *buttons_lock() = Vec::new();
        CURRENT_DIALOG.store(0, Ordering::SeqCst);

        let spawned = std::thread::Builder::new()
            .name("mossambi-dialog-companion".to_string())
            .spawn(move || unsafe { companion_thread_main(paths) });
        if spawned.is_err() {
            RUNNING.store(false, Ordering::SeqCst);
        }
    }
}

/// Stops the companion thread: posts WM_QUIT to it. The thread unhooks,
/// destroys the bar window, and cleans up on its own. No-op when not running.
pub fn stop() {
    #[cfg(windows)]
    {
        if !RUNNING.swap(false, Ordering::SeqCst) {
            return;
        }
        // Wait briefly for the thread to register its id, then post WM_QUIT.
        for _ in 0..50 {
            let id = THREAD_ID.load(Ordering::SeqCst);
            if id != 0 {
                unsafe {
                    PostThreadMessageW(id, WM_QUIT, 0, 0);
                }
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    }
}

/// Companion thread entry: registers the bar class, creates the hidden popup
/// bar with one button per folder, installs the WinEvent hooks, and runs the
/// message loop until WM_QUIT. Cleans up unconditionally on the way out.
#[cfg(windows)]
unsafe fn companion_thread_main(paths: Vec<PathBuf>) {
    THREAD_ID.store(GetCurrentThreadId(), Ordering::SeqCst);

    let mut bar: HWND = std::ptr::null_mut();
    let mut hook_fg: HWINEVENTHOOK = std::ptr::null_mut();
    let mut hook_move: HWINEVENTHOOK = std::ptr::null_mut();

    let hinstance = GetModuleHandleW(std::ptr::null());
    if !hinstance.is_null() {
        let class_w = to_wide(CLASS_NAME);
        let mut wc: WNDCLASSW = std::mem::zeroed();
        wc.lpfnWndProc = Some(companion_wndproc);
        wc.hInstance = hinstance;
        wc.hCursor = LoadCursorW(std::ptr::null_mut(), IDC_ARROW);
        wc.hbrBackground = 6 as HBRUSH; // COLOR_BTNFACE + 1
        wc.lpszClassName = class_w.as_ptr();
        if RegisterClassW(&wc) != 0 {
            let title_w = to_wide("Mossambi");
            // Plain popup window, created hidden and parked off-screen; the
            // hook shows it only while a file dialog is active.
            bar = CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                class_w.as_ptr(),
                title_w.as_ptr(),
                WS_POPUP,
                -32000,
                -32000,
                BAR_WIDTH,
                240,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                hinstance,
                std::ptr::null(),
            );
            if !bar.is_null() {
                BAR_HWND.store(bar as isize, Ordering::SeqCst);

                // One button per bookmarked folder (max 12).
                let btn_class = to_wide("BUTTON");
                let mut handles: Vec<isize> = Vec::new();
                for (i, path) in paths.iter().enumerate() {
                    let label = button_label(path);
                    let label_w = to_wide(&label);
                    let btn = CreateWindowExW(
                        0,
                        btn_class.as_ptr(),
                        label_w.as_ptr(),
                        WS_CHILD | WS_VISIBLE,
                        0,
                        (i as i32) * 36,
                        BAR_WIDTH,
                        36,
                        bar,
                        (BUTTON_ID_BASE + i) as HMENU,
                        hinstance,
                        std::ptr::null(),
                    );
                    if !btn.is_null() {
                        SendMessageW(
                            btn,
                            WM_SETFONT,
                            GetStockObject(DEFAULT_GUI_FONT) as WPARAM,
                            1,
                        );
                        handles.push(btn as isize);
                    }
                }
                *buttons_lock() = handles;
                layout_buttons(BAR_WIDTH, 240);

                // Make sure this thread owns a message queue before stop()
                // could ever post to it.
                let mut peek: MSG = std::mem::zeroed();
                PeekMessageW(&mut peek, std::ptr::null_mut(), 0, 0, 0);

                if RUNNING.load(Ordering::SeqCst) {
                    hook_fg = SetWinEventHook(
                        EVENT_SYSTEM_FOREGROUND,
                        EVENT_SYSTEM_FOREGROUND,
                        std::ptr::null_mut(),
                        Some(on_winevent),
                        0,
                        0,
                        WINEVENT_OUTOFCONTEXT,
                    );
                    hook_move = SetWinEventHook(
                        EVENT_OBJECT_LOCATIONCHANGE,
                        EVENT_OBJECT_LOCATIONCHANGE,
                        std::ptr::null_mut(),
                        Some(on_winevent),
                        0,
                        0,
                        WINEVENT_OUTOFCONTEXT,
                    );

                    let mut msg: MSG = std::mem::zeroed();
                    while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                        TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                }
            }
        }
    }

    if !hook_fg.is_null() {
        UnhookWinEvent(hook_fg);
    }
    if !hook_move.is_null() {
        UnhookWinEvent(hook_move);
    }
    if !bar.is_null() {
        DestroyWindow(bar);
    }
    BAR_HWND.store(0, Ordering::SeqCst);
    CURRENT_DIALOG.store(0, Ordering::SeqCst);
    THREAD_ID.store(0, Ordering::SeqCst);
    folders_lock().clear();
    buttons_lock().clear();
    RUNNING.store(false, Ordering::SeqCst);
}

#[cfg(windows)]
fn button_label(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string())
}

/// WinEvent callback (arrives on the companion thread). OBJID_WINDOW == 0.
#[cfg(windows)]
unsafe extern "system" fn on_winevent(
    _hook: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    id_object: i32,
    _id_child: i32,
    _thread: u32,
    _time_ms: u32,
) {
    if hwnd.is_null() || id_object != 0 {
        return;
    }
    match event {
        EVENT_SYSTEM_FOREGROUND => {
            if is_dialog_window(hwnd) {
                CURRENT_DIALOG.store(hwnd as isize, Ordering::SeqCst);
                dock_bar(hwnd);
            } else {
                CURRENT_DIALOG.store(0, Ordering::SeqCst);
                hide_bar();
            }
        }
        EVENT_OBJECT_LOCATIONCHANGE => {
            let current = CURRENT_DIALOG.load(Ordering::SeqCst);
            if current == 0 {
                return;
            }
            if hwnd as isize == current {
                // Follow the dialog as it moves or resizes.
                if IsWindow(hwnd) != 0 {
                    dock_bar(hwnd);
                } else {
                    CURRENT_DIALOG.store(0, Ordering::SeqCst);
                    hide_bar();
                }
            } else if IsWindow(current as HWND) == 0 {
                // The tracked dialog went away entirely.
                CURRENT_DIALOG.store(0, Ordering::SeqCst);
                hide_bar();
            }
        }
        _ => {}
    }
}

/// Enumerates the dialog's child class names and runs the pure classifier.
#[cfg(windows)]
unsafe fn is_dialog_window(hwnd: HWND) -> bool {
    let mut buf = [0u16; 256];
    let n = GetClassNameW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
    if n <= 0 {
        return false;
    }
    let class = String::from_utf16_lossy(&buf[..n as usize]);
    let mut children: Vec<String> = Vec::new();
    EnumChildWindows(
        hwnd,
        Some(collect_class_proc),
        &mut children as *mut Vec<String> as LPARAM,
    );
    let child_refs: Vec<&str> = children.iter().map(String::as_str).collect();
    is_file_dialog(&class, &child_refs)
}

/// EnumChildWindows callback collecting child class names.
#[cfg(windows)]
unsafe extern "system" fn collect_class_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let list = &mut *(lparam as *mut Vec<String>);
    let mut buf = [0u16; 256];
    let n = GetClassNameW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
    if n > 0 {
        list.push(String::from_utf16_lossy(&buf[..n as usize]));
    }
    1 // continue enumeration
}

/// Moves and shows the bar docked beside the dialog (right, else left).
#[cfg(windows)]
unsafe fn dock_bar(dialog: HWND) {
    let bar = BAR_HWND.load(Ordering::SeqCst);
    if bar == 0 {
        return;
    }
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    if GetWindowRect(dialog, &mut rect) == 0 {
        return;
    }
    let dialog_rect = (
        rect.left,
        rect.top,
        rect.right - rect.left,
        rect.bottom - rect.top,
    );
    let screen = (
        0,
        0,
        GetSystemMetrics(SM_CXSCREEN),
        GetSystemMetrics(SM_CYSCREEN),
    );
    let (x, y, w, h) = dock_rect(dialog_rect, BAR_WIDTH, screen);
    SetWindowPos(
        bar as HWND,
        HWND_TOPMOST,
        x,
        y,
        w,
        h,
        SWP_NOACTIVATE | SWP_SHOWWINDOW,
    );
}

/// Hides the bar without activating it.
#[cfg(windows)]
unsafe fn hide_bar() {
    let bar = BAR_HWND.load(Ordering::SeqCst);
    if bar == 0 {
        return;
    }
    SetWindowPos(
        bar as HWND,
        std::ptr::null_mut(),
        0,
        0,
        0,
        0,
        SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_HIDEWINDOW | SWP_NOACTIVATE,
    );
}

/// Stacks the folder buttons evenly across the bar's client area.
#[cfg(windows)]
unsafe fn layout_buttons(cx: i32, cy: i32) {
    let buttons = buttons_lock().clone();
    if buttons.is_empty() || cx <= 0 || cy <= 0 {
        return;
    }
    let band = cy / buttons.len() as i32;
    if band <= 0 {
        return;
    }
    for (i, bh) in buttons.iter().enumerate() {
        MoveWindow(*bh as HWND, 0, (i as i32) * band, cx, band, 1);
    }
}

/// Bar window procedure: layout on resize, folder navigation on click.
#[cfg(windows)]
unsafe extern "system" fn companion_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_SIZE => {
            let cx = (lparam & 0xFFFF) as i32;
            let cy = ((lparam >> 16) & 0xFFFF) as i32;
            layout_buttons(cx, cy);
            0
        }
        WM_COMMAND => {
            let id = wparam & 0xFFFF;
            let folder = folders_lock()
                .iter()
                .find(|(fid, _)| *fid == id)
                .map(|(_, p)| p.clone());
            if let Some(folder) = folder {
                let dialog = CURRENT_DIALOG.load(Ordering::SeqCst);
                if dialog != 0 && IsWindow(dialog as HWND) != 0 {
                    navigate_file_dialog(dialog as HWND, &folder);
                }
            }
            0
        }
        WM_DESTROY => 0,
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// Navigates a file dialog to `folder`: WM_SETTEXT the path into the
/// filename Edit (Edit inside ComboBoxEx32; classic fallback ids 1148/1152),
/// then send IDOK (WM_COMMAND, IDOK) to the dialog itself.
#[cfg(windows)]
unsafe fn navigate_file_dialog(dialog: HWND, folder: &Path) -> bool {
    let class_combo = to_wide("ComboBoxEx32");
    let class_edit = to_wide("Edit");
    let mut edit: HWND = std::ptr::null_mut();
    let combo = FindWindowExW(
        dialog,
        std::ptr::null_mut(),
        class_combo.as_ptr(),
        std::ptr::null(),
    );
    if !combo.is_null() {
        edit = FindWindowExW(
            combo,
            std::ptr::null_mut(),
            class_edit.as_ptr(),
            std::ptr::null(),
        );
    }
    if edit.is_null() {
        edit = GetDlgItem(dialog, 1148);
    }
    if edit.is_null() {
        edit = GetDlgItem(dialog, 1152);
    }
    if edit.is_null() {
        return false;
    }
    let path = dunce::canonicalize(folder).unwrap_or_else(|_| folder.to_path_buf());
    let text = to_wide(&path.to_string_lossy());
    SendMessageW(edit, WM_SETTEXT, 0, text.as_ptr() as LPARAM);
    PostMessageW(dialog, WM_COMMAND, IDOK as WPARAM, 0);
    true
}
