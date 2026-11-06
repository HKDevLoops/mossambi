//! Cross-platform OS file drag-out dispatch.
//!
//! One entry point the GUI layers call when an internal drag leaves the
//! window bounds (Explorer) or crosses the motion threshold (Widget hub):
//!
//! - Windows: real OLE `DoDragDrop` source, COPY-only (`ole_drag_out`).
//! - Linux: typed file-clipboard fallback (`linux_clipboard_files`, `wl-copy`
//!   on Wayland / `xclip` on X11) - the receiver pastes from its own file
//!   manager, since a continuous cross-app drag cannot be carried over.
//! - Any other OS: a clear "unsupported" error, no silent degradation.

use std::path::PathBuf;

/// Hands `paths` to the OS drag/clipboard machinery.
///
/// Returns a user-presentable summary on success (the Windows case reports
/// how the OLE drag ended; Linux reports which clipboard helper stored the
/// payload), or an actionable error string.
pub fn begin_file_drag_out(paths: &[PathBuf]) -> Result<String, String> {
    if paths.is_empty() {
        return Err("Nothing to drag: no file(s) selected".to_string());
    }
    for path in paths {
        if !path.exists() {
            return Err(format!(
                "Cannot drag \"{}\": file not found",
                path.display()
            ));
        }
    }

    #[cfg(target_os = "windows")]
    {
        // Real OLE drag session (MODAL: blocks until the drag ends).
        crate::core::ole_drag_out::begin_drag_out(paths)
            .map(|outcome| outcome.describe().to_string())
    }

    #[cfg(not(target_os = "windows"))]
    {
        #[cfg(target_os = "linux")]
        {
            // Typed file-clipboard fallback: the receiver pastes from its
            // own file manager (wl-copy on Wayland, xclip on X11).
            let outcome = crate::core::linux_clipboard_files::linux::copy_files(
                paths,
                crate::core::linux_clipboard_files::linux::ClipboardOp::Copy,
            )?;
            return Ok(format!(
                "{} file(s) copied to the file clipboard via {} - paste into the target app",
                paths.len(),
                outcome.via
            ));
        }
        #[cfg(not(target_os = "linux"))]
        {
            Err("Drag-out is not supported on this platform".to_string())
        }
    }
}
