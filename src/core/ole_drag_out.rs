//! Windows OLE Drag-Out Source (real OS-level drag from Mossambi)
//!
//! Implements the three COM surfaces `DoDragDrop` needs:
//! - `IDataObject`  : carries a CF_HDROP list of paths (+ `Preferred DropEffect`)
//! - `IEnumFORMATETC`: lets targets enumerate the offered formats
//! - `IDropSource`  : cursor feedback + drop/cancel decisions
//!
//! Built on the `windows` crate's `#[implement]` macro (windows 0.59 / windows-core 0.59).
//! The 0.59 contract says: the `#[implement(IDataObject)]` attribute on a user struct
//! `FilesDataObject` generates an outer type `FilesDataObject_Impl` that owns the inner
//! value, implements `windows_core::IUnknownImpl`, and derefs to the inner struct. Code
//! therefore implements `<Interface>_Impl` **on the generated outer type** and reaches
//! state through that deref. The interface handle itself comes from a plain
//! `value.into()` conversion.
//!
//! Design rules:
//! - COPY-only. The source advertises `DROPEFFECT_COPY` exclusively, so a target app
//!   can never instruct this process to delete moved files.
//! - `GetData` hands out a fresh `GMEM_MOVEABLE` clone per request, because the
//!   target owns and frees whatever it receives.
//! - The master HGLOBAL blocks are freed when the inner object `Drop`s (refcount zero).
//! - Must be called on the thread that owns the mouse press (the egui/winit UI
//!   thread); `DoDragDrop` runs its own modal message loop there.
//!
//! Pointer hygiene note: the generated COM plumbing wants the payload type usable in
//! shared contexts, so master handles are stored as `usize` and materialized into
//! `HGLOBAL` on demand rather than stored as raw pointers.

#![cfg(target_os = "windows")]

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use windows::core::{implement, w, Error, HRESULT};
use windows::Win32::Foundation::{
    GlobalFree, BOOL, COLORREF, DATA_S_SAMEFORMATETC, DRAGDROP_S_CANCEL, DRAGDROP_S_DROP,
    DRAGDROP_S_USEDEFAULTCURSORS, DV_E_FORMATETC, DV_E_TYMED, E_FAIL, E_NOTIMPL, E_POINTER,
    HGLOBAL, POINT, SIZE,
};
use windows::Win32::Graphics::Gdi::{CreateBitmap, DeleteObject};
use windows::Win32::System::Com::{
    CoCreateInstance, IAdviseSink, IDataObject, IDataObject_Impl, IEnumFORMATETC,
    IEnumFORMATETC_Impl, IEnumSTATDATA, CLSCTX_INPROC_SERVER, DATADIR_GET, DVASPECT_CONTENT,
    FORMATETC, STGMEDIUM, STGMEDIUM_0, TYMED_HGLOBAL,
};
use windows::Win32::System::DataExchange::RegisterClipboardFormatW;
use windows::Win32::System::Memory::{
    GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE,
};
use windows::Win32::System::Ole::{
    DoDragDrop, IDropSource, IDropSource_Impl, OleUninitialize, CF_HDROP, DROPEFFECT,
    DROPEFFECT_COPY, DROPEFFECT_LINK, DROPEFFECT_MOVE,
};
use windows::Win32::System::SystemServices::{
    MK_LBUTTON, MK_MBUTTON, MK_RBUTTON, MODIFIERKEYS_FLAGS,
};
use windows::Win32::UI::Shell::{DragQueryFileW, IDragSourceHelper, DROPFILES, HDROP, SHDRAGIMAGE};

/// Button state that still counts as "user is dragging".
const DRAG_BUTTON_MASK: u32 = MK_LBUTTON.0 | MK_MBUTTON.0 | MK_RBUTTON.0;

const HR_S_OK: HRESULT = HRESULT(0);
const HR_S_FALSE: HRESULT = HRESULT(1);
/// COM already initialized on this thread in a different apartment.
const HR_RPC_E_CHANGED_MODE: HRESULT = HRESULT(0x80010106u32 as i32);

/// HRESULT -> Error conversion in one place.
fn hr(code: HRESULT) -> Error {
    Error::from_hresult(code)
}

/// How an OLE drag-out ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DragOutOutcome {
    /// True = a target accepted the drop.
    pub dropped: bool,
    /// True = the user aborted (Esc / focus loss / release off any target).
    pub cancelled: bool,
    /// Dropeffect returned by target (COPY / MOVE / LINK).
    pub effect: u32,
}

