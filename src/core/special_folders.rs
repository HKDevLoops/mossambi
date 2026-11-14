//! Path-based special folder detection and taxonomy.
//!
//! Windows: Uses `SHGetKnownFolderPath` (Shell32) cached in a `OnceLock` map to identify
//! special system directories (Home/Profile, Desktop, Documents, Downloads, Pictures,
//! Music, Videos, Favorites, Saved Games, Links, Contacts, Searches, 3D Objects,
//! OneDrive, Program Files, Users, and Recycle Bin).
//!
//! Non-Windows: Uses standard environment variables ($HOME, $XDG_*_DIR) and standard paths.
//!
//! Fallback: Case-insensitive folder name lookup matching standard naming conventions.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

#[cfg(windows)]
use windows::core::GUID;
#[cfg(windows)]
use windows::Win32::System::Com::CoTaskMemFree;
#[cfg(windows)]
use windows::Win32::UI::Shell::{
    FOLDERID_Contacts, FOLDERID_Desktop, FOLDERID_Documents, FOLDERID_Downloads,
    FOLDERID_Favorites, FOLDERID_Links, FOLDERID_Music, FOLDERID_Objects3D, FOLDERID_Pictures,
    FOLDERID_Profile, FOLDERID_ProgramFiles, FOLDERID_SavedGames, FOLDERID_UserProfiles,
    FOLDERID_Videos, SHGetKnownFolderPath, KF_FLAG_DEFAULT,
};

#[cfg(windows)]
const FOLDERID_SEARCHES: GUID = GUID::from_u128(0x7D1D3A04_DEBB_4115_95CF_2F29DA2920DA);

/// Recognized special folder kinds across all supported platforms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpecialFolder {
    Home,
    Desktop,
    Documents,
    Downloads,
    Pictures,
    Music,
    Videos,
    Favorites,
    SavedGames,
    Links,
    Contacts,
    Searches,
    Objects3D,
    OneDrive,
    ProgramFiles,
    Users,
    RecycleBin,
}

impl SpecialFolder {
    /// All 17 special folder kinds.
    pub fn all() -> &'static [SpecialFolder] {
        &[
            SpecialFolder::Home,
            SpecialFolder::Desktop,
            SpecialFolder::Documents,
            SpecialFolder::Downloads,
            SpecialFolder::Pictures,
            SpecialFolder::Music,
            SpecialFolder::Videos,
            SpecialFolder::Favorites,
            SpecialFolder::SavedGames,
            SpecialFolder::Links,
            SpecialFolder::Contacts,
            SpecialFolder::Searches,
            SpecialFolder::Objects3D,
            SpecialFolder::OneDrive,
            SpecialFolder::ProgramFiles,
            SpecialFolder::Users,
            SpecialFolder::RecycleBin,
        ]
    }

    /// Identifier string for serialization or diagnostics.
    pub fn id(&self) -> &'static str {
        match self {
            SpecialFolder::Home => "home",
            SpecialFolder::Desktop => "desktop",
            SpecialFolder::Documents => "documents",
            SpecialFolder::Downloads => "downloads",
            SpecialFolder::Pictures => "pictures",
            SpecialFolder::Music => "music",
            SpecialFolder::Videos => "videos",
            SpecialFolder::Favorites => "favorites",
            SpecialFolder::SavedGames => "saved_games",
            SpecialFolder::Links => "links",
            SpecialFolder::Contacts => "contacts",
            SpecialFolder::Searches => "searches",
            SpecialFolder::Objects3D => "objects_3d",
            SpecialFolder::OneDrive => "onedrive",
            SpecialFolder::ProgramFiles => "program_files",
            SpecialFolder::Users => "users",
            SpecialFolder::RecycleBin => "recycle_bin",
        }
    }

    /// Default accent RGB color associated with this folder type.
    pub fn default_color(&self) -> [u8; 3] {
        match self {
            SpecialFolder::Desktop => [0, 139, 139],      // Teal
            SpecialFolder::Documents => [0, 120, 212],    // Blue
            SpecialFolder::Downloads => [16, 124, 65],    // Green
            SpecialFolder::Music => [216, 59, 1],         // Orange
            SpecialFolder::Pictures => [0, 99, 177],      // Cyan-Blue
            SpecialFolder::Videos => [136, 23, 152],      // Purple
            SpecialFolder::Home => [0, 120, 215],         // Home Blue
            SpecialFolder::OneDrive => [0, 120, 212],     // Cloud Blue
            SpecialFolder::Favorites => [212, 136, 6],    // Amber
            SpecialFolder::SavedGames => [109, 40, 217],  // Violet
            SpecialFolder::Links => [79, 70, 229],        // Indigo
            SpecialFolder::Contacts => [219, 39, 119],    // Pink
            SpecialFolder::Searches => [71, 85, 105],     // Slate
            SpecialFolder::Objects3D => [14, 116, 144],   // Dark Cyan
            SpecialFolder::ProgramFiles => [30, 58, 138], // Navy
            SpecialFolder::Users => [51, 65, 85],         // Slate
            SpecialFolder::RecycleBin => [100, 116, 139], // Silver-Grey
        }
    }
}

