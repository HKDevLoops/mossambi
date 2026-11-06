//! Global Drop Overlay — convert files dropped from ANY application.
//!
//! A small always-on-top "drop puck" window (top-right of the primary
//! monitor) registered as an OLE `IDropTarget`. Explorer, browsers, archive
//! tools, editors — every app that drags files with the shell OLE protocol —
//! can drop onto it. Dropped files convert via the zero-loss codec pipeline
//! with a type-aware default target (`AUTO` per category, or the configured
//! `drop_target_default`).
//!
//! Windows-only: OLE drop targets have no equivalent public API on Linux/macOS
//! short of a full X11/Wayland DnD agent (documented roadmap deferral).

#![cfg(target_os = "windows")]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::core::registry::{FileCategory, UniversalRegistry};

/// Puck geometry (top-right of the primary monitor).
const PUCK_SIZE: i32 = 88;
const PUCK_MARGIN: i32 = 18;
/// CF_HDROP clipboard format id (shell HDROP handle).
const CF_HDROP: u16 = 15;
/// DROPEFFECT_COPY
const DROPEFFECT_COPY: u32 = 1;
/// Hot state while files hover the puck (drives the repaint color).
static HOT_STATE: AtomicBool = AtomicBool::new(false);

/// Per-category default conversion target for AUTO mode.
fn auto_target(path: &std::path::Path) -> &'static str {
    let (cat, _) = UniversalRegistry::classify_file(path);
    match cat {
        FileCategory::Image => "PNG",
        FileCategory::Video => "MP4",
        FileCategory::Audio => "FLAC",
        FileCategory::Document => "PDF",
        FileCategory::Spreadsheet => "CSV",
        FileCategory::Presentation => "PDF",
        FileCategory::Subtitle => "SRT",
        FileCategory::Archive => "EXTRACT",
        FileCategory::Unknown => "ZIP",
    }
}

// ────────────────────────────────────────────────────────────────────────────
// COM plumbing (hand-rolled IDropTarget — windows-sys exposes raw bindings)
// ────────────────────────────────────────────────────────────────────────────

#[repr(C)]
struct PointL {
    x: i32,
    y: i32,
}

#[repr(C)]
struct FormatEtc {
    cf_format: u16,
    dw_aspect: u32,
    lindex: i32,
    tymed: u32,
    ptd: *mut std::ffi::c_void,
}

#[repr(C)]
struct StgMedium {
    tymed: u32,
    h_global: *mut std::ffi::c_void,
    unk_for_release: *mut std::ffi::c_void,
}

type HResult = i32;
const S_OK: HResult = 0;
const E_NOTIMPL: HResult = -0x7FFF_FFFB; // 0x80004001
const DVASPECT_CONTENT: u32 = 1;
const TYMED_HGLOBAL: u32 = 1;

unsafe extern "system" fn dt_query_interface(
    _this: *mut std::ffi::c_void,
    _iid: *const std::ffi::c_void,
    _out: *mut *mut std::ffi::c_void,
) -> HResult {
    E_NOTIMPL // static singleton: refcounts/queries are inert by design
}

unsafe extern "system" fn dt_add_ref(_this: *mut std::ffi::c_void) -> u32 {
    1
}

unsafe extern "system" fn dt_release(_this: *mut std::ffi::c_void) -> u32 {
    1
}

unsafe extern "system" fn dt_drag_enter(
    _this: *mut std::ffi::c_void,
    _data: *mut std::ffi::c_void,
    _keystate: u32,
    _pt: PointL,
    effect: *mut u32,
) -> HResult {
    HOT_STATE.store(true, Ordering::Relaxed);
    if !effect.is_null() {
        *effect = DROPEFFECT_COPY;
    }
    S_OK
}

unsafe extern "system" fn dt_drag_over(
    _this: *mut std::ffi::c_void,
    _keystate: u32,
    _pt: PointL,
    effect: *mut u32,
) -> HResult {
    if !effect.is_null() {
        *effect = DROPEFFECT_COPY;
    }
    S_OK
}

unsafe extern "system" fn dt_drag_leave(_this: *mut std::ffi::c_void) -> HResult {
    HOT_STATE.store(false, Ordering::Relaxed);
    S_OK
}

