//! Mossambi Core Library
//! Format registry, Pantone color palette, sibling output path logic, and persistent configuration.

pub mod action_journal;
pub mod async_worker;
pub mod config;
pub mod default_explorer;
pub mod dir_watcher;
pub mod dnd_backend;
pub mod donut_config;
pub mod drag_listener;
pub mod drag_out;
pub mod fields_config;
pub mod file_icons;
pub mod fs_utils;
pub mod fuzzy;
pub mod gesture;
pub mod icon_packs;
pub mod linux_clipboard_files;
pub mod linux_dna;
pub mod mouse_nav;
#[cfg(target_os = "windows")]
pub mod ole_drag_out;
pub mod ole_drop_target;
pub mod pantone;
pub mod platform_paths;
pub mod preview;
pub mod registry;
pub mod route_planner;
pub mod scripting;
pub mod state_db;
pub mod tags;
pub mod web_drop;

pub use action_journal::{
    record_copy, record_create_dir, record_move, record_rename, record_tag_set, stage_delete,
    undo_last,
};
pub use async_worker::{AsyncWorkerPool, TaskUpdate};
pub use config::{ExplorerStyle, GlassEffect, MossambiConfig, OutputMode, ThemeMode};
pub use dir_watcher::DirWatcher;
pub use donut_config::{DonutAppearance, DonutConfig, DonutProfile, DonutSlicePreset};
#[cfg(target_os = "windows")]
pub use drag_listener::query_active_explorer_files;
pub use drag_listener::{extract_clipboard_files, DragGestureEvent, DragGestureListener};
pub use drag_out::begin_file_drag_out;
pub use fields_config::{FieldItemSpec, FieldsConfig};
pub use file_icons::{resolve_file_icon, FileIconSpec};
pub use fs_utils::{
    copy_entry_recursive, format_file_size, get_system_drives, is_library_folder_name,
    is_protected_os_file, list_directory_sync, move_entry, parse_breadcrumbs, pluralize_items,
    remove_entry_recursive, resolve_recycle_bin_target, resolve_unique_paste_path,
    spawn_async_dir_scan, BreadcrumbSegment, DriveInfo, FileItem, ScanBatch,
};
pub use fuzzy::{fuzzy_match, sort_by_fuzzy_match};
pub use gesture::{GestureKind, GesturePoint, KinematicGestureTracker};
pub use linux_dna::{
    discover_linux_mounts, install_linux_desktop_integration, send_linux_notification,
};
pub use mouse_nav::MouseNavListener;
pub use pantone::{
    all_pantone_themes, get_pantone_theme, CitrusShade, PantoneColor, PantonePalette, PantoneTheme,
    THEMES,
};
pub use registry::{
    DynamicWheelSector, FileCategory, FormatInfo, FormatNature, FormatSpecification,
    UniversalRegistry,
};

use std::path::{Path, PathBuf};

/// Resolves the clean sibling output path placed directly beside the original file in the same directory.
/// Performs collision detection and rename protection:
/// - If <stem>.<target_ext> does not exist, uses that clean name (e.g. song.wav -> song.flac).
/// - If <stem>.<target_ext> exists, safely increments to <stem> (1).<target_ext>, <stem> (2)... to avoid accidental overwrite.
/// - If target_ext matches input extension, uses <stem> (1).<target_ext>.
pub fn resolve_clean_sibling_output_path(
    input: &Path,
    target_ext: &str,
    custom_dir: Option<&Path>,
) -> PathBuf {
    let parent = custom_dir.unwrap_or_else(|| input.parent().unwrap_or_else(|| Path::new(".")));
    let stem = input
        .file_stem()
        .map(|s| s.to_string_lossy())
        .unwrap_or_else(|| "output".into());

    let ext_clean = target_ext.trim_start_matches('.').to_lowercase();
    let in_ext = input
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    let mut candidate = if in_ext != ext_clean {
        parent.join(format!("{}.{}", stem, ext_clean))
    } else {
        parent.join(format!("{} (1).{}", stem, ext_clean))
    };

    let mut counter = 1u32;
    while candidate.exists() {
        candidate = parent.join(format!("{} ({}).{}", stem, counter, ext_clean));
        counter += 1;
    }

    candidate
}

/// Computes a non-destructive sibling output path.
/// Format: <parent_dir>/<file_stem>_converted.<target_ext>
/// Automatically appends _1, _2... if a collision occurs so original files are never overwritten.
pub fn resolve_sibling_output_path(
    input: &Path,
    suffix: Option<&str>,
    target_ext: &str,
    custom_dir: Option<&Path>,
) -> PathBuf {
    let parent = custom_dir.unwrap_or_else(|| input.parent().unwrap_or_else(|| Path::new(".")));
    let stem = input
        .file_stem()
        .map(|s| s.to_string_lossy())
        .unwrap_or_else(|| "output".into());

    let base_stem = match suffix {
        Some(sfx) if !sfx.is_empty() => format!("{}_{}", stem, sfx),
        _ => format!("{}_converted", stem),
    };

    let ext_clean = target_ext.trim_start_matches('.').to_lowercase();
    let mut candidate = parent.join(format!("{}.{}", base_stem, ext_clean));

    let mut counter = 1u32;
    while candidate.exists() {
        candidate = parent.join(format!("{}_{}.{}", base_stem, counter, ext_clean));
        counter += 1;
    }

    candidate
}

/// Target CPU architecture and platform information.
pub fn get_system_arch_info() -> (&'static str, &'static str) {
    let os = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "unknown"
    };

    let arch = if cfg!(target_arch = "x86_64") {
        "x86_64"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else if cfg!(target_arch = "arm") {
        "arm"
    } else {
        "unknown"
    };

    (os, arch)
}
