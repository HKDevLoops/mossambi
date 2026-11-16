//! Platform Abstraction Layer for Mossambi
//!
//! Pure 7-bit ASCII. Provides clean OS-neutral APIs for paths, shell actions,
//! window backdrop, and native integrations with fallback support for non-Windows platforms.

use std::path::{Path, PathBuf};

#[cfg(windows)]
pub mod windows;

#[cfg(unix)]
pub mod unix;

/// Returns the unified configuration directory `<home>/.config/mossambi` across ALL OSes.
/// (Windows: `%USERPROFILE%\.config\mossambi`, macOS/Linux: `$HOME/.config/mossambi`).
/// Guaranteed to be `.config/mossambi` across all targets (not AppData, not ~/Library).
pub fn config_dir() -> PathBuf {
    crate::core::platform_paths::config_dir()
}

/// Open `path` with the system's default application:
/// - Windows: ShellExecuteW / cmd start
/// - macOS: `open`
/// - Linux: `xdg-open`
pub fn open_in_default_app(path: &Path) -> bool {
    #[cfg(target_os = "windows")]
    {
        windows::open_in_default_app(path)
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(path).spawn().is_ok()
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .is_ok()
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        let _ = path;
        false
    }
}

/// Open the system "Open With" dialog / application chooser:
/// - Windows: ShellExecuteW "openas"
/// - macOS: `open`
/// - Linux: `xdg-open`
pub fn open_with_chooser(path: &Path) -> bool {
    crate::gui::context_menu::open_with_choose(path)
}

/// Whether native transparent/acrylic/mica glass backdrop is supported.
/// True on Windows 10 1809+, false on macOS/Linux (solid container fallback).
pub fn is_glass_supported() -> bool {
    crate::gui::dwm_helper::backdrop_supported()
}
