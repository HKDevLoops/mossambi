//! Windows Shell Context Menu Integration & Background Async Extraction
//!
//! Pure 7-bit ASCII. Queries native Windows Explorer context menus via
//! IShellFolder / IContextMenu / IContextMenu2 / IContextMenu3, preserving
//! 3rd-party shell extensions, submenus, and icons without blocking the UI.

use eframe::egui::ColorImage;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, LazyLock, Mutex};

/// A single node in the shell context menu tree.
#[derive(Debug, Clone)]
pub struct ShellMenuItem {
    pub id: u32,
    pub verb: String,
    pub title: String,
    pub is_separator: bool,
    pub is_disabled: bool,
    pub is_checked: bool,
    pub icon: Option<ColorImage>,
    pub children: Vec<ShellMenuItem>,
}

/// Request to build a shell menu on the STA worker thread.
#[derive(Debug, Clone)]
pub struct ShellMenuBuildRequest {
    pub session_id: u64,
    pub paths: Vec<PathBuf>,
    pub is_background: bool,
    pub shift_held: bool,
    pub hwnd: usize,
}

/// Request to invoke a shell command on the STA worker thread.
#[derive(Debug, Clone)]
pub struct ShellMenuInvokeRequest {
    pub session_id: u64,
    pub id: u32,
    pub pt: (i32, i32),
}

/// Clean menu item title by stripping single '&' accelerators but keeping '&&' as '&'.
pub fn clean_menu_title(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '&' {
            if chars.peek() == Some(&'&') {
                out.push('&');
                chars.next();
            }
            // Skip single accelerator marker
        } else {
            out.push(ch);
        }
    }
    out.trim().to_string()
}

/// Whether a verb or title is one of Mossambi's pre-existing top-level menu actions.
pub fn is_duplicate_verb(verb: &str, title: &str) -> bool {
    let v = verb.trim().to_lowercase();
    let t = clean_menu_title(title).to_lowercase();
    matches!(
        v.as_str(),
        "open" | "edit" | "cut" | "copy" | "paste" | "delete" | "rename" | "properties" | "print"
    ) || matches!(
        t.as_str(),
        "open" | "edit" | "cut" | "copy" | "paste" | "delete" | "rename" | "properties" | "print"
    )
}

#[cfg(target_os = "windows")]
pub mod win32_impl {
    use super::*;
    use std::os::windows::ffi::OsStrExt;
    use windows::core::{Interface, PCWSTR, PSTR, PWSTR};
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::Graphics::Gdi::{
        CreateCompatibleDC, DeleteDC, GetDIBits, GetObjectW, BITMAP, BITMAPINFO, BITMAPINFOHEADER,
        BI_RGB, DIB_RGB_COLORS, HBITMAP,
    };
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
    pub use windows::Win32::UI::Shell::Common::ITEMIDLIST;
    pub use windows::Win32::UI::Shell::{
        IContextMenu, IContextMenu2, IContextMenu3, ILFindLastID, ILFree, IShellFolder,
        SHBindToParent, SHGetDesktopFolder, SHParseDisplayName, CMF_EXTENDEDVERBS, CMF_NORMAL,
        CMIC_MASK_PTINVOKE, CMINVOKECOMMANDINFO, CMINVOKECOMMANDINFOEX, GCS_VERBW,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        CreatePopupMenu, DestroyMenu, GetMenuItemCount, GetMenuItemInfoW, HMENU, MENUITEMINFOW,
        MFS_CHECKED, MFS_DISABLED, MFT_SEPARATOR, MIIM_BITMAP, MIIM_DATA, MIIM_FTYPE, MIIM_ID,
        MIIM_STATE, MIIM_STRING, MIIM_SUBMENU, WM_INITMENUPOPUP,
    };

    pub const CMIC_MASK_UNICODE: u32 = 0x0000_4000;
    pub const GCS_VERBA: u32 = 0x0000_0000;
    pub const ID_CMD_FIRST: u32 = 1;
    pub const ID_CMD_LAST: u32 = 0x7FFF;