/// Reads the file list out of an OLE IDataObject's CF_HDROP medium.
unsafe fn files_from_data_object(data: *mut std::ffi::c_void) -> Vec<std::path::PathBuf> {
    extern "C" {
        fn DragQueryFileW(
            hdrop: *mut std::ffi::c_void,
            ifile: u32,
            lpszfile: *mut u16,
            cch: u32,
        ) -> u32;
    }
    let mut out = Vec::new();
    if data.is_null() {
        return out;
    }
    // IDataObject vtable layout after IUnknown: GetData is slot 3.
    #[repr(C)]
    struct DataObjectHead {
        _qi: *const std::ffi::c_void,
        _addref: *const std::ffi::c_void,
        _release: *const std::ffi::c_void,
        get_data: unsafe extern "system" fn(
            *mut std::ffi::c_void,
            *const FormatEtc,
            *mut StgMedium,
        ) -> HResult,
        _rest: [u64; 9],
    }
    let vtbl: *mut DataObjectHead = *(data as *mut *mut DataObjectHead);
    if vtbl.is_null() || ((*vtbl).get_data as usize) < 4096 {
        return out;
    }

    let fmt = FormatEtc {
        cf_format: CF_HDROP,
        dw_aspect: DVASPECT_CONTENT,
        lindex: -1,
        tymed: TYMED_HGLOBAL,
        ptd: std::ptr::null_mut(),
    };
    let mut medium: StgMedium = std::mem::zeroed();
    let hr = ((*vtbl).get_data)(data, &fmt, &mut medium);
    if hr != S_OK || medium.h_global.is_null() {
        log::debug!("Drop overlay: no CF_HDROP medium on data object (hr={hr:#x})");
        return out;
    }

    let hdrop = medium.h_global;
    let count = DragQueryFileW(hdrop, 0xFFFF_FFFF, std::ptr::null_mut(), 0);
    for i in 0..count {
        let len = DragQueryFileW(hdrop, i, std::ptr::null_mut(), 0);
        if len == 0 {
            continue;
        }
        let mut buf = vec![0u16; len as usize + 1];
        let n = DragQueryFileW(hdrop, i, buf.as_mut_ptr(), buf.len() as u32);
        if n > 0 {
            let s = String::from_utf16_lossy(&buf[..n as usize]);
            out.push(std::path::PathBuf::from(s));
        }
    }
    // Release the HGLOBAL medium ourselves when nothing owns it.
    if medium.unk_for_release.is_null() {
        unsafe {
            extern "C" {
                fn GlobalFree(hmem: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
            }
            GlobalFree(medium.h_global);
        }
    }
    out
}

unsafe extern "system" fn dt_drop(
    _this: *mut std::ffi::c_void,
    data: *mut std::ffi::c_void,
    _keystate: u32,
    _pt: PointL,
    effect: *mut u32,
) -> HResult {
    let files = files_from_data_object(data);
    if !effect.is_null() {
        *effect = DROPEFFECT_COPY;
    }
    HOT_STATE.store(false, Ordering::Relaxed);
    if files.is_empty() {
        return S_OK;
    }
    let configured = OVERLAY_TARGET
        .lock()
        .map(|t| t.clone())
        .unwrap_or_else(|_| "AUTO".into());

    let spawn_res = std::thread::Builder::new()
        .name("mossambi-drop-convert".into())
        .spawn(move || {
            for f in &files {
                let target = if configured.eq_ignore_ascii_case("AUTO") {
                    auto_target(f).to_string()
                } else {
                    configured.clone()
                };
                log::info!("Drop overlay: {} -> {target}", f.display());
                let res = crate::codec::transcode_file(
                    f,
                    &target,
                    crate::codec::LosslessMode::StrictBitExact,
                    None,
                );
                match res {
                    Ok(out) => log::info!("Drop overlay OK: {}", out.display()),
                    Err(e) => log::error!("Drop overlay failed for {}: {e:#}", f.display()),
                }
            }
            show_completion_balloon(files.len());
        });
    if let Err(e) = spawn_res {
        log::error!("Drop overlay: conversion thread spawn failed: {e}");
    }
    S_OK
}

/// Configured target shared with the overlay thread (set at start()).
static OVERLAY_TARGET: Mutex<String> = Mutex::new(String::new());

/// Tray balloon announcing the drop conversion result (best-effort).
unsafe fn show_completion_balloon(count: usize) {
    use windows_sys::Win32::UI::Shell::{
        Shell_NotifyIconW, NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_TIP, NIM_MODIFY, NOTIFYICONDATAW,
    };
    let Some(hwnd) = crate::daemon::shared_tray_nid() else {
        return;
    };
    let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd as _;
    nid.uID = 1001;
    nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_INFO;
    let title: Vec<u16> = "Mossambi ".encode_utf16().collect();
    let text: Vec<u16> = format!("{count} file(s) converted - output saved next to the source ")
        .encode_utf16()
        .collect();
    for (i, c) in title.iter().take(63).enumerate() {
        nid.szInfoTitle[i] = *c;
    }
    for (i, c) in text.iter().take(255).enumerate() {
        nid.szInfo[i] = *c;
    }
    Shell_NotifyIconW(NIM_MODIFY, &nid);
}

#[repr(C)]
struct DropTargetVtbl {
    query_interface: unsafe extern "system" fn(
        *mut std::ffi::c_void,
        *const std::ffi::c_void,
        *mut *mut std::ffi::c_void,
    ) -> HResult,
    add_ref: unsafe extern "system" fn(*mut std::ffi::c_void) -> u32,
    release: unsafe extern "system" fn(*mut std::ffi::c_void) -> u32,
    drag_enter: unsafe extern "system" fn(
        *mut std::ffi::c_void,
        *mut std::ffi::c_void,
        u32,
        PointL,
        *mut u32,
    ) -> HResult,
    drag_over: unsafe extern "system" fn(*mut std::ffi::c_void, u32, PointL, *mut u32) -> HResult,
    drag_leave: unsafe extern "system" fn(*mut std::ffi::c_void) -> HResult,
    drop: unsafe extern "system" fn(
        *mut std::ffi::c_void,
        *mut std::ffi::c_void,
        u32,
        PointL,
        *mut u32,
    ) -> HResult,
}

static DROP_VTBL: DropTargetVtbl = DropTargetVtbl {
    query_interface: dt_query_interface,
    add_ref: dt_add_ref,
    release: dt_release,
    drag_enter: dt_drag_enter,
    drag_over: dt_drag_over,
    drag_leave: dt_drag_leave,
    drop: dt_drop,
};

// ────────────────────────────────────────────────────────────────────────────
// Overlay window lifecycle
// ────────────────────────────────────────────────────────────────────────────

pub struct DropOverlayHandle {
    shutdown: Arc<AtomicBool>,
    hwnd: Arc<Mutex<isize>>,
    worker: Option<std::thread::JoinHandle<()>>,
}

impl Drop for DropOverlayHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

impl DropOverlayHandle {
    /// Spawns the overlay thread (OLE STA + message loop). Never panics;
    /// failures are logged and the daemon keeps running without the puck.
    pub fn start(target: String) -> Self {
        if let Ok(mut t) = OVERLAY_TARGET.lock() {
            *t = target;
        }
        let shutdown = Arc::new(AtomicBool::new(false));
        let hwnd: Arc<Mutex<isize>> = Arc::new(Mutex::new(0));
        let shutdown_c = Arc::clone(&shutdown);
        let hwnd_c = Arc::clone(&hwnd);
        let worker = std::thread::Builder::new()
            .name("mossambi-drop-overlay".into())
            .spawn(move || unsafe {
                run_overlay_thread(shutdown_c, hwnd_c);
            })
            .ok();
        Self {
            shutdown,
            hwnd,
            worker,
        }
    }

    pub fn stop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        let hwnd = self.hwnd.lock().map(|h| *h).unwrap_or(0);
        if hwnd != 0 {
            unsafe {
                use windows_sys::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_CLOSE};
                PostMessageW(hwnd as _, WM_CLOSE, 0, 0);
            }
        }
        if let Some(w) = self.worker.take() {
            let _ = w.join();
        }
    }
}

