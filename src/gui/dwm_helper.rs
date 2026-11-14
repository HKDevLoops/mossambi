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
            // Both glass modes need a real blur of what is behind the
            // window; Mica only tints with the wallpaper colour.
            GlassEffect::FrostedGlass => BackdropKind::Acrylic,
            GlassEffect::LiquidGlass => BackdropKind::Acrylic,
            GlassEffect::Opaque => BackdropKind::None,
        }
    }
}

/// True when the OS can blur behind the window (Windows 10 1809+).
pub fn backdrop_supported() -> bool {
    #[cfg(target_os = "windows")]
    {
        static SUPPORTED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        *SUPPORTED.get_or_init(|| win32_dwm::os_build() >= 17763)
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
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
        DwmExtendFrameIntoClientArea, DwmSetWindowAttribute, DWMWA_SYSTEMBACKDROP_TYPE,
        DWMWA_USE_IMMERSIVE_DARK_MODE,
    };
    use windows_sys::Win32::UI::Controls::MARGINS;

    #[repr(C)]
    struct AccentPolicy {
        accent_state: u32,
        accent_flags: u32,
        gradient_color: u32,
        animation_id: u32,
    }

    #[repr(C)]
    struct WindowCompositionAttribData {
        attrib: u32,
        pv_data: *mut c_void,
        cb_data: usize,
    }

    // Undocumented but long-stable user32 export (Windows 10 1803+); not in
    // the SDK import library, hence raw-dylib.
    #[link(name = "user32", kind = "raw-dylib")]
    extern "system" {
        fn SetWindowCompositionAttribute(
            hwnd: HWND,
            data: *mut WindowCompositionAttribData,
        ) -> BOOL;
    }

    /// Gaussian blur of whatever is behind the window, under the app's own
    /// translucent fills (ACCENT_ENABLE_BLURBEHIND), or off.
    fn set_blur_behind(hwnd: HWND, on: bool) {
        const WCA_ACCENT_POLICY: u32 = 19;
        const ACCENT_DISABLED: u32 = 0;
        const ACCENT_ENABLE_BLURBEHIND: u32 = 3;
        let mut policy = AccentPolicy {
            accent_state: if on {
                ACCENT_ENABLE_BLURBEHIND
            } else {
                ACCENT_DISABLED
            },
            accent_flags: 0,
            gradient_color: 0,
            animation_id: 0,
        };
        let mut data = WindowCompositionAttribData {
            attrib: WCA_ACCENT_POLICY,
            pv_data: &mut policy as *mut AccentPolicy as *mut c_void,
            cb_data: std::mem::size_of::<AccentPolicy>(),
        };
        unsafe {
            SetWindowCompositionAttribute(hwnd, &mut data);
        }
    }

    /// Windows build number from the registry (0 when unknown).
    pub fn os_build() -> u32 {
        use windows_sys::Win32::System::Registry::{
            RegGetValueW, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ,
        };
        let key: Vec<u16> = "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\0"
            .encode_utf16()
            .collect();
        let val: Vec<u16> = "CurrentBuildNumber\0".encode_utf16().collect();
        let mut buf = [0u16; 32];
        let mut size = (buf.len() * 2) as u32;
        let rc = unsafe {
            RegGetValueW(
                HKEY_LOCAL_MACHINE,
                key.as_ptr(),
                val.as_ptr(),
                RRF_RT_REG_SZ,
                std::ptr::null_mut(),
                buf.as_mut_ptr() as *mut c_void,
                &mut size,
            )
        };
        if rc != 0 {
            return 0;
        }
        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        String::from_utf16_lossy(&buf[..len])
            .trim()
            .parse()
            .unwrap_or(0)
    }
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

                // 3. Blur behind the client area. DWMWA_SYSTEMBACKDROP_TYPE
                // only shows through when the frame is extended into the
                // client area, and an extended frame makes DWM paint the
                // native caption buttons over the app's own title bar. The
                // accent-policy blur works on the client area directly, so
                // keep the system backdrop off and the frame unextended.
                let none = super::BackdropKind::None as i32;
                let _ = DwmSetWindowAttribute(
                    hwnd,
                    DWMWA_SYSTEMBACKDROP_TYPE as u32,
                    &none as *const i32 as *const c_void,
                    4,
                );
                let margins = MARGINS {
                    cxLeftWidth: 0,
                    cxRightWidth: 0,
                    cyTopHeight: 0,
                    cyBottomHeight: 0,
                };
                let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);
                set_blur_behind(hwnd, backdrop_val != none);

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
