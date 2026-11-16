//! Dedicated Background STA OLE Drop Target Receiver.
//!
//! Provides an asynchronous, non-blocking IDropTarget implementation for desktop
//! overlays, explorer views, and resident drop pucks.
//! Pure 7-bit ASCII.

use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::Mutex;

/// Event emitted when files are dropped onto an OLE target window.
#[derive(Debug, Clone)]
pub struct OleDropEvent {
    pub paths: Vec<PathBuf>,
    pub screen_pos: (i32, i32),
    pub effect: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DragHoverState {
    pub screen_pos: (i32, i32),
    pub is_active: bool,
}

static EXTERNAL_DROPS: Mutex<Vec<OleDropEvent>> = Mutex::new(Vec::new());
static DRAG_HOVER: Mutex<DragHoverState> = Mutex::new(DragHoverState {
    screen_pos: (0, 0),
    is_active: false,
});

/// Polls and drains any pending external drops received via OLE IDropTarget.
pub fn poll_external_drops() -> Vec<OleDropEvent> {
    if let Ok(mut lock) = EXTERNAL_DROPS.lock() {
        std::mem::take(&mut *lock)
    } else {
        Vec::new()
    }
}

/// Returns the current external drag hover status and screen position.
pub fn current_drag_hover() -> DragHoverState {
    if let Ok(lock) = DRAG_HOVER.lock() {
        *lock
    } else {
        DragHoverState {
            screen_pos: (0, 0),
            is_active: false,
        }
    }
}

pub fn update_drag_hover(screen_pos: (i32, i32), is_active: bool) {
    if let Ok(mut lock) = DRAG_HOVER.lock() {
        *lock = DragHoverState {
            screen_pos,
            is_active,
        };
    }
}

#[cfg(windows)]
mod imp {
    use super::*;
    use windows::core::{implement, w};
    use windows::Win32::Foundation::{HGLOBAL, HWND, POINT, POINTL};
    use windows::Win32::System::Com::{
        CoCreateInstance, IDataObject, CLSCTX_INPROC_SERVER, FORMATETC, TYMED_HGLOBAL,
    };
    use windows::Win32::System::DataExchange::RegisterClipboardFormatW;
    use windows::Win32::System::Memory::{GlobalLock, GlobalUnlock};
    use windows::Win32::System::Ole::{
        IDropTarget, IDropTarget_Impl, RegisterDragDrop, ReleaseStgMedium, RevokeDragDrop,
        CF_HDROP, DROPEFFECT, DROPEFFECT_COPY, DROPEFFECT_LINK, DROPEFFECT_MOVE, DROPEFFECT_NONE,
    };
    use windows::Win32::System::SystemServices::{MK_CONTROL, MK_SHIFT};
    use windows::Win32::UI::Shell::{IDropTargetHelper, DROPFILES};

    const CLSID_DRAG_DROP_HELPER: windows::core::GUID =
        windows::core::GUID::from_u128(0x4657278a_411b_11d2_839a_00c04fd918d0);
    const MK_ALT_FLAG: u32 = 0x0020;

    #[implement(IDropTarget)]
    pub struct DesktopDropTarget {
        hwnd: HWND,
        helper: Option<IDropTargetHelper>,
        tx: Option<Sender<OleDropEvent>>,
    }

    impl DesktopDropTarget {
        pub fn new(hwnd: HWND, tx: Option<Sender<OleDropEvent>>) -> Self {
            let helper: Option<IDropTargetHelper> = unsafe {
                CoCreateInstance(&CLSID_DRAG_DROP_HELPER, None, CLSCTX_INPROC_SERVER).ok()
            };
            Self { hwnd, helper, tx }
        }
    }

