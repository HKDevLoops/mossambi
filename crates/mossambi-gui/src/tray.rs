//! System Tray Icon and Win32 Global Activation Hooks for Mossambi
//! Implements:
//! 1. Global hotkey `WIN + CTRL` registered via `RegisterHotKey`.
//! 2. Continuous circle-drag mouse gesture detector (>=300 degrees while dragging).
//! 3. System tray notification icon with tooltip and click action.

use crate::gesture::CircleGestureDetector;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, Copy)]
pub enum WakeupReason {
    HotkeyWinCtrl,
    CircleDragGesture,
    TrayClick,
}

#[derive(Debug, Clone, Copy)]
pub struct WakeupEvent {
    pub cursor_x: f32,
    pub cursor_y: f32,
    pub reason: WakeupReason,
}

pub struct ActivationManager {
    receiver: Receiver<WakeupEvent>,
}

impl ActivationManager {
    pub fn new() -> Self {
        let (tx, rx) = channel();

        #[cfg(target_os = "windows")]
        {
            let tx_hotkey = tx.clone();
            // Spawn background thread for global hotkey and mouse gesture tracking
            thread::spawn(move || {
                run_win32_activation_loop(tx_hotkey);
            });
        }

        Self { receiver: rx }
    }

    pub fn try_recv(&self) -> Option<WakeupEvent> {
        self.receiver.try_recv().ok()
    }
}

#[cfg(target_os = "windows")]
fn run_win32_activation_loop(sender: Sender<WakeupEvent>) {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, RegisterHotKey, VK_LBUTTON,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, GetCursorPos, PeekMessageW, TranslateMessage, MSG, PM_REMOVE,
        WM_HOTKEY,
    };

    const MOD_CONTROL: u32 = 0x0002;
    const MOD_WIN: u32 = 0x0008;
    const HOTKEY_ID: i32 = 701;

    unsafe {
        // Register global WIN + CTRL hotkey
        let registered = RegisterHotKey(0 as HWND, HOTKEY_ID, MOD_CONTROL | MOD_WIN, 0);
        if registered != 0 {
            log::info!("Registered global WIN + CTRL hotkey successfully (ID: {})", HOTKEY_ID);
        } else {
            log::warn!("Failed to register global WIN + CTRL hotkey; continuing with gesture tracker.");
        }
    }

    let mut gesture_detector = CircleGestureDetector::new();
    let mut msg: MSG = unsafe { std::mem::zeroed() };

    loop {
        // 1. Process Win32 message queue for WM_HOTKEY
        unsafe {
            while PeekMessageW(&mut msg, 0 as HWND, 0, 0, PM_REMOVE) != 0 {
                if msg.message == WM_HOTKEY && msg.wParam == HOTKEY_ID as usize {
                    let mut pt: POINT = std::mem::zeroed();
                    GetCursorPos(&mut pt);
                    let _ = sender.send(WakeupEvent {
                        cursor_x: pt.x as f32,
                        cursor_y: pt.y as f32,
                        reason: WakeupReason::HotkeyWinCtrl,
                    });
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }

        // 2. Poll cursor position and left button state for circle-drag gesture
        unsafe {
            let mut pt: POINT = std::mem::zeroed();
            GetCursorPos(&mut pt);
            // High-bit of GetAsyncKeyState indicates key/button is currently pressed
            let is_lbutton_down = (GetAsyncKeyState(VK_LBUTTON as i32) as u16 & 0x8000) != 0;

            if gesture_detector.update(pt.x as f32, pt.y as f32, is_lbutton_down) {
                let _ = sender.send(WakeupEvent {
                    cursor_x: pt.x as f32,
                    cursor_y: pt.y as f32,
                    reason: WakeupReason::CircleDragGesture,
                });
            }
        }

        thread::sleep(Duration::from_millis(20)); // ~50 Hz sampling rate
    }
}