    /// Scoped RAII guard that temporarily redirects Win32 STD_ERROR_HANDLE and CRT fd 2
    /// to the NUL device, suppressing noisy error logging from third-party shell extensions
    /// (e.g. Google Drive, Dropbox, shell IPC clients writing unhandled socket errors to stderr).
    pub struct StderrSilenceGuard {
        saved_std_handle: isize,
        null_handle: isize,
        saved_fd: i32,
        null_fd: i32,
    }

    impl StderrSilenceGuard {
        pub fn new() -> Self {
            #[link(name = "kernel32")]
            extern "system" {
                fn GetStdHandle(nStdHandle: u32) -> isize;
                fn SetStdHandle(nStdHandle: u32, hHandle: isize) -> i32;
                fn CreateFileW(
                    lpFileName: *const u16,
                    dwDesiredAccess: u32,
                    dwShareMode: u32,
                    lpSecurityAttributes: *const std::ffi::c_void,
                    dwCreationDisposition: u32,
                    dwFlagsAndAttributes: u32,
                    hTemplateFile: isize,
                ) -> isize;
            }

            extern "C" {
                fn _open(filename: *const u8, oflag: i32, ...) -> i32;
                fn _dup(fd: i32) -> i32;
                fn _dup2(fd1: i32, fd2: i32) -> i32;
            }

            const STD_ERROR_HANDLE: u32 = 0xFFFF_FFF4;
            const GENERIC_WRITE: u32 = 0x4000_0000;
            const FILE_SHARE_READ: u32 = 1;
            const FILE_SHARE_WRITE: u32 = 2;
            const OPEN_EXISTING: u32 = 3;
            const INVALID_HANDLE: isize = -1;

            let (saved_std_handle, null_handle) = unsafe {
                let saved = GetStdHandle(STD_ERROR_HANDLE);
                let wide_nul: [u16; 4] = [b'N' as u16, b'U' as u16, b'L' as u16, 0];
                let null = CreateFileW(
                    wide_nul.as_ptr(),
                    GENERIC_WRITE,
                    FILE_SHARE_READ | FILE_SHARE_WRITE,
                    std::ptr::null(),
                    OPEN_EXISTING,
                    0,
                    0,
                );
                if null != INVALID_HANDLE && null != 0 {
                    let _ = SetStdHandle(STD_ERROR_HANDLE, null);
                }
                (saved, null)
            };

            let (saved_fd, null_fd) = unsafe {
                let saved = _dup(2);
                let null = _open(c"NUL".as_ptr() as *const u8, 0x0001, 0);
                if saved >= 0 && null >= 0 {
                    let _ = _dup2(null, 2);
                }
                (saved, null)
            };

            Self {
                saved_std_handle,
                null_handle,
                saved_fd,
                null_fd,
            }
        }
    }

    impl Default for StderrSilenceGuard {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Drop for StderrSilenceGuard {
        fn drop(&mut self) {
            #[link(name = "kernel32")]
            extern "system" {
                fn SetStdHandle(nStdHandle: u32, hHandle: isize) -> i32;
                fn CloseHandle(hObject: isize) -> i32;
            }

            extern "C" {
                fn _dup2(fd1: i32, fd2: i32) -> i32;
                fn _close(fd: i32) -> i32;
            }

            const STD_ERROR_HANDLE: u32 = 0xFFFF_FFF4;
            const INVALID_HANDLE: isize = -1;

            unsafe {
                if self.saved_fd >= 0 {
                    let _ = _dup2(self.saved_fd, 2);
                    let _ = _close(self.saved_fd);
                }
                if self.null_fd >= 0 {
                    let _ = _close(self.null_fd);
                }
                if self.saved_std_handle != INVALID_HANDLE && self.saved_std_handle != 0 {
                    let _ = SetStdHandle(STD_ERROR_HANDLE, self.saved_std_handle);
                }
                if self.null_handle != INVALID_HANDLE && self.null_handle != 0 {
                    let _ = CloseHandle(self.null_handle);
                }
            }
        }
    }

