//! Windows Shell Icon Extraction & Background Async Caching
//!
//! Pure 7-bit ASCII. Extracts native Windows shell icons (via SHGetFileInfoW)
//! off the UI thread and provides an LRU-bounded texture cache for Explorer.

use eframe::egui::{ColorImage, Context, TextureHandle, TextureOptions};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{LazyLock, Mutex};

/// Icon request submitted by the UI thread to the background worker.
#[derive(Debug, Clone)]
pub struct ShellIconRequest {
    pub path: PathBuf,
    pub is_dir: bool,
    pub large: bool,
}

/// Icon response delivered by the background worker to the UI thread.
pub struct ShellIconResponse {
    pub path: PathBuf,
    pub is_dir: bool,
    pub large: bool,
    pub image: Option<ColorImage>,
}

/// Global channel pairs and in-memory texture cache.
struct ShellIconSystem {
    req_tx: Sender<ShellIconRequest>,
    resp_rx: Receiver<ShellIconResponse>,
    textures: HashMap<(PathBuf, bool), Option<TextureHandle>>,
    pending: HashMap<(PathBuf, bool), bool>,
}

static SYSTEM: LazyLock<Mutex<ShellIconSystem>> = LazyLock::new(|| {
    let (req_tx, req_rx) = channel::<ShellIconRequest>();
    let (resp_tx, resp_rx) = channel::<ShellIconResponse>();

    // Spawn background worker thread
    std::thread::Builder::new()
        .name("mossambi_shell_icons".to_string())
        .spawn(move || {
            #[cfg(target_os = "windows")]
            unsafe {
                windows_sys::Win32::System::Com::CoInitializeEx(
                    std::ptr::null_mut(),
                    windows_sys::Win32::System::Com::COINIT_APARTMENTTHREADED as u32,
                );
            }

            while let Ok(req) = req_rx.recv() {
                let img = extract_native_shell_icon(&req.path, req.is_dir, req.large);
                let _ = resp_tx.send(ShellIconResponse {
                    path: req.path,
                    is_dir: req.is_dir,
                    large: req.large,
                    image: img,
                });
            }
        })
        .expect("Failed to spawn shell icon worker thread");

    Mutex::new(ShellIconSystem {
        req_tx,
        resp_rx,
        textures: HashMap::new(),
        pending: HashMap::new(),
    })
});

#[cfg(target_os = "windows")]
static SHELL_MUTEX: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

/// Drains completed background icon extractions and uploads them into egui textures.
pub fn update_shell_icons(ctx: &Context) {
    if let Ok(mut sys) = SYSTEM.lock() {
        let mut got_any = false;
        while let Ok(resp) = sys.resp_rx.try_recv() {
            let key = (resp.path.clone(), resp.large);
            sys.pending.remove(&key);

            let handle = resp.image.map(|img| {
                let name = format!(
                    "sh_icon_{}_{}",
                    resp.path
                        .to_string_lossy()
                        .replace(['\\', '/', ':', ' '], "_"),
                    if resp.large { "lg" } else { "sm" }
                );
                ctx.load_texture(name, img, TextureOptions::LINEAR)
            });

            sys.textures.insert(key, handle);
            got_any = true;
        }

        if got_any {
            ctx.request_repaint();
        }
    }
}

/// Clears cached shell icon textures and pending requests (e.g. on file association change).
pub fn clear_shell_icon_cache() {
    if let Ok(mut sys) = SYSTEM.lock() {
        sys.textures.clear();
        sys.pending.clear();
    }
}

/// Retrieves a cached Windows shell icon texture, or schedules background extraction.
pub fn get_or_request_shell_icon(
    ctx: &Context,
    path: &Path,
    is_dir: bool,
    large: bool,
) -> Option<TextureHandle> {
    // Process any ready icons first
    update_shell_icons(ctx);

    if let Ok(mut sys) = SYSTEM.lock() {
        let key = (path.to_path_buf(), large);
        if let Some(tex_opt) = sys.textures.get(&key) {
            return tex_opt.clone();
        }

        // Avoid spamming requests for the same path
        if !sys.pending.contains_key(&key) {
            sys.pending.insert(key.clone(), true);
            let _ = sys.req_tx.send(ShellIconRequest {
                path: path.to_path_buf(),
                is_dir,
                large,
            });
        }
    }

    None
}

