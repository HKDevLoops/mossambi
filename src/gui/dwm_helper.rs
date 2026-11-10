//! Windows DWM System-Level Backdrop (Mica / Acrylic) & Compositor Integration
//!
//! Provides hardware-accelerated Windows Desktop Window Manager (DWM) Acrylic and Mica blur,
//! immersive dark mode synchronization, and frame margin extension for transparent windows.

use crate::core::GlassEffect;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackdropKind {
    None = 1,
    Mica = 2,       // DWMSBT_MAINWINDOW (Windows 11)
    Acrylic = 3,    // DWMSBT_TRANSIENTWINDOW (Acrylic blur behind)
    TabbedMica = 4, // DWMSBT_TABBEDWINDOW (Mica Alt)
}

impl From<GlassEffect> for BackdropKind {
    fn from(effect: GlassEffect) -> Self {
        match effect {
            GlassEffect::FrostedGlass => BackdropKind::Acrylic,
            GlassEffect::LiquidGlass => BackdropKind::TabbedMica,
            GlassEffect::Opaque => BackdropKind::None,
        }
    }
}

/// Applies native DWM system backdrop (Acrylic / Mica) and extends frame into client area
#[allow(unused_variables)]
pub fn apply_system_backdrop(dark_mode: bool, effect: GlassEffect) {
    #[cfg(target_os = "windows")]
    {
        win32_dwm::apply_dwm_attributes(dark_mode, BackdropKind::from(effect));
    }
}

#[cfg(target_os = "windows")]
mod win32_dwm {
    use super::BackdropKind;
    use std::ffi::c_void;
    use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM};
    use windows_sys::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMWA_SYSTEMBACKDROP_TYPE, DWMWA_USE_IMMERSIVE_DARK_MODE,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetClassNameW, GetWindowThreadProcessId,
    };

    pub fn apply_dwm_attributes(dark_mode: bool, backdrop: BackdropKind) {
        let param = (backdrop as usize) | if dark_mode { 0x100 } else { 0 };
        unsafe {
            EnumWindows(Some(enum_proc), param as LPARAM);
        }
    }

    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let current_pid = std::process::id();
        let mut wnd_pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut wnd_pid);

        if wnd_pid == current_pid {
            let mut class_name = [0u16; 256];
            let len = GetClassNameW(hwnd, class_name.as_mut_ptr(), 256);
            let class_str = String::from_utf16_lossy(&class_name[..len as usize]);

            // Matches eframe / winit primary window class (skip internal event targets)
            if class_str == "Window Class" {
                let dark_mode = (lparam & 0x100) != 0;
                let backdrop_val = (lparam & 0xFF) as i32;

                // 1. Synchronize Immersive Dark Mode with active theme
                let dark_val: i32 = if dark_mode { 1 } else { 0 };
                let _ = DwmSetWindowAttribute(
                    hwnd,
                    DWMWA_USE_IMMERSIVE_DARK_MODE as u32,
                    &dark_val as *const i32 as *const c_void,
                    4,
                );

                // 3. Apply DWM System Backdrop Type (Acrylic / Mica)
                let _ = DwmSetWindowAttribute(
                    hwnd,
                    DWMWA_SYSTEMBACKDROP_TYPE as u32,
                    &backdrop_val as *const i32 as *const c_void,
                    4,
                );

                // 4. Windows 11 Rounded Corners (DWMWA_WINDOW_CORNER_PREFERENCE = 33, DWMWCP_ROUND = 2)
                const DWMWA_WINDOW_CORNER_PREFERENCE: u32 = 33;
                let corner_pref: i32 = 2; // DWMWCP_ROUND
                let _ = DwmSetWindowAttribute(
                    hwnd,
                    DWMWA_WINDOW_CORNER_PREFERENCE,
                    &corner_pref as *const i32 as *const c_void,
                    4,
                );
            }
        }
        1
    }
}