    /// Extract an egui ColorImage from a Win32 HBITMAP.
    pub fn extract_hbitmap_image(hbmp: HBITMAP) -> Option<ColorImage> {
        if hbmp.is_invalid() || (hbmp.0 as usize) <= 10 {
            return None;
        }

        unsafe {
            let mut bmp: BITMAP = std::mem::zeroed();
            if GetObjectW(
                hbmp.into(),
                std::mem::size_of::<BITMAP>() as i32,
                Some(&mut bmp as *mut _ as *mut _),
            ) == 0
            {
                return None;
            }

            let width = bmp.bmWidth as usize;
            let height = bmp.bmHeight as usize;
            if width == 0 || height == 0 || width > 256 || height > 256 {
                return None;
            }

            let mut bi: BITMAPINFO = std::mem::zeroed();
            bi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            bi.bmiHeader.biWidth = width as i32;
            bi.bmiHeader.biHeight = -(height as i32); // Top-down DIB
            bi.bmiHeader.biPlanes = 1;
            bi.bmiHeader.biBitCount = 32;
            bi.bmiHeader.biCompression = BI_RGB.0;

            let hdc = CreateCompatibleDC(None);
            let mut bgra_buf: Vec<u8> = vec![0u8; width * height * 4];

            let lines = GetDIBits(
                hdc,
                hbmp,
                0,
                height as u32,
                Some(bgra_buf.as_mut_ptr() as *mut _),
                &mut bi,
                DIB_RGB_COLORS,
            );
            let _ = DeleteDC(hdc);

            if lines == 0 {
                return None;
            }

            // Detect if alpha channel contains valid non-zero data
            let has_alpha = bgra_buf.chunks_exact(4).any(|p| p[3] != 0);

            let mut rgba_buf = Vec::with_capacity(width * height * 4);
            for chunk in bgra_buf.chunks_exact(4) {
                let b = chunk[0];
                let g = chunk[1];
                let r = chunk[2];
                let a = if has_alpha { chunk[3] } else { 255 };
                rgba_buf.push(r);
                rgba_buf.push(g);
                rgba_buf.push(b);
                rgba_buf.push(a);
            }

            Some(ColorImage::from_rgba_unmultiplied(
                [width, height],
                &rgba_buf,
            ))
        }
    }

    /// Read verb string from IContextMenu via GetCommandString.
    pub fn query_command_verb(cm: &IContextMenu, offset: usize) -> String {
        unsafe {
            // Try Unicode GCS_VERBW first
            let mut buf_w = [0u16; 64];
            let hr_w = cm.GetCommandString(
                offset,
                GCS_VERBW,
                None,
                PSTR(buf_w.as_mut_ptr() as *mut u8),
                buf_w.len() as u32,
            );
            if hr_w.is_ok() {
                let len = buf_w.iter().position(|&c| c == 0).unwrap_or(buf_w.len());
                let s = String::from_utf16_lossy(&buf_w[..len]);
                if !s.is_empty() {
                    return s.to_lowercase();
                }
            }

            // Fallback to ANSI GCS_VERBA
            let mut buf_a = [0u8; 64];
            let hr_a = cm.GetCommandString(
                offset,
                GCS_VERBA,
                None,
                PSTR(buf_a.as_mut_ptr()),
                buf_a.len() as u32,
            );
            if hr_a.is_ok() {
                let len = buf_a.iter().position(|&b| b == 0).unwrap_or(buf_a.len());
                let s = String::from_utf8_lossy(&buf_a[..len]);
                return s.to_lowercase();
            }
        }
        String::new()
    }