impl DragOutOutcome {
    pub fn describe(&self) -> &'static str {
        if self.dropped {
            if self.effect == DROPEFFECT_MOVE.0 {
                "File(s) dropped out (move)"
            } else if self.effect == DROPEFFECT_LINK.0 {
                "File(s) dropped out (link)"
            } else {
                "File(s) dropped out (copy)"
            }
        } else if self.cancelled {
            "Drag cancelled"
        } else {
            "Drag ended without a drop"
        }
    }
}

// ---------------------------------------------------------------------------
// Pure buffer builders (unit-testable, no COM state)
// ---------------------------------------------------------------------------

/// `DROPFILES` + double-NUL-terminated wide file list, exactly what
/// `CF_HDROP` expects (`pFiles` points at the first path).
fn build_dropfiles_buffer(paths: &[PathBuf]) -> Vec<u8> {
    use std::os::windows::ffi::OsStrExt;

    let mut names: Vec<u16> = Vec::new();
    for path in paths {
        names.extend(path.as_os_str().encode_wide().chain(std::iter::once(0)));
    }
    names.push(0); // CF_HDROP lists end with an extra NUL

    let df_size = std::mem::size_of::<DROPFILES>(); // 20 bytes (repr(C, packed(1)))
    let mut buffer = vec![0u8; df_size + names.len() * 2];

    let df = DROPFILES {
        pFiles: df_size as u32,
        pt: POINT { x: 0, y: 0 },
        fNC: BOOL(0),
        fWide: BOOL(1),
    };
    // SAFETY: copies one fully initialized repr(C) struct into local space.
    let df_bytes =
        unsafe { std::slice::from_raw_parts(&df as *const DROPFILES as *const u8, df_size) };
    buffer[..df_size].copy_from_slice(df_bytes);

    // SAFETY: 16-bit units reinterpreted as little-endian bytes.
    let names_bytes =
        unsafe { std::slice::from_raw_parts(names.as_ptr() as *const u8, names.len() * 2) };
    buffer[df_size..].copy_from_slice(names_bytes);
    buffer
}

/// 4-byte `Preferred DropEffect` payload (DROPEFFECT_COPY).
fn build_pref_dropeffect_buffer(effect: u32) -> [u8; 4] {
    effect.to_ne_bytes()
}

/// Offered FORMATETC description without pointers (Sync-safe payload).
#[derive(Debug, Clone, Copy)]
struct OfferedFormat {
    cf_format: u16,
    lindex: i32,
    tymed: u32,
    dw_aspect: u32,
}

impl OfferedFormat {
    const fn hdrop() -> Self {
        Self {
            cf_format: CF_HDROP.0,
            lindex: -1,
            tymed: TYMED_HGLOBAL.0 as u32,
            dw_aspect: DVASPECT_CONTENT.0,
        }
    }

    const fn preferred(cf_pref: u16) -> Self {
        Self {
            cf_format: cf_pref,
            lindex: -1,
            tymed: TYMED_HGLOBAL.0 as u32,
            dw_aspect: DVASPECT_CONTENT.0,
        }
    }
}

// ---------------------------------------------------------------------------
// COM object: IDataObject (CF_HDROP + Preferred DropEffect)
// ---------------------------------------------------------------------------

#[implement(IDataObject)]
struct FilesDataObject {
    /// Master CF_HDROP block (usize-stored HGLOBAL).
    hdrop: usize,
    /// Master `Preferred DropEffect` block.
    hpref: usize,
    /// Registered clipboard format id of `Preferred DropEffect`.
    cf_pref: u32,
}

impl FilesDataObject {
    fn materialize(stored: usize) -> HGLOBAL {
        HGLOBAL(stored as *mut core::ffi::c_void)
    }

    fn new(paths: &[PathBuf]) -> Result<Self, Error> {
        let hdrop = unsafe { alloc_global_from_buffer(&build_dropfiles_buffer(paths)) }?.0 as usize;

        let cf_pref = unsafe { RegisterClipboardFormatW(w!("Preferred DropEffect")) };
        if cf_pref == 0 {
            // SAFETY: undo our successful allocation before unwinding.
            unsafe {
                let _ = GlobalFree(Some(Self::materialize(hdrop)));
            }
            return Err(hr(E_FAIL));
        }

        // SAFETY: plain kernel allocation; payload is the COPY effect word.
        let hpref = unsafe {
            alloc_global_from_buffer(&build_pref_dropeffect_buffer(DROPEFFECT_COPY.0))
        }?
        .0 as usize;
        Ok(Self {
            hdrop,
            hpref,
            cf_pref,
        })
    }