    fn calculate_effect(
        keys: windows::Win32::System::SystemServices::MODIFIERKEYS_FLAGS,
    ) -> DROPEFFECT {
        let ctrl = (keys.0 & MK_CONTROL.0) != 0;
        let shift = (keys.0 & MK_SHIFT.0) != 0;
        let alt = (keys.0 & MK_ALT_FLAG) != 0;

        if (ctrl && shift) || alt {
            DROPEFFECT_LINK
        } else if shift {
            DROPEFFECT_MOVE
        } else if ctrl {
            DROPEFFECT_COPY
        } else {
            DROPEFFECT_COPY | DROPEFFECT_MOVE
        }
    }

    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    impl IDropTarget_Impl for DesktopDropTarget_Impl {
        fn DragEnter(
            &self,
            pdataobject: windows_core::Ref<'_, IDataObject>,
            grfkeystate: windows::Win32::System::SystemServices::MODIFIERKEYS_FLAGS,
            pt: &POINTL,
            pdweffect: *mut DROPEFFECT,
        ) -> windows::core::Result<()> {
            update_drag_hover((pt.x, pt.y), true);
            let effect = calculate_effect(grfkeystate);
            unsafe {
                if !pdweffect.is_null() {
                    *pdweffect = effect;
                }
            }
            if let (Some(helper), Some(data_obj)) = (&self.helper, pdataobject.as_ref()) {
                let pt_win = POINT { x: pt.x, y: pt.y };
                unsafe {
                    let _ = helper.DragEnter(self.hwnd, data_obj, &pt_win, effect);
                }
            }
            Ok(())
        }

        fn DragOver(
            &self,
            grfkeystate: windows::Win32::System::SystemServices::MODIFIERKEYS_FLAGS,
            pt: &POINTL,
            pdweffect: *mut DROPEFFECT,
        ) -> windows::core::Result<()> {
            update_drag_hover((pt.x, pt.y), true);
            let effect = calculate_effect(grfkeystate);
            unsafe {
                if !pdweffect.is_null() {
                    *pdweffect = effect;
                }
            }
            if let Some(helper) = &self.helper {
                let pt_win = POINT { x: pt.x, y: pt.y };
                unsafe {
                    let _ = helper.DragOver(&pt_win, effect);
                }
            }
            Ok(())
        }

        fn DragLeave(&self) -> windows::core::Result<()> {
            update_drag_hover((0, 0), false);
            if let Some(helper) = &self.helper {
                unsafe {
                    let _ = helper.DragLeave();
                }
            }
            Ok(())
        }

        fn Drop(
            &self,
            pdataobject: windows_core::Ref<'_, IDataObject>,
            grfkeystate: windows::Win32::System::SystemServices::MODIFIERKEYS_FLAGS,
            pt: &POINTL,
            pdweffect: *mut DROPEFFECT,
        ) -> windows::core::Result<()> {
            update_drag_hover((0, 0), false);
            let effect = calculate_effect(grfkeystate);
            if let (Some(helper), Some(data_obj)) = (&self.helper, pdataobject.as_ref()) {
                let pt_win = POINT { x: pt.x, y: pt.y };
                unsafe {
                    let _ = helper.Drop(data_obj, &pt_win, effect);
                }
            }

            let data_obj = match pdataobject.as_ref() {
                Some(obj) => obj,
                None => return Ok(()),
            };

            let mut paths = Vec::new();

            // 1. Try CF_HDROP
            let format_hdrop = FORMATETC {
                cfFormat: CF_HDROP.0,
                ptd: std::ptr::null_mut(),
                dwAspect: 1, // DVASPECT_CONTENT
                lindex: -1,
                tymed: TYMED_HGLOBAL.0 as u32,
            };

            unsafe {
                if let Ok(mut medium) = data_obj.GetData(&format_hdrop) {
                    paths = extract_paths_from_hdrop(medium.u.hGlobal);
                    ReleaseStgMedium(&mut medium);
                }
            }

            // 2. Try Virtual Files: FileGroupDescriptorW + FileContents
            if paths.is_empty() {
                let cf_fgd = unsafe { RegisterClipboardFormatW(w!("FileGroupDescriptorW")) };
                if cf_fgd != 0 {
                    let fmt_fgd = FORMATETC {
                        cfFormat: cf_fgd as u16,
                        ptd: std::ptr::null_mut(),
                        dwAspect: 1,
                        lindex: -1,
                        tymed: TYMED_HGLOBAL.0 as u32,
                    };
                    unsafe {
                        if let Ok(mut medium) = data_obj.GetData(&fmt_fgd) {
                            let virt = extract_virtual_file_names(medium.u.hGlobal);
                            ReleaseStgMedium(&mut medium);
                            if !virt.is_empty() {
                                let cf_fc = RegisterClipboardFormatW(w!("FileContents"));
                                paths = extract_virtual_contents(data_obj, cf_fc as u16, &virt);
                            }
                        }
                    }
                }
            }

            // 3. Browser Fallback: CF_UNICODETEXT (13) for text/uri-list, web URLs, and data URLs
            if paths.is_empty() {
                let format_text = FORMATETC {
                    cfFormat: 13, // CF_UNICODETEXT
                    ptd: std::ptr::null_mut(),
                    dwAspect: 1,
                    lindex: -1,
                    tymed: TYMED_HGLOBAL.0 as u32,
                };
                unsafe {
                    if let Ok(mut medium) = data_obj.GetData(&format_text) {
                        let text = extract_string_from_hglobal(medium.u.hGlobal);
                        ReleaseStgMedium(&mut medium);
                        let uris = crate::core::web_drop::parse_uri_list(&text);
                        for u in uris {
                            if let Ok(staged) = crate::core::web_drop::stage_url_or_data(&u) {
                                paths.push(staged);
                            }
                        }
                        if paths.is_empty() {
                            if let Some(src) = crate::core::web_drop::parse_html_image_src(&text) {
                                if let Ok(staged) = crate::core::web_drop::stage_url_or_data(&src) {
                                    paths.push(staged);
                                }
                            }
                        }
                    }
                }
            }

            let final_effect = if !paths.is_empty() {
                let event = OleDropEvent {
                    paths,
                    screen_pos: (pt.x, pt.y),
                    effect: effect.0,
                };

                if let Some(ref tx) = self.tx {
                    let _ = tx.send(event.clone());
                }
                if let Ok(mut lock) = EXTERNAL_DROPS.lock() {
                    lock.push(event);
                }

                effect
            } else {
                DROPEFFECT_NONE
            };

            unsafe {
                if !pdweffect.is_null() {
                    *pdweffect = final_effect;
                }
            }

            Ok(())
        }
    }

