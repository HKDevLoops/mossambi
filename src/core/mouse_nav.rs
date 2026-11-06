//! System-wide mouse side-button & horizontal-swipe navigation listener
//!
//! Exposes WH_MOUSE_LL as an owned thread so the Studio never loses XBUTTON
//! (thumb buttons) or a middle-button horizontal swipe. Foreground gating keeps
//! other apps unaffected.
//!
//! Gestures recognised (when Mossambi is the foreground window):
//!   • Mouse thumb Back  → go_back()
//!   • Mouse thumb Fwd   → go_forward()
//!   • Long horizontal middle-drag (mouse swipe) → Back / Forward

use std::sync::atomic::AtomicBool;
#[cfg(target_os = "windows")]
use std::sync::atomic::Ordering;
use std::sync::mpsc::{channel, Receiver};
#[cfg(target_os = "windows")]
use std::sync::mpsc::Sender;
use std::sync::Arc;
#[cfg(target_os = "windows")]
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseNavEvent {
    Back,
    Forward,
}

pub struct MouseNavListener {
    receiver: Receiver<MouseNavEvent>,
    shutdown: Arc<AtomicBool>,
}

impl Default for MouseNavListener {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for MouseNavListener {
    fn drop(&mut self) {
        self.shutdown
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

impl MouseNavListener {
    pub fn new() -> Self {
        let (sender, receiver) = channel();
        let shutdown = Arc::new(AtomicBool::new(false));

        #[cfg(target_os = "windows")]
        {
            let shutdown_clone = shutdown.clone();
            let _ = std::thread::spawn(move || run_mouse_hook(sender, shutdown_clone));
        }
        // Non-Windows backend is a fiber-friendly poll stub: it never
        // spawns the hook thread, so its channel half stays unused.
        #[cfg(not(target_os = "windows"))]
        drop(sender);

        Self { receiver, shutdown }
    }

    pub fn try_recv(&self) -> Option<MouseNavEvent> {
        self.receiver.try_recv().ok()
    }
}

#[cfg(target_os = "windows")]
struct HookContext {
    shutdown: Arc<AtomicBool>,
    sender: Sender<MouseNavEvent>,
    /// Middle button currently held?
    mb_down: bool,
    mb_origin: (i32, i32),
    mb_consumed: bool,
    /// Gesture delta accumulator
    mb_moved: i32,
    /// Debounce: minimum time between two nav fires
    last_nav: Instant,
}

#[cfg(target_os = "windows")]
static GLOBAL_CTX: std::sync::Mutex<Option<HookContext>> = std::sync::Mutex::new(None);

#[cfg(target_os = "windows")]
fn is_mossambi_foreground() -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextW};

    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.is_null() {
        return false;
    }
    let mut buf = [0u16; 256];
    let len = unsafe { GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32) };
    if len <= 0 {
        return false;
    }
    let title = String::from_utf16_lossy(&buf[..len as usize]);
    title.starts_with("Mossambi")
}

#[cfg(target_os = "windows")]
fn run_mouse_hook(sender: Sender<MouseNavEvent>, shutdown: Arc<AtomicBool>) {
    use std::ptr::null_mut;
    use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, SetWindowsHookExW, UnhookWindowsHookEx, WH_MOUSE_LL,
    };
    const WM_QUIT_MSG: u32 = 0x0012;

    GLOBAL_CTX.lock().unwrap().replace(HookContext {
        shutdown: shutdown.clone(),
        sender,
        mb_down: false,
        mb_origin: (0, 0),
        mb_consumed: false,
        mb_moved: 0,
        last_nav: Instant::now() - Duration::from_secs(1),
    });

    unsafe extern "system" fn mouse_proc(n_code: i32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            MSLLHOOKSTRUCT, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEMOVE, WM_XBUTTONDOWN,
        };

        if n_code < 0 || l_param == 0 {
            return CallNextHookEx(null_mut(), n_code, w_param, l_param);
        }

        let info = *(l_param as *const MSLLHOOKSTRUCT);
        let msg = w_param as u32;

        if let Ok(mut guard) = GLOBAL_CTX.try_lock() {
            if let Some(ctx) = guard.as_mut() {
                if ctx.shutdown.load(std::sync::atomic::Ordering::Relaxed) {
                    return CallNextHookEx(null_mut(), n_code, w_param, l_param);
                }
                if !is_mossambi_foreground() {
                    return CallNextHookEx(null_mut(), n_code, w_param, l_param);
                }
                match msg {
                    WM_XBUTTONDOWN => {
                        // mouseData: low word = XBUTTON1(1) / XBUTTON2(2)
                        let xbtn = info.mouseData & 0xFFFF;
                        if xbtn == 1 {
                            let _ = ctx.sender.send(MouseNavEvent::Back);
                        } else if xbtn == 2 {
                            let _ = ctx.sender.send(MouseNavEvent::Forward);
                        }
                    }
                    WM_MBUTTONDOWN => {
                        ctx.mb_down = true;
                        ctx.mb_origin = (info.pt.x, info.pt.y);
                        ctx.mb_moved = 0;
                        ctx.mb_consumed = false;
                    }
                    WM_MOUSEMOVE => {
                        if ctx.mb_down {
                            let dx = info.pt.x - ctx.mb_origin.0;
                            ctx.mb_moved = dx;
                            if !ctx.mb_consumed && dx.abs() > 60 {
                                let evt = if dx > 0 {
                                    MouseNavEvent::Forward
                                } else {
                                    MouseNavEvent::Back
                                };
                                // Debounce so a long drag does not fire repeatedly
                                if ctx.last_nav.elapsed() > Duration::from_millis(700) {
                                    let _ = ctx.sender.send(evt);
                                    ctx.last_nav = Instant::now();
                                }
                                ctx.mb_consumed = true;
                            }
                        }
                    }
                    WM_MBUTTONUP => {
                        ctx.mb_down = false;
                        ctx.mb_consumed = false;
                    }
                    _ => {}
                }
            }
        }

        CallNextHookEx(null_mut(), n_code, w_param, l_param)
    }

    unsafe {
        let hook = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), null_mut(), 0);
        if hook.is_null() {
            log::warn!("Could not register WH_MOUSE_LL for mouse navigation.");
            return;
        }

        let mut msg = std::mem::zeroed::<windows_sys::Win32::UI::WindowsAndMessaging::MSG>();
        while !shutdown.load(Ordering::Relaxed) {
            use windows_sys::Win32::UI::WindowsAndMessaging::{PeekMessageW, PM_REMOVE};
            while PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) != 0 {
                if msg.message == WM_QUIT_MSG {
                    break;
                }
                windows_sys::Win32::UI::WindowsAndMessaging::TranslateMessage(&msg);
                windows_sys::Win32::UI::WindowsAndMessaging::DispatchMessageW(&msg);
            }
            std::thread::sleep(Duration::from_millis(16));
        }

        UnhookWindowsHookEx(hook);
    }
}