    /// Recursively traverse an HMENU, handling lazy submenus via IContextMenu2/3.
    pub fn walk_hmenu(
        hmenu: HMENU,
        cm: &IContextMenu,
        cm2: Option<&IContextMenu2>,
        cm3: Option<&IContextMenu3>,
    ) -> Vec<ShellMenuItem> {
        let count = unsafe { GetMenuItemCount(Some(hmenu)) };
        if count <= 0 {
            return Vec::new();
        }

        let mut items = Vec::new();
        for i in 0..(count as u32) {
            let mut mii: MENUITEMINFOW = unsafe { std::mem::zeroed() };
            mii.cbSize = std::mem::size_of::<MENUITEMINFOW>() as u32;
            mii.fMask = MIIM_ID
                | MIIM_STRING
                | MIIM_SUBMENU
                | MIIM_FTYPE
                | MIIM_STATE
                | MIIM_BITMAP
                | MIIM_DATA;

            // Query string buffer length
            let ok = unsafe { GetMenuItemInfoW(hmenu, i, true, &mut mii) };
            if ok.is_err() {
                continue;
            }

            let is_sep = (mii.fType & MFT_SEPARATOR).0 != 0;
            let mut text = String::new();

            if !is_sep && mii.cch > 0 {
                let mut buf = vec![0u16; (mii.cch + 2) as usize];
                mii.dwTypeData = PWSTR(buf.as_mut_ptr());
                mii.cch = buf.len() as u32;
                if unsafe { GetMenuItemInfoW(hmenu, i, true, &mut mii) }.is_ok() {
                    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
                    text = String::from_utf16_lossy(&buf[..len]);
                }
            }

            let id = mii.wID;
            let is_disabled = (mii.fState & MFS_DISABLED).0 != 0;
            let is_checked = (mii.fState & MFS_CHECKED).0 != 0;
            let icon = extract_hbitmap_image(mii.hbmpItem);

            let verb = if !is_sep && (ID_CMD_FIRST..=ID_CMD_LAST).contains(&id) {
                query_command_verb(cm, (id - ID_CMD_FIRST) as usize)
            } else {
                String::new()
            };

            // Process submenus
            let mut children = Vec::new();
            if !mii.hSubMenu.is_invalid() {
                // Trigger lazy menu population for submenus (Send to, Open with, etc.)
                let wparam = WPARAM(mii.hSubMenu.0 as usize);
                let lparam = LPARAM(((i as usize) & 0xFFFF) as isize);
                unsafe {
                    if let Some(c3) = cm3 {
                        let mut lres = LRESULT(0);
                        let _ =
                            c3.HandleMenuMsg2(WM_INITMENUPOPUP, wparam, lparam, Some(&mut lres));
                    } else if let Some(c2) = cm2 {
                        let _ = c2.HandleMenuMsg(WM_INITMENUPOPUP, wparam, lparam);
                    }
                }

                children = walk_hmenu(mii.hSubMenu, cm, cm2, cm3);
            }

            items.push(ShellMenuItem {
                id,
                verb,
                title: text,
                is_separator: is_sep,
                is_disabled,
                is_checked,
                icon,
                children,
            });
        }

        items
    }

    /// Obtain IContextMenu for either background folder or selected paths.
    pub fn get_context_menu_com(
        paths: &[PathBuf],
        is_background: bool,
    ) -> Result<IContextMenu, String> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

            if paths.is_empty() {
                return Err("No paths provided for shell context menu".to_string());
            }