    fn offered_formats(&self) -> Vec<OfferedFormat> {
        vec![
            OfferedFormat::hdrop(),
            OfferedFormat::preferred(self.cf_pref as u16),
        ]
    }

    /// Resolve a requested `FORMATETC` to one of our master handles.
    fn resolve_source(&self, request: &FORMATETC) -> Result<usize, HRESULT> {
        if request.lindex != -1 {
            return Err(DV_E_FORMATETC);
        }
        if request.dwAspect != DVASPECT_CONTENT.0 {
            return Err(DV_E_FORMATETC);
        }
        if request.tymed != TYMED_HGLOBAL.0 as u32 && request.tymed != 0 {
            return Err(DV_E_TYMED);
        }
        let source = if request.cfFormat == CF_HDROP.0 {
            self.hdrop
        } else if request.cfFormat == self.cf_pref as u16 {
            self.hpref
        } else {
            return Err(DV_E_FORMATETC);
        };
        if source == 0 {
            return Err(E_FAIL);
        }
        Ok(source)
    }
}

impl Drop for FilesDataObject {
    fn drop(&mut self) {
        for stored in [self.hdrop, self.hpref] {
            if stored != 0 {
                // SAFETY: masters are never shared (GetData hands out clones);
                // freeing at refcount zero is the documented ownership hand-off.
                unsafe {
                    let _ = GlobalFree(Some(Self::materialize(stored)));
                }
            }
        }
    }
}

// The `_Impl` type is generated by `#[implement]` in this same module and
// derefs to `FilesDataObject`, so every field access below resolves to the
// inner struct through that Deref.
impl IDataObject_Impl for FilesDataObject_Impl {
    fn GetData(&self, pformatetcin: *const FORMATETC) -> windows::core::Result<STGMEDIUM> {
        if pformatetcin.is_null() {
            return Err(hr(E_POINTER));
        }
        // SAFETY: caller-provided pointer validated non-null above.
        let request = unsafe { &*pformatetcin };
        let stored = match self.resolve_source(request) {
            Ok(handle) => handle,
            Err(code) => return Err(hr(code)),
        };

        // SAFETY: master is live; the produced clone transfers ownership of a
        // fresh private GMEM_MOVEABLE block to the receiving target.
        let clone = unsafe { clone_hglobal(FilesDataObject::materialize(stored)) }?;
        Ok(STGMEDIUM {
            tymed: TYMED_HGLOBAL.0 as u32,
            u: STGMEDIUM_0 { hGlobal: clone },
            pUnkForRelease: std::mem::ManuallyDrop::new(None),
        })
    }

    fn GetDataHere(
        &self,
        _pformatetc: *const FORMATETC,
        _pmedium: *mut STGMEDIUM,
    ) -> windows::core::Result<()> {
        Err(hr(E_NOTIMPL))
    }

    fn QueryGetData(&self, pformatetc: *const FORMATETC) -> HRESULT {
        if pformatetc.is_null() {
            return E_POINTER;
        }
        // SAFETY: non-null validated above.
        let request = unsafe { &*pformatetc };
        match self.resolve_source(request) {
            Ok(_) => HR_S_OK,
            Err(code) => code,
        }
    }

    fn GetCanonicalFormatEtc(
        &self,
        pformatectin: *const FORMATETC,
        pformatetcout: *mut FORMATETC,
    ) -> HRESULT {
        if pformatectin.is_null() || pformatetcout.is_null() {
            return E_POINTER;
        }
        // SAFETY: both pointers validated above; the write targets caller memory.
        unsafe {
            std::ptr::write(pformatetcout, std::ptr::read(pformatectin));
            (*pformatetcout).ptd = std::ptr::null_mut();
        }
        DATA_S_SAMEFORMATETC
    }

    fn SetData(
        &self,
        _pformatetc: *const FORMATETC,
        _pmedium: *const STGMEDIUM,
        _frelease: BOOL,
    ) -> windows::core::Result<()> {
        Err(hr(E_NOTIMPL))
    }