    unsafe fn extract_virtual_file_names(hglobal: HGLOBAL) -> Vec<String> {
        let ptr = GlobalLock(hglobal);
        if ptr.is_null() {
            return Vec::new();
        }
        // FILEGROUPDESCRIPTORW structure:
        // cItems: u32
        // fgd: [FILEDESCRIPTORW; cItems]
        // FILEDESCRIPTORW has cFileName at offset 76 (260 WCHARs)
        let count = *(ptr as *const u32);
        let mut names = Vec::new();
        let descriptor_size = 592; // sizeof(FILEDESCRIPTORW) in 64-bit
        let name_offset = 76; // offset of cFileName in FILEDESCRIPTORW
        for i in 0..count.min(64) {
            let offset = 4 + (i as usize) * descriptor_size + name_offset;
            let name_ptr = (ptr as *const u8).add(offset) as *const u16;
            let mut len = 0;
            while *name_ptr.add(len) != 0 && len < 260 {
                len += 1;
            }
            if len > 0 {
                let slice = std::slice::from_raw_parts(name_ptr, len);
                names.push(String::from_utf16_lossy(slice));
            }
        }
        let _ = GlobalUnlock(hglobal);
        names
    }

    unsafe fn extract_virtual_contents(
        data_obj: &IDataObject,
        cf_fc: u16,
        names: &[String],
    ) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        let tmp_dir = std::env::temp_dir().join("MossambiVirtualDrops");
        let _ = std::fs::create_dir_all(&tmp_dir);

