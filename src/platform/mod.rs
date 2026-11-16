//! Platform Abstraction Layer for Mossambi
//!
//! Pure 7-bit ASCII. Provides clean OS-neutral APIs for paths, shell actions,
//! window backdrop, and native integrations with fallback support for non-Windows platforms.

use std::path::{Path, PathBuf};

pub mod args;
pub mod default_handler;
pub mod dialog_companion;
pub mod single_instance;

pub mod shell_context_menu;
pub use shell_context_menu::{
    build_shell_menu_sync, clean_menu_title, ensure_shell_menu_requested, get_active_shell_menu,
    invoke_shell_command, is_duplicate_verb, is_shell_menu_loading, render_shell_menu_section,
    request_shell_menu, ShellMenuItem,
};

#[cfg(windows)]
pub mod windows;

#[cfg(unix)]
pub mod unix;

/// Native toast notifications: Windows Action Center toasts; a logged
/// no-op on other platforms. Pure builders here are headless-testable.
pub mod notify;

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

/// Dialog companion bar lifecycle for standard Windows Open/Save dialogs
/// (W4b, opt-in setting `dialog_companion`). No-op on other OSes.
pub fn dialog_companion_start() {
    #[cfg(windows)]
    {
        windows::dialog_companion::start();
    }
}

/// Stop the dialog companion bar thread (posts WM_QUIT). No-op on other OSes.
pub fn dialog_companion_stop() {
    #[cfg(windows)]
    {
        windows::dialog_companion::stop();
    }
}

/// Start the dialog companion bar if the stored setting is enabled.
/// Called once at GUI startup; no-op on other OSes and when the setting is off.
pub fn dialog_companion_start_if_enabled() {
    #[cfg(windows)]
    {
        if crate::core::MossambiConfig::load().dialog_companion {
            windows::dialog_companion::start();
        }
    }
}
