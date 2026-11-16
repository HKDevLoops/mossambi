//! Windows-specific platform implementation details.
//!
//! Pure 7-bit ASCII.

use std::path::Path;

pub fn open_in_default_app(path: &Path) -> bool {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide.push(0);

    let mut op_open: Vec<u16> = std::ffi::OsStr::new("open").encode_wide().collect();
    op_open.push(0);

    unsafe {
        let res = ShellExecuteW(
            std::ptr::null_mut(),
            op_open.as_ptr(),
            wide.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        );
        if res as usize <= 32 {
            use std::os::windows::process::CommandExt;
            std::process::Command::new("cmd")
                .args(["/c", "start", "", path.to_string_lossy().as_ref()])
                .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
                .spawn()
                .is_ok()
        } else {
            true
        }
    }
}

/// W4b dialog companion bar (opt-in): pure classifier functions plus the
/// Win32 companion thread that docks a quick-jump bar beside Open/Save dialogs.
pub mod dialog_companion;
