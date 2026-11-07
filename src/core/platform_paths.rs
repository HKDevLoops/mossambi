//! Platform-native known path resolution.
//!
//! Windows: Uses `SHGetKnownFolderPath` (Shell32.dll) to resolve standard system locations
//! accurately even when OneDrive folder protection or redirected folders are active.
//! Falls back gracefully to `USERPROFILE` / `HOME` if the shell call fails.
//!
//! macOS / Linux: Uses standard environment variables ($HOME, $XDG_DATA_HOME) and standard directories.

use std::path::PathBuf;

#[cfg(windows)]
use windows::core::GUID;
#[cfg(windows)]
use windows::Win32::System::Com::CoTaskMemFree;
#[cfg(windows)]
use windows::Win32::UI::Shell::{
    SHGetKnownFolderPath, FOLDERID_Desktop, FOLDERID_Documents, FOLDERID_Downloads,
    FOLDERID_Music, FOLDERID_Pictures, FOLDERID_Videos, KF_FLAG_DEFAULT,
};

#[cfg(windows)]
fn get_known_folder(rfid: &GUID) -> Option<PathBuf> {
    unsafe {
        let path_pwstr = SHGetKnownFolderPath(rfid, KF_FLAG_DEFAULT, None).ok()?;
        let path_str = path_pwstr.to_string().ok()?;
        CoTaskMemFree(Some(path_pwstr.as_ptr() as *const _));
        Some(PathBuf::from(path_str))
    }
}

pub fn home_dir() -> PathBuf {
    std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn desktop_dir() -> PathBuf {
    #[cfg(windows)]
    if let Some(p) = get_known_folder(&FOLDERID_Desktop) {
        if p.exists() {
            return p;
        }
    }
    home_dir().join("Desktop")
}

pub fn downloads_dir() -> PathBuf {
    #[cfg(windows)]
    if let Some(p) = get_known_folder(&FOLDERID_Downloads) {
        if p.exists() {
            return p;
        }
    }
    home_dir().join("Downloads")
}

pub fn documents_dir() -> PathBuf {
    #[cfg(windows)]
    if let Some(p) = get_known_folder(&FOLDERID_Documents) {
        if p.exists() {
            return p;
        }
    }
    home_dir().join("Documents")
}

pub fn pictures_dir() -> PathBuf {
    #[cfg(windows)]
    if let Some(p) = get_known_folder(&FOLDERID_Pictures) {
        if p.exists() {
            return p;
        }
    }
    home_dir().join("Pictures")
}

pub fn videos_dir() -> PathBuf {
    #[cfg(windows)]
    if let Some(p) = get_known_folder(&FOLDERID_Videos) {
        if p.exists() {
            return p;
        }
    }
    home_dir().join("Videos")
}

pub fn music_dir() -> PathBuf {
    #[cfg(windows)]
    if let Some(p) = get_known_folder(&FOLDERID_Music) {
        if p.exists() {
            return p;
        }
    }
    home_dir().join("Music")
}

pub fn app_data_dir() -> PathBuf {
    #[cfg(windows)]
    {
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            let p = PathBuf::from(local).join("mossambi");
            let _ = std::fs::create_dir_all(&p);
            return p;
        }
        let p = home_dir().join(".mossambi");
        let _ = std::fs::create_dir_all(&p);
        p
    }
    #[cfg(target_os = "macos")]
    {
        let p = home_dir().join("Library/Application Support/mossambi");
        let _ = std::fs::create_dir_all(&p);
        p
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        if let Ok(data) = std::env::var("XDG_DATA_HOME") {
            let p = PathBuf::from(data).join("mossambi");
            let _ = std::fs::create_dir_all(&p);
            return p;
        }
        let p = home_dir().join(".local/share/mossambi");
        let _ = std::fs::create_dir_all(&p);
        p
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_paths_resolve() {
        let home = home_dir();
        assert!(home.exists(), "Home directory should exist");

        let app_data = app_data_dir();
        assert!(app_data.exists(), "App data directory should exist");

        let desktop = desktop_dir();
        assert!(desktop.is_absolute());

        let downloads = downloads_dir();
        assert!(downloads.is_absolute());
    }
}
