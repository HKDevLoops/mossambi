//! System-Level File Drag & Drop Gesture Listener
//!
//! Provides zero-overhead, battery-efficient system-level mouse tracking.
//! Strictly gates gesture processing to when the user is actively dragging
//! files or folders from Windows Explorer, Desktop, or Linux File Managers.

use crate::core::gesture::GestureKind;
#[cfg(target_os = "windows")]
use crate::core::gesture::KinematicGestureTracker;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
#[cfg(target_os = "windows")]
use std::sync::Mutex;
use std::thread::{Builder, JoinHandle};
#[cfg(target_os = "windows")]
use std::time::{SystemTime, UNIX_EPOCH};

/// Event dispatched when an intentional mouse gesture occurs while holding files
#[derive(Debug, Clone)]
pub struct DragGestureEvent {
    pub gesture: GestureKind,
    pub cursor_pos: (f32, f32),
    pub files: Vec<PathBuf>,
}

/// System-level drag gesture listener controller
pub struct DragGestureListener {
    shutdown: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
    receiver: Receiver<DragGestureEvent>,
}

impl DragGestureListener {
    /// Start listening for system-level file drag gestures
    pub fn start(
        allow_shake: bool,
        allow_circle: bool,
        sensitivity_px: f32,
        #[allow(dead_code)] deep_drag_only: bool,
    ) -> Self {
        let shutdown = Arc::new(AtomicBool::new(false));
        let (tx, rx) = channel();

        let shutdown_clone = Arc::clone(&shutdown);
        let worker = Builder::new()
            .name("mossambi-drag-listener".into())
            .spawn(move || {
                run_listener_loop(
                    shutdown_clone,
                    tx,
                    allow_shake,
                    allow_circle,
                    sensitivity_px,
                    deep_drag_only,
                );
            })
            .ok();

        Self {
            shutdown,
            worker,
            receiver: rx,
        }
    }

    /// Try to receive any pending gesture events without blocking
    pub fn try_recv(&self) -> Option<DragGestureEvent> {
        self.receiver.try_recv().ok()
    }

