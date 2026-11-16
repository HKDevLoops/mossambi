//! Unix-specific (Linux & macOS) platform helpers.
//!
//! Pure 7-bit ASCII.

use std::path::Path;

pub fn open_in_default_app(path: &Path) -> bool {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(path).spawn().is_ok()
    }
    #[cfg(not(target_os = "macos"))]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .is_ok()
    }
}