    fn EnumFormatEtc(&self, dwdirection: u32) -> windows::core::Result<IEnumFORMATETC> {
        if dwdirection != DATADIR_GET.0 as u32 {
            return Err(hr(E_NOTIMPL));
        }
        Ok(FormatsEnumerator::new(self.offered_formats()).into())
    }

    fn DAdvise(
        &self,
        _pformatetc: *const FORMATETC,
        _advf: u32,
        _padvsink: windows::core::Ref<'_, IAdviseSink>,
    ) -> windows::core::Result<u32> {
        Err(hr(E_NOTIMPL))
    }

    fn DUnadvise(&self, _dwconnection: u32) -> windows::core::Result<()> {
        Err(hr(E_NOTIMPL))
    }

    fn EnumDAdvise(&self) -> windows::core::Result<IEnumSTATDATA> {
        Err(hr(E_NOTIMPL))
    }
}

/// SAFETY: `source` is a live GMEM_MOVEABLE block we own; the fresh clone is
/// returned with a reference count of one and becomes the target's problem.
unsafe fn clone_hglobal(source: HGLOBAL) -> Result<HGLOBAL, Error> {
    let size = unsafe { GlobalSize(source) };
    // SAFETY: allocation sized by GlobalSize; failure handling below.
    let target = unsafe { GlobalAlloc(GMEM_MOVEABLE, size) }?;
    let src = unsafe { GlobalLock(source) };
    let dst = unsafe { GlobalLock(target) };
    if src.is_null() || dst.is_null() {
        // SAFETY: unlocking/freeing only allocations that this function owns.
        unsafe {
            if !dst.is_null() {
                let _ = GlobalUnlock(target);
            }
            if !src.is_null() {
                let _ = GlobalUnlock(source);
            }
            let _ = GlobalFree(Some(target));
        }
        return Err(hr(E_FAIL));
    }
    // SAFETY: both pointers lock blocks of `size` bytes.
    unsafe {
        std::ptr::copy_nonoverlapping(src.cast::<u8>(), dst.cast::<u8>(), size);
        let _ = GlobalUnlock(target);
        let _ = GlobalUnlock(source);
    }
    Ok(target)
}

/// Allocate a `GMEM_MOVEABLE` copy of `buffer`.
unsafe fn alloc_global_from_buffer(buffer: &[u8]) -> Result<HGLOBAL, Error> {
    // SAFETY: plain kernel32 allocation, no COM state involved.
    let handle = unsafe { GlobalAlloc(GMEM_MOVEABLE, buffer.len()) }?;
    // SAFETY: block freshly allocated at requested size.
    let ptr = unsafe { GlobalLock(handle) };
    if ptr.is_null() {
        // SAFETY: releasing our failed allocation; nobody else holds it.
        unsafe {
            let _ = GlobalFree(Some(handle));
        }
        return Err(hr(E_FAIL));
    }
    // SAFETY: `ptr` points to at least `buffer.len()` bytes.
    unsafe {
        std::ptr::copy_nonoverlapping(buffer.as_ptr(), ptr.cast::<u8>(), buffer.len());
        let _ = GlobalUnlock(handle);
    }
    Ok(handle)
}

// ---------------------------------------------------------------------------
// COM object: IEnumFORMATETC
// ---------------------------------------------------------------------------

#[implement(IEnumFORMATETC)]
struct FormatsEnumerator {
    formats: Vec<OfferedFormat>,
    /// Interior mutability is required because enum methods take `&self`.
    index: AtomicUsize,
}

impl FormatsEnumerator {
    fn new(formats: Vec<OfferedFormat>) -> Self {
        Self {
            formats,
            index: AtomicUsize::new(0),
        }
    }
}