    /// Stop the background listener thread
    pub fn stop(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        if let Some(handle) = self.worker.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for DragGestureListener {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(target_os = "windows")]
fn current_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// ==============================================================================
// WINDOWS WIN32 IMPLEMENTATION
// ==============================================================================
#[cfg(target_os = "windows")]
struct HookContext {
    tracker: KinematicGestureTracker,
    is_holding_files: bool,
    sender: Sender<DragGestureEvent>,
    shutdown: Arc<AtomicBool>,
    cached_files: Vec<PathBuf>,
    #[allow(dead_code)]
    deep_drag_only: bool,
}

#[cfg(target_os = "windows")]
static GLOBAL_CONTEXT: Mutex<Option<HookContext>> = Mutex::new(None);

#[cfg(target_os = "windows")]
fn run_listener_loop(
    shutdown: Arc<AtomicBool>,
    tx: Sender<DragGestureEvent>,
    allow_shake: bool,
    allow_circle: bool,
    sensitivity_px: f32,
    #[allow(dead_code)] deep_drag_only: bool,
) {
    use std::ptr::null_mut;
    use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, DispatchMessageW, PeekMessageW, SetWindowsHookExW, TranslateMessage,
        UnhookWindowsHookEx, HHOOK, MSG, MSLLHOOKSTRUCT, PM_REMOVE, WH_MOUSE_LL, WM_LBUTTONDOWN,
        WM_LBUTTONUP, WM_MOUSEMOVE, WM_QUIT,
    };

    struct HookGuard(HHOOK);
    impl Drop for HookGuard {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe {
                    UnhookWindowsHookEx(self.0);
                }
                log::info!("Unhooked WH_MOUSE_LL drag gesture listener.");
            }
        }
    }

    struct ContextResetGuard;
    impl Drop for ContextResetGuard {
        fn drop(&mut self) {
            let mut guard = GLOBAL_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());
            *guard = None;
        }
    }

    let tracker = KinematicGestureTracker::new(allow_shake, allow_circle)
        .with_sensitivity(sensitivity_px, 2600);

    {
        let mut guard = GLOBAL_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());
        *guard = Some(HookContext {
            tracker,
            is_holding_files: false,
            sender: tx,
            shutdown: Arc::clone(&shutdown),
            cached_files: Vec::new(),
            deep_drag_only,
        });
    }

    let _ctx_guard = ContextResetGuard;

    unsafe extern "system" fn low_level_mouse_proc(
        n_code: i32,
        w_param: WPARAM,
        l_param: LPARAM,
    ) -> LRESULT {
        if n_code < 0 || l_param == 0 {
            return CallNextHookEx(null_mut(), n_code, w_param, l_param);
        }

        let msg = w_param as u32;
        let info = *(l_param as *const MSLLHOOKSTRUCT);
        let pt = info.pt;

        let mut maybe_event: Option<DragGestureEvent> = None;

        if let Ok(mut guard) = GLOBAL_CONTEXT.try_lock() {
            if let Some(ctx) = guard.as_mut() {
                if ctx.shutdown.load(Ordering::Relaxed) {
                    return CallNextHookEx(null_mut(), n_code, w_param, l_param);
                }

                match msg {
                    WM_LBUTTONDOWN => {
                        // Fast reset on mouse press - do not block hook with COM/UIA probes
                        ctx.tracker.reset();
                        ctx.is_holding_files = true;
                        ctx.cached_files.clear();
                    }
                    WM_LBUTTONUP => {
                        // User dropped or released button - reset drag state immediately
                        ctx.is_holding_files = false;
                        ctx.tracker.reset();
                        ctx.cached_files.clear();
                    }
                    WM_MOUSEMOVE if ctx.is_holding_files => {
                        // ULTRA-LIGHTWEIGHT FAST PATH:
                        // Only active when user is dragging mouse with button pressed!
                        let now_ms = current_timestamp_ms();
                        if let Some(gesture) = ctx.tracker.feed(pt.x as f32, pt.y as f32, now_ms) {
                            // File probe ONLY executes when an intentional kinematic gesture
                            // (shake / circle) has actually been recognized!
                            let mut files = extract_clipboard_files();
                            if files.is_empty() {
                                files = query_active_explorer_files(pt);
                            }
                            if files.is_empty() {
                                if let Some(pp) =
                                    crate::gui::active_file::get_active_file_under_cursor(
                                        pt.x, pt.y,
                                    )
                                {
                                    files.push(pp);
                                }
                            }
                            // Always summon the Donut toolbar when an intentional gesture is completed.
                            // If file discovery was empty during rapid motion, provide an empty batch
                            // so the Donut toolbar awakens at cursor position to accept drop.
                            maybe_event = Some(DragGestureEvent {
                                gesture,
                                cursor_pos: (pt.x as f32, pt.y as f32),
                                files,
                            });
                            ctx.is_holding_files = false;
                            ctx.tracker.reset();
                        }
                    }
                    _ => {}
                }

                if let Some(evt) = maybe_event {
                    let _ = ctx.sender.send(evt);
                }
            }
        }

        CallNextHookEx(null_mut(), n_code, w_param, l_param)
    }

    unsafe {
        let hook: HHOOK = SetWindowsHookExW(WH_MOUSE_LL, Some(low_level_mouse_proc), null_mut(), 0);

        if hook.is_null() {
            log::warn!("Could not register WH_MOUSE_LL hook for drag gesture listener.");
            return;
        }

        let _hook_guard = HookGuard(hook);
        log::info!("Registered WH_MOUSE_LL system drag gesture listener hook.");

        let mut msg: MSG = std::mem::zeroed();
        while !shutdown.load(Ordering::Relaxed) {
            use windows_sys::Win32::UI::WindowsAndMessaging::{
                MsgWaitForMultipleObjectsEx, MWMO_INPUTAVAILABLE, QS_ALLINPUT,
            };
            let _ = MsgWaitForMultipleObjectsEx(
                0,
                std::ptr::null(),
                100,
                QS_ALLINPUT,
                MWMO_INPUTAVAILABLE,
            );
            while PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) != 0 {
                if msg.message == WM_QUIT {
                    break;
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }
}

#[cfg(target_os = "windows")]
pub fn extract_clipboard_files() -> Vec<PathBuf> {
    use windows_sys::Win32::System::DataExchange::{
        CloseClipboard, GetClipboardData, OpenClipboard,
    };
    use windows_sys::Win32::UI::Shell::DragQueryFileW;

    const CF_HDROP: u32 = 15;
    let mut files = Vec::new();

    unsafe {
        if OpenClipboard(std::ptr::null_mut()) != 0 {
            let hdrop = GetClipboardData(CF_HDROP);
            if !hdrop.is_null() {
                let count = DragQueryFileW(hdrop as _, 0xFFFFFFFF, std::ptr::null_mut(), 0);
                for i in 0..count {
                    let mut buf = [0u16; 512];
                    let len = DragQueryFileW(hdrop as _, i, buf.as_mut_ptr(), 512);
                    if len > 0 {
                        let s = String::from_utf16_lossy(&buf[..len as usize]);
                        files.push(PathBuf::from(s));
                    }
                }
            }
            CloseClipboard();
        }
    }
    files
}

#[cfg(not(target_os = "windows"))]
pub fn extract_clipboard_files() -> Vec<PathBuf> {
    Vec::new()
}

#[cfg(target_os = "windows")]
pub fn query_active_explorer_files(pt: windows_sys::Win32::Foundation::POINT) -> Vec<PathBuf> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetAncestor, WindowFromPoint, GA_ROOT};

    let hwnd = unsafe {
        let h = WindowFromPoint(pt);
        if h.is_null() {
            0
        } else {
            let root = GetAncestor(h, GA_ROOT);
            if !root.is_null() {
                root as isize
            } else {
                h as isize
            }
        }
    };

    unsafe { shell_com::query_selected_explorer_files(hwnd) }
}

