//! Background Resident Daemon & Global Hotkey Service for Mossambi
//! Runs quietly in the background like Whisper Flow:
//! - [Win + Shift + C]: Summon Floating Radial Wheel centered under cursor
//! - [Win + Ctrl + C]:  Open Desktop Studio GUI
//! - System Tray Icon with menu, themes, and autostart toggle

use anyhow::Result;
use std::io::{self, Write};

pub fn handle_daemon_command(action: &str) -> Result<()> {
    match action.to_lowercase().as_str() {
        "run" => {
            println!("Starting Mossambi background daemon...");
            run_daemon_loop()?;
        }
        "start" => {
            start_daemon_background()?;
            println!("✓ Mossambi daemon started in background.");
            println!("  Hotkeys: Win+Shift+C (Wheel) · Win+Ctrl+C (Studio)");
        }
        "stop" => {
            stop_daemon()?;
        }
        "status" => {
            print_daemon_status();
        }
        "gesture" | "gesture-on" => {
            let mut cfg = mossambi_core::MossambiConfig::load();
            cfg.enable_gesture_wakeup = true;
            let _ = cfg.save();
            println!("✓ File drag mouse gesture activation enabled (Gated to active file drags).");
        }
        "gesture-off" => {
            let mut cfg = mossambi_core::MossambiConfig::load();
            cfg.enable_gesture_wakeup = false;
            let _ = cfg.save();
            println!("✓ File drag mouse gesture activation disabled.");
        }
        "install" => {
            mossambi_core::MossambiConfig::set_autostart(true)?;
            let mut cfg = mossambi_core::MossambiConfig::load();
            cfg.launch_at_login = true;
            cfg.autostart_prompted = Some(true);
            let _ = cfg.save();
            start_daemon_background()?;
            println!("✓ Mossambi background daemon configured and started!");
            println!("  Hotkeys: Win+Shift+C (Wheel) · Win+Ctrl+C (Studio)");
        }
        "uninstall" => {
            mossambi_core::MossambiConfig::set_autostart(false)?;
            let mut cfg = mossambi_core::MossambiConfig::load();
            cfg.launch_at_login = false;
            let _ = cfg.save();
            stop_daemon()?;
            println!("✓ Mossambi background daemon uninstalled from startup.");
        }
        _ => {
            // Default interactive prompt
            prompt_user_autostart()?;
        }
    }
    Ok(())
}

pub fn print_daemon_status() {
    let autostart = mossambi_core::MossambiConfig::is_autostart_configured();
    let cfg = mossambi_core::MossambiConfig::load();
    let gesture_status = if cfg.enable_gesture_wakeup {
        "ENABLED  [Gated to active file drags]"
    } else {
        "DISABLED"
    };

    println!("┌────────────────────────────────────────────────────────┐");
    println!("│  MOSSAMBI BACKGROUND DAEMON STATUS                    │");
    println!("├────────────────────────────────────────────────────────┤");
    println!("│  Autostart on Boot:  {}", if autostart { "ENABLED  [Active at login]" } else { "DISABLED" });
    println!("│  File Drag Gesture:  {}  │", gesture_status);
    println!("│  Global Hotkey 1:    Win + Shift + C  (Radial Wheel)   │");
    println!("│  Global Hotkey 2:    Win + Ctrl + C   (Studio GUI)     │");
    println!("│  Tray Tooltip:       Sweet Lime Transcoder & Studio    │");
    println!("└────────────────────────────────────────────────────────┘");
}