        for (idx, name) in names.iter().enumerate() {
            let fmt_fc = FORMATETC {
                cfFormat: cf_fc,
                ptd: std::ptr::null_mut(),
                dwAspect: 1,
                lindex: idx as i32,
                tymed: TYMED_HGLOBAL.0 as u32,
            };
            if let Ok(mut med) = data_obj.GetData(&fmt_fc) {
                let hg = med.u.hGlobal;
                let ptr = GlobalLock(hg);
                if !ptr.is_null() {
                    let size = windows::Win32::System::Memory::GlobalSize(hg);
                    if size > 0 {
                        let data = std::slice::from_raw_parts(ptr as *const u8, size);
                        let clean_name = std::path::Path::new(name)
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| format!("drop_item_{idx}.bin"));
                        let out_path = tmp_dir.join(clean_name);
                        if std::fs::write(&out_path, data).is_ok() {
                            paths.push(out_path);
                        }
                    }
                    let _ = GlobalUnlock(hg);
                }
                ReleaseStgMedium(&mut med);
            }
        }
        paths
    }

    unsafe fn extract_paths_from_hdrop(hglobal: HGLOBAL) -> Vec<PathBuf> {
        let ptr = GlobalLock(hglobal);
        if ptr.is_null() {
            return Vec::new();
        }
        let df = &*(ptr as *const DROPFILES);
        let mut paths = Vec::new();
        let wide_ptr = (ptr as *const u8).add(df.pFiles as usize) as *const u16;
        let mut offset = 0;
        loop {
            let current_str_ptr = wide_ptr.add(offset);
            if *current_str_ptr == 0 {
                break;
            }
            let mut len = 0;
            while *current_str_ptr.add(len) != 0 {
                len += 1;
            }
            let slice = std::slice::from_raw_parts(current_str_ptr, len);
            let s = String::from_utf16_lossy(slice);
            paths.push(PathBuf::from(s));
            offset += len + 1;
        }
        let _ = GlobalUnlock(hglobal);
        paths
    }

    unsafe fn extract_string_from_hglobal(hglobal: HGLOBAL) -> String {
        let ptr = GlobalLock(hglobal);
        if ptr.is_null() {
            return String::new();
        }
        let wide_ptr = ptr as *const u16;
        let mut len = 0;
        while *wide_ptr.add(len) != 0 {
            len += 1;
        }
        let slice = std::slice::from_raw_parts(wide_ptr, len);
        let s = String::from_utf16_lossy(slice);
        let _ = GlobalUnlock(hglobal);
        s
    }

    /// Register the OLE drop target for a native window handle.
    pub fn register_drop_target(hwnd_raw: usize, tx: Sender<OleDropEvent>) -> Result<(), String> {
        let target: IDropTarget = DesktopDropTarget::new(HWND(hwnd_raw as *mut _), Some(tx)).into();
        unsafe {
            RegisterDragDrop(HWND(hwnd_raw as *mut _), &target)
                .map_err(|e| format!("RegisterDragDrop failed: {e}"))
        }
    }

    /// Register the OLE drop target for the main window handle.
    pub fn register_main_drop_target(hwnd_raw: usize) -> Result<(), String> {
        let target: IDropTarget = DesktopDropTarget::new(HWND(hwnd_raw as *mut _), None).into();
        unsafe {
            RegisterDragDrop(HWND(hwnd_raw as *mut _), &target)
                .map_err(|e| format!("RegisterDragDrop failed: {e}"))
        }
    }

    /// Revoke the OLE drop target for a native window handle.
    pub fn revoke_drop_target(hwnd_raw: usize) -> Result<(), String> {
        unsafe {
            RevokeDragDrop(HWND(hwnd_raw as *mut _))
                .map_err(|e| format!("RevokeDragDrop failed: {e}"))
        }
    }
}

#[cfg(windows)]
pub use imp::*;

#[cfg(not(windows))]
pub fn register_drop_target(_hwnd_raw: usize, _tx: Sender<OleDropEvent>) -> Result<(), String> {
    Ok(())
}

#[cfg(not(windows))]
pub fn register_main_drop_target(_hwnd_raw: usize) -> Result<(), String> {
    Ok(())
}

#[cfg(not(windows))]
pub fn revoke_drop_target(_hwnd_raw: usize) -> Result<(), String> {
    Ok(())
}