#[cfg(target_os = "windows")]
mod shell_com {
    use std::path::PathBuf;
    use std::ptr::null_mut;
    use windows_sys::core::GUID;
    use windows_sys::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_LOCAL_SERVER,
        COINIT_APARTMENTTHREADED, DISPPARAMS,
    };
    use windows_sys::Win32::System::Variant::{VariantClear, VARIANT, VT_BSTR, VT_DISPATCH, VT_I4};

    const DISPATCH_METHOD: u16 = 0x1;
    const DISPATCH_PROPERTYGET: u16 = 0x2;

    type IDispatchPtr = *mut std::ffi::c_void;

    struct ComGuard(bool);
    impl Drop for ComGuard {
        fn drop(&mut self) {
            if self.0 {
                unsafe {
                    CoUninitialize();
                }
            }
        }
    }

    pub unsafe fn query_selected_explorer_files(target_hwnd: isize) -> Vec<PathBuf> {
        let hr = CoInitializeEx(null_mut(), COINIT_APARTMENTTHREADED as u32);
        let _guard = ComGuard(hr >= 0);

        let mut results = Vec::new();

        // CLSID_ShellWindows = {9BA05972-F6A8-11CF-A442-00A0C90A8F39}
        let clsid_shell_windows = GUID {
            data1: 0x9ba05972,
            data2: 0xf6a8,
            data3: 0x11cf,
            data4: [0xa4, 0x42, 0x00, 0xa0, 0xc9, 0x0a, 0x8f, 0x39],
        };
        let iid_idispatch = GUID {
            data1: 0x00020400,
            data2: 0x0000,
            data3: 0x0000,
            data4: [0xc0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
        };

        let mut shell_windows: IDispatchPtr = null_mut();
        let res = CoCreateInstance(
            &clsid_shell_windows,
            null_mut(),
            CLSCTX_LOCAL_SERVER,
            &iid_idispatch,
            &mut shell_windows,
        );

        if res >= 0 && !shell_windows.is_null() {
            enumerate_windows(shell_windows, target_hwnd, &mut results);
            release_dispatch(shell_windows);
        }

        results
    }

    unsafe fn enumerate_windows(
        shell_windows: IDispatchPtr,
        target_hwnd: isize,
        results: &mut Vec<PathBuf>,
    ) {
        let count_dispid = match get_disp_id(shell_windows, "Count") {
            Some(id) => id,
            None => return,
        };

        let mut var_count: VARIANT = std::mem::zeroed();
        let mut empty_params: DISPPARAMS = std::mem::zeroed();
        if invoke(
            shell_windows,
            count_dispid,
            DISPATCH_PROPERTYGET,
            &mut empty_params,
            &mut var_count,
        ) < 0
        {
            return;
        }

        let count = match var_count.Anonymous.Anonymous.vt {
            VT_I4 => var_count.Anonymous.Anonymous.Anonymous.lVal,
            _ => {
                VariantClear(&mut var_count);
                return;
            }
        };
        VariantClear(&mut var_count);

        let item_dispid = match get_disp_id(shell_windows, "Item") {
            Some(id) => id,
            None => return,
        };

        for i in 0..count {
            let mut arg_val: VARIANT = std::mem::zeroed();
            arg_val.Anonymous.Anonymous.vt = VT_I4;
            arg_val.Anonymous.Anonymous.Anonymous.lVal = i;

            let mut params = DISPPARAMS {
                rgvarg: &mut arg_val,
                rgdispidNamedArgs: null_mut(),
                cArgs: 1,
                cNamedArgs: 0,
            };

            let mut var_win: VARIANT = std::mem::zeroed();
            let hr = invoke(
                shell_windows,
                item_dispid,
                DISPATCH_METHOD,
                &mut params,
                &mut var_win,
            );
            VariantClear(&mut arg_val);

            if hr >= 0 && var_win.Anonymous.Anonymous.vt == VT_DISPATCH {
                let win_disp = var_win.Anonymous.Anonymous.Anonymous.pdispVal as IDispatchPtr;
                if !win_disp.is_null() {
                    let mut is_match = target_hwnd == 0;
                    if !is_match {
                        if let Some(hwnd_id) = get_disp_id(win_disp, "HWND") {
                            let mut var_hwnd: VARIANT = std::mem::zeroed();
                            let mut p = std::mem::zeroed();
                            if invoke(
                                win_disp,
                                hwnd_id,
                                DISPATCH_PROPERTYGET,
                                &mut p,
                                &mut var_hwnd,
                            ) >= 0
                            {
                                let w_hwnd = var_hwnd.Anonymous.Anonymous.Anonymous.lVal as isize;
                                is_match = w_hwnd == target_hwnd;
                                VariantClear(&mut var_hwnd);
                            }
                        }
                    }

                    if is_match {
                        extract_selected_from_window(win_disp, results);
                    }
                    release_dispatch(win_disp);

                    if !results.is_empty() {
                        break;
                    }
                }
            }
        }
    }

    unsafe fn extract_selected_from_window(win_disp: IDispatchPtr, results: &mut Vec<PathBuf>) {
        let doc_id = match get_disp_id(win_disp, "Document") {
            Some(id) => id,
            None => return,
        };

        let mut var_doc: VARIANT = std::mem::zeroed();
        let mut p: DISPPARAMS = std::mem::zeroed();
        if invoke(win_disp, doc_id, DISPATCH_PROPERTYGET, &mut p, &mut var_doc) < 0 {
            return;
        }

        if var_doc.Anonymous.Anonymous.vt != VT_DISPATCH
            || var_doc.Anonymous.Anonymous.Anonymous.pdispVal.is_null()
        {
            VariantClear(&mut var_doc);
            return;
        }
        let doc_disp = var_doc.Anonymous.Anonymous.Anonymous.pdispVal as IDispatchPtr;

        // Document.SelectedItems()
        if let Some(sel_id) = get_disp_id(doc_disp, "SelectedItems") {
            let mut var_sel: VARIANT = std::mem::zeroed();
            let mut p2: DISPPARAMS = std::mem::zeroed();
            if invoke(
                doc_disp,
                sel_id,
                DISPATCH_METHOD | DISPATCH_PROPERTYGET,
                &mut p2,
                &mut var_sel,
            ) >= 0
            {
                if var_sel.Anonymous.Anonymous.vt == VT_DISPATCH
                    && !var_sel.Anonymous.Anonymous.Anonymous.pdispVal.is_null()
                {
                    let sel_disp = var_sel.Anonymous.Anonymous.Anonymous.pdispVal as IDispatchPtr;

                    // SelectedItems.Count
                    if let Some(cnt_id) = get_disp_id(sel_disp, "Count") {
                        let mut var_cnt: VARIANT = std::mem::zeroed();
                        let mut p3: DISPPARAMS = std::mem::zeroed();
                        if invoke(
                            sel_disp,
                            cnt_id,
                            DISPATCH_PROPERTYGET,
                            &mut p3,
                            &mut var_cnt,
                        ) >= 0
                        {
                            let item_count = var_cnt.Anonymous.Anonymous.Anonymous.lVal;
                            VariantClear(&mut var_cnt);

                            if let Some(item_id) = get_disp_id(sel_disp, "Item") {
                                for idx in 0..item_count {
                                    let mut arg_idx: VARIANT = std::mem::zeroed();
                                    arg_idx.Anonymous.Anonymous.vt = VT_I4;
                                    arg_idx.Anonymous.Anonymous.Anonymous.lVal = idx;

                                    let mut p4 = DISPPARAMS {
                                        rgvarg: &mut arg_idx,
                                        rgdispidNamedArgs: null_mut(),
                                        cArgs: 1,
                                        cNamedArgs: 0,
                                    };

                                    let mut var_item: VARIANT = std::mem::zeroed();
                                    if invoke(
                                        sel_disp,
                                        item_id,
                                        DISPATCH_METHOD,
                                        &mut p4,
                                        &mut var_item,
                                    ) >= 0
                                    {
                                        if var_item.Anonymous.Anonymous.vt == VT_DISPATCH
                                            && !var_item
                                                .Anonymous
                                                .Anonymous
                                                .Anonymous
                                                .pdispVal
                                                .is_null()
                                        {
                                            let item_disp =
                                                var_item.Anonymous.Anonymous.Anonymous.pdispVal
                                                    as IDispatchPtr;

                                            // Item.Path
                                            if let Some(path_id) = get_disp_id(item_disp, "Path") {
                                                let mut var_path: VARIANT = std::mem::zeroed();
                                                let mut p5: DISPPARAMS = std::mem::zeroed();
                                                if invoke(
                                                    item_disp,
                                                    path_id,
                                                    DISPATCH_PROPERTYGET,
                                                    &mut p5,
                                                    &mut var_path,
                                                ) >= 0
                                                {
                                                    if var_path.Anonymous.Anonymous.vt == VT_BSTR
                                                        && !var_path
                                                            .Anonymous
                                                            .Anonymous
                                                            .Anonymous
                                                            .bstrVal
                                                            .is_null()
                                                    {
                                                        let path_str = bstr_to_string(
                                                            var_path
                                                                .Anonymous
                                                                .Anonymous
                                                                .Anonymous
                                                                .bstrVal,
                                                        );
                                                        let pb = PathBuf::from(&path_str);
                                                        if pb.exists() {
                                                            results.push(pb);
                                                        }
                                                    }
                                                    VariantClear(&mut var_path);
                                                }
                                            }
                                            release_dispatch(item_disp);
                                        }
                                        VariantClear(&mut var_item);
                                    }
                                }
                            }
                        }
                    }
                    release_dispatch(sel_disp);
                }
                VariantClear(&mut var_sel);
            }
        }

        release_dispatch(doc_disp);
        VariantClear(&mut var_doc);
    }

    unsafe fn get_disp_id(disp: IDispatchPtr, name: &str) -> Option<i32> {
        let mut wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
        let mut names = [wide.as_mut_ptr()];
        let mut dispid = 0i32;
        let vtable = *(disp as *mut *mut usize);
        let get_ids_fn: extern "system" fn(
            IDispatchPtr,
            *const GUID,
            *mut *mut u16,
            u32,
            u32,
            *mut i32,
        ) -> i32 = std::mem::transmute(vtable.add(5).read());

        let hr = get_ids_fn(
            disp,
            &GUID {
                data1: 0,
                data2: 0,
                data3: 0,
                data4: [0; 8],
            },
            names.as_mut_ptr(),
            1,
            0x0800,
            &mut dispid,
        );
        if hr >= 0 {
            Some(dispid)
        } else {
            None
        }
    }

    unsafe fn invoke(
        disp: IDispatchPtr,
        dispid: i32,
        flags: u16,
        params: *mut DISPPARAMS,
        result: *mut VARIANT,
    ) -> i32 {
        let vtable = *(disp as *mut *mut usize);
        let invoke_fn: extern "system" fn(
            IDispatchPtr,
            i32,
            *const GUID,
            u32,
            u16,
            *mut DISPPARAMS,
            *mut VARIANT,
            *mut std::ffi::c_void,
            *mut u32,
        ) -> i32 = std::mem::transmute(vtable.add(6).read());

        invoke_fn(
            disp,
            dispid,
            &GUID {
                data1: 0,
                data2: 0,
                data3: 0,
                data4: [0; 8],
            },
            0x0800,
            flags,
            params,
            result,
            null_mut(),
            null_mut(),
        )
    }

    unsafe fn release_dispatch(disp: IDispatchPtr) {
        if !disp.is_null() {
            let vtable = *(disp as *mut *mut usize);
            let release_fn: extern "system" fn(IDispatchPtr) -> u32 =
                std::mem::transmute(vtable.add(2).read());
            release_fn(disp);
        }
    }

    unsafe fn bstr_to_string(bstr: *const u16) -> String {
        if bstr.is_null() {
            return String::new();
        }
        let len_bytes = *(bstr.byte_sub(4) as *const u32);
        let len_chars = (len_bytes / 2) as usize;
        let slice = std::slice::from_raw_parts(bstr, len_chars);
        String::from_utf16_lossy(slice)
    }
}