impl IEnumFORMATETC_Impl for FormatsEnumerator_Impl {
    fn Next(&self, celt: u32, rgelt: *mut FORMATETC, pceltfetched: *mut u32) -> HRESULT {
        if celt != 0 && rgelt.is_null() {
            return E_POINTER;
        }
        let current = self.index.load(Ordering::Relaxed);
        if current >= self.formats.len() || celt == 0 {
            if !pceltfetched.is_null() {
                // SAFETY: caller declared a writable slot when celt > 0; a
                // non-null slot at celt == 0 is also fair game.
                unsafe {
                    *pceltfetched = 0;
                }
            }
            return HR_S_FALSE;
        }
        let take = (celt as usize).min(self.formats.len() - current);
        // SAFETY: `rgelt` covers `take` elements per the ODL contract and the
        // sources are in-bounds; fills caller FORMATETC slots from our stored
        // descriptions (ptd stays null in every offer).
        unsafe {
            for i in 0..take {
                let offered = &self.formats[current + i];
                std::ptr::write(
                    rgelt.add(i),
                    FORMATETC {
                        cfFormat: offered.cf_format,
                        ptd: std::ptr::null_mut(),
                        dwAspect: offered.dw_aspect,
                        lindex: offered.lindex,
                        tymed: offered.tymed,
                    },
                );
            }
            if !pceltfetched.is_null() {
                *pceltfetched = take as u32;
            }
        }
        self.index.store(current + take, Ordering::Relaxed);
        if take == celt as usize {
            HR_S_OK
        } else {
            HR_S_FALSE
        }
    }

    fn Skip(&self, celt: u32) -> windows::core::Result<()> {
        let current = self.index.load(Ordering::Relaxed);
        let target = current.saturating_add(celt as usize);
        self.index.store(target, Ordering::Relaxed);
        if target <= self.formats.len() {
            Ok(())
        } else {
            Err(hr(HR_S_FALSE))
        }
    }

    fn Reset(&self) -> windows::core::Result<()> {
        self.index.store(0, Ordering::Relaxed);
        Ok(())
    }

    fn Clone(&self) -> windows::core::Result<IEnumFORMATETC> {
        let copy = FormatsEnumerator {
            formats: self.formats.clone(),
            index: AtomicUsize::new(self.index.load(Ordering::Relaxed)),
        };
        Ok(copy.into())
    }
}

// ---------------------------------------------------------------------------
// COM object: IDropSource
// ---------------------------------------------------------------------------

#[implement(IDropSource)]
struct DropSourceDefault;