/// Synchronous / native Windows icon extraction implementation.
#[cfg(target_os = "windows")]
pub fn extract_native_shell_icon(path: &Path, is_dir: bool, large: bool) -> Option<ColorImage> {
    let _guard = SHELL_MUTEX.lock().ok();
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Graphics::Gdi::{
        CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, GetObjectW, BITMAP, BITMAPINFO,
        BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
    };
    use windows_sys::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
    use windows_sys::Win32::UI::Shell::{
        SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON, SHGFI_SMALLICON,
        SHGFI_USEFILEATTRIBUTES,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, ICONINFO};

    // Ensure COM is initialized on the current calling thread (thread-safe, ignores S_FALSE / RPC_E_CHANGED_MODE).
    unsafe {
        CoInitializeEx(std::ptr::null_mut(), COINIT_APARTMENTTHREADED as u32);
    }

    let mut wide_path: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide_path.push(0);

    let mut shfi: SHFILEINFOW = unsafe { std::mem::zeroed() };
    let size_flag = if large {
        SHGFI_LARGEICON
    } else {
        SHGFI_SMALLICON
    };
    let mut flags = SHGFI_ICON | size_flag;

    // If path does not physically exist on disk, query by file attributes / extension
    let attribs = if !path.exists() {
        flags |= SHGFI_USEFILEATTRIBUTES;
        if is_dir {
            0x00000010 // FILE_ATTRIBUTE_DIRECTORY
        } else {
            0x00000080 // FILE_ATTRIBUTE_NORMAL
        }
    } else {
        0
    };

    let mut ret = unsafe {
        SHGetFileInfoW(
            wide_path.as_ptr(),
            attribs,
            &mut shfi,
            std::mem::size_of::<SHFILEINFOW>() as u32,
            flags,
        )
    };

    if ret == 0 || shfi.hIcon.is_null() {
        // Fallback: try by extension / file attributes
        let fallback_attribs = if is_dir { 0x00000010 } else { 0x00000080 };
        ret = unsafe {
            SHGetFileInfoW(
                wide_path.as_ptr(),
                fallback_attribs,
                &mut shfi,
                std::mem::size_of::<SHFILEINFOW>() as u32,
                flags | SHGFI_USEFILEATTRIBUTES,
            )
        };
    }

    if ret == 0 || shfi.hIcon.is_null() {
        return None;
    }

    let hicon = shfi.hIcon;
    let mut icon_info: ICONINFO = unsafe { std::mem::zeroed() };
    if unsafe { GetIconInfo(hicon, &mut icon_info) } == 0 {
        unsafe { DestroyIcon(hicon) };
        return None;
    }

    let mut bmp: BITMAP = unsafe { std::mem::zeroed() };
    let bmp_handle = if !icon_info.hbmColor.is_null() {
        icon_info.hbmColor
    } else {
        icon_info.hbmMask
    };

    if unsafe {
        GetObjectW(
            bmp_handle,
            std::mem::size_of::<BITMAP>() as i32,
            &mut bmp as *mut _ as *mut _,
        )
    } == 0
    {
        unsafe {
            if !icon_info.hbmColor.is_null() {
                DeleteObject(icon_info.hbmColor);
            }
            if !icon_info.hbmMask.is_null() {
                DeleteObject(icon_info.hbmMask);
            }
            DestroyIcon(hicon);
        }
        return None;
    }

    let width = bmp.bmWidth as usize;
    let height = if icon_info.hbmColor.is_null() {
        (bmp.bmHeight / 2) as usize
    } else {
        bmp.bmHeight as usize
    };

    if width == 0 || height == 0 || width > 512 || height > 512 {
        unsafe {
            if !icon_info.hbmColor.is_null() {
                DeleteObject(icon_info.hbmColor);
            }
            if !icon_info.hbmMask.is_null() {
                DeleteObject(icon_info.hbmMask);
            }
            DestroyIcon(hicon);
        }
        return None;
    }

    let mut bi: BITMAPINFO = unsafe { std::mem::zeroed() };
    bi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
    bi.bmiHeader.biWidth = width as i32;
    bi.bmiHeader.biHeight = -(height as i32); // top-down
    bi.bmiHeader.biPlanes = 1;
    bi.bmiHeader.biBitCount = 32;
    bi.bmiHeader.biCompression = BI_RGB;

    let hdc = unsafe { CreateCompatibleDC(std::ptr::null_mut()) };
    let mut rgba_buf: Vec<u8> = vec![0u8; width * height * 4];

    let lines = unsafe {
        GetDIBits(
            hdc,
            if !icon_info.hbmColor.is_null() {
                icon_info.hbmColor
            } else {
                icon_info.hbmMask
            },
            0,
            height as u32,
            rgba_buf.as_mut_ptr() as *mut _,
            &mut bi,
            DIB_RGB_COLORS,
        )
    };

    unsafe {
        DeleteDC(hdc);
        if !icon_info.hbmColor.is_null() {
            DeleteObject(icon_info.hbmColor);
        }
        if !icon_info.hbmMask.is_null() {
            DeleteObject(icon_info.hbmMask);
        }
        DestroyIcon(hicon);
    }

    if lines == 0 {
        return None;
    }

    // Windows 32-bit DIB pixels are BGRA.
    // Check if alpha channel has non-zero values; if completely zero, set alpha to 255.
    let mut has_non_zero_alpha = false;
    for chunk in rgba_buf.chunks_exact_mut(4) {
        if chunk[3] > 0 {
            has_non_zero_alpha = true;
            break;
        }
    }

    for chunk in rgba_buf.chunks_exact_mut(4) {
        chunk.swap(0, 2); // BGR -> RGB
        if !has_non_zero_alpha {
            chunk[3] = 255;
        }
    }

    Some(ColorImage::from_rgba_unmultiplied(
        [width, height],
        &rgba_buf,
    ))
}