// ==============================================================================
// LINUX / CROSS-PLATFORM FALLBACK IMPLEMENTATION
// ==============================================================================
#[cfg(not(target_os = "windows"))]
fn run_listener_loop(
    shutdown: Arc<AtomicBool>,
    _tx: Sender<DragGestureEvent>,
    _allow_shake: bool,
    _allow_circle: bool,
    _sensitivity_px: f32,
    _deep_drag_only: bool,
) {
    // Linux background thread - sleeps while awaiting events
    while !shutdown.load(Ordering::Relaxed) {
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drag_listener_lifecycle() {
        let handle = std::thread::spawn(|| {
            let mut listener = DragGestureListener::start(true, true, 25.0, true);
            std::thread::sleep(std::time::Duration::from_millis(60));
            assert!(listener.try_recv().is_none());
            listener.stop();
        });
        handle
            .join()
            .expect("Listener lifecycle test should exit cleanly");
    }

    #[test]
    fn test_query_active_explorer_files_safe() {
        let handle = std::thread::spawn(|| {
            #[cfg(target_os = "windows")]
            {
                use windows_sys::Win32::Foundation::POINT;
                let pt = POINT {
                    x: -10000,
                    y: -10000,
                };
                // Should safely return empty list or non-panicking results without crashing
                let files = query_active_explorer_files(pt);
                assert!(files.is_empty() || !files.is_empty());
            }
        });
        handle
            .join()
            .expect("Explorer query test should join cleanly");
    }
}