            if is_background {
                let dir_path = &paths[0];
                let wide: Vec<u16> = dir_path
                    .as_os_str()
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect();

                let mut pidl_folder: *mut ITEMIDLIST = std::ptr::null_mut();
                SHParseDisplayName(PCWSTR(wide.as_ptr()), None, &mut pidl_folder, 0, None)
                    .map_err(|e| format!("SHParseDisplayName failed: {e}"))?;

                let desktop: IShellFolder =
                    SHGetDesktopFolder().map_err(|e| format!("SHGetDesktopFolder failed: {e}"))?;

                let folder: IShellFolder = desktop
                    .BindToObject(pidl_folder, None)
                    .map_err(|e| format!("desktop.BindToObject failed: {e}"))?;

                ILFree(Some(pidl_folder));

                let cm: IContextMenu = folder
                    .CreateViewObject(HWND(std::ptr::null_mut()))
                    .map_err(|e| format!("CreateViewObject failed: {e}"))?;

                Ok(cm)
            } else {
                let mut parent_folder: Option<IShellFolder> = None;
                let mut child_pidls: Vec<*const ITEMIDLIST> = Vec::new();
                let mut full_pidls: Vec<*mut ITEMIDLIST> = Vec::new();

                for path in paths {
                    let wide: Vec<u16> = path
                        .as_os_str()
                        .encode_wide()
                        .chain(std::iter::once(0))
                        .collect();
                    let mut full_pidl: *mut ITEMIDLIST = std::ptr::null_mut();

                    if SHParseDisplayName(PCWSTR(wide.as_ptr()), None, &mut full_pidl, 0, None)
                        .is_ok()
                    {
                        if parent_folder.is_none() {
                            let mut child_pidl: *mut ITEMIDLIST = std::ptr::null_mut();
                            if let Ok(pf) =
                                SHBindToParent::<IShellFolder>(full_pidl, Some(&mut child_pidl))
                            {
                                parent_folder = Some(pf);
                                child_pidls.push(child_pidl);
                            }
                        } else {
                            let last = ILFindLastID(full_pidl);
                            child_pidls.push(last);
                        }
                        full_pidls.push(full_pidl);
                    }
                }

                let cm_res = if let Some(pf) = parent_folder {
                    if !child_pidls.is_empty() {
                        pf.GetUIObjectOf::<IContextMenu>(
                            HWND(std::ptr::null_mut()),
                            &child_pidls,
                            None,
                        )
                        .map_err(|e| format!("GetUIObjectOf failed: {e}"))
                    } else {
                        Err("Empty child pidl list".to_string())
                    }
                } else {
                    Err("Failed to resolve parent folder".to_string())
                };

                for pidl in full_pidls {
                    ILFree(Some(pidl));
                }

                cm_res
            }
        }
    }

    /// Synchronously build shell menu items (for testing or direct evaluation).
    pub fn build_shell_menu_sync(
        paths: &[PathBuf],
        is_background: bool,
        shift_held: bool,
    ) -> Result<Vec<ShellMenuItem>, String> {
        let _silence = StderrSilenceGuard::new();
        unsafe {
            let cm = get_context_menu_com(paths, is_background)?;
            let cm2 = cm.cast::<IContextMenu2>().ok();
            let cm3 = cm.cast::<IContextMenu3>().ok();

            let hmenu = CreatePopupMenu().map_err(|e| format!("CreatePopupMenu failed: {e}"))?;

            let mut flags = CMF_NORMAL;
            if shift_held {
                flags |= CMF_EXTENDEDVERBS;
            }

            let hr = cm.QueryContextMenu(hmenu, 0, ID_CMD_FIRST, ID_CMD_LAST, flags);

            if hr.0 < 0 {
                let _ = DestroyMenu(hmenu);
                return Err(format!(
                    "QueryContextMenu failed with HRESULT 0x{:08X}",
                    hr.0
                ));
            }

            let items = walk_hmenu(hmenu, &cm, cm2.as_ref(), cm3.as_ref());
            let _ = DestroyMenu(hmenu);
            Ok(items)
        }
    }
}

/// Active shell context menu manager holding the STA thread and lifetime of the active menu.
pub struct ShellMenuManager {
    req_tx: Sender<ShellMenuCommand>,
    active_menu: Mutex<Option<Arc<Vec<ShellMenuItem>>>>,
    loading: Mutex<bool>,
    current_session: AtomicU64,
}

enum ShellMenuCommand {
    Build {
        req: ShellMenuBuildRequest,
        resp_tx: Sender<(u64, Result<Vec<ShellMenuItem>, String>)>,
    },
    Invoke {
        session_id: u64,
        id: u32,
        pt: (i32, i32),
    },
}