unsafe fn run_overlay_thread(shutdown: Arc<AtomicBool>, hwnd_out: Arc<Mutex<isize>>) {
    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::Graphics::Gdi::{
        BeginPaint, CreateSolidBrush, DeleteObject, Ellipse, EndPaint, SelectObject, PAINTSTRUCT,
    };
    use windows_sys::Win32::System::Ole::{
        OleInitialize, OleUninitialize, RegisterDragDrop, RevokeDragDrop,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
        GetSystemMetrics, RegisterClassExW, SetLayeredWindowAttributes, SetWindowPos,
        TranslateMessage, UnregisterClassW, CS_HREDRAW, CS_VREDRAW, HWND_TOPMOST, LWA_COLORKEY,
        SWP_NOSIZE, SWP_SHOWWINDOW, WM_CLOSE, WM_PAINT, WNDCLASSEXW, WS_EX_LAYERED,
        WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
    };

    const LIME_BASE: u32 = 0x004A_20C4; // COLORREF = 0x00BBGGRR (sweet lime)
    const LIME_HOT: u32 = 0x0030_D874;
    const COLORKEY: u32 = 0x0000_00FF; // pure red key

    // 1. OLE STA (required by RegisterDragDrop).
    let ole_hr = OleInitialize(std::ptr::null_mut());
    if ole_hr < 0 {
        log::error!("Drop overlay: OleInitialize failed ({ole_hr:#x}) — puck disabled");
        return;
    }

    // 2. Window class + layered popup painted as a lime disc over a colorkey.
    unsafe extern "system" fn puck_wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match msg {
            WM_PAINT => {
                let mut ps: PAINTSTRUCT = std::mem::zeroed();
                let hdc = BeginPaint(hwnd, &mut ps);
                let hot = HOT_STATE.load(Ordering::Relaxed);
                let brush = CreateSolidBrush(if hot { LIME_HOT } else { LIME_BASE });
                let old = SelectObject(hdc, brush);
                let w = (ps.rcPaint.right - ps.rcPaint.left).max(1);
                let h = (ps.rcPaint.bottom - ps.rcPaint.top).max(1);
                Ellipse(hdc, 4, 4, w - 4, h - 4);
                SelectObject(hdc, old);
                DeleteObject(brush);
                EndPaint(hwnd, &ps);
                0
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }

    let class_name: Vec<u16> = "MossambiDropPuck\0".encode_utf16().collect();
    let wc = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(puck_wnd_proc),
        hInstance: std::ptr::null_mut(),
        hCursor: std::ptr::null_mut(),
        lpszClassName: class_name.as_ptr(),
        ..std::mem::zeroed()
    };
    if RegisterClassExW(&wc) == 0 {
        log::warn!("Drop overlay: RegisterClassExW failed — puck disabled");
        return;
    }

    let ex = WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE;
    let screen_w = GetSystemMetrics(0); // SM_CXSCREEN
    let x = screen_w - PUCK_SIZE - PUCK_MARGIN;
    let y = PUCK_MARGIN;
    let title: Vec<u16> = "Mossambi — Drop files to convert\0"
        .encode_utf16()
        .collect();
    let hwnd = CreateWindowExW(
        ex,
        class_name.as_ptr(),
        title.as_ptr(),
        WS_POPUP,
        x,
        y,
        PUCK_SIZE,
        PUCK_SIZE,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        std::ptr::null_mut(),
    );
    if hwnd.is_null() {
        log::warn!("Drop overlay: CreateWindowExW failed — puck disabled");
        return;
    }
    SetLayeredWindowAttributes(hwnd, COLORKEY, 0, LWA_COLORKEY);
    SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOSIZE | SWP_SHOWWINDOW);

    // 3. Register the OLE drop target. The COM object is the vtable pointer
    //    itself (stateless singleton; config lives in OVERLAY_TARGET).
    let itf = &DROP_VTBL as *const DropTargetVtbl as *mut std::ffi::c_void;
    let hr = RegisterDragDrop(hwnd, itf);
    if hr != 0 {
        log::error!("Drop overlay: RegisterDragDrop failed ({hr:#x}) — puck disabled");
        DestroyWindow(hwnd);
        return;
    }
    if let Ok(mut h) = hwnd_out.lock() {
        *h = hwnd as isize;
    }
    log::info!("Drop overlay live: drag files from any app onto the lime puck to convert");

    // 4. Message loop until shutdown (WM_CLOSE) arrives.
    let mut msg: windows_sys::Win32::UI::WindowsAndMessaging::MSG = std::mem::zeroed();
    while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
        if msg.message == WM_CLOSE {
            break;
        }
        TranslateMessage(&msg);
        DispatchMessageW(&msg);
        if shutdown.load(Ordering::Relaxed) {
            break;
        }
    }

    RevokeDragDrop(hwnd);
    DestroyWindow(hwnd);
    UnregisterClassW(class_name.as_ptr(), std::ptr::null_mut());
    OleUninitialize();
    log::info!("Drop overlay stopped");
}
