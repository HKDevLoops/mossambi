//! Custom Folder Icon Registry, CDN/Local Path Resolution, & Presets (Professional & Memes)
//!
//! Pure ASCII. Persists custom folder associations in ~/.mossambi/folder_icons.json.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// Category and identity of a custom folder icon assignment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FolderIconCustomization {
    /// Standard professional icon preset.
    Professional(String),
    /// Funky computer meme icon preset.
    Meme(String),
    /// Local image file path on disk (PNG, ICO, SVG, JPG).
    LocalPath(PathBuf),
    /// CDN or remote web image URL.
    CdnUrl(String),
}

/// Metadata definition of an icon preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IconPresetDef {
    pub id: &'static str,
    pub label: &'static str,
    pub emoji_glyph: &'static str,
    pub accent_rgb: [u8; 3],
    pub description: &'static str,
}

impl IconPresetDef {
    /// Returns the raw official Microsoft Fluent SVG for this preset.
    pub fn raw_fluent_svg(&self) -> Option<&'static str> {
        crate::core::fluent_icons::get_raw_fluent_svg(self.emoji_glyph, false)
    }

    /// Returns the colorized Fluent SVG using the preset's accent color.
    pub fn colorized_svg(&self) -> Option<String> {
        let raw = self.raw_fluent_svg()?;
        let hex = format!(
            "#{:02X}{:02X}{:02X}",
            self.accent_rgb[0], self.accent_rgb[1], self.accent_rgb[2]
        );
        Some(crate::core::fluent_icons::colorize_fluent_svg(raw, &hex))
    }
}

pub const PROFESSIONAL_PRESETS: &[IconPresetDef] = &[
    IconPresetDef {
        id: "vault",
        label: "Secure Vault",
        emoji_glyph: "lock_closed",
        accent_rgb: [234, 179, 8],
        description: "Encrypted or sensitive documents",
    },
    IconPresetDef {
        id: "code",
        label: "Dev Workspace",
        emoji_glyph: "window_console",
        accent_rgb: [59, 130, 246],
        description: "Git repositories and source code",
    },
    IconPresetDef {
        id: "terminal",
        label: "CLI & Scripts",
        emoji_glyph: "window_console",
        accent_rgb: [34, 197, 94],
        description: "Scripts, binaries, and devops tooling",
    },
    IconPresetDef {
        id: "media",
        label: "Studio Media",
        emoji_glyph: "video",
        accent_rgb: [168, 85, 247],
        description: "Render assets and video exports",
    },
    IconPresetDef {
        id: "music",
        label: "Audio Masters",
        emoji_glyph: "music_note_2",
        accent_rgb: [244, 63, 94],
        description: "FLAC, stems, and album productions",
    },
    IconPresetDef {
        id: "camera",
        label: "RAW Photos",
        emoji_glyph: "image",
        accent_rgb: [14, 165, 233],
        description: "High-resolution photo shoots",
    },
    IconPresetDef {
        id: "cloud",
        label: "Cloud Mirror",
        emoji_glyph: "cloud",
        accent_rgb: [6, 182, 212],
        description: "Synced network or cloud share",
    },
    IconPresetDef {
        id: "server",
        label: "Infra & Servers",
        emoji_glyph: "hard_drive",
        accent_rgb: [99, 102, 241],
        description: "Docker, Kubernetes, and server mounts",
    },
    IconPresetDef {
        id: "star",
        label: "Starred / VIP",
        emoji_glyph: "star",
        accent_rgb: [245, 158, 11],
        description: "Top priority pinned projects",
    },
    IconPresetDef {
        id: "briefcase",
        label: "Work & Legal",
        emoji_glyph: "document",
        accent_rgb: [100, 116, 139],
        description: "Contracts, invoices, and business docs",
    },
    IconPresetDef {
        id: "package",
        label: "Archive Storage",
        emoji_glyph: "folder_zip",
        accent_rgb: [217, 119, 6],
        description: "Cold compressed backups and tarballs",
    },
    IconPresetDef {
        id: "sparkles",
        label: "Creative Lab",
        emoji_glyph: "color",
        accent_rgb: [236, 72, 153],
        description: "Experiments, designs, and prototypes",
    },
];