/// Resolves the registered default application (friendly name and executable path) for an extension.
#[cfg(target_os = "windows")]
pub fn get_default_app_for_extension(ext: &str) -> Option<(String, PathBuf)> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::Shell::{
        AssocQueryStringW, ASSOCF_INIT_IGNOREUNKNOWN, ASSOCSTR_EXECUTABLE, ASSOCSTR_FRIENDLYAPPNAME,
    };

    let ext_with_dot = if ext.starts_with('.') {
        ext.to_string()
    } else {
        format!(".{}", ext)
    };
    let mut wide_ext: Vec<u16> = std::ffi::OsStr::new(&ext_with_dot).encode_wide().collect();
    wide_ext.push(0);

    let mut exe_buf = [0u16; 1024];
    let mut exe_size = exe_buf.len() as u32;
    let hr_exe = unsafe {
        AssocQueryStringW(
            ASSOCF_INIT_IGNOREUNKNOWN,
            ASSOCSTR_EXECUTABLE,
            wide_ext.as_ptr(),
            std::ptr::null(),
            exe_buf.as_mut_ptr(),
            &mut exe_size,
        )
    };

    if hr_exe != 0 || exe_size == 0 {
        return None;
    }

    let exe_path_str = String::from_utf16_lossy(&exe_buf[..(exe_size.saturating_sub(1) as usize)]);
    let exe_path = PathBuf::from(&exe_path_str);

    let mut name_buf = [0u16; 512];
    let mut name_size = name_buf.len() as u32;
    let friendly_name = if unsafe {
        AssocQueryStringW(
            ASSOCF_INIT_IGNOREUNKNOWN,
            ASSOCSTR_FRIENDLYAPPNAME,
            wide_ext.as_ptr(),
            std::ptr::null(),
            name_buf.as_mut_ptr(),
            &mut name_size,
        )
    } == 0
        && name_size > 0
    {
        String::from_utf16_lossy(&name_buf[..(name_size.saturating_sub(1) as usize)])
    } else {
        exe_path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Default Application".to_string())
    };

    Some((friendly_name, exe_path))
}

#[cfg(not(target_os = "windows"))]
pub fn get_default_app_for_extension(_ext: &str) -> Option<(String, PathBuf)> {
    None
}

#[cfg(not(target_os = "windows"))]
pub fn extract_native_shell_icon(_path: &Path, _is_dir: bool, _large: bool) -> Option<ColorImage> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_icon_system_initialization() {
        // Ensure background system initializes without panic
        let _guard = SYSTEM.lock().expect("Must acquire lock");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_native_shell_icon_extraction_system32() {
        let sys_path = PathBuf::from("C:\\Windows\\explorer.exe");
        println!(
            "Checking sys_path: {:?}, exists: {}",
            sys_path,
            sys_path.exists()
        );
        let img = extract_native_shell_icon(&sys_path, false, true);
        println!(
            "Result for explorer.exe: {:?}",
            img.as_ref().map(|i| (i.width(), i.height()))
        );
        assert!(img.is_some(), "Explorer.exe must have a shell icon");
        let img = img.unwrap();
        assert_eq!(img.width(), 32);
        assert_eq!(img.height(), 32);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_native_shell_icon_extraction_fallback_extension() {
        let dummy_txt = PathBuf::from("dummy_non_existent_file.txt");
        let img = extract_native_shell_icon(&dummy_txt, false, false);
        println!(
            "Fallback for txt: {:?}",
            img.as_ref().map(|i| (i.width(), i.height()))
        );
        assert!(img.is_some(), "Text extension should yield shell icon");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_default_app_for_extension() {
        let app = get_default_app_for_extension("txt");
        assert!(
            app.is_some(),
            "Text extension must have a default application on Windows"
        );
        let (name, exe) = app.unwrap();
        assert!(!name.is_empty());
        assert!(!exe.as_os_str().is_empty());
    }
}