#[cfg(windows)]
fn call_known_folder(rfid: &GUID) -> Option<PathBuf> {
    unsafe {
        let pwstr = SHGetKnownFolderPath(rfid, KF_FLAG_DEFAULT, None).ok()?;
        let path_str = pwstr.to_string().ok()?;
        CoTaskMemFree(Some(pwstr.as_ptr() as *const _));
        Some(PathBuf::from(path_str))
    }
}

/// Normalizes path for comparison (case-insensitive on Windows, uniform slashes).
pub fn normalize_path(p: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        let s = p.to_string_lossy().replace('/', "\\");
        let trimmed = s.trim_end_matches('\\');
        PathBuf::from(trimmed.to_lowercase())
    }
    #[cfg(not(windows))]
    {
        p.to_path_buf()
    }
}

static KNOWN_FOLDERS_MAP: OnceLock<HashMap<PathBuf, SpecialFolder>> = OnceLock::new();

/// Builds the path-to-SpecialFolder map once for the current system.
fn init_known_folders_map() -> HashMap<PathBuf, SpecialFolder> {
    let mut map = HashMap::new();

    #[cfg(windows)]
    {
        let pairs: &[(&GUID, SpecialFolder)] = &[
            (&FOLDERID_Profile, SpecialFolder::Home),
            (&FOLDERID_Desktop, SpecialFolder::Desktop),
            (&FOLDERID_Documents, SpecialFolder::Documents),
            (&FOLDERID_Downloads, SpecialFolder::Downloads),
            (&FOLDERID_Pictures, SpecialFolder::Pictures),
            (&FOLDERID_Music, SpecialFolder::Music),
            (&FOLDERID_Videos, SpecialFolder::Videos),
            (&FOLDERID_Favorites, SpecialFolder::Favorites),
            (&FOLDERID_SavedGames, SpecialFolder::SavedGames),
            (&FOLDERID_Links, SpecialFolder::Links),
            (&FOLDERID_Contacts, SpecialFolder::Contacts),
            (&FOLDERID_SEARCHES, SpecialFolder::Searches),
            (&FOLDERID_Objects3D, SpecialFolder::Objects3D),
            (&FOLDERID_ProgramFiles, SpecialFolder::ProgramFiles),
            (&FOLDERID_UserProfiles, SpecialFolder::Users),
        ];

        for &(guid, kind) in pairs {
            if let Some(path) = call_known_folder(guid) {
                map.insert(normalize_path(&path), kind);
            }
        }

        // Program Files (x86)
        if let Ok(pf86) = std::env::var("ProgramFiles(x86)") {
            map.insert(
                normalize_path(Path::new(&pf86)),
                SpecialFolder::ProgramFiles,
            );
        }

        // OneDrive from environment variables
        for env_var in &["OneDrive", "OneDriveConsumer", "OneDriveCommercial"] {
            if let Ok(od) = std::env::var(env_var) {
                if !od.trim().is_empty() {
                    map.insert(normalize_path(Path::new(&od)), SpecialFolder::OneDrive);
                }
            }
        }
    }

    #[cfg(not(windows))]
    {
        let home = crate::core::platform_paths::home_dir();
        map.insert(normalize_path(&home), SpecialFolder::Home);
        map.insert(
            normalize_path(&home.join("Desktop")),
            SpecialFolder::Desktop,
        );
        map.insert(
            normalize_path(&home.join("Documents")),
            SpecialFolder::Documents,
        );
        map.insert(
            normalize_path(&home.join("Downloads")),
            SpecialFolder::Downloads,
        );
        map.insert(
            normalize_path(&home.join("Pictures")),
            SpecialFolder::Pictures,
        );
        map.insert(normalize_path(&home.join("Music")), SpecialFolder::Music);
        map.insert(normalize_path(&home.join("Videos")), SpecialFolder::Videos);
        map.insert(
            normalize_path(&home.join(".local/share/Trash")),
            SpecialFolder::RecycleBin,
        );
    }

    map
}