static MANAGER: LazyLock<ShellMenuManager> = LazyLock::new(|| {
    let (req_tx, req_rx) = channel::<ShellMenuCommand>();

    // Dedicated background thread running Win32 STA COM apartment
    std::thread::Builder::new()
        .name("mossambi_shell_context_menu".to_string())
        .spawn(move || {
            #[cfg(target_os = "windows")]
            unsafe {
                use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
                let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            }

            #[cfg(target_os = "windows")]
            struct ActiveSession {
                session_id: u64,
                context_menu: win32_impl::IContextMenu,
                hwnd: windows::Win32::Foundation::HWND,
            }

            #[cfg(target_os = "windows")]
            let mut active_session: Option<ActiveSession> = None;

            while let Ok(cmd) = req_rx.recv() {
                match cmd {
                    ShellMenuCommand::Build { req, resp_tx } => {
                        #[cfg(target_os = "windows")]
                        {
                            let _silence = win32_impl::StderrSilenceGuard::new();
                            unsafe {
                                use windows::Win32::Foundation::HWND;
                                use windows::Win32::UI::Shell::{CMF_EXTENDEDVERBS, CMF_NORMAL};
                                use windows::Win32::UI::WindowsAndMessaging::{
                                    CreatePopupMenu, DestroyMenu,
                                };
                                use windows_core::Interface;

                                match win32_impl::get_context_menu_com(
                                    &req.paths,
                                    req.is_background,
                                ) {
                                    Ok(cm) => {
                                        let cm2 = cm.cast::<win32_impl::IContextMenu2>().ok();
                                        let cm3 = cm.cast::<win32_impl::IContextMenu3>().ok();

                                        if let Ok(hmenu) = CreatePopupMenu() {
                                            let mut flags = CMF_NORMAL;
                                            if req.shift_held {
                                                flags |= CMF_EXTENDEDVERBS;
                                            }

                                            let hr = cm.QueryContextMenu(
                                                hmenu,
                                                0,
                                                win32_impl::ID_CMD_FIRST,
                                                win32_impl::ID_CMD_LAST,
                                                flags,
                                            );

                                            if hr.0 >= 0 {
                                                let items = win32_impl::walk_hmenu(
                                                    hmenu,
                                                    &cm,
                                                    cm2.as_ref(),
                                                    cm3.as_ref(),
                                                );
                                                let _ = DestroyMenu(hmenu);

                                                active_session = Some(ActiveSession {
                                                    session_id: req.session_id,
                                                    context_menu: cm,
                                                    hwnd: HWND(req.hwnd as *mut _),
                                                });

                                                let _ = resp_tx.send((req.session_id, Ok(items)));
                                                continue;
                                            }
                                            let _ = DestroyMenu(hmenu);
                                        }
                                        let _ = resp_tx.send((
                                            req.session_id,
                                            Err("QueryContextMenu or CreatePopupMenu failed"
                                                .to_string()),
                                        ));
                                    }
                                    Err(e) => {
                                        let _ = resp_tx.send((req.session_id, Err(e)));
                                    }
                                }
                            }
                        }
                        #[cfg(not(target_os = "windows"))]
                        {
                            let _ = resp_tx.send((req.session_id, Ok(Vec::new())));
                        }
                    }
                    ShellMenuCommand::Invoke { session_id, id, pt } => {
                        #[cfg(target_os = "windows")]
                        {
                            let _silence = win32_impl::StderrSilenceGuard::new();
                            if let Some(session) = &active_session {
                                if session.session_id == session_id {
                                    unsafe {
                                        use windows::core::{PCSTR, PCWSTR};
                                        use windows::Win32::Foundation::POINT;
                                        use windows::Win32::UI::Shell::{
                                            CMIC_MASK_PTINVOKE, CMINVOKECOMMANDINFO,
                                            CMINVOKECOMMANDINFOEX,
                                        };

                                        let offset = id.saturating_sub(win32_impl::ID_CMD_FIRST);
                                        let ici = CMINVOKECOMMANDINFOEX {
                                            cbSize: std::mem::size_of::<CMINVOKECOMMANDINFOEX>()
                                                as u32,
                                            fMask: win32_impl::CMIC_MASK_UNICODE
                                                | CMIC_MASK_PTINVOKE,
                                            hwnd: session.hwnd,
                                            lpVerb: PCSTR(offset as *const u8),
                                            lpVerbW: PCWSTR(offset as *const u16),
                                            nShow: 1, // SW_SHOWNORMAL
                                            ptInvoke: POINT { x: pt.0, y: pt.1 },
                                            ..Default::default()
                                        };

                                        let pici = &ici as *const _ as *const CMINVOKECOMMANDINFO;
                                        if session.context_menu.InvokeCommand(pici).is_err() {
                                            // Fallback to ANSI / basic struct
                                            let basic = CMINVOKECOMMANDINFO {
                                                cbSize: std::mem::size_of::<CMINVOKECOMMANDINFO>()
                                                    as u32,
                                                hwnd: session.hwnd,
                                                lpVerb: PCSTR(offset as *const u8),
                                                nShow: 1,
                                                ..Default::default()
                                            };
                                            let _ = session.context_menu.InvokeCommand(&basic);
                                        }
                                    }
                                }
                            }
                        }
                        #[cfg(not(target_os = "windows"))]
                        {
                            let _ = (session_id, id, pt);
                        }
                    }
                }
            }
        })
        .expect("Failed to spawn shell context menu worker thread");

    // Spawn registry/association change watcher thread for instant freshness
    #[cfg(target_os = "windows")]
    {
        std::thread::Builder::new()
            .name("mossambi_assoc_watcher".to_string())
            .spawn(|| unsafe {
                use windows_sys::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
                use windows_sys::Win32::System::Registry::{
                    RegCloseKey, RegNotifyChangeKeyValue, RegOpenKeyExW, HKEY, KEY_NOTIFY,
                    REG_NOTIFY_CHANGE_LAST_SET, REG_NOTIFY_CHANGE_NAME,
                };
                use windows_sys::Win32::System::Threading::{
                    CreateEventW, WaitForSingleObject, INFINITE,
                };

                const HKEY_CLASSES_ROOT: isize = 0x8000_0000_isize;
                let event = CreateEventW(std::ptr::null(), 0, 0, std::ptr::null());
                if !event.is_null() {
                    let mut hk: HKEY = std::ptr::null_mut();
                    if RegOpenKeyExW(
                        HKEY_CLASSES_ROOT as _,
                        std::ptr::null(),
                        0,
                        KEY_NOTIFY,
                        &mut hk,
                    ) == 0
                    {
                        loop {
                            let res = RegNotifyChangeKeyValue(
                                hk,
                                1, // Watch subtree
                                REG_NOTIFY_CHANGE_NAME | REG_NOTIFY_CHANGE_LAST_SET,
                                event,
                                1, // Async
                            );
                            if res != 0 {
                                break;
                            }
                            if WaitForSingleObject(event, INFINITE) == WAIT_OBJECT_0 {
                                // Associations or shell extensions modified! Clear caches instantly
                                crate::core::shell_icons::clear_shell_icon_cache();
                            }
                        }
                        RegCloseKey(hk);
                    }
                    CloseHandle(event);
                }
            })
            .ok();
    }

    ShellMenuManager {
        req_tx,
        active_menu: Mutex::new(None),
        loading: Mutex::new(false),
        current_session: AtomicU64::new(1),
    }
});

