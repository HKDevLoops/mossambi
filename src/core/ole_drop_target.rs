//! Dedicated Background STA OLE Drop Target Receiver.
//!
//! Provides an asynchronous, non-blocking IDropTarget implementation for desktop
//! overlays and resident drop pucks. Runs on an STA COM worker thread.

use std::path::PathBuf;
use std::sync::mpsc::Sender;

/// Event emitted when files are dropped onto an OLE target window.
#[derive(Debug, Clone)]
pub struct OleDropEvent {
    pub paths: Vec<PathBuf>,
    pub screen_pos: (i32, i32),
}

#[cfg(windows)]
mod imp {
    use super::*;
    use windows::core::implement;
    use windows::Win32::Foundation::{HGLOBAL, HWND, POINTL};
    use windows::Win32::System::Com::{IDataObject, FORMATETC, TYMED_HGLOBAL};
    use windows::Win32::System::Memory::{GlobalLock, GlobalUnlock};
    use windows::Win32::System::Ole::{
        IDropTarget, IDropTarget_Impl, RegisterDragDrop, RevokeDragDrop, CF_HDROP, DROPEFFECT,
        DROPEFFECT_COPY, DROPEFFECT_NONE,
    };
    use windows::Win32::UI::Shell::DROPFILES;

    #[implement(IDropTarget)]
    pub struct DesktopDropTarget {
        tx: Sender<OleDropEvent>,
    }

    impl DesktopDropTarget {
        pub fn new(tx: Sender<OleDropEvent>) -> Self {
            Self { tx }
        }
    }

    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    impl IDropTarget_Impl for DesktopDropTarget_Impl {
        fn DragEnter(
            &self,
            _pdataobject: windows_core::Ref<'_, IDataObject>,
            _grfkeystate: windows::Win32::System::SystemServices::MODIFIERKEYS_FLAGS,
            _pt: &POINTL,
            pdweffect: *mut DROPEFFECT,
        ) -> windows::core::Result<()> {
            unsafe {
                if !pdweffect.is_null() {
                    *pdweffect = DROPEFFECT_COPY;
                }
            }
            Ok(())
        }

        fn DragOver(
            &self,
            _grfkeystate: windows::Win32::System::SystemServices::MODIFIERKEYS_FLAGS,
            _pt: &POINTL,
            pdweffect: *mut DROPEFFECT,
        ) -> windows::core::Result<()> {
            unsafe {
                if !pdweffect.is_null() {
                    *pdweffect = DROPEFFECT_COPY;
                }
            }
            Ok(())
        }

        fn DragLeave(&self) -> windows::core::Result<()> {
            Ok(())
        }

        fn Drop(
            &self,
            pdataobject: windows_core::Ref<'_, IDataObject>,
            _grfkeystate: windows::Win32::System::SystemServices::MODIFIERKEYS_FLAGS,
            pt: &POINTL,
            pdweffect: *mut DROPEFFECT,
        ) -> windows::core::Result<()> {
            let data_obj = match pdataobject.as_ref() {
                Some(obj) => obj,
                None => return Ok(()),
            };

            let format = FORMATETC {
                cfFormat: CF_HDROP.0,
                ptd: std::ptr::null_mut(),
                dwAspect: 1, // DVASPECT_CONTENT
                lindex: -1,
                tymed: TYMED_HGLOBAL.0 as u32,
            };

            unsafe {
                if let Ok(medium) = data_obj.GetData(&format) {
                    let hglobal = medium.u.hGlobal;
                    let paths = extract_paths_from_hdrop(hglobal);
                    if !paths.is_empty() {
                        let _ = self.tx.send(OleDropEvent {
                            paths,
                            screen_pos: (pt.x, pt.y),
                        });
                    }
                    if !pdweffect.is_null() {
                        *pdweffect = DROPEFFECT_COPY;
                    }
                } else if !pdweffect.is_null() {
                    *pdweffect = DROPEFFECT_NONE;
                }
            }
            Ok(())
        }
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

    /// Register the OLE drop target for a native window handle.
    pub fn register_drop_target(
        hwnd_raw: usize,
        tx: Sender<OleDropEvent>,
    ) -> Result<(), String> {
        let target: IDropTarget = DesktopDropTarget::new(tx).into();
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
pub fn register_drop_target(
    _hwnd_raw: usize,
    _tx: Sender<OleDropEvent>,
) -> Result<(), String> {
    Ok(())
}

#[cfg(not(windows))]
pub fn revoke_drop_target(_hwnd_raw: usize) -> Result<(), String> {
    Ok(())
}
