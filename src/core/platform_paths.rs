//! Platform-native known path resolution.
//!
//! Windows: Uses `SHGetKnownFolderPath` (Shell32.dll) to resolve standard system locations
//! accurately even when OneDrive folder protection or redirected folders are active.
//! Falls back gracefully to `USERPROFILE` / `HOME` if the shell call fails.
//!
//! macOS / Linux: Uses standard environment variables ($HOME, $XDG_DATA_HOME) and standard directories.

use std::path::{Path, PathBuf};

/// Special virtual location URI for the home dashboard view.
/// This is not a filesystem path and must never match any real directory.
pub const DASHBOARD_URI: &str = "home:";

/// Returns the virtual path representing the dashboard location.
pub fn dashboard_location() -> PathBuf {
    PathBuf::from(DASHBOARD_URI)
}

/// Checks if a path is the virtual dashboard location.
pub fn is_dashboard_location(path: &Path) -> bool {
    path.as_os_str() == DASHBOARD_URI || path.to_string_lossy() == DASHBOARD_URI
}

#[cfg(windows)]
use windows::core::GUID;
#[cfg(windows)]
use windows::Win32::System::Com::CoTaskMemFree;
#[cfg(windows)]
use windows::Win32::UI::Shell::{
    FOLDERID_Desktop, FOLDERID_Documents, FOLDERID_Downloads, FOLDERID_Music, FOLDERID_Pictures,
    FOLDERID_Profile, FOLDERID_Videos, SHGetKnownFolderPath, KF_FLAG_DEFAULT,
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
    #[cfg(windows)]
    if let Some(p) = get_known_folder(&FOLDERID_Profile) {
        if p.exists() {
            return p;
        }
    }
    std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Expands environment variables (%USERPROFILE%, %APPDATA%, %TEMP%, etc. on Windows)
/// and Unix tilde (`~` / `~/`) into a fully resolved PathBuf.
pub fn expand_path_str(raw: &str) -> PathBuf {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return PathBuf::new();
    }

    // Special virtual dashboard location
    if is_dashboard_location(Path::new(trimmed)) || trimmed.eq_ignore_ascii_case("home:") {
        return dashboard_location();
    }

    // 1. Tilde expansion
    let with_tilde = if trimmed == "~" {
        home_dir().to_string_lossy().to_string()
    } else if let Some(rest) = trimmed
        .strip_prefix("~/")
        .or_else(|| trimmed.strip_prefix("~\\"))
    {
        home_dir().join(rest).to_string_lossy().to_string()
    } else {
        trimmed.to_string()
    };

    // 2. Environment variable expansion (%VAR% on Windows, $VAR on Unix)
    let mut out = String::with_capacity(with_tilde.len());
    let mut rest = with_tilde.as_str();

    while let Some(i) = rest.find('%') {
        out.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        match after.find('%') {
            Some(j) => {
                let var_name = &after[..j];
                if var_name.eq_ignore_ascii_case("USERPROFILE") {
                    out.push_str(&home_dir().to_string_lossy());
                } else if let Ok(val) = std::env::var(var_name) {
                    out.push_str(&val);
                } else if let Ok(val) = std::env::var(var_name.to_uppercase()) {
                    out.push_str(&val);
                } else {
                    out.push('%');
                    out.push_str(var_name);
                    out.push('%');
                }
                rest = &after[j + 1..];
            }
            None => {
                out.push('%');
                rest = after;
            }
        }
    }
    out.push_str(rest);

    PathBuf::from(out)
}

/// Home directory as an `Option` for call sites that must distinguish a
/// resolved profile directory from the "." placeholder. Returns `None` when
/// only the placeholder is available (no USERPROFILE / HOME in the
/// environment).
pub fn home_dir_opt() -> Option<PathBuf> {
    let home = home_dir();
    if home.as_os_str() == "." {
        None
    } else {
        Some(home)
    }
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

/// Unified configuration directory returning `<home>/.config/mossambi` on ALL OSes
/// (Windows: `%USERPROFILE%\.config\mossambi`, macOS/Linux: `$HOME/.config/mossambi`).
/// Not AppData, not ~/Library.
pub fn config_dir() -> PathBuf {
    home_dir().join(".config").join("mossambi")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_dir_unified_across_all_platforms() {
        let cfg = config_dir();
        let expected_suffix = std::path::Path::new(".config").join("mossambi");
        assert!(
            cfg.ends_with(&expected_suffix),
            "config_dir {:?} must end with .config/mossambi",
            cfg
        );
        let home = home_dir();
        assert!(
            cfg.starts_with(&home),
            "config_dir {:?} must be rooted in home_dir {:?}",
            cfg,
            home
        );
        let s = cfg.to_string_lossy();
        assert!(
            !s.contains("AppData"),
            "config_dir must not use AppData: {}",
            s
        );
        assert!(
            !s.contains("Library"),
            "config_dir must not use Library: {}",
            s
        );
    }

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

    #[test]
    fn test_home_dir_opt_agrees_with_home_dir() {
        match home_dir_opt() {
            Some(home) => assert_eq!(home, home_dir()),
            None => assert_eq!(home_dir().as_os_str(), "."),
        }
    }

    #[test]
    fn test_dashboard_location_invariants() {
        let dash = dashboard_location();
        assert_eq!(dash.as_os_str(), DASHBOARD_URI);
        assert!(is_dashboard_location(&dash));
        assert!(is_dashboard_location(Path::new("home:")));

        let home = home_dir();
        assert_ne!(dash, home);
        assert!(!is_dashboard_location(&home));
        assert!(!is_dashboard_location(Path::new("C:\\Users\\user")));
        assert!(!is_dashboard_location(Path::new("/home/user")));
    }
}