/// Look up a path against known system folders (exact path match).
pub fn lookup_path(path: &Path) -> Option<SpecialFolder> {
    // 1. Direct match in cached known folders map
    let map = KNOWN_FOLDERS_MAP.get_or_init(init_known_folders_map);
    let norm = normalize_path(path);
    if let Some(&sf) = map.get(&norm) {
        return Some(sf);
    }

    // 2. Special handling for Recycle Bin on any drive (e.g. C:\$Recycle.Bin)
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        let lower = name.to_lowercase();
        if lower == "$recycle.bin" || lower == "recycle.bin" || lower == "trash" {
            return Some(SpecialFolder::RecycleBin);
        }
    }

    None
}

/// Fallback lookup based on folder name conventions.
pub fn lookup_name(name: &str) -> Option<SpecialFolder> {
    let lower = name.trim().to_lowercase();
    let matches = |k: &str| lower == k || lower.starts_with(k);

    if lower == "$recycle.bin"
        || lower == "recycle.bin"
        || lower == "recycle bin"
        || lower == "trash"
    {
        Some(SpecialFolder::RecycleBin)
    } else if matches("onedrive") {
        Some(SpecialFolder::OneDrive)
    } else if matches("desktop") {
        Some(SpecialFolder::Desktop)
    } else if matches("downloads") || matches("download") {
        Some(SpecialFolder::Downloads)
    } else if matches("documents") || matches("document") || matches("my documents") {
        Some(SpecialFolder::Documents)
    } else if matches("pictures") || matches("photos") || matches("wallpaper") {
        Some(SpecialFolder::Pictures)
    } else if matches("videos") || matches("movies") {
        Some(SpecialFolder::Videos)
    } else if matches("music") || matches("audio") {
        Some(SpecialFolder::Music)
    } else if matches("home") || matches("profile") {
        Some(SpecialFolder::Home)
    } else if matches("favorites") {
        Some(SpecialFolder::Favorites)
    } else if matches("saved games") || matches("savedgames") {
        Some(SpecialFolder::SavedGames)
    } else if matches("links") {
        Some(SpecialFolder::Links)
    } else if matches("contacts") {
        Some(SpecialFolder::Contacts)
    } else if matches("searches") {
        Some(SpecialFolder::Searches)
    } else if matches("3d objects") || matches("objects3d") || matches("3d models") {
        Some(SpecialFolder::Objects3D)
    } else if matches("program files") {
        Some(SpecialFolder::ProgramFiles)
    } else if matches("users") {
        Some(SpecialFolder::Users)
    } else {
        None
    }
}

