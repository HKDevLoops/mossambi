//! Windows 11 Explorer-parity context menu verbs.
//!
//! These wrap native Windows shell actions (verbs, clipboard, wallpaper,
//! Defender scan…) so Mossambi's right-click flyouts match what Explorer shows.
//! No external crates required — only Win32 via windows-sys.

use std::path::Path;

/// Whether the Defender scan verb can execute (Windows only).
pub fn win_defender_available() -> bool {
    cfg!(target_os = "windows")
}

/// Execute a registered shell verb on `path` (e.g. "open", "print", "play", "edit")
pub fn shell_verb(path: &Path, verb: &str) -> bool {
    #[cfg(target_os = "windows")]
    unsafe {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::UI::Shell::ShellExecuteW;
        use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

        let wide: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
        let action: Vec<u16> = OsStr::new(verb)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let res = ShellExecuteW(
            std::ptr::null_mut(),
            action.as_ptr(),
            wide.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        );
        // returns HINSTANCE > 32 on success
        (res as usize) > 32
    }
    #[cfg(not(target_os = "windows"))]
    {
        use std::process::Command;
        Command::new("xdg-open").arg(path).spawn().is_ok()
    }
}

/// Open the system "How do you want to open this file?" (openas)
pub fn open_with_choose(path: &Path) -> bool {
    #[cfg(target_os = "windows")]
    unsafe {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::UI::Shell::ShellExecuteW;
        use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

        // rundll32 shell32.dll,OpenAs_RunDLL <file> is the classic openas picker
        let args = format!("shell32.dll,OpenAs_RunDLL \"{}\"", path.to_string_lossy());
        let wide: Vec<u16> = std::ffi::OsStr::new("rundll32")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let wide_args: Vec<u16> = std::ffi::OsStr::new(&args)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let raw = wide_args.as_ptr();
        let res = ShellExecuteW(
            std::ptr::null_mut(),
            std::ptr::null(),
            wide.as_ptr(),
            raw,
            std::ptr::null(),
            SW_SHOWNORMAL,
        );
        // openas verb on Win10+ also works:
        if (res as usize) > 32 {
            return true;
        }
        shell_verb(path, "openas")
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

/// Print a document with the associated app (verb "print")
pub fn print_file(path: &Path) -> bool {
    shell_verb(path, "print")
}

/// Play with the associated media player (verb "play")
pub fn play_file(path: &Path) -> bool {
    shell_verb(path, "play")
}

/// Copy a list of files to the OLE clipboard (CF_HDROP + PreferredDropEffect) so
/// Explorer can paste them with the intended verb (copy or move).
pub fn set_clipboard_files(paths: &[std::path::PathBuf]) -> bool {
    set_clipboard_files_with_effect(paths, false)
}

pub fn set_clipboard_files_with_effect(paths: &[std::path::PathBuf], cut: bool) -> bool {
    #[cfg(target_os = "windows")]
    unsafe {
        use std::os::windows::ffi::OsStrExt;
        use std::mem::size_of;
        use windows_sys::Win32::System::DataExchange::{
            CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
        };
        use windows_sys::Win32::UI::Shell::DROPFILES;
        use windows_sys::Win32::Foundation::HANDLE;

        const CF_HDROP: u32 = 15;

        let joined: Vec<u16> = paths
            .iter()
            .flat_map(|p| p.as_os_str().encode_wide().chain(std::iter::once(0)))
            .chain(std::iter::once(0))
            .collect();
        if joined.is_empty() {
            return false;
        }
        let bytes = std::mem::size_of::<u16>() * joined.len();
        if OpenClipboard(std::ptr::null_mut()) == 0 {
            return false;
        }
        EmptyClipboard();

        let mem_bytes = size_of::<DROPFILES>() + bytes;
        let mem = windows_sys::Win32::System::Memory::GlobalAlloc(
            windows_sys::Win32::System::Memory::GMEM_MOVEABLE,
            mem_bytes,
        );
        if mem.is_null() {
            let _ = CloseClipboard();
            return false;
        }

        let p_mem = windows_sys::Win32::System::Memory::GlobalLock(mem);
        if p_mem.is_null() {
            let _ = CloseClipboard();
            return false;
        }

        let dropfiles = DROPFILES {
            pFiles: size_of::<DROPFILES>() as u32,
            pt: windows_sys::Win32::Foundation::POINT { x: 0, y: 0 },
            fNC: 0,
            fWide: 1,
        };
        std::ptr::write_bytes(p_mem, 0, mem_bytes);
        std::ptr::copy_nonoverlapping(
            &dropfiles as *const DROPFILES as *const u8,
            p_mem as *mut u8,
            size_of::<DROPFILES>(),
        );
        std::ptr::copy_nonoverlapping(
            joined.as_ptr() as *const u8,
            (p_mem as *mut u8).add(size_of::<DROPFILES>()),
            bytes,
        );
        let _ = windows_sys::Win32::System::Memory::GlobalUnlock(mem);

        if SetClipboardData(CF_HDROP, mem as HANDLE).is_null() {
            let _ = CloseClipboard();
            // On failure we still own hmem (system did not take ownership).
            // Avoid relying on GlobalFree feature availability by exiting safely.
            return false;
        }

        // FileNameW + PreferredDropEffect: tell Explorer whether to Move or Copy
        const DROPEFFECT_COPY: u32 = 1;
        const DROPEFFECT_MOVE: u32 = 2;
        let effect = if cut { DROPEFFECT_MOVE } else { DROPEFFECT_COPY };
        let pref_name: Vec<u16> = "Preferred DropEffect".encode_utf16().chain(std::iter::once(0)).collect();
        let pref_id = windows_sys::Win32::System::DataExchange::RegisterClipboardFormatW(pref_name.as_ptr());

        let pref_mem = windows_sys::Win32::System::Memory::GlobalAlloc(
            windows_sys::Win32::System::Memory::GMEM_MOVEABLE,
            4,
        );
        if !pref_mem.is_null() {
            let pref_ptr = windows_sys::Win32::System::Memory::GlobalLock(pref_mem);
            if !pref_ptr.is_null() {
                std::ptr::write_bytes(pref_ptr as *mut u32, 0, 1);
                std::ptr::write_unaligned(pref_ptr as *mut u32, effect);
                let _ = windows_sys::Win32::System::Memory::GlobalUnlock(pref_mem);
                let _ = SetClipboardData(pref_id, pref_mem as HANDLE);
            }
            // On success the system takes ownership; on lock failure we simply abandon the allocation (tiny leak, app continues)
        }

        // On success the system takes ownership of hmem
        let _ = CloseClipboard();
        true
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

/// Set the given image path as the desktop wallpaper (SPI_SETDESKWALLPAPER)
pub fn set_desktop_wallpaper(path: &Path) -> bool {
    #[cfg(target_os = "windows")]
    unsafe {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            SystemParametersInfoW, SPIF_SENDCHANGE, SPIF_UPDATEINIFILE,
        };

        const SPI_SETDESKWALLPAPER: u32 = 0x0014;
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
        let res = SystemParametersInfoW(
            SPI_SETDESKWALLPAPER,
            0,
            wide.as_ptr() as *mut _,
            SPIF_SENDCHANGE | SPIF_UPDATEINIFILE,
        );
        res != 0
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

/// Create a `.lnk` shortcut to `target` alongside it via PowerShell COM (WScript.Shell)
pub fn create_shortcut(target: &Path) -> bool {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let shortcut = target.with_extension("lnk");
        // Use PowerShell hidden window for a simple, reliable shortcut creation
        let cmd = format!(
            "$ws=New-Object -ComObject WScript.Shell;$s=$ws.CreateShortcut('{}');$s.TargetPath='{}';$s.WorkingDirectory='{}';$s.Save()",
            shortcut.to_string_lossy(),
            target.to_string_lossy(),
            target.parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
        );
        std::process::Command::new("powershell")
            .arg("-NoProfile")
            .arg("-WindowStyle")
            .arg("Hidden")
            .arg("-Command")
            .arg(cmd)
            .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
            .spawn()
            .is_ok()
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

/// Run a Microsoft Defender custom scan on `path` (fire and forget, hidden)
pub fn scan_with_defender(path: &Path) -> bool {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let cmd = format!(
            "Start-MpScan -ScanPath '{}'",
            path.to_string_lossy().replace('\'', "''")
        );
        std::process::Command::new("powershell")
            .arg("-NoProfile")
            .arg("-WindowStyle")
            .arg("Hidden")
            .arg("-Command")
            .arg(cmd)
            .creation_flags(0x0800_0000)
            .spawn()
            .is_ok()
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

/// Rotate an image by 90° without touching EXIF orientation flags
pub fn rotate_image_90(path: &Path) -> bool {
    if let Some(parent) = path.parent() {
        let _ = parent;
    }
    let img = match image::open(path) {
        Ok(i) => i,
        Err(_) => return false,
    };
    let rotated = img.rotate90();
    rotated.save(path).is_ok()
}
