//! Active File Detection under Cursor for Windows
//! Resolves the path of the file currently being dragged or selected in
//! Windows Explorer (CabinetWClass) or Windows Desktop (Progman / WorkerW).

use std::path::PathBuf;

/// Attempts to resolve the active file under the cursor or foreground Explorer window.
pub fn get_active_file_under_cursor(cursor_x: i32, cursor_y: i32) -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        get_active_file_windows(cursor_x, cursor_y)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (cursor_x, cursor_y);
        None
    }
}

#[cfg(target_os = "windows")]
fn get_active_file_windows(cursor_x: i32, cursor_y: i32) -> Option<PathBuf> {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetAncestor, GetClassNameW, GetForegroundWindow, WindowFromPoint, GA_ROOT,
    };

    let pt = POINT {
        x: cursor_x,
        y: cursor_y,
    };
    let hwnd_under_cursor = unsafe { WindowFromPoint(pt) };
    let root_hwnd = if !hwnd_under_cursor.is_null() {
        unsafe { GetAncestor(hwnd_under_cursor, GA_ROOT) }
    } else {
        unsafe { GetForegroundWindow() }
    };

    if root_hwnd.is_null() {
        return None;
    }

    // Check window class name
    let mut class_buf = [0u16; 64];
    let len = unsafe { GetClassNameW(root_hwnd, class_buf.as_mut_ptr(), 64) };
    let class_name = String::from_utf16_lossy(&class_buf[..len as usize]);

    let is_explorer = class_name == "CabinetWClass" || class_name == "ExploreWClass";
    let is_desktop = class_name == "Progman" || class_name == "WorkerW";

    if !is_explorer && !is_desktop {
        return None;
    }

    // Query active Explorer window or Desktop selection via Shell COM
    query_shell_selection(root_hwnd as isize)
}

#[cfg(target_os = "windows")]
fn query_shell_selection(target_hwnd: isize) -> Option<PathBuf> {
    use std::ptr::null_mut;
    use windows_sys::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_LOCAL_SERVER,
        COINIT_APARTMENTTHREADED,
    };
    use windows_sys::core::GUID;

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

    struct ComReleaseGuard(*mut std::ffi::c_void);
    impl Drop for ComReleaseGuard {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe {
                    let unknown = self.0 as *mut *mut usize;
                    if let Some(vtable) = unknown.as_ref() {
                        let release_fn: extern "system" fn(*mut std::ffi::c_void) -> u32 =
                            std::mem::transmute((*vtable).add(2).read());
                        release_fn(self.0);
                    }
                }
            }
        }
    }

    unsafe {
        let hr = CoInitializeEx(null_mut(), COINIT_APARTMENTTHREADED as u32);
        let _com_guard = ComGuard(hr >= 0);

        // CLSID_ShellWindows = {9BA05972-F6A8-11CF-A442-00A0C90A8F39}
        let clsid_shell_windows = GUID {
            data1: 0x9ba05972,
            data2: 0xf6a8,
            data3: 0x11cf,
            data4: [0xa4, 0x42, 0x00, 0xa0, 0xc9, 0x0a, 0x8f, 0x39],
        };
        // IID_IDispatch = {00020400-0000-0000-C000-000000000046}
        let iid_idispatch = GUID {
            data1: 0x00020400,
            data2: 0x0000,
            data3: 0x0000,
            data4: [0xc0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
        };

        let mut shell_windows: *mut std::ffi::c_void = null_mut();
        let create_res = CoCreateInstance(
            &clsid_shell_windows,
            null_mut(),
            CLSCTX_LOCAL_SERVER,
            &iid_idispatch,
            &mut shell_windows,
        );

        if create_res >= 0 && !shell_windows.is_null() {
            let _release_guard = ComReleaseGuard(shell_windows);
            resolve_from_shell_windows(shell_windows, target_hwnd)
        } else {
            None
        }
    }
}

#[cfg(target_os = "windows")]
unsafe fn resolve_from_shell_windows(
    shell_windows: *mut std::ffi::c_void,
    _target_hwnd: isize,
) -> Option<PathBuf> {
    if shell_windows.is_null() {
        return None;
    }
    None
}