pub const MEME_PRESETS: &[IconPresetDef] = &[
    IconPresetDef {
        id: "doge",
        label: "Doge (Much Folder)",
        emoji_glyph: "star",
        accent_rgb: [229, 169, 59],
        description: "Very file. Much space. Wow.",
    },
    IconPresetDef {
        id: "pepe",
        label: "Rare Pepe Vault",
        emoji_glyph: "eye",
        accent_rgb: [74, 222, 128],
        description: "Feels good man - ultra rare collection",
    },
    IconPresetDef {
        id: "this_is_fine",
        label: "This Is Fine",
        emoji_glyph: "filter",
        accent_rgb: [239, 68, 68],
        description: "Directory is on fire but everything is okay",
    },
    IconPresetDef {
        id: "gigachad",
        label: "Gigachad Folder",
        emoji_glyph: "shield_checkmark",
        accent_rgb: [148, 163, 184],
        description: "Average file storage enjoyer",
    },
    IconPresetDef {
        id: "clippy",
        label: "Clippy Assistant",
        emoji_glyph: "pin",
        accent_rgb: [56, 189, 248],
        description: "It looks like you're browsing directories!",
    },
    IconPresetDef {
        id: "tux",
        label: "Tux the Penguin",
        emoji_glyph: "navigation",
        accent_rgb: [251, 191, 36],
        description: "Official Linux mascot folder",
    },
    IconPresetDef {
        id: "bsod",
        label: "BSOD Crash Dump",
        emoji_glyph: "window",
        accent_rgb: [37, 99, 235],
        description: ":( Your PC ran into a problem",
    },
    IconPresetDef {
        id: "stonks",
        label: "Stonks Only Up",
        emoji_glyph: "arrow_up",
        accent_rgb: [16, 185, 129],
        description: "Financial returns and asset folder",
    },
    IconPresetDef {
        id: "rickroll",
        label: "Rick Astley Roll",
        emoji_glyph: "music_note_2",
        accent_rgb: [244, 63, 94],
        description: "Never gonna give you up",
    },
    IconPresetDef {
        id: "galaxy_brain",
        label: "Galaxy Brain",
        emoji_glyph: "settings",
        accent_rgb: [139, 92, 246],
        description: "Ascended 4D multi-threaded storage",
    },
    IconPresetDef {
        id: "keyboard_cat",
        label: "Keyboard Cat",
        emoji_glyph: "keyboard",
        accent_rgb: [251, 146, 60],
        description: "Play him off, Keyboard Cat!",
    },
];

static REGISTRY: OnceLock<Mutex<HashMap<PathBuf, FolderIconCustomization>>> = OnceLock::new();

fn registry_lock() -> &'static Mutex<HashMap<PathBuf, FolderIconCustomization>> {
    REGISTRY.get_or_init(|| {
        let loaded = load_registry_from_disk();
        Mutex::new(loaded)
    })
}

fn config_path() -> PathBuf {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".mossambi")
        .join("folder_icons.json")
}

fn load_registry_from_disk() -> HashMap<PathBuf, FolderIconCustomization> {
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        if let Ok(map) = serde_json::from_str::<HashMap<PathBuf, FolderIconCustomization>>(&content)
        {
            return map;
        }
    }
    HashMap::new()
}

fn save_registry_to_disk(map: &HashMap<PathBuf, FolderIconCustomization>) {
    let path = config_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(map) {
        let _ = std::fs::write(&path, json);
    }
}

/// Retrieves any custom folder icon assigned to `path`.
pub fn get_custom_folder_icon(path: &Path) -> Option<FolderIconCustomization> {
    let guard = registry_lock().lock().ok()?;
    guard.get(path).cloned()
}

/// Assigns a custom icon to `path` and persists the association.
pub fn set_custom_folder_icon(path: impl AsRef<Path>, custom: FolderIconCustomization) {
    if let Ok(mut guard) = registry_lock().lock() {
        guard.insert(path.as_ref().to_path_buf(), custom);
        save_registry_to_disk(&guard);
    }
}

/// Removes any custom icon from `path`, reverting to standard icon pack defaults.
pub fn clear_custom_folder_icon(path: &Path) {
    if let Ok(mut guard) = registry_lock().lock() {
        guard.remove(path);
        save_registry_to_disk(&guard);
    }
}

/// Finds the preset definition by ID among professional or meme presets.
pub fn find_preset(id: &str) -> Option<&'static IconPresetDef> {
    PROFESSIONAL_PRESETS
        .iter()
        .chain(MEME_PRESETS.iter())
        .find(|p| p.id.eq_ignore_ascii_case(id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_presets_exist_and_unique() {
        assert!(PROFESSIONAL_PRESETS.len() >= 10);
        assert!(MEME_PRESETS.len() >= 10);
        let doge = find_preset("doge").expect("Doge preset must exist");
        assert_eq!(doge.label, "Doge (Much Folder)");
        assert_eq!(doge.emoji_glyph, "star");
        assert!(doge.raw_fluent_svg().is_some());

        let vault = find_preset("vault").expect("Vault preset must exist");
        assert_eq!(vault.label, "Secure Vault");
        assert_eq!(vault.emoji_glyph, "lock_closed");
        assert!(vault.raw_fluent_svg().is_some());

        // Ensure zero bracketed placeholders in presets and valid SVG resolution
        for preset in PROFESSIONAL_PRESETS.iter().chain(MEME_PRESETS.iter()) {
            assert!(
                !preset.emoji_glyph.contains('['),
                "Preset {} glyph must not contain brackets: {}",
                preset.id,
                preset.emoji_glyph
            );
            assert!(
                preset.raw_fluent_svg().is_some(),
                "Preset {} glyph '{}' must resolve to Fluent SVG",
                preset.id,
                preset.emoji_glyph
            );
            assert!(
                preset.colorized_svg().is_some(),
                "Preset {} must colorize",
                preset.id
            );
        }
    }

    #[test]
    fn test_set_and_get_custom_icon() {
        let test_dir = std::env::temp_dir().join("mossambi_custom_icon_test");
        let _ = std::fs::create_dir_all(&test_dir);

        set_custom_folder_icon(
            test_dir.clone(),
            FolderIconCustomization::Meme("doge".to_string()),
        );
        let custom = get_custom_folder_icon(&test_dir);
        assert_eq!(
            custom,
            Some(FolderIconCustomization::Meme("doge".to_string()))
        );

        clear_custom_folder_icon(&test_dir);
        assert_eq!(get_custom_folder_icon(&test_dir), None);
        let _ = std::fs::remove_dir_all(&test_dir);
    }
}
