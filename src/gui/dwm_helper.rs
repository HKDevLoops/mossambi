//! Windows DWM System-Level Backdrop (Mica / Acrylic) & Compositor Integration
//!
//! Provides hardware-accelerated Windows Desktop Window Manager (DWM) Acrylic and Mica blur,
//! immersive dark mode synchronization, and frame margin extension for transparent windows.

use crate::core::{choose_backdrop, Backdrop, GlassEffect};

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

/// Applies native DWM system backdrop (Acrylic / Mica) and extends frame into client area.
#[allow(unused_variables)]
pub fn apply_system_backdrop(dark_mode: bool, effect: GlassEffect, opacity: f32) {
    #[cfg(target_os = "windows")]
    {
        win32_dwm::apply_dwm_attributes(dark_mode, effect);
    }
}

/// Returns the HWND of the application's top-level window (0 if not found).
pub fn find_main_window_hwnd() -> isize {
    #[cfg(target_os = "windows")]
    {
        win32_dwm::find_main_hwnd()
    }
    #[cfg(not(target_os = "windows"))]
    {
        0
    }
}

#[cfg(target_os = "windows")]
mod win32_dwm {
    use super::{choose_backdrop, Backdrop, GlassEffect};
    use std::ffi::c_void;
    use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM};
    use windows_sys::Win32::Graphics::Dwm::{
        DwmExtendFrameIntoClientArea, DwmSetWindowAttribute, DWMWA_SYSTEMBACKDROP_TYPE,
        DWMWA_USE_IMMERSIVE_DARK_MODE,
    };
    use windows_sys::Win32::UI::Controls::MARGINS;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetClassNameW, GetParent, GetWindowLongPtrW, GetWindowThreadProcessId,
        SetWindowLongPtrW, GWL_EXSTYLE, GWL_STYLE, WS_CHILD, WS_EX_LAYERED,
    };

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

    // Undocumented but stable user32 export for Windows 10 acrylic/blur behind
    #[link(name = "user32", kind = "raw-dylib")]
    extern "system" {
        fn SetWindowCompositionAttribute(
            hwnd: HWND,
            data: *mut WindowCompositionAttribData,
        ) -> BOOL;
    }

    fn set_accent_policy(hwnd: HWND, state: u32, gradient_color: u32) {
        const WCA_ACCENT_POLICY: u32 = 19;
        let mut policy = AccentPolicy {
            accent_state: state,
            accent_flags: if state == 4 { 2 } else { 0 },
            gradient_color,
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

    pub fn find_main_hwnd() -> isize {
        unsafe extern "system" fn enum_find_main(hwnd: HWND, lparam: LPARAM) -> BOOL {
            let current_pid = std::process::id();
            let mut wnd_pid = 0u32;
            GetWindowThreadProcessId(hwnd, &mut wnd_pid);

            if wnd_pid == current_pid {
                let mut class_name = [0u16; 256];
                let len = GetClassNameW(hwnd, class_name.as_mut_ptr(), 256);
                let class_str = String::from_utf16_lossy(&class_name[..len as usize]);

                let is_top_level = GetParent(hwnd).is_null();
                let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
                let is_not_child = (style & (WS_CHILD as isize)) == 0;
                if is_top_level
                    && is_not_child
                    && !class_str.is_empty()
                    && !class_str.contains("Message")
                {
                    let out = lparam as *mut isize;
                    *out = hwnd as isize;
                    return 0; // stop
                }
            }
            1
        }

        let mut found: isize = 0;
        unsafe {
            EnumWindows(Some(enum_find_main), &mut found as *mut isize as LPARAM);
        }
        found
    }

    pub fn apply_dwm_attributes(dark_mode: bool, effect: GlassEffect) {
        let build = os_build();
        let backdrop = choose_backdrop(effect, build);

        // Pack backdrop and dark mode into LPARAM:
        // Tag: 0 = None, 1 = DwmSystem, 2 = Win10Accent
        let (tag, val) = match backdrop {
            Backdrop::None => (0u32, 0u32),
            Backdrop::DwmSystem(k) => (1u32, k),
            Backdrop::Win10Accent(s) => (2u32, s),
        };
        let param = (tag << 8) | (val & 0xFF) | if dark_mode { 0x10000 } else { 0 };

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

            // Matches primary top-level window for eframe / winit (winit window, Window Class, etc.)
            let is_top_level = GetParent(hwnd).is_null();
            let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
            let is_not_child = (style & (WS_CHILD as isize)) == 0;
            let is_candidate = is_top_level
                && is_not_child
                && !class_str.is_empty()
                && !class_str.contains("Message");

            if is_candidate {
                let dark_mode = (lparam & 0x10000) != 0;
                let tag = ((lparam >> 8) & 0xFF) as u32;
                let val = (lparam & 0xFF) as u32;

                // 1. Extend frame into client area (-1 margins = whole client area)
                let margins = MARGINS {
                    cxLeftWidth: -1,
                    cxRightWidth: -1,
                    cyTopHeight: -1,
                    cyBottomHeight: -1,
                };
                let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);

                // 2. Synchronize Immersive Dark Mode with active theme
                let dark_val: i32 = if dark_mode { 1 } else { 0 };
                let _ = DwmSetWindowAttribute(
                    hwnd,
                    DWMWA_USE_IMMERSIVE_DARK_MODE as u32,
                    &dark_val as *const i32 as *const c_void,
                    4,
                );

                // 3. Remove WS_EX_LAYERED whole-window alpha path entirely
                let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                if ex & (WS_EX_LAYERED as isize) != 0 {
                    SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex & !(WS_EX_LAYERED as isize));
                }

                // 4. Apply system backdrop
                match tag {
                    1 => {
                        // Win11 DWMWA_SYSTEMBACKDROP_TYPE (build 22621+)
                        let sys_val = val as i32;
                        let _ = DwmSetWindowAttribute(
                            hwnd,
                            DWMWA_SYSTEMBACKDROP_TYPE as u32,
                            &sys_val as *const i32 as *const c_void,
                            4,
                        );
                        // Windows 11 build 22000 fallback (DWMWA_MICA_EFFECT = 1029)
                        if val == 2 {
                            const DWMWA_MICA_EFFECT: u32 = 1029;
                            let mica_on = 1i32;
                            let _ = DwmSetWindowAttribute(
                                hwnd,
                                DWMWA_MICA_EFFECT,
                                &mica_on as *const i32 as *const c_void,
                                4,
                            );
                        }
                        // winit's per-pixel-alpha GL path resets
                        // DWMWA_SYSTEMBACKDROP_TYPE on the visible window
                        // (readback stays 0 and external re-sets do not
                        // stick), so acrylic-family backdrops also apply the
                        // accent-policy acrylic, which composes correctly
                        // with the transparent GL surface. The gradient tint
                        // stays nearly transparent (alpha = 1): the egui
                        // glass fills supply the tint, DWM only the blur.
                        if val == 3 {
                            let tint_abgr = if dark_mode { 0x011E1E1E } else { 0x01FFFFFF };
                            set_accent_policy(hwnd, 4, tint_abgr);
                        } else {
                            set_accent_policy(hwnd, 0, 0);
                        }
                    }
                    2 => {
                        // Win10 SetWindowCompositionAttribute accent policy
                        let none = 1i32; // DWMSBT_NONE
                        let _ = DwmSetWindowAttribute(
                            hwnd,
                            DWMWA_SYSTEMBACKDROP_TYPE as u32,
                            &none as *const i32 as *const c_void,
                            4,
                        );
                        let tint_abgr = if dark_mode { 0x011E1E1E } else { 0x01FFFFFF };
                        set_accent_policy(hwnd, val, tint_abgr);
                    }
                    _ => {
                        // Off / Solid container
                        let none = 1i32;
                        let _ = DwmSetWindowAttribute(
                            hwnd,
                            DWMWA_SYSTEMBACKDROP_TYPE as u32,
                            &none as *const i32 as *const c_void,
                            4,
                        );
                        set_accent_policy(hwnd, 0, 0);
                    }
                }

                // 5. Windows 11 Rounded Corners (DWMWA_WINDOW_CORNER_PREFERENCE = 33, DWMWCP_ROUND = 2)
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