/// Complete resolver: exact path match first, then name fallback.
pub fn lookup(name: &str, path: Option<&Path>) -> Option<SpecialFolder> {
    if let Some(p) = path {
        if let Some(sf) = lookup_path(p) {
            return Some(sf);
        }
    }
    if name.contains('\\') || name.contains('/') {
        if let Some(sf) = lookup_path(Path::new(name)) {
            return Some(sf);
        }
    }
    lookup_name(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_folder_path_lookup() {
        let home = crate::core::platform_paths::home_dir();
        assert_eq!(lookup_path(&home), Some(SpecialFolder::Home));

        let desktop = crate::core::platform_paths::desktop_dir();
        assert_eq!(lookup_path(&desktop), Some(SpecialFolder::Desktop));

        let downloads = crate::core::platform_paths::downloads_dir();
        assert_eq!(lookup_path(&downloads), Some(SpecialFolder::Downloads));

        let docs = crate::core::platform_paths::documents_dir();
        assert_eq!(lookup_path(&docs), Some(SpecialFolder::Documents));

        let pics = crate::core::platform_paths::pictures_dir();
        assert_eq!(lookup_path(&pics), Some(SpecialFolder::Pictures));

        let music = crate::core::platform_paths::music_dir();
        assert_eq!(lookup_path(&music), Some(SpecialFolder::Music));

        let videos = crate::core::platform_paths::videos_dir();
        assert_eq!(lookup_path(&videos), Some(SpecialFolder::Videos));

        // Recycle bin path
        assert_eq!(
            lookup_path(Path::new("C:\\$Recycle.Bin")),
            Some(SpecialFolder::RecycleBin)
        );

        // Unknown path returns None
        assert_eq!(lookup_path(Path::new("C:\\UnmappedRandomDir12345")), None);
    }

    #[test]
    fn test_folder_name_fallback() {
        assert_eq!(lookup_name("Desktop"), Some(SpecialFolder::Desktop));
        assert_eq!(lookup_name("desktop"), Some(SpecialFolder::Desktop));
        assert_eq!(lookup_name("Downloads"), Some(SpecialFolder::Downloads));
        assert_eq!(lookup_name("download"), Some(SpecialFolder::Downloads));
        assert_eq!(lookup_name("Documents"), Some(SpecialFolder::Documents));
        assert_eq!(lookup_name("my documents"), Some(SpecialFolder::Documents));
        assert_eq!(lookup_name("Pictures"), Some(SpecialFolder::Pictures));
        assert_eq!(lookup_name("wallpaper"), Some(SpecialFolder::Pictures));
        assert_eq!(lookup_name("Music"), Some(SpecialFolder::Music));
        assert_eq!(lookup_name("audio"), Some(SpecialFolder::Music));
        assert_eq!(lookup_name("Videos"), Some(SpecialFolder::Videos));
        assert_eq!(lookup_name("movies"), Some(SpecialFolder::Videos));
        assert_eq!(lookup_name("Home"), Some(SpecialFolder::Home));
        assert_eq!(lookup_name("profile"), Some(SpecialFolder::Home));
        assert_eq!(lookup_name("OneDrive"), Some(SpecialFolder::OneDrive));
        assert_eq!(
            lookup_name("OneDrive - Personal"),
            Some(SpecialFolder::OneDrive)
        );
        assert_eq!(lookup_name("$Recycle.Bin"), Some(SpecialFolder::RecycleBin));
        assert_eq!(lookup_name("Recycle Bin"), Some(SpecialFolder::RecycleBin));
        assert_eq!(lookup_name("Trash"), Some(SpecialFolder::RecycleBin));
        assert_eq!(lookup_name("Favorites"), Some(SpecialFolder::Favorites));
        assert_eq!(lookup_name("Saved Games"), Some(SpecialFolder::SavedGames));
        assert_eq!(lookup_name("savedgames"), Some(SpecialFolder::SavedGames));
        assert_eq!(lookup_name("Links"), Some(SpecialFolder::Links));
        assert_eq!(lookup_name("Contacts"), Some(SpecialFolder::Contacts));
        assert_eq!(lookup_name("Searches"), Some(SpecialFolder::Searches));
        assert_eq!(lookup_name("3D Objects"), Some(SpecialFolder::Objects3D));
        assert_eq!(lookup_name("objects3d"), Some(SpecialFolder::Objects3D));
        assert_eq!(
            lookup_name("Program Files"),
            Some(SpecialFolder::ProgramFiles)
        );
        assert_eq!(lookup_name("Users"), Some(SpecialFolder::Users));

        // Non-special names return None
        assert_eq!(lookup_name("custom_project"), None);
        assert_eq!(lookup_name("build"), None);
    }

    #[test]
    fn test_lookup_combined() {
        let home = crate::core::platform_paths::home_dir();
        // Path match takes priority
        assert_eq!(
            lookup("arbitrary_name", Some(&home)),
            Some(SpecialFolder::Home)
        );

        // Name fallback when path is None or unmapped
        assert_eq!(lookup("Downloads", None), Some(SpecialFolder::Downloads));
        assert_eq!(lookup("random_dir", None), None);
    }
}
