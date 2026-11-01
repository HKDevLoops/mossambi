//! System Tray Icon and Win32 Global Activation Hooks for Mossambi
//! Implements:
//! 1. Global hotkey `WIN + CTRL` registered via `RegisterHotKey`.
//! 2. 2-Time circular motion mouse gesture detector (>=720 degrees while dragging).
//! 3. Active file-under-cursor capture when 2-circle gesture triggers.
//! 4. Native Windows System Tray icon with complete popup menu for Themes,
//!    Indian Exam Presets, Studio / Radial Widget toggles, and Zero-Loss settings.

use crate::active_file::get_active_file_under_cursor;
use crate::gesture::CircleGestureDetector;
use crate::macos_theme::ThemeStyle;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WakeupReason {
    HotkeyWinCtrl,
    CircleDragGesture,
    TraySummonWidget,
    TrayOpenStudio,
}

#[derive(Debug, Clone)]
pub struct WakeupEvent {
    pub cursor_x: f32,
    pub cursor_y: f32,
    pub reason: WakeupReason,
    pub active_file: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub enum AppControlEvent {
    Wakeup(WakeupEvent),
    ChangeTheme(ThemeStyle),
    ChangeExamPreset(String),
    ToggleZeroLoss(bool),
    OpenStudio,
    QuitApp,
}

pub struct ActivationManager {
    receiver: Receiver<AppControlEvent>,
    shutdown: Arc<AtomicBool>,
}

impl Drop for ActivationManager {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }
}

impl Default for ActivationManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ActivationManager {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        let shutdown = Arc::new(AtomicBool::new(false));

        #[cfg(target_os = "windows")]
        {
            let tx_loop = tx.clone();
            let shutdown_clone = shutdown.clone();
            thread::spawn(move || {
                run_win32_tray_and_activation_loop(tx_loop, shutdown_clone);
            });
        }

        Self {
            receiver: rx,
            shutdown,
        }
    }

    pub fn try_recv(&self) -> Option<AppControlEvent> {
        self.receiver.try_recv().ok()
    }
}

