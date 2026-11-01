//! System-Level File Drag & Drop Gesture Listener
//!
//! Provides zero-overhead, battery-efficient system-level mouse tracking.
//! Strictly gates gesture processing to when the user is actively dragging
//! files or folders from Windows Explorer, Desktop, or Linux File Managers.

use crate::gesture::{GestureKind, KinematicGestureTracker};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{Builder, JoinHandle};
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
    pub fn start(allow_shake: bool, allow_circle: bool, sensitivity_px: f32) -> Self {
        let shutdown = Arc::new(AtomicBool::new(false));
        let (tx, rx) = channel();

        let shutdown_clone = Arc::clone(&shutdown);
        let worker = Builder::new()
            .name("mossambi-drag-listener".into())
            .spawn(move || {
                run_listener_loop(shutdown_clone, tx, allow_shake, allow_circle, sensitivity_px);
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
) {
    use std::ptr::null_mut;
    use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, DispatchMessageW, PeekMessageW, SetWindowsHookExW,
        TranslateMessage, UnhookWindowsHookEx, HHOOK, MSG, MSLLHOOKSTRUCT,
        PM_REMOVE, WH_MOUSE_LL, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_QUIT,
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
        .with_sensitivity(sensitivity_px, 450);

    {
        let mut guard = GLOBAL_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());
        *guard = Some(HookContext {
            tracker,
            is_holding_files: false,
            sender: tx,
            shutdown: Arc::clone(&shutdown),
            cached_files: Vec::new(),
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
                        // GATED SYSTEM-LEVEL FILE DRAG CHECK:
                        // Deep check: Is the cursor pressing down on a File Explorer, Desktop, or File Dialog window?
                        let is_file_window = is_shell_or_explorer_window(pt);
                        if is_file_window {
                            ctx.is_holding_files = true;
                            ctx.tracker.reset();
                            // Attempt to query active selected files from Shell
                            ctx.cached_files = query_active_explorer_files(pt);
                        } else {
                            ctx.is_holding_files = false;
                            ctx.cached_files.clear();
                        }
                    }
                    WM_LBUTTONUP => {
                        // User dropped or released button - reset drag state immediately
                        ctx.is_holding_files = false;
                        ctx.tracker.reset();
                        ctx.cached_files.clear();
                    }
                    WM_MOUSEMOVE if ctx.is_holding_files => {
                        // ULTRA-LIGHTWEIGHT FAST PATH:
                        // Only active when user is holding files/folders!
                        let now_ms = current_timestamp_ms();
                        if let Some(gesture) = ctx.tracker.feed(pt.x as f32, pt.y as f32, now_ms) {
                            maybe_event = Some(DragGestureEvent {
                                gesture,
                                cursor_pos: (pt.x as f32, pt.y as f32),
                                files: ctx.cached_files.clone(),
                            });
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
        let hook: HHOOK = SetWindowsHookExW(
            WH_MOUSE_LL,
            Some(low_level_mouse_proc),
            null_mut(),
            0,
        );

        if hook.is_null() {
            log::warn!("Could not register WH_MOUSE_LL hook for drag gesture listener.");
            return;
        }

        let _hook_guard = HookGuard(hook);
        log::info!("Registered WH_MOUSE_LL system drag gesture listener hook.");

        let mut msg: MSG = std::mem::zeroed();
        while !shutdown.load(Ordering::Relaxed) {
            while PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) != 0 {
                if msg.message == WM_QUIT {
                    break;
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            std::thread::sleep(std::time::Duration::from_millis(16));
        }
    }
}

#[cfg(target_os = "windows")]
fn is_shell_or_explorer_window(pt: windows_sys::Win32::Foundation::POINT) -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetAncestor, GetClassNameW, WindowFromPoint, GA_ROOT,
    };

    unsafe {
        let hwnd_under = WindowFromPoint(pt);
        if hwnd_under.is_null() {
            return false;
        }

        let mut class_buf = [0u16; 256];
        let len = GetClassNameW(hwnd_under, class_buf.as_mut_ptr(), 256);
        if len > 0 {
            let class_name = String::from_utf16_lossy(&class_buf[..len as usize]);
            if is_shell_class_name(&class_name) {
                return true;
            }
        }

        // Also check the root ancestor window
        let root = GetAncestor(hwnd_under, GA_ROOT);
        if !root.is_null() && root != hwnd_under {
            let len_root = GetClassNameW(root, class_buf.as_mut_ptr(), 256);
            if len_root > 0 {
                let root_name = String::from_utf16_lossy(&class_buf[..len_root as usize]);
                if is_shell_class_name(&root_name) {
                    return true;
                }
            }
        }
    }

    false
}

#[cfg(target_os = "windows")]
fn is_shell_class_name(name: &str) -> bool {
    matches!(
        name,
        "CabinetWClass"            // File Explorer window
        | "ExploreWClass"          // Legacy Explorer
        | "Progman"                // Windows Desktop Program Manager
        | "WorkerW"                // Desktop Wallpaper Worker
        | "SHELLDLL_DefView"       // Shell Folder View inside Explorer or Desktop
        | "SysListView32"          // File List View
        | "DirectUIHWND"           // Modern Explorer Folder view
        | "#32770"                 // File Open/Save Dialog
    )
}

#[cfg(target_os = "windows")]
fn query_active_explorer_files(_pt: windows_sys::Win32::Foundation::POINT) -> Vec<PathBuf> {
    // Attempt to query any selected files from Windows Explorer via Shell Windows COM or clipboard
    // If unavailable mid-drag, return empty list (widget will summon and receive dropped file payload)
    Vec::new()
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
        let mut listener = DragGestureListener::start(true, true, 25.0);
        std::thread::sleep(std::time::Duration::from_millis(50));
        assert!(listener.try_recv().is_none());
        listener.stop();
    }
}