pub fn prompt_user_autostart() -> Result<()> {
    println!("\n=======================================================");
    println!("  MOSSAMBI BACKGROUND RESIDENT DAEMON (Whisper Flow)");
    println!("=======================================================");
    println!("Runs quietly in the background with zero idle resource load.");
    println!("Provides instant global hotkeys from anywhere in the OS:");
    println!("  • [Win + Shift + C]  Summon Floating Radial Wheel under cursor");
    println!("  • [Win + Ctrl + C]   Open Full Desktop Studio GUI\n");

    let is_configured = mossambi_core::MossambiConfig::is_autostart_configured();
    if is_configured {
        println!("Status: Currently ENABLED to start automatically with Windows.");
    } else {
        println!("Status: Currently DISABLED.");
    }

    print!("\nDo you want Mossambi to run in the background on startup? [Y/n]: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let trimmed = input.trim().to_lowercase();

    if trimmed == "y" || trimmed == "yes" || trimmed.is_empty() {
        mossambi_core::MossambiConfig::set_autostart(true)?;
        let mut cfg = mossambi_core::MossambiConfig::load();
        cfg.launch_at_login = true;
        cfg.autostart_prompted = Some(true);
        let _ = cfg.save();
        start_daemon_background()?;
        println!("\n✓ Mossambi background daemon enabled and started!");
        println!("  Press Win+Shift+C anytime to summon the Sweet Lime radial wheel.");
        println!("  Press Win+Ctrl+C anytime to open the desktop studio.");
    } else {
        mossambi_core::MossambiConfig::set_autostart(false)?;
        let mut cfg = mossambi_core::MossambiConfig::load();
        cfg.launch_at_login = false;
        cfg.autostart_prompted = Some(true);
        let _ = cfg.save();
        println!("\n✓ Background autostart disabled. You can launch manually or re-enable via:");
        println!("  'mossambi daemon install' or in GUI Preferences.");
    }

    Ok(())
}

#[cfg(target_os = "windows")]
pub fn start_daemon_background() -> Result<()> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let exe = std::env::current_exe()?;
    let _ = std::process::Command::new(exe)
        .args(["daemon", "run"])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()?;
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn start_daemon_background() -> Result<()> {
    let exe = std::env::current_exe()?;
    let _ = std::process::Command::new(exe)
        .args(["daemon", "run"])
        .spawn()?;
    Ok(())
}

pub fn stop_daemon() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        // Gracefully kill daemon instances
        let _ = std::process::Command::new("taskkill")
            .args(["/F", "/IM", "mossambi.exe", "/FI", "WINDOWTITLE eq MossambiDaemon*"])
            .output();
        println!("✓ Mossambi daemon stopped.");
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = std::process::Command::new("pkill")
            .args(["-f", "mossambi daemon run"])
            .output();
        println!("✓ Mossambi daemon stopped.");
    }
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn run_daemon_loop() -> Result<()> {
    use std::ptr::null_mut;
    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{RegisterHotKey, UnregisterHotKey};
    use windows_sys::Win32::UI::Shell::{
        Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu,
        DestroyWindow, DispatchMessageW, GetCursorPos, LoadIconW, PeekMessageW,
        RegisterClassExW, SetForegroundWindow, TrackPopupMenu, TranslateMessage,
        UnregisterClassW, CS_HREDRAW, CS_VREDRAW, IDI_APPLICATION, MF_CHECKED,
        MF_SEPARATOR, MF_STRING, MF_UNCHECKED, MSG, PM_REMOVE, TPM_BOTTOMALIGN,
        TPM_LEFTALIGN, TPM_NONOTIFY, TPM_RETURNCMD, TPM_RIGHTBUTTON, WM_HOTKEY,
        WM_USER, WNDCLASSEXW, WS_OVERLAPPEDWINDOW,
    };

    const MOD_CONTROL: u32 = 0x0002;
    const MOD_SHIFT: u32 = 0x0004;
    const MOD_WIN: u32 = 0x0008;
    const MOD_NOREPEAT: u32 = 0x4000;
    const HOTKEY_WIDGET_ID: i32 = 701; // Win + Shift + C
    const HOTKEY_GUI_ID: i32 = 702;    // Win + Ctrl + C
    const VK_C: u32 = 0x43;
    const WM_TRAYICON: u32 = WM_USER + 101;
    const TRAY_UID: u32 = 1001;

    const CMD_SUMMON_WIDGET: u32 = 2001;
    const CMD_OPEN_STUDIO: u32 = 2002;
    const CMD_TOGGLE_AUTOSTART: u32 = 2003;
    const CMD_QUIT: u32 = 2099;

    unsafe extern "system" fn daemon_wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }

    let class_name: Vec<u16> = "MossambiDaemonClass\0".encode_utf16().collect();
    let wnd_class = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(daemon_wnd_proc),
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

    if hwnd.is_null() {
        unsafe {
            UnregisterClassW(class_name.as_ptr(), null_mut());
        }
        anyhow::bail!("Failed to create hidden window for Mossambi daemon message loop");
    }

    // Register Win + Shift + C & Win + Ctrl + C
    unsafe {
        let reg1 = RegisterHotKey(hwnd, HOTKEY_WIDGET_ID, MOD_WIN | MOD_SHIFT | MOD_NOREPEAT, VK_C);
        if reg1 != 0 {
            log::info!("Registered global Win+Shift+C hotkey (Radial Wheel).");
        }
        let reg2 = RegisterHotKey(hwnd, HOTKEY_GUI_ID, MOD_WIN | MOD_CONTROL | MOD_NOREPEAT, VK_C);
        if reg2 != 0 {
            log::info!("Registered global Win+Ctrl+C hotkey (Studio GUI).");
        }
    }

    // System Tray
    let mut nid: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = TRAY_UID;
    nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    nid.uCallbackMessage = WM_TRAYICON;
    nid.hIcon = unsafe { LoadIconW(null_mut(), IDI_APPLICATION) };

    let tip = "Mossambi: Win+Shift+C (Wheel) · Win+Ctrl+C (Studio)\0";
    for (i, ch) in tip.encode_utf16().enumerate().take(127) {
        nid.szTip[i] = ch;
    }

    unsafe {
        Shell_NotifyIconW(NIM_ADD, &nid);
    }

    fn to_u16(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    let cfg = mossambi_core::MossambiConfig::load();
    let allow_shake = cfg.gesture_type == "shake" || cfg.gesture_type == "both";
    let allow_circle = cfg.gesture_type == "circle" || cfg.gesture_type == "both";
    let mut drag_listener = if cfg.enable_gesture_wakeup {
        Some(mossambi_core::DragGestureListener::start(
            allow_shake,
            allow_circle,
            cfg.shake_sensitivity_px,
        ))
    } else {
        None
    };

    let mut msg: MSG = unsafe { std::mem::zeroed() };
    loop {
        unsafe {
            while PeekMessageW(&mut msg, hwnd, 0, 0, PM_REMOVE) != 0 {
                if msg.message == WM_HOTKEY {
                    if msg.wParam == HOTKEY_WIDGET_ID as usize {
                        let mut pt: POINT = std::mem::zeroed();
                        GetCursorPos(&mut pt);
                        spawn_widget_at(pt.x, pt.y);
                    } else if msg.wParam == HOTKEY_GUI_ID as usize {
                        spawn_gui_studio();
                    }
                } else if msg.message == WM_TRAYICON {
                    let mouse_event = msg.lParam as u32;
                    const WM_LBUTTONUP: u32 = 0x0202;
                    const WM_RBUTTONUP: u32 = 0x0205;

                    if mouse_event == WM_LBUTTONUP {
                        let mut pt: POINT = std::mem::zeroed();
                        GetCursorPos(&mut pt);
                        spawn_widget_at(pt.x, pt.y);
                    } else if mouse_event == WM_RBUTTONUP {
                        let mut pt: POINT = std::mem::zeroed();
                        GetCursorPos(&mut pt);
                        SetForegroundWindow(hwnd);

                        let hmenu = CreatePopupMenu();
                        AppendMenuW(hmenu, MF_STRING, CMD_SUMMON_WIDGET as usize, to_u16("🍊 Summon Radial Wheel (Win+Shift+C)").as_ptr());
                        AppendMenuW(hmenu, MF_STRING, CMD_OPEN_STUDIO as usize, to_u16("🖥️ Open Studio (Win+Ctrl+C)").as_ptr());
                        AppendMenuW(hmenu, MF_SEPARATOR, 0, null_mut());

                        let is_autostart = mossambi_core::MossambiConfig::is_autostart_configured();
                        AppendMenuW(
                            hmenu,
                            MF_STRING | if is_autostart { MF_CHECKED } else { MF_UNCHECKED },
                            CMD_TOGGLE_AUTOSTART as usize,
                            to_u16("🚀 Run in Background at Startup").as_ptr(),
                        );
                        AppendMenuW(hmenu, MF_SEPARATOR, 0, null_mut());
                        AppendMenuW(hmenu, MF_STRING, CMD_QUIT as usize, to_u16("❌ Exit Background Daemon").as_ptr());

                        let cmd = TrackPopupMenu(
                            hmenu,
                            TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON | TPM_LEFTALIGN | TPM_BOTTOMALIGN,
                            pt.x,
                            pt.y,
                            0,
                            hwnd,
                            null_mut(),
                        );
                        DestroyMenu(hmenu);

                        match cmd as u32 {
                            CMD_SUMMON_WIDGET => {
                                spawn_widget_at(pt.x, pt.y);
                            }
                            CMD_OPEN_STUDIO => {
                                spawn_gui_studio();
                            }
                            CMD_TOGGLE_AUTOSTART => {
                                let new_setting = !mossambi_core::MossambiConfig::is_autostart_configured();
                                let _ = mossambi_core::MossambiConfig::set_autostart(new_setting);
                            }
                            CMD_QUIT => {
                                if let Some(mut listener) = drag_listener.take() {
                                    listener.stop();
                                }
                                UnregisterHotKey(hwnd, HOTKEY_WIDGET_ID);
                                UnregisterHotKey(hwnd, HOTKEY_GUI_ID);
                                Shell_NotifyIconW(NIM_DELETE, &nid);
                                DestroyWindow(hwnd);
                                UnregisterClassW(class_name.as_ptr(), null_mut());
                                return Ok(());
                            }
                            _ => {}
                        }
                    }
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }

        // Check for system-level mouse drag gestures while holding files
        if let Some(ref listener) = drag_listener {
            if let Some(evt) = listener.try_recv() {
                spawn_widget_at_with_files(evt.cursor_pos.0 as i32, evt.cursor_pos.1 as i32, &evt.files, true);
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(30));
    }
}

#[cfg(not(target_os = "windows"))]
pub fn run_daemon_loop() -> Result<()> {
    println!("Mossambi background daemon running on Linux/macOS.");
    println!("Global Shortcuts configured via Desktop Environment / Autostart (Super+Shift+C / Super+Ctrl+C).");
    let cfg = mossambi_core::MossambiConfig::load();
    let allow_shake = cfg.gesture_type == "shake" || cfg.gesture_type == "both";
    let allow_circle = cfg.gesture_type == "circle" || cfg.gesture_type == "both";
    let drag_listener = if cfg.enable_gesture_wakeup {
        Some(mossambi_core::DragGestureListener::start(
            allow_shake,
            allow_circle,
            cfg.shake_sensitivity_px,
        ))
    } else {
        None
    };

    loop {
        if let Some(ref listener) = drag_listener {
            if let Some(evt) = listener.try_recv() {
                spawn_widget_at_with_files(evt.cursor_pos.0 as i32, evt.cursor_pos.1 as i32, &evt.files, true);
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

fn spawn_widget_at(x: i32, y: i32) {
    spawn_widget_at_with_files(x, y, &[], false);
}

fn spawn_widget_at_with_files(x: i32, y: i32, files: &[std::path::PathBuf], active_drag: bool) {
    #[cfg(target_os = "windows")]
    let bin = "mossambi-widget.exe";
    #[cfg(not(target_os = "windows"))]
    let bin = "mossambi-widget";

    let mut cmd = std::process::Command::new(bin);
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let local = parent.join(bin);
            if local.exists() {
                cmd = std::process::Command::new(local);
            }
        }
    }
    cmd.arg("--pos").arg(format!("{},{}", x, y));
    if active_drag {
        cmd.arg("--active-drag");
    }
    for file in files {
        cmd.arg(file);
    }
    let _ = cmd.spawn();
}

fn spawn_gui_studio() {
    #[cfg(target_os = "windows")]
    let bin = "mossambi-gui.exe";
    #[cfg(not(target_os = "windows"))]
    let bin = "mossambi-gui";

    let mut cmd = std::process::Command::new(bin);
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let local = parent.join(bin);
            if local.exists() {
                cmd = std::process::Command::new(local);
            }
        }
    }
    let _ = cmd.spawn();
}