#[cfg(target_os = "windows")]
fn run_win32_tray_and_activation_loop(sender: Sender<AppControlEvent>, shutdown: Arc<AtomicBool>) {
    use std::ptr::null_mut;
    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, RegisterHotKey, VK_LBUTTON,
    };
    use windows_sys::Win32::UI::Shell::{
        Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE,
        NOTIFYICONDATAW,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu,
        DestroyWindow, DispatchMessageW, GetCursorPos, LoadIconW, PeekMessageW,
        RegisterClassExW, SetForegroundWindow, TrackPopupMenu, TranslateMessage,
        CS_HREDRAW, CS_VREDRAW, IDI_APPLICATION, MF_CHECKED, MF_POPUP, MF_SEPARATOR,
        MF_STRING, MF_UNCHECKED, MSG, PM_REMOVE, TPM_BOTTOMALIGN, TPM_LEFTALIGN,
        TPM_NONOTIFY, TPM_RETURNCMD, TPM_RIGHTBUTTON, WM_HOTKEY, WM_USER, WNDCLASSEXW,
        WS_OVERLAPPEDWINDOW,
    };

    const MOD_CONTROL: u32 = 0x0002;
    const MOD_WIN: u32 = 0x0008;
    const HOTKEY_ID: i32 = 701;
    const WM_TRAYICON: u32 = WM_USER + 101;
    const TRAY_UID: u32 = 1001;

    // Menu Command IDs
    const CMD_OPEN_STUDIO: u32 = 2001;
    const CMD_SUMMON_WIDGET: u32 = 2002;
    const CMD_THEME_LIQUID_GLASS: u32 = 2010;
    const CMD_THEME_FROSTED_GLASS: u32 = 2011;
    const CMD_THEME_DARK_ANTHRACITE: u32 = 2012;
    const CMD_THEME_LIGHT_AQUA: u32 = 2013;
    const CMD_EXAM_UPSC_PHOTO: u32 = 2020;
    const CMD_EXAM_UPSC_SIGN: u32 = 2021;
    const CMD_EXAM_SSC_PHOTO: u32 = 2022;
    const CMD_EXAM_SSC_SIGN: u32 = 2023;
    const CMD_EXAM_IBPS_PHOTO: u32 = 2024;
    const CMD_EXAM_IBPS_SIGN: u32 = 2025;
    const CMD_TOGGLE_ZERO_LOSS: u32 = 2030;
    const CMD_QUIT: u32 = 2099;

    unsafe extern "system" fn tray_wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }

    // Register a hidden utility window class to receive tray messages
    let class_name: Vec<u16> = "MossambiTrayMsgClass\0".encode_utf16().collect();
    let wnd_class = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(tray_wnd_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: null_mut(),
        hIcon: null_mut(),
        hCursor: null_mut(),
        hbrBackground: null_mut(),
        lpszMenuName: null_mut(),
        lpszClassName: class_name.as_ptr(),
        hIconSm: null_mut(),
    };

    unsafe {
        RegisterClassExW(&wnd_class);
    }

    let hwnd = unsafe {
        CreateWindowExW(
            0,
            class_name.as_ptr(),
            class_name.as_ptr(),
            WS_OVERLAPPEDWINDOW,
            0,
            0,
            0,
            0,
            null_mut(),
            null_mut(),
            null_mut(),
            null_mut(),
        )
    };

    // Register global WIN + CTRL hotkey
    unsafe {
        let registered = RegisterHotKey(hwnd, HOTKEY_ID, MOD_CONTROL | MOD_WIN, 0);
        if registered != 0 {
            log::info!("Registered global WIN + CTRL hotkey successfully.");
        } else {
            log::warn!("Could not register global WIN + CTRL hotkey; continuing with gesture tracker.");
        }
    }

    // Initialize System Tray Icon
    let mut nid: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = TRAY_UID;
    nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    nid.uCallbackMessage = WM_TRAYICON;
    nid.hIcon = unsafe { LoadIconW(null_mut(), IDI_APPLICATION) };

    let tip = "Mossambi — Sweet Lime Transcoder & Indian Exam Studio\0";
    for (i, ch) in tip.encode_utf16().enumerate().take(127) {
        nid.szTip[i] = ch;
    }

    unsafe {
        Shell_NotifyIconW(NIM_ADD, &nid);
    }

    let mut gesture_detector = CircleGestureDetector::new();
    let mut msg: MSG = unsafe { std::mem::zeroed() };
    let mut zero_loss_enabled = true;
    let mut current_theme = ThemeStyle::LiquidGlass;

    loop {
        if shutdown.load(Ordering::Relaxed) {
            break;
        }

        // 1. Process Win32 message queue for hotkeys and tray events
        unsafe {
            while PeekMessageW(&mut msg, hwnd, 0, 0, PM_REMOVE) != 0 {
                if msg.message == WM_HOTKEY && msg.wParam == HOTKEY_ID as usize {
                    let mut pt: POINT = std::mem::zeroed();
                    GetCursorPos(&mut pt);
                    let active_file = get_active_file_under_cursor(pt.x, pt.y);
                    let _ = sender.send(AppControlEvent::Wakeup(WakeupEvent {
                        cursor_x: pt.x as f32,
                        cursor_y: pt.y as f32,
                        reason: WakeupReason::HotkeyWinCtrl,
                        active_file,
                    }));
                } else if msg.message == WM_TRAYICON {
                    let mouse_event = msg.lParam as u32;
                    const WM_LBUTTONUP: u32 = 0x0202;
                    const WM_RBUTTONUP: u32 = 0x0205;

                    if mouse_event == WM_LBUTTONUP {
                        // Left click opens studio
                        let _ = sender.send(AppControlEvent::OpenStudio);
                    } else if mouse_event == WM_RBUTTONUP {
                        // Right click shows rich native macOS-styled configuration menu
                        let mut pt: POINT = std::mem::zeroed();
                        GetCursorPos(&mut pt);
                        SetForegroundWindow(hwnd);

                        let hmenu = CreatePopupMenu();
                        let hmenu_themes = CreatePopupMenu();
                        let hmenu_exams = CreatePopupMenu();

                        // Helper to encode UTF-16
                        fn to_u16(s: &str) -> Vec<u16> {
                            s.encode_utf16().chain(std::iter::once(0)).collect()
                        }

                        // Theme Submenu
                        AppendMenuW(
                            hmenu_themes,
                            MF_STRING | if current_theme == ThemeStyle::LiquidGlass { MF_CHECKED } else { MF_UNCHECKED },
                            CMD_THEME_LIQUID_GLASS as usize,
                            to_u16("Liquid Glass (macOS Sequoia Specular)").as_ptr(),
                        );
                        AppendMenuW(
                            hmenu_themes,
                            MF_STRING | if current_theme == ThemeStyle::FrostedGlass { MF_CHECKED } else { MF_UNCHECKED },
                            CMD_THEME_FROSTED_GLASS as usize,
                            to_u16("Frosted Glass (macOS Sonoma Acrylic)").as_ptr(),
                        );
                        AppendMenuW(
                            hmenu_themes,
                            MF_STRING | if current_theme == ThemeStyle::DarkAnthracite { MF_CHECKED } else { MF_UNCHECKED },
                            CMD_THEME_DARK_ANTHRACITE as usize,
                            to_u16("Dark Anthracite (Pro #161618)").as_ptr(),
                        );
                        AppendMenuW(
                            hmenu_themes,
                            MF_STRING | if current_theme == ThemeStyle::LightAqua { MF_CHECKED } else { MF_UNCHECKED },
                            CMD_THEME_LIGHT_AQUA as usize,
                            to_u16("Light Aqua (Clean Light #F5F5F7)").as_ptr(),
                        );

                        // Exam Presets Submenu
                        AppendMenuW(hmenu_exams, MF_STRING, CMD_EXAM_UPSC_PHOTO as usize, to_u16("UPSC Civil Services Photo (350x450, 300 DPI)").as_ptr());
                        AppendMenuW(hmenu_exams, MF_STRING, CMD_EXAM_UPSC_SIGN as usize, to_u16("UPSC Civil Services Signature (350x150, 300 DPI)").as_ptr());
                        AppendMenuW(hmenu_exams, MF_STRING, CMD_EXAM_SSC_PHOTO as usize, to_u16("SSC CGL Photo (100x120, 200 DPI)").as_ptr());
                        AppendMenuW(hmenu_exams, MF_STRING, CMD_EXAM_SSC_SIGN as usize, to_u16("SSC CGL Signature (140x60, 200 DPI)").as_ptr());
                        AppendMenuW(hmenu_exams, MF_STRING, CMD_EXAM_IBPS_PHOTO as usize, to_u16("IBPS Bank PO Photo (200x230, 200 DPI)").as_ptr());
                        AppendMenuW(hmenu_exams, MF_STRING, CMD_EXAM_IBPS_SIGN as usize, to_u16("IBPS Bank PO Signature (140x60, 200 DPI)").as_ptr());

                        // Root Menu
                        AppendMenuW(hmenu, MF_STRING, CMD_OPEN_STUDIO as usize, to_u16("🍋 Open Mossambi Studio").as_ptr());
                        AppendMenuW(hmenu, MF_STRING, CMD_SUMMON_WIDGET as usize, to_u16("🍊 Summon Sweet Lime Radial Widget").as_ptr());
                        AppendMenuW(hmenu, MF_SEPARATOR, 0, null_mut());
                        AppendMenuW(hmenu, MF_POPUP, hmenu_themes as usize, to_u16("🎨 Themes (Liquid & Frosted Glass)").as_ptr());
                        AppendMenuW(hmenu, MF_POPUP, hmenu_exams as usize, to_u16("🇮🇳 Indian Exam Presets (UPSC/SSC/IBPS)").as_ptr());
                        AppendMenuW(
                            hmenu,
                            MF_STRING | if zero_loss_enabled { MF_CHECKED } else { MF_UNCHECKED },
                            CMD_TOGGLE_ZERO_LOSS as usize,
                            to_u16("⚡ Enforce 0.00% Zero-Loss Mode").as_ptr(),
                        );
                        AppendMenuW(hmenu, MF_SEPARATOR, 0, null_mut());
                        AppendMenuW(hmenu, MF_STRING, CMD_QUIT as usize, to_u16("❌ Quit Mossambi").as_ptr());

                        let cmd = TrackPopupMenu(
                            hmenu,
                            TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON | TPM_LEFTALIGN | TPM_BOTTOMALIGN,
                            pt.x,
                            pt.y,
                            0,
                            hwnd,
                            null_mut(),
                        );

                        DestroyMenu(hmenu_themes);
                        DestroyMenu(hmenu_exams);
                        DestroyMenu(hmenu);

                        match cmd as u32 {
                            CMD_OPEN_STUDIO => {
                                let _ = sender.send(AppControlEvent::OpenStudio);
                            }
                            CMD_SUMMON_WIDGET => {
                                let active_file = get_active_file_under_cursor(pt.x, pt.y);
                                let _ = sender.send(AppControlEvent::Wakeup(WakeupEvent {
                                    cursor_x: pt.x as f32,
                                    cursor_y: pt.y as f32,
                                    reason: WakeupReason::TraySummonWidget,
                                    active_file,
                                }));
                            }
                            CMD_THEME_LIQUID_GLASS => {
                                current_theme = ThemeStyle::LiquidGlass;
                                let _ = sender.send(AppControlEvent::ChangeTheme(ThemeStyle::LiquidGlass));
                            }
                            CMD_THEME_FROSTED_GLASS => {
                                current_theme = ThemeStyle::FrostedGlass;
                                let _ = sender.send(AppControlEvent::ChangeTheme(ThemeStyle::FrostedGlass));
                            }
                            CMD_THEME_DARK_ANTHRACITE => {
                                current_theme = ThemeStyle::DarkAnthracite;
                                let _ = sender.send(AppControlEvent::ChangeTheme(ThemeStyle::DarkAnthracite));
                            }
                            CMD_THEME_LIGHT_AQUA => {
                                current_theme = ThemeStyle::LightAqua;
                                let _ = sender.send(AppControlEvent::ChangeTheme(ThemeStyle::LightAqua));
                            }
                            CMD_EXAM_UPSC_PHOTO => {
                                let _ = sender.send(AppControlEvent::ChangeExamPreset("UPSC Civil Services Photo (350x450, 300DPI)".to_string()));
                            }
                            CMD_EXAM_UPSC_SIGN => {
                                let _ = sender.send(AppControlEvent::ChangeExamPreset("UPSC Civil Services Signature (350x150, 300DPI)".to_string()));
                            }
                            CMD_EXAM_SSC_PHOTO => {
                                let _ = sender.send(AppControlEvent::ChangeExamPreset("SSC CGL Photo (100x120, 200DPI)".to_string()));
                            }
                            CMD_EXAM_SSC_SIGN => {
                                let _ = sender.send(AppControlEvent::ChangeExamPreset("SSC CGL Signature (140x60, 200DPI)".to_string()));
                            }
                            CMD_EXAM_IBPS_PHOTO => {
                                let _ = sender.send(AppControlEvent::ChangeExamPreset("IBPS Bank PO Photo (200x230, 200DPI)".to_string()));
                            }
                            CMD_EXAM_IBPS_SIGN => {
                                let _ = sender.send(AppControlEvent::ChangeExamPreset("IBPS Bank PO Signature (140x60, 200DPI)".to_string()));
                            }
                            CMD_TOGGLE_ZERO_LOSS => {
                                zero_loss_enabled = !zero_loss_enabled;
                                let _ = sender.send(AppControlEvent::ToggleZeroLoss(zero_loss_enabled));
                            }
                            CMD_QUIT => {
                                let _ = sender.send(AppControlEvent::QuitApp);
                                shutdown.store(true, Ordering::Relaxed);
                                break;
                            }
                            _ => {}
                        }
                    }
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }

        if shutdown.load(Ordering::Relaxed) {
            break;
        }

        // 2. Poll cursor position and left button state for 2-time circular motion gesture
        unsafe {
            let mut pt: POINT = std::mem::zeroed();
            GetCursorPos(&mut pt);
            let is_lbutton_down = (GetAsyncKeyState(VK_LBUTTON as i32) as u16 & 0x8000) != 0;

            if let Some((cx, cy)) = gesture_detector.update(pt.x as f32, pt.y as f32, is_lbutton_down) {
                // When 2 circular loops are detected while dragging, resolve the active file under cursor!
                let active_file = get_active_file_under_cursor(pt.x, pt.y);
                if sender.send(AppControlEvent::Wakeup(WakeupEvent {
                    cursor_x: cx,
                    cursor_y: cy,
                    reason: WakeupReason::CircleDragGesture,
                    active_file,
                })).is_err() {
                    break;
                }
            }
        }

        thread::sleep(Duration::from_millis(20)); // 50 Hz sampling rate
    }

    unsafe {
        Shell_NotifyIconW(NIM_DELETE, &nid);
        DestroyWindow(hwnd);
    }
}