impl IDropSource_Impl for DropSourceDefault_Impl {
    fn QueryContinueDrag(&self, fescapepressed: BOOL, grfkeystate: MODIFIERKEYS_FLAGS) -> HRESULT {
        if fescapepressed.0 != 0 {
            return DRAGDROP_S_CANCEL;
        }
        if grfkeystate.0 & DRAG_BUTTON_MASK == 0 {
            return DRAGDROP_S_DROP;
        }
        // Hardware failsafe: if physical mouse buttons are no longer depressed,
        // complete the drop immediately so DoDragDrop never hangs the UI thread.
        unsafe {
            use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
                GetAsyncKeyState, VK_LBUTTON, VK_MBUTTON, VK_RBUTTON,
            };
            let l_down = (GetAsyncKeyState(VK_LBUTTON as i32) as u16 & 0x8000) != 0;
            let m_down = (GetAsyncKeyState(VK_MBUTTON as i32) as u16 & 0x8000) != 0;
            let r_down = (GetAsyncKeyState(VK_RBUTTON as i32) as u16 & 0x8000) != 0;
            if !l_down && !m_down && !r_down {
                return DRAGDROP_S_DROP;
            }
        }
        HR_S_OK
    }

    fn GiveFeedback(&self, _dweffect: DROPEFFECT) -> HRESULT {
        DRAGDROP_S_USEDEFAULTCURSORS
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Runs a modal OLE drag of `paths` until the user drops or cancels.
///
/// Call on the UI thread while the primary mouse button is held; blocks until
/// the OS finishes the drag, then reports how it ended.
pub fn begin_drag_out(paths: &[PathBuf]) -> Result<DragOutOutcome, String> {
    if paths.is_empty() {
        return Err("No file(s) selected to drag".to_string());
    }
    for path in paths {
        if !path.exists() {
            return Err(format!(
                "Cannot drag \"{}\": file not found",
                path.display()
            ));
        }
    }
    // SAFETY: entry contract is "call on the thread owning the mouse press";
    // egui/winit satisfy this on Windows; `run_ole_drag` keeps apartment init
    // scoped to this call via the guard below.
    unsafe { run_ole_drag(paths).map_err(|e| e.to_string()) }
}

struct OleThreadGuard {
    initialized: bool,
}

impl Drop for OleThreadGuard {
    fn drop(&mut self) {
        if self.initialized {
            // SAFETY: balanced against this thread's own OleInitialize.
            unsafe {
                OleUninitialize();
            }
        }
    }
}

const CLSID_DRAG_DROP_HELPER: windows::core::GUID =
    windows::core::GUID::from_u128(0x4657278a_411b_11d2_839a_00c04fd918d0);

/// Attaches a custom drag image (thumbnail stack + count badge) to `IDataObject`
/// using `IDragSourceHelper::InitializeFromBitmap`.
pub fn attach_drag_image(data_object: &IDataObject, count: usize) {
    unsafe {
        let helper: Result<IDragSourceHelper, _> =
            CoCreateInstance(&CLSID_DRAG_DROP_HELPER, None, CLSCTX_INPROC_SERVER);
        let helper = match helper {
            Ok(h) => h,
            Err(e) => {
                log::debug!("IDragSourceHelper unavailable: {:?}", e);
                return;
            }
        };

        let width = 64i32;
        let height = 64i32;
        let mut pixels = vec![0u8; (width * height * 4) as usize];

        let card_w = 44;
        let card_h = 44;
        let cards = count.clamp(1, 3);
        for c in (0..cards).rev() {
            let ox = (c as i32) * 4 + 4;
            let oy = (c as i32) * 4 + 4;
            for y in oy..(oy + card_h).min(height) {
                for x in ox..(ox + card_w).min(width) {
                    let idx = ((y * width + x) * 4) as usize;
                    let is_border =
                        x == ox || x == ox + card_w - 1 || y == oy || y == oy + card_h - 1;
                    if is_border {
                        pixels[idx] = 120;
                        pixels[idx + 1] = 120;
                        pixels[idx + 2] = 120;
                        pixels[idx + 3] = 255;
                    } else {
                        let tint = 50 + (c as u8) * 20;
                        pixels[idx] = tint + 20;
                        pixels[idx + 1] = tint + 10;
                        pixels[idx + 2] = tint;
                        pixels[idx + 3] = 230;
                    }
                }
            }
        }

        if count > 1 {
            let bx = 36;
            let by = 36;
            let bw = 24;
            let bh = 20;
            for y in by..(by + bh).min(height) {
                for x in bx..(bx + bw).min(width) {
                    let idx = ((y * width + x) * 4) as usize;
                    pixels[idx] = 79;
                    pixels[idx + 1] = 88;
                    pixels[idx + 2] = 228;
                    pixels[idx + 3] = 255;
                }
            }
        }

        let hbmp = CreateBitmap(width, height, 1, 32, Some(pixels.as_ptr() as *const _));

        if !hbmp.is_invalid() {
            let shdi = SHDRAGIMAGE {
                sizeDragImage: SIZE {
                    cx: width,
                    cy: height,
                },
                ptOffset: POINT {
                    x: width / 2,
                    y: height / 2,
                },
                hbmpDragImage: hbmp,
                crColorKey: COLORREF(0),
            };

            let init_res = helper.InitializeFromBitmap(&shdi, data_object);
            if let Err(e) = init_res {
                log::debug!("InitializeFromBitmap failed: {:?}", e);
                let _ = DeleteObject(hbmp.into());
            }
        }
    }
}

/// Helper to extract list of PathBuf from an HGLOBAL containing DROPFILES.
pub fn extract_paths_from_hdrop(hglobal: HGLOBAL) -> Vec<PathBuf> {
    if hglobal.is_invalid() {
        return Vec::new();
    }
    unsafe {
        let hdrop = HDROP(hglobal.0);
        let count = DragQueryFileW(hdrop, 0xFFFFFFFF, None);
        let mut paths = Vec::with_capacity(count as usize);
        let mut buf = vec![0u16; 1024];
        for i in 0..count {
            let len = DragQueryFileW(hdrop, i, Some(&mut buf));
            if len > 0 {
                let s = String::from_utf16_lossy(&buf[..len as usize]);
                paths.push(PathBuf::from(s));
            }
        }
        paths
    }
}

/// Builds an IDataObject carrying CF_HDROP for the given paths.
pub fn create_files_data_object(paths: &[PathBuf]) -> Result<IDataObject, String> {
    let obj = FilesDataObject::new(paths)
        .map_err(|e| format!("Failed to create FilesDataObject: {:?}", e))?;
    let data_object: IDataObject = obj.into();
    attach_drag_image(&data_object, paths.len());
    Ok(data_object)
}

/// Runs the modal `DoDragDrop` loop for the given paths.
///
/// SAFETY: must run on the thread that owns the mouse press. `DoDragDrop`
/// borrows both interface handles for the duration; references satisfy the
/// `Param<...>` bounds without extra AddRef.
unsafe fn run_ole_drag(paths: &[PathBuf]) -> Result<DragOutOutcome, Error> {
    let mut guard = OleThreadGuard { initialized: false };
    // Call the raw ole32 entry point so S_OK and S_FALSE can be told apart.
    // The `windows` crate maps both to Ok(()); S_FALSE means winit already
    // initialized COM on this thread, and calling OleUninitialize from the
    // guard's Drop would tear down the host's apartment mid-frame and break
    // every later COM call. Only S_OK means *we* established the apartment.
    let init_hr = unsafe { windows_sys::Win32::System::Ole::OleInitialize(std::ptr::null()) };
    if init_hr == HR_S_OK.0 {
        guard.initialized = true;
    } else if init_hr == HR_S_FALSE.0 || init_hr == HR_RPC_E_CHANGED_MODE.0 {
        // COM already initialized on this thread; proceed without unwinding.
    } else {
        return Err(hr(HRESULT(init_hr)));
    }

    let data_object: IDataObject = FilesDataObject::new(paths)?.into();
    attach_drag_image(&data_object, paths.len());
    let drop_source: IDropSource = DropSourceDefault.into();

    log::info!(
        "OLE drag-out started ({} file(s), COPY|MOVE|LINK)",
        paths.len()
    );

    let allowed_effects = DROPEFFECT_COPY | DROPEFFECT_MOVE | DROPEFFECT_LINK;
    let mut effect = DROPEFFECT(0); // DROPEFFECT_NONE
                                    // SAFETY: both interfaces outlive the modal loop; the effect pointer
                                    // is written by OLE during the call.
    let verdict_hr =
        unsafe { DoDragDrop(&data_object, &drop_source, allowed_effects, &mut effect) };

    if verdict_hr == DRAGDROP_S_DROP || verdict_hr == HRESULT(0) {
        log::info!(
            "OLE drag-out dropped (effect={}), result=0x{:08X}",
            effect.0,
            verdict_hr.0 as u32
        );
        Ok(DragOutOutcome {
            dropped: true,
            cancelled: false,
            effect: effect.0,
        })
    } else if verdict_hr == DRAGDROP_S_CANCEL {
        log::info!("OLE drag-out cancelled by user");
        Ok(DragOutOutcome {
            dropped: false,
            cancelled: true,
            effect: 0,
        })
    } else {
        Err(hr(verdict_hr))
    }
}

// ---------------------------------------------------------------------------
// Tests (GlobalAlloc/GlobalLock need no COM init)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dropfiles_buffer_is_shell_layout() {
        let paths = vec![
            PathBuf::from(r"C:\dir one\a.txt"),
            PathBuf::from(r"C:\b.bin"),
        ];
        let buffer = build_dropfiles_buffer(&paths);

        let df_size = std::mem::size_of::<DROPFILES>(); // 20 bytes, packed(1)
                                                        // pFiles offset equals the struct size (wide paths start there).
        let p_files = u32::from_le_bytes(buffer[0..4].try_into().unwrap());
        assert_eq!(p_files as usize, df_size);
        // fNC (offset 12..16) must be 0, fWide (offset 16..20) must be 1.
        assert_eq!(&buffer[12..16], &[0, 0, 0, 0]);
        assert_eq!(&buffer[16..20], &[1, 0, 0, 0]);

        // Decode the wide string list back and compare.
        let names: Vec<u16> = buffer[p_files as usize..]
            .chunks_exact(2)
            .map(|p| u16::from_le_bytes(p.try_into().unwrap()))
            .collect();
        let mut expected: Vec<u16> = Vec::new();
        for path in &paths {
            use std::os::windows::ffi::OsStrExt;
            expected.extend(path.as_os_str().encode_wide().chain(std::iter::once(0)));
        }
        expected.push(0);
        assert_eq!(names, expected);
    }

    #[test]
    fn pref_dropeffect_is_copy() {
        let bytes = build_pref_dropeffect_buffer(DROPEFFECT_COPY.0);
        assert_eq!(bytes, 1u32.to_ne_bytes());
    }
}