/// Public API to request shell menu build asynchronously.
pub fn request_shell_menu(
    paths: Vec<PathBuf>,
    is_background: bool,
    shift_held: bool,
    hwnd: usize,
) -> u64 {
    let session_id = MANAGER.current_session.fetch_add(1, Ordering::SeqCst) + 1;
    if let Ok(mut loading) = MANAGER.loading.lock() {
        *loading = true;
    }
    if let Ok(mut active) = MANAGER.active_menu.lock() {
        *active = None;
    }

    let (resp_tx, resp_rx) = channel();
    let req = ShellMenuBuildRequest {
        session_id,
        paths,
        is_background,
        shift_held,
        hwnd,
    };

    let _ = MANAGER
        .req_tx
        .send(ShellMenuCommand::Build { req, resp_tx });

    // Spawn a lightweight waiter that updates active_menu when done
    std::thread::spawn(move || {
        if let Ok((resp_session, result)) = resp_rx.recv() {
            if resp_session == MANAGER.current_session.load(Ordering::SeqCst) {
                if let Ok(mut active) = MANAGER.active_menu.lock() {
                    if let Ok(items) = result {
                        *active = Some(Arc::new(items));
                    }
                }
                if let Ok(mut loading) = MANAGER.loading.lock() {
                    *loading = false;
                }
            }
        }
    });

    session_id
}

/// Retrieve the currently loaded shell menu items (or None if loading/failed).
pub fn get_active_shell_menu() -> Option<Arc<Vec<ShellMenuItem>>> {
    if let Ok(guard) = MANAGER.active_menu.lock() {
        guard.clone()
    } else {
        None
    }
}

/// Whether the shell menu is currently being built in the background.
pub fn is_shell_menu_loading() -> bool {
    if let Ok(guard) = MANAGER.loading.lock() {
        *guard
    } else {
        false
    }
}

/// Invoke a shell command by id.
pub fn invoke_shell_command(session_id: u64, id: u32, pt: (i32, i32)) {
    let _ = MANAGER
        .req_tx
        .send(ShellMenuCommand::Invoke { session_id, id, pt });
}

