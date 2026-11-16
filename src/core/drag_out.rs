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
            Ok(format!(
                "{} file(s) copied to the file clipboard via {} - paste into the target app",
                paths.len(),
                outcome.via
            ))
        }
        #[cfg(not(target_os = "linux"))]
        {
            Err("Drag-out is not supported on this platform".to_string())
        }
    }
}

/// Layout metrics and badge text for DnD ghost stack following the pointer.
#[derive(Debug, Clone, PartialEq)]
pub struct DragGhostLayout {
    pub card_count: usize,
    pub card_offsets: Vec<(f32, f32)>,
    pub badge_text: String,
    pub badge_offset: (f32, f32),
    pub pill_width: f32,
    pub pill_height: f32,
}

/// Compute DnD ghost stack layout and count badge text.
pub fn compute_drag_ghost_layout(
    count: usize,
    ctrl: bool,
    shift: bool,
    alt: bool,
    drop_pref: crate::core::config::DropActionPreference,
) -> DragGhostLayout {
    let card_count = count.clamp(1, 3);
    let mut card_offsets = Vec::with_capacity(card_count);
    for i in 0..card_count {
        let offset = (i as f32) * 4.0;
        card_offsets.push((-offset, -offset));
    }

    let badge_text = if alt || (ctrl && shift) {
        "Link".to_string()
    } else if shift {
        "Move".to_string()
    } else if ctrl {
        "Copy".to_string()
    } else if count > 1 {
        count.to_string()
    } else {
        match drop_pref {
            crate::core::config::DropActionPreference::AlwaysCopy => "Copy".to_string(),
            crate::core::config::DropActionPreference::AlwaysMove => "Move".to_string(),
            crate::core::config::DropActionPreference::AlwaysAsk => "Ask".to_string(),
            crate::core::config::DropActionPreference::SmartDefault => "Move".to_string(),
        }
    };

    let pill_width = (badge_text.len() as f32 * 8.0 + 16.0).max(24.0);
    let pill_height = 20.0;
    let badge_offset = (24.0, -8.0);

    DragGhostLayout {
        card_count,
        card_offsets,
        badge_text,
        badge_offset,
        pill_width,
        pill_height,
    }
}