/// Synchronous helper for testing or direct evaluation.
pub fn build_shell_menu_sync(
    paths: &[PathBuf],
    is_background: bool,
    shift_held: bool,
) -> Result<Vec<ShellMenuItem>, String> {
    #[cfg(target_os = "windows")]
    {
        win32_impl::build_shell_menu_sync(paths, is_background, shift_held)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (paths, is_background, shift_held);
        Ok(Vec::new())
    }
}

/// Ensures a shell menu request has been dispatched for the active menu opening.
pub fn ensure_shell_menu_requested(
    ctx: &eframe::egui::Context,
    paths: &[PathBuf],
    is_background: bool,
    shift_held: bool,
    hwnd: usize,
) -> u64 {
    #[cfg(target_os = "windows")]
    {
        ctx.data_mut(|d| {
            let entry = d.get_temp_mut_or_insert_with(
                eframe::egui::Id::new("mossambi_active_shell_menu_req"),
                || (Vec::<PathBuf>::new(), false, false, 0u64),
            );
            if entry.0 != paths || entry.1 != is_background || entry.2 != shift_held {
                let sid = request_shell_menu(paths.to_vec(), is_background, shift_held, hwnd);
                *entry = (paths.to_vec(), is_background, shift_held, sid);
            }
            entry.3
        })
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (ctx, paths, is_background, shift_held, hwnd);
        0
    }
}

/// Renders the "Windows / More options" section with real shell extension items.
pub fn render_shell_menu_section(
    ui: &mut eframe::egui::Ui,
    session_id: u64,
    out_cmd: &mut Option<(u64, u32)>,
) {
    #[cfg(target_os = "windows")]
    {
        ui.separator();
        ui.menu_button("Windows / More options", |sub_ui| {
            sub_ui.set_min_width(220.0);
            if is_shell_menu_loading() {
                sub_ui.horizontal(|h_ui| {
                    h_ui.spinner();
                    h_ui.label("Loading Windows shell options...");
                });
            } else if let Some(items) = get_active_shell_menu() {
                let mut any = false;
                for item in items.iter() {
                    if item.is_separator {
                        sub_ui.separator();
                        continue;
                    }
                    if is_duplicate_verb(&item.verb, &item.title) {
                        continue;
                    }
                    let cleaned = clean_menu_title(&item.title);
                    if cleaned.is_empty() {
                        continue;
                    }
                    any = true;
                    if !item.children.is_empty() {
                        sub_ui.menu_button(&cleaned, |child_ui| {
                            child_ui.set_min_width(200.0);
                            render_shell_menu_children(
                                child_ui,
                                &item.children,
                                session_id,
                                out_cmd,
                            );
                        });
                    } else {
                        let resp = sub_ui
                            .add_enabled(!item.is_disabled, eframe::egui::Button::new(&cleaned));
                        if resp.clicked() {
                            *out_cmd = Some((session_id, item.id));
                            sub_ui.close_menu();
                        }
                    }
                }
                if !any {
                    sub_ui.label("No additional shell options");
                }
            } else {
                sub_ui.label("No shell options available");
            }
        });
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (ui, session_id, out_cmd);
    }
}

#[cfg(target_os = "windows")]
fn render_shell_menu_children(
    ui: &mut eframe::egui::Ui,
    children: &[ShellMenuItem],
    session_id: u64,
    out_cmd: &mut Option<(u64, u32)>,
) {
    for child in children {
        if child.is_separator {
            ui.separator();
            continue;
        }
        let cleaned = clean_menu_title(&child.title);
        if cleaned.is_empty() {
            continue;
        }
        if !child.children.is_empty() {
            ui.menu_button(&cleaned, |sub_ui| {
                sub_ui.set_min_width(200.0);
                render_shell_menu_children(sub_ui, &child.children, session_id, out_cmd);
            });
        } else {
            let resp = ui.add_enabled(!child.is_disabled, eframe::egui::Button::new(&cleaned));
            if resp.clicked() {
                *out_cmd = Some((session_id, child.id));
                ui.close_menu();
            }
        }
    }
}
