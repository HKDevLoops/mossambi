//! Multi-Platform Vector Icon Engine (Source-of-truth: embedded SVG packs)
//!
//! Every icon pack is a hand-authored SVG template rendered at exact physical
//! pixels through `resvg` (antialiased vector rasterizer) into egui textures —
//! eliminating the blurry small emoji glyphs entirely.
//!
//! Packs (as demanded): macOS Finder, Windows 11 / 10 / 8 / 7 / XP, Fluent,
//! GNOME, Adwaita, KDE Breeze, Cinnamon Mint, Material, Android and a
//! Hyprland-inspired custom pack.
//!
//! Design crafting rules:
//! - Each pack carries a full visual language (panel colors, gradients, corner
//!   radius, tab style, gloss, outline, paper tone) so packs are *distinct*,
//!   not recolors.
//! - Per-file accent colors come from the existing category engine so type
//!   colors stay consistent across every pack.
//! - Textures are cached per (pack, glyph, physical pixel size); rendering is
//!   done once per size with full antialiasing and LINEAR filtering.

use crate::core::SpecialFolder;
use eframe::egui::{Color32, ColorImage, Pos2, Rect, TextureHandle, TextureOptions, Ui, Vec2};
use std::collections::HashMap;
use std::sync::Mutex;

// ──────────────────────────────────────────────────────────────────────────
// Icon pack taxonomy
// ──────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IconPack {
    MacosFinder,
    Windows11,
    Windows10,
    Windows8,
    Windows7,
    WindowsXp,
    Fluent,
    Gnome,
    Adwaita,
    Kde,
    Cinnamon,
    Material,
    Android,
    Hyprland,
}

impl IconPack {
    /// Every pack, in settings-menu order
    pub fn all() -> [IconPack; 14] {
        [
            Self::MacosFinder,
            Self::Windows11,
            Self::Windows10,
            Self::Windows8,
            Self::Windows7,
            Self::WindowsXp,
            Self::Fluent,
            Self::Gnome,
            Self::Adwaita,
            Self::Kde,
            Self::Cinnamon,
            Self::Material,
            Self::Android,
            Self::Hyprland,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::MacosFinder => "macOS Finder (Sonoma)",
            Self::Windows11 => "Windows 11",
            Self::Windows10 => "Windows 10",
            Self::Windows8 => "Windows 8",
            Self::Windows7 => "Windows 7",
            Self::WindowsXp => "Windows XP",
            Self::Fluent => "Fluent (Microsoft)",
            Self::Gnome => "Linux GNOME",
            Self::Adwaita => "GNOME Adwaita",
            Self::Kde => "KDE Breeze",
            Self::Cinnamon => "Cinnamon Mint",
            Self::Material => "Material (Google)",
            Self::Android => "Android",
            Self::Hyprland => "Hyprland Custom",
        }
    }

    /// Stable config string (persisted)
    pub fn id(&self) -> &'static str {
        match self {
            Self::MacosFinder => "macos",
            Self::Windows11 => "win11",
            Self::Windows10 => "win10",
            Self::Windows8 => "win8",
            Self::Windows7 => "win7",
            Self::WindowsXp => "winxp",
            Self::Fluent => "fluent",
            Self::Gnome => "gnome",
            Self::Adwaita => "adwaita",
            Self::Kde => "kde",
            Self::Cinnamon => "cinnamon",
            Self::Material => "material",
            Self::Android => "android",
            Self::Hyprland => "hyprland",
        }
    }

    pub fn from_config_id(s: &str) -> IconPack {
        match s {
            "win11" => Self::Windows11,
            "win10" => Self::Windows10,
            "win8" => Self::Windows8,
            "win7" => Self::Windows7,
            "winxp" => Self::WindowsXp,
            "fluent" => Self::Fluent,
            "gnome" => Self::Gnome,
            "adwaita" => Self::Adwaita,
            "kde" => Self::Kde,
            "cinnamon" => Self::Cinnamon,
            "material" => Self::Material,
            "android" => Self::Android,
            "hyprland" => Self::Hyprland,
            _ => Self::MacosFinder,
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────
// Glyph taxonomy: every drawable icon shape
// ──────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IconGlyph {
    // Folders
    Folder,
    FolderSpecial(crate::core::SpecialFolder),
    FolderDesktop,
    FolderDownloads,
    FolderDocuments,
    FolderPictures,
    FolderVideos,
    FolderMusic,
    FolderRepo,
    FolderCode,
    FolderHome,
    FolderOneDrive,
    FolderRecycleBin,
    FolderFavorites,
    FolderSavedGames,
    FolderLinks,
    FolderContacts,
    FolderSearches,
    FolderObjects3D,
    FolderProgramFiles,
    FolderUsers,
    // System places
    Home,
    RecycleBin,
    Drive,
    DriveSystem,
    Cloud,
    Search,
    Gear,
    Power,
    Git,
    // Documents on paper
    PageDefault,
    PageText,
    PageDoc,
    PageSheet,
    PageSlide,
    PagePdf,
    PageImage,
    PageVideo,
    PageAudio,
    PageCode,
    PageConfig,
    PageArchive,
    PageExe,
    PageLibrary,
    // ── Action glyphs (context menus, toolbars, buttons) ──
    ActionOpen,
    ActionEdit,
    ActionDelete,
    ActionCopy,
    ActionCut,
    ActionRename,
    ActionProperties,
    ActionTerminal,
    ActionPrint,
    ActionMedia,
    ActionShield,
    ActionRefresh,
    ActionSelectAll,
    ActionPaste,
    ActionNewFolder,
    ActionWallpaper,
    ActionRotate,
    ActionExtract,
    ActionShortcut,
    ActionPath,
    ActionSort,
    ActionView,
    ActionFilter,
    ActionMore,
    ActionCheck,
    ActionEye,
    ActionFolderOpen,
    ActionCompress,
    ActionPin,
    ActionMinimize,
    ActionMaximize,
    ActionRestore,
    ActionClose,
    ActionPlus,
    ActionBack,
    ActionForward,
    ActionUp,
    ActionList,
    ActionColumns,
    ActionSun,
    ActionMoon,
    ActionPlay,
    ActionHamburger,
    ActionSidebarToggle,
    ActionNewWindow,
    ActionMenu,
    ActionSliders,
}

impl IconGlyph {
    /// Associated special folder kind if this glyph represents one.
    pub fn special_folder(&self) -> Option<crate::core::SpecialFolder> {
        match self {
            IconGlyph::FolderSpecial(k) => Some(*k),
            IconGlyph::FolderDesktop => Some(crate::core::SpecialFolder::Desktop),
            IconGlyph::FolderDownloads => Some(crate::core::SpecialFolder::Downloads),
            IconGlyph::FolderDocuments => Some(crate::core::SpecialFolder::Documents),
            IconGlyph::FolderPictures => Some(crate::core::SpecialFolder::Pictures),
            IconGlyph::FolderVideos => Some(crate::core::SpecialFolder::Videos),
            IconGlyph::FolderMusic => Some(crate::core::SpecialFolder::Music),
            IconGlyph::FolderHome | IconGlyph::Home => Some(crate::core::SpecialFolder::Home),
            IconGlyph::FolderOneDrive | IconGlyph::Cloud => {
                Some(crate::core::SpecialFolder::OneDrive)
            }
            IconGlyph::FolderRecycleBin | IconGlyph::RecycleBin => {
                Some(crate::core::SpecialFolder::RecycleBin)
            }
            IconGlyph::FolderFavorites => Some(crate::core::SpecialFolder::Favorites),
            IconGlyph::FolderSavedGames => Some(crate::core::SpecialFolder::SavedGames),
            IconGlyph::FolderLinks => Some(crate::core::SpecialFolder::Links),
            IconGlyph::FolderContacts => Some(crate::core::SpecialFolder::Contacts),
            IconGlyph::FolderSearches => Some(crate::core::SpecialFolder::Searches),
            IconGlyph::FolderObjects3D => Some(crate::core::SpecialFolder::Objects3D),
            IconGlyph::FolderProgramFiles => Some(crate::core::SpecialFolder::ProgramFiles),
            IconGlyph::FolderUsers => Some(crate::core::SpecialFolder::Users),
            _ => None,
        }
    }

    /// Creates an IconGlyph for a special folder kind.
    pub fn from_special_folder(kind: crate::core::SpecialFolder) -> IconGlyph {
        IconGlyph::FolderSpecial(kind)
    }

    /// Special folder names follow exact path match first, then name lookup
    pub fn folder_special(name: &str) -> Option<IconGlyph> {
        Self::folder_special_with_path(name, None)
    }

    fn folder_special_with_path(name: &str, path: Option<&std::path::Path>) -> Option<IconGlyph> {
        if let Some(kind) = crate::core::special_folders::lookup(name, path) {
            return Some(IconGlyph::FolderSpecial(kind));
        }
        Self::folder_name_lookup(name.to_lowercase().as_str())
    }

    fn folder_name_lookup(lower: &str) -> Option<IconGlyph> {
        // Use starts-with buckets so variants like "Documents 2" still match
        let matches = |k: &str| lower == k || lower.starts_with(k);
        if matches(".git") {
            Some(IconGlyph::FolderRepo)
        } else if matches("src")
            || matches("crate")
            || matches("crates")
            || matches("pkg")
            || matches("code")
        {
            Some(IconGlyph::FolderCode)
        } else {
            crate::core::special_folders::lookup_name(lower).map(IconGlyph::FolderSpecial)
        }
    }

    /// Map a resolved FileIconSpec (category flags + rgb) to our glyph + accent
    #[allow(clippy::too_many_arguments)]
    pub fn resolve(
        name: &str,
        extension: &str,
        is_dir: bool,
        is_image: bool,
        is_media: bool,
        is_archive: bool,
        is_document: bool,
        is_code: bool,
        color_rgb: [u8; 3],
    ) -> (IconGlyph, [u8; 3]) {
        Self::resolve_with_path(
            None,
            name,
            extension,
            is_dir,
            is_image,
            is_media,
            is_archive,
            is_document,
            is_code,
            color_rgb,
        )
    }

    /// Map with explicit path for exact known-folder matching.
    #[allow(clippy::too_many_arguments)]
    pub fn resolve_with_path(
        path: Option<&std::path::Path>,
        name: &str,
        extension: &str,
        is_dir: bool,
        is_image: bool,
        is_media: bool,
        is_archive: bool,
        is_document: bool,
        is_code: bool,
        color_rgb: [u8; 3],
    ) -> (IconGlyph, [u8; 3]) {
        if is_dir {
            if let Some(p) = path {
                if let Some(custom) = crate::core::folder_icons::get_custom_folder_icon(p) {
                    match custom {
                        crate::core::folder_icons::FolderIconCustomization::Professional(
                            ref id,
                        )
                        | crate::core::folder_icons::FolderIconCustomization::Meme(ref id) => {
                            if let Some(def) = crate::core::folder_icons::find_preset(id) {
                                return (IconGlyph::Folder, def.accent_rgb);
                            }
                        }
                        crate::core::folder_icons::FolderIconCustomization::LocalPath(_)
                        | crate::core::folder_icons::FolderIconCustomization::CdnUrl(_) => {
                            return (IconGlyph::Folder, [59, 130, 246]);
                        }
                    }
                }
            }
            if let Some(sf) = crate::core::special_folders::lookup(name, path) {
                return (IconGlyph::FolderSpecial(sf), sf.default_color());
            }
            return match Self::folder_special_with_path(name, path) {
                Some(g) => (g, color_rgb),
                None => (IconGlyph::Folder, color_rgb),
            };
        }
        let ext = extension.to_ascii_uppercase();

        let glyph = if ext == "PDF" {
            IconGlyph::PagePdf
        } else if ext == "DOC" || ext == "DOCX" || ext == "ODT" || ext == "RTF" {
            IconGlyph::PageDoc
        } else if ext == "XLS" || ext == "XLSX" || ext == "CSV" || ext == "ODS" {
            IconGlyph::PageSheet
        } else if ext == "PPT" || ext == "PPTX" || ext == "ODP" {
            IconGlyph::PageSlide
        } else if is_image {
            IconGlyph::PageImage
        } else if is_media && matches!(ext.as_str(), "MP4" | "MKV" | "MOV" | "AVI" | "WEBM" | "M4V")
        {
            IconGlyph::PageVideo
        } else if is_media {
            IconGlyph::PageAudio
        } else if is_archive {
            IconGlyph::PageArchive
        } else if matches!(ext.as_str(), "EXE" | "MSI" | "APP" | "BIN" | "APK") {
            IconGlyph::PageExe
        } else if matches!(ext.as_str(), "DLL" | "SO" | "DYLIB" | "LIB" | "PYD" | "OCX") {
            IconGlyph::PageLibrary
        } else if is_code
            && matches!(
                ext.as_str(),
                "JSON" | "TOML" | "YAML" | "YML" | "XML" | "INI"
            )
        {
            IconGlyph::PageConfig
        } else if is_code {
            IconGlyph::PageCode
        } else if is_document || matches!(ext.as_str(), "TXT" | "LOG" | "MD" | "MARKDOWN" | "CFG") {
            IconGlyph::PageText
        } else {
            IconGlyph::PageDefault
        };
        (glyph, color_rgb)
    }
}

// ──────────────────────────────────────────────────────────────────────────
// Pack visual language
// ──────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TabStyle {
    /// No tab; folder mouth sits on top edge (Material/Fluent/Android style)
    Flush,
    /// Distinct top-left tab (classic Finder/Win10/XP)
    Tab,
    /// GNOME/Adwaita detached small tab
    Notch,
}

struct PackStyle {
    /// Back panel of the folder
    back: &'static str,
    /// Front flap of the folder
    front: &'static str,
    /// Tab color (usually mirrors the back panel)
    tab: &'static str,
    /// Optional outline stroke color
    outline: Option<&'static str>,
    /// Corner radius on the 100×100 viewBox
    radius: f32,
    tab_style: TabStyle,
    /// Draw glossy shine over the folder front
    gloss: bool,
    /// File page body color
    page: &'static str,
    /// File page border stroke
    page_edge: &'static str,
    /// Folded corner triangle color
    fold: &'static str,
    /// Neutral document-line color
    line: &'static str,
}

impl IconPack {
    fn style(&self) -> PackStyle {
        match self {
            Self::MacosFinder => PackStyle {
                back: "#4B9AF4",
                front: "#6FBBFF",
                tab: "#3E8BE0",
                outline: None,
                radius: 9.0,
                tab_style: TabStyle::Tab,
                gloss: true,
                page: "#FFFFFF",
                page_edge: "#DAE3F0",
                fold: "#D5E4F7",
                line: "#7D8AA5",
            },
            Self::Windows11 => PackStyle {
                back: "#FFA726",
                front: "#FFD54F",
                tab: "#FB8C00",
                outline: None,
                radius: 8.0,
                tab_style: TabStyle::Tab,
                gloss: true,
                page: "#FFFFFF",
                page_edge: "#E2D9C8",
                fold: "#F4E7CE",
                line: "#9AA0A6",
            },
            Self::Windows10 => PackStyle {
                back: "#FFB300",
                front: "#FFE082",
                tab: "#FFA000",
                outline: None,
                radius: 3.0,
                tab_style: TabStyle::Tab,
                gloss: true,
                page: "#FFFFFF",
                page_edge: "#DCD5C6",
                fold: "#EFE6D2",
                line: "#8B97A8",
            },
            Self::Windows8 => PackStyle {
                back: "#FFC107",
                front: "#FFE082",
                tab: "#FFB300",
                outline: None,
                radius: 0.5,
                tab_style: TabStyle::Tab,
                gloss: false,
                page: "#F8F8F8",
                page_edge: "#C9C9C9",
                fold: "#E0E0E0",
                line: "#767676",
            },
            Self::Windows7 => PackStyle {
                back: "#FFB300",
                front: "#FFE082",
                tab: "#FFA000",
                outline: Some("#FF8F00"),
                radius: 6.0,
                tab_style: TabStyle::Tab,
                gloss: true,
                page: "#FFFFFF",
                page_edge: "#D8CBAF",
                fold: "#EFE3C6",
                line: "#95805A",
            },
            Self::WindowsXp => PackStyle {
                back: "#FFB300",
                front: "#FFE082",
                tab: "#FFA000",
                outline: Some("#FF8F00"),
                radius: 2.0,
                tab_style: TabStyle::Tab,
                gloss: true,
                page: "#FFFFFF",
                page_edge: "#8A7B55",
                fold: "#E3D9B9",
                line: "#6E6A4F",
            },
            Self::Fluent => PackStyle {
                back: "#FFA000",
                front: "#FFD54F",
                tab: "#FF8F00",
                outline: None,
                radius: 9.0,
                tab_style: TabStyle::Flush,
                gloss: true,
                page: "#FFFFFF",
                page_edge: "#E5D8C0",
                fold: "#FFF0CC",
                line: "#9E8F76",
            },
            Self::Gnome => PackStyle {
                back: "#C8BFAE",
                front: "#EDE6D8",
                tab: "#B4AA98",
                outline: None,
                radius: 7.0,
                tab_style: TabStyle::Notch,
                gloss: false,
                page: "#FFFFFF",
                page_edge: "#D3CBBE",
                fold: "#E7E0D2",
                line: "#948B7C",
            },
            Self::Adwaita => PackStyle {
                back: "#B5B0BA",
                front: "#D6D2DA",
                tab: "#9E99A5",
                outline: None,
                radius: 4.0,
                tab_style: TabStyle::Notch,
                gloss: false,
                page: "#FBF9F8",
                page_edge: "#C0BCC3",
                fold: "#E8E6EA",
                line: "#77767E",
            },
            Self::Kde => PackStyle {
                back: "#2D8FD8",
                front: "#55B1F4",
                tab: "#1E71AE",
                outline: None,
                radius: 3.0,
                tab_style: TabStyle::Tab,
                gloss: true,
                page: "#FCFDFE",
                page_edge: "#C4D6E4",
                fold: "#DFEDF8",
                line: "#5E6F7E",
            },
            Self::Cinnamon => PackStyle {
                back: "#3F9C5C",
                front: "#5FC77F",
                tab: "#2F7C48",
                outline: None,
                radius: 5.0,
                tab_style: TabStyle::Tab,
                gloss: true,
                page: "#FFFFFF",
                page_edge: "#CBE2D0",
                fold: "#E1F2E6",
                line: "#6B7E6F",
            },
            Self::Material => PackStyle {
                back: "#FFA000",
                front: "#FFCA28",
                tab: "#E08E00",
                outline: None,
                radius: 12.0,
                tab_style: TabStyle::Flush,
                gloss: false,
                page: "#FFFFFF",
                page_edge: "#E0E0E0",
                fold: "#FFE9A8",
                line: "#9E9E9E",
            },
            Self::Android => PackStyle {
                back: "#1267DE",
                front: "#2E90FA",
                tab: "#0E4FA8",
                outline: None,
                radius: 8.0,
                tab_style: TabStyle::Flush,
                gloss: false,
                page: "#FFFFFF",
                page_edge: "#D2DEEB",
                fold: "#CFE4FF",
                line: "#66788E",
            },
            Self::Hyprland => PackStyle {
                back: "#22253B",
                front: "#31364F",
                tab: "#191C2C",
                outline: Some("#7BD4F5"),
                radius: 6.0,
                tab_style: TabStyle::Notch,
                gloss: false,
                page: "#3A405C",
                page_edge: "#7BD4F5",
                fold: "#22253B",
                line: "#9DB8CC",
            },
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────
// SVG template builders
// ──────────────────────────────────────────────────────────────────────────

/// Default folder tint colors per SpecialFolder (front, back, tab).
fn type_colors(kind: SpecialFolder) -> (&'static str, &'static str, &'static str) {
    match kind {
        SpecialFolder::Desktop => ("#20B2AA", "#168B85", "#1B9C95"),
        SpecialFolder::Documents => ("#2B88D8", "#1E6BB8", "#2479C8"),
        SpecialFolder::Downloads => ("#28A745", "#1E7E34", "#23923C"),
        SpecialFolder::Music => ("#F7630C", "#D83B01", "#E75006"),
        SpecialFolder::Pictures => ("#0078D7", "#005A9E", "#0068B8"),
        SpecialFolder::Videos => ("#881798", "#681174", "#771485"),
        SpecialFolder::Home => ("#0078D4", "#005A9E", "#0068B8"),
        SpecialFolder::OneDrive => ("#0078D4", "#005A9E", "#0068B8"),
        SpecialFolder::Favorites => ("#EAA300", "#C68500", "#D89400"),
        SpecialFolder::SavedGames => ("#7C3AED", "#5B21B6", "#6D28D9"),
        SpecialFolder::Links => ("#4F46E5", "#3730A3", "#4338CA"),
        SpecialFolder::Contacts => ("#DB2777", "#9D174D", "#BE185D"),
        SpecialFolder::Searches => ("#475569", "#334155", "#3E4C5F"),
        SpecialFolder::Objects3D => ("#0891B2", "#0E7490", "#0B82A1"),
        SpecialFolder::ProgramFiles => ("#2563EB", "#1D4ED8", "#2159E5"),
        SpecialFolder::Users => ("#475569", "#334155", "#3E4C5F"),
        SpecialFolder::RecycleBin => ("#64748B", "#475569", "#56667C"),
    }
}

/// Computes the folder PackStyle and emblem foreground / emboss colors for a pack + kind.
fn special_folder_styling(
    pack: IconPack,
    kind: SpecialFolder,
) -> (PackStyle, &'static str, Option<&'static str>) {
    let mut st = pack.style();
    let (t_front, t_back, t_tab) = type_colors(kind);

    match pack {
        // Win11 / Win10 / Fluent: colored per type + white emblem
        IconPack::Windows11 | IconPack::Windows10 | IconPack::Fluent => {
            st.front = t_front;
            st.back = t_back;
            st.tab = t_tab;
            (st, "#FFFFFF", None)
        }
        // macOS: blue folder with darker embossed emblem (#1B5EAA + #BAE6FD highlight)
        IconPack::MacosFinder => (st, "#1B5EAA", Some("#BAE6FD")),
        // Gnome / Adwaita / Cinnamon: pack folder + symbolic emblem (#2C3E50)
        IconPack::Gnome | IconPack::Adwaita | IconPack::Cinnamon => (st, "#2C3E50", None),
        // KDE: blue folder + white emblem (#FFFFFF)
        IconPack::Kde => (st, "#FFFFFF", None),
        // Material / Android: per-type folder color + white emblem
        IconPack::Material | IconPack::Android => {
            st.front = t_front;
            st.back = t_back;
            st.tab = t_tab;
            (st, "#FFFFFF", None)
        }
        // Win7 / XP / 8: pack folder (classic yellow) + colored emblem
        IconPack::Windows7 | IconPack::WindowsXp | IconPack::Windows8 => (st, t_front, None),
        // Hyprland: dark body + cyan outline emblem (#7BD4F5)
        IconPack::Hyprland => (st, "#7BD4F5", None),
    }
}

/// Raw vector geometry for each kind of special folder emblem.
fn raw_emblem(kind: SpecialFolder, is_small: bool, f: &str) -> String {
    match kind {
        SpecialFolder::Home => {
            let sw = if is_small { "4.6" } else { "3.0" };
            format!(
                "<path d='M33 55 L50 41 L67 55' stroke='{f}' stroke-width='{sw}' fill='none' stroke-linecap='round' stroke-linejoin='round'/>\
                 <path d='M38 53 L38 73 L62 73 L62 53' stroke='{f}' stroke-width='{sw}' fill='none' stroke-linecap='round' stroke-linejoin='round'/>\
                 <rect x='46' y='63' width='8' height='10' fill='{f}'/>"
            )
        }
        SpecialFolder::Desktop => {
            if is_small {
                format!(
                    "<rect x='33' y='44' width='34' height='21' rx='2.5' fill='none' stroke='{f}' stroke-width='4.5'/>\
                     <rect x='41' y='69' width='18' height='3.5' rx='1.5' fill='{f}'/>\
                     <rect x='48' y='64' width='4' height='5' fill='{f}'/>"
                )
            } else {
                format!(
                    "<rect x='33' y='43' width='34' height='22' rx='2.5' fill='none' stroke='{f}' stroke-width='2.8'/>\
                     <rect x='42' y='69' width='16' height='2.5' rx='1.2' fill='{f}'/>\
                     <rect x='48' y='65' width='4' height='4' fill='{f}'/>"
                )
            }
        }
        SpecialFolder::Documents => {
            if is_small {
                format!(
                    "<path d='M36 43 L56 43 L64 51 L64 73 L36 73 Z' fill='none' stroke='{f}' stroke-width='4.2' stroke-linejoin='round'/>\
                     <path d='M56 43 L56 51 L64 51' fill='none' stroke='{f}' stroke-width='4.2' stroke-linejoin='round'/>\
                     <rect x='42' y='57' width='16' height='3.5' rx='1.7' fill='{f}'/>\
                     <rect x='42' y='64' width='11' height='3.5' rx='1.7' fill='{f}'/>"
                )
            } else {
                format!(
                    "<path d='M36 43 L56 43 L64 51 L64 73 L36 73 Z' fill='none' stroke='{f}' stroke-width='2.6' stroke-linejoin='round'/>\
                     <path d='M56 43 L56 51 L64 51' fill='none' stroke='{f}' stroke-width='2.6' stroke-linejoin='round'/>\
                     <rect x='41' y='55' width='18' height='2.4' rx='1.2' fill='{f}'/>\
                     <rect x='41' y='60.5' width='18' height='2.4' rx='1.2' fill='{f}'/>\
                     <rect x='41' y='66' width='11' height='2.4' rx='1.2' fill='{f}'/>"
                )
            }
        }
        SpecialFolder::Downloads => {
            let sw = if is_small { "4.8" } else { "3.2" };
            format!(
                "<path d='M50 43 L50 64 M41 55 L50 64 L59 55' stroke='{f}' stroke-width='{sw}' fill='none' stroke-linecap='round' stroke-linejoin='round'/>\
                 <path d='M35 63 L35 72 L65 72 L65 63' stroke='{f}' stroke-width='{sw}' fill='none' stroke-linecap='round' stroke-linejoin='round'/>"
            )
        }
        SpecialFolder::Pictures => {
            if is_small {
                format!(
                    "<rect x='33' y='45' width='34' height='24' rx='3' fill='none' stroke='{f}' stroke-width='4.2'/>\
                     <circle cx='41' cy='52' r='3.2' fill='{f}'/>\
                     <path d='M35 67 L48 55 L65 67' stroke='{f}' stroke-width='4.2' fill='none' stroke-linecap='round' stroke-linejoin='round'/>"
                )
            } else {
                format!(
                    "<rect x='33' y='45' width='34' height='24' rx='3' fill='none' stroke='{f}' stroke-width='2.6'/>\
                     <circle cx='41' cy='52' r='2.8' fill='{f}'/>\
                     <path d='M35 67 L44 56 L50 62 L56 54 L65 65' stroke='{f}' stroke-width='2.4' fill='none' stroke-linecap='round' stroke-linejoin='round'/>"
                )
            }
        }
        SpecialFolder::Music => {
            if is_small {
                format!(
                    "<path d='M46 65 L46 45 M62 61 L62 41' stroke='{f}' stroke-width='4.5' stroke-linecap='round'/>\
                     <path d='M44 46 L64 42 L64 47 L44 51 Z' fill='{f}'/>\
                     <ellipse cx='42' cy='66' rx='4.8' ry='3.6' fill='{f}'/>\
                     <ellipse cx='58' cy='62' rx='4.8' ry='3.6' fill='{f}'/>"
                )
            } else {
                format!(
                    "<path d='M45.5 66 L45.5 46 M61.5 62 L61.5 42' stroke='{f}' stroke-width='2.8' stroke-linecap='round'/>\
                     <path d='M44 47 L63 43 L63 48 L44 52 Z' fill='{f}'/>\
                     <ellipse cx='42' cy='66' rx='4.2' ry='3.2' fill='{f}'/>\
                     <ellipse cx='58' cy='62' rx='4.2' ry='3.2' fill='{f}'/>"
                )
            }
        }
        SpecialFolder::Videos => {
            let sw = if is_small { "4.4" } else { "2.8" };
            format!(
                "<rect x='33' y='45' width='34' height='24' rx='3.5' fill='none' stroke='{f}' stroke-width='{sw}'/>\
                 <path d='M47 51 L47 63 L57 57 Z' fill='{f}'/>"
            )
        }
        SpecialFolder::Favorites => {
            let sw = if is_small { "1.5" } else { "0" };
            format!(
                "<path d='M50 43 L53.8 52.8 L63.3 53.7 L56.1 60.1 L58.2 69.5 L50 64.7 L41.8 69.5 L43.9 60.1 L36.7 53.7 L46.2 52.8 Z' fill='{f}' stroke='{f}' stroke-width='{sw}' stroke-linejoin='round'/>"
            )
        }
        SpecialFolder::SavedGames => {
            let sw = if is_small { "4.2" } else { "2.8" };
            let bsw = if is_small { "3.6" } else { "2.4" };
            let br = if is_small { "2.4" } else { "1.8" };
            format!(
                "<path d='M36 50 C33 50 31 53 31 57 C31 64 35 73 39 73 C42 73 44 68 46 68 L54 68 C56 68 58 73 61 73 C65 73 69 64 69 57 C69 53 67 50 64 50 Z' fill='none' stroke='{f}' stroke-width='{sw}' stroke-linejoin='round'/>\
                 <path d='M38 59 L44 59 M41 56 L41 62' stroke='{f}' stroke-width='{bsw}' stroke-linecap='round'/>\
                 <circle cx='59' cy='57' r='{br}' fill='{f}'/><circle cx='63' cy='61' r='{br}' fill='{f}'/>"
            )
        }
        SpecialFolder::Links => {
            let sw = if is_small { "4.6" } else { "3.0" };
            format!(
                "<rect x='34' y='52' width='18' height='10' rx='5' transform='rotate(-30 43 57)' fill='none' stroke='{f}' stroke-width='{sw}'/>\
                 <rect x='48' y='52' width='18' height='10' rx='5' transform='rotate(-30 57 57)' fill='none' stroke='{f}' stroke-width='{sw}'/>"
            )
        }
        SpecialFolder::Contacts => {
            let sw = if is_small { "4.5" } else { "2.8" };
            format!(
                "<circle cx='50' cy='49' r='6' fill='none' stroke='{f}' stroke-width='{sw}'/>\
                 <path d='M37 72 C37 62 43 59 50 59 C57 59 63 62 63 72' fill='none' stroke='{f}' stroke-width='{sw}' stroke-linecap='round'/>"
            )
        }
        SpecialFolder::Searches => {
            let (csw, hsw) = if is_small {
                ("4.8", "6.0")
            } else {
                ("3.2", "4.5")
            };
            format!(
                "<circle cx='46' cy='53' r='10' fill='none' stroke='{f}' stroke-width='{csw}'/>\
                 <path d='M54 61 L65 72' stroke='{f}' stroke-width='{hsw}' stroke-linecap='round'/>"
            )
        }
        SpecialFolder::Objects3D => {
            let sw = if is_small { "4.2" } else { "2.6" };
            format!(
                "<path d='M50 43 L65 50 L50 57 L35 50 Z' fill='none' stroke='{f}' stroke-width='{sw}' stroke-linejoin='round'/>\
                 <path d='M35 50 L35 67 L50 74 L50 57 Z' fill='none' stroke='{f}' stroke-width='{sw}' stroke-linejoin='round'/>\
                 <path d='M65 50 L65 67 L50 74 L50 57 Z' fill='none' stroke='{f}' stroke-width='{sw}' stroke-linejoin='round'/>"
            )
        }
        SpecialFolder::OneDrive => {
            if is_small {
                format!(
                    "<path d='M36 68 C30 68 28 62 31 57 C31 49 39 46 45 49 C48 44 58 44 61 49 C67 48 71 54 69 60 C71 63 70 68 64 68 Z' fill='{f}'/>"
                )
            } else {
                format!(
                    "<path d='M36 68 C30 68 28 62 31 57 C31 49 39 46 45 49 C48 44 58 44 61 49 C67 48 71 54 69 60 C71 63 70 68 64 68 Z' fill='none' stroke='{f}' stroke-width='3.0' stroke-linejoin='round'/>"
                )
            }
        }
        SpecialFolder::ProgramFiles => {
            let sw = if is_small { "4.2" } else { "2.8" };
            format!(
                "<circle cx='50' cy='58' r='5.5' fill='none' stroke='{f}' stroke-width='{sw}'/>\
                 <circle cx='50' cy='58' r='11' fill='none' stroke='{f}' stroke-width='{sw}' stroke-dasharray='4.6 4.1'/>"
            )
        }
        SpecialFolder::Users => {
            let sw = if is_small { "4.2" } else { "2.6" };
            format!(
                "<circle cx='46' cy='51' r='5' fill='none' stroke='{f}' stroke-width='{sw}'/>\
                 <path d='M36 71 C36 63 41 60 46 60 C51 60 56 63 56 71' fill='none' stroke='{f}' stroke-width='{sw}' stroke-linecap='round'/>\
                 <circle cx='58' cy='48' r='4' fill='none' stroke='{f}' stroke-width='{sw}' opacity='0.75'/>\
                 <path d='M56 60 C59 58 63 58 65 60 C68 63 68 66 68 70' fill='none' stroke='{f}' stroke-width='{sw}' stroke-linecap='round' opacity='0.75'/>"
            )
        }
        SpecialFolder::RecycleBin => {
            let sw = if is_small { "4.2" } else { "2.6" };
            format!(
                "<rect x='37' y='46' width='26' height='3.2' rx='1.6' fill='{f}'/>\
                 <path d='M46 46 L46 43 L54 43 L54 46' stroke='{f}' stroke-width='{sw}' fill='none' stroke-linecap='round'/>\
                 <path d='M39 51 L41 72 C41 74 43 75 45 75 L55 75 C57 75 59 74 59 72 L61 51 Z' fill='none' stroke='{f}' stroke-width='{sw}' stroke-linejoin='round'/>\
                 <line x1='46' y1='56' x2='46' y2='70' stroke='{f}' stroke-width='{sw}' stroke-linecap='round'/>\
                 <line x1='54' y1='56' x2='54' y2='70' stroke='{f}' stroke-width='{sw}' stroke-linecap='round'/>"
            )
        }
    }
}

/// Generates the complete emblem SVG fragment, with optional macOS emboss highlight.
fn emblem_path_data(
    kind: SpecialFolder,
    is_small: bool,
    fg: &str,
    emboss_hl: Option<&str>,
) -> String {
    let fg_art = raw_emblem(kind, is_small, fg);
    if let Some(hl) = emboss_hl {
        let hl_art = raw_emblem(kind, is_small, hl);
        format!("<g transform='translate(0, 0.9)' opacity='0.75'>{hl_art}</g><g>{fg_art}</g>")
    } else {
        format!("<g>{fg_art}</g>")
    }
}

/// Full 100x100 SVG for a special folder in the requested pack and size mode.
pub fn special_folder_svg_sized(pack: IconPack, kind: SpecialFolder, is_small: bool) -> String {
    let (st, fg, emboss_hl) = special_folder_styling(pack, kind);
    let head =
        "<svg xmlns='http://www.w3.org/2000/svg' width='100' height='100' viewBox='0 0 100 100'>\n";
    let shapes = folder_shapes(&st);
    let gloss = if st.gloss {
        gloss_overlay(36.0, 64.0)
    } else {
        String::new()
    };
    let emblem = emblem_path_data(kind, is_small, fg, emboss_hl);
    format!("{head}{shapes}{gloss}{emblem}</svg>")
}

/// Full 100x100 SVG for a special folder in the requested pack (default size).
pub fn special_folder_svg(pack: IconPack, kind: SpecialFolder) -> String {
    special_folder_svg_sized(pack, kind, false)
}

/// "paper stack" emblem inside special folders (documents/documents style)
fn emblem_for(glyph: IconGlyph, fg: &str, fg_soft: &str) -> Option<String> {
    let _ = fg_soft;
    Some(match glyph {
        IconGlyph::FolderDesktop => format!(
            "<rect x='40' y='52' width='22' height='14' rx='1.5' fill='{f}'/><rect x='45' y='66' width='12' height='2.4' rx='1' fill='{f}'/>",
            f = fg
        ),
        IconGlyph::FolderDownloads => format!(
            "<path d='M50 50 L50 66 M43.5 59.5 L50 66 L56.5 59.5' stroke='{f}' stroke-width='3.2' stroke-linecap='round' fill='none'/>",
            f = fg
        ),
        IconGlyph::FolderDocuments => format!(
            "<rect x='42' y='50' width='17.5' height='2.6' rx='1.3' fill='{f}'/><rect x='42' y='56.4' width='17.5' height='2.6' rx='1.3' fill='{f}'/><rect x='42' y='62.8' width='11' height='2.6' rx='1.3' fill='{f}'/>",
            f = fg
        ),
        IconGlyph::FolderPictures => format!(
            "<rect x='39' y='50' width='22' height='17' rx='2' fill='none' stroke='{f}' stroke-width='2.4'/><circle cx='46' cy='56.4' r='2.1' fill='{f}'/><path d='M41 66 L48.5 58.5 L52 62 L56 56.5 L60.5 62.9 ' stroke='{f}' stroke-width='2.2' fill='none' stroke-linejoin='round'/>",
            f = fg
        ),
        IconGlyph::FolderVideos => format!(
            "<path d='M42.5 50.5 L42.5 66.5 L58.5 58.5 Z' fill='{f}'/>",
            f = fg
        ),
        IconGlyph::FolderMusic => format!(
            "<path d='M53 49.5 L53 64.5' stroke='{f}' stroke-width='2.6' stroke-linecap='round'/><circle cx='49.4' cy='65.4' r='3.4' fill='{f}'/><circle cx='56.6' cy='62.4' r='2.6' fill='{f}'/>",
            f = fg
        ),
        IconGlyph::FolderRepo => format!(
            "<circle cx='44' cy='53.5' r='3' fill='{f}'/><circle cx='56' cy='53.5' r='3' fill='{f}'/><circle cx='44' cy='65.5' r='3' fill='{f}'/><path d='M44 56.5 L44 62.5 M53 53.5 L56 53.5' stroke='{f}' stroke-width='2.4' stroke-linecap='round'/>",
            f = fg
        ),
        IconGlyph::FolderCode => format!(
            "<path d='M46.5 53.5 L41.5 58.5 L46.5 63.5 M54.5 53.5 L59.5 58.5 L54.5 63.5' stroke='{f}' stroke-width='2.8' fill='none' stroke-linecap='round' stroke-linejoin='round'/>",
            f = fg
        ),
        _ => return None,
    })
}

/// Parses a #RRGGBB color into its three channel bytes.
fn parse_hex(c: &str) -> Option<[u8; 3]> {
    let h = c.strip_prefix('#')?;
    if h.len() != 6 || !h.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    Some([
        u8::from_str_radix(&h[0..2], 16).ok()?,
        u8::from_str_radix(&h[2..4], 16).ok()?,
        u8::from_str_radix(&h[4..6], 16).ok()?,
    ])
}

/// Blends two #RRGGBB colors per channel: t = 0 keeps `a`, t = 1 keeps
/// `b`. Malformed input returns `a` unchanged.
pub fn mix_hex(a: &str, b: &str, t: f32) -> String {
    let blend = |x: u8, y: u8| {
        ((x as f32) + ((y as f32) - (x as f32)) * t)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    match (parse_hex(a), parse_hex(b)) {
        (Some(pa), Some(pb)) => format!(
            "#{:02X}{:02X}{:02X}",
            blend(pa[0], pb[0]),
            blend(pa[1], pb[1]),
            blend(pa[2], pb[2])
        ),
        _ => a.to_string(),
    }
}

/// Scales each channel of a #RRGGBB color by `f`. Malformed input
/// returns the input unchanged.
pub fn shade_hex(a: &str, f: f32) -> String {
    match parse_hex(a) {
        Some(p) => {
            let s = |x: u8| ((x as f32) * f).round().clamp(0.0, 255.0) as u8;
            format!("#{:02X}{:02X}{:02X}", s(p[0]), s(p[1]), s(p[2]))
        }
        None => a.to_string(),
    }
}

/// Specular top edge plus a faint bottom inner-edge catch, inset from a
/// front flap (x, y, width, height) so corner rounding never clips.
fn glass_layers(fx: f32, fy: f32, fw: f32, fh: f32) -> String {
    format!(
        "<rect x='{sx:.1}' y='{sy:.1}' width='{sw:.1}' height='2.2' rx='1.1' fill='#FFFFFF' opacity='0.35'/> \
         <rect x='{sx:.1}' y='{by:.1}' width='{sw:.1}' height='1.5' rx='0.75' fill='#FFFFFF' opacity='0.14'/>",
        sx = fx + 3.0,
        sy = fy + 2.2,
        by = fy + fh - 3.7,
        sw = fw - 6.0
    )
}

/// Folder body shapes per pack tab style.
///
/// Golden Gate glass anatomy: a back wall, a front flap carrying a
/// vertical light gradient (`ggfront`), a specular highlight along the
/// front's top edge and a faint bottom catch. Tab anchors are unchanged
/// so every pack keeps its recognisable silhouette.
fn folder_shapes(st: &PackStyle) -> String {
    let r = st.radius;
    let out = match st.outline {
        Some(c) => format!(
            " stroke='{}' stroke-width='2.2' paint-order='stroke fill'",
            c
        ),
        None => String::new(),
    };
    let ggtop = mix_hex(st.front, "#FFFFFF", 0.45);
    let ggbot = shade_hex(st.front, 0.94);
    let defs = format!(
        "<defs><linearGradient id='ggfront' x1='0' y1='0' x2='0' y2='1'> \
         <stop offset='0' stop-color='{top}'/> \
         <stop offset='0.5' stop-color='{mid}'/> \
         <stop offset='1' stop-color='{bot}'/> \
         </linearGradient></defs>",
        top = ggtop,
        mid = st.front,
        bot = ggbot
    );
    let (back, tab) = (st.back, st.tab);
    match st.tab_style {
        // Material / Fluent / Android: open-mouth top band, full-width front
        TabStyle::Flush => format!(
            "{defs} \
             <rect x='8' y='20' width='84' height='16' rx='{r}' fill='{back}'{out}/> \
             <rect x='8' y='30' width='84' height='52' rx='{r}' fill='url(#ggfront)'{out}/> \
             {glass}",
            defs = defs,
            r = r,
            back = back,
            out = out,
            glass = glass_layers(8.0, 30.0, 84.0, 52.0)
        ),
        // Classic Finder / Win7 / XP style: distinct tab on top-left
        TabStyle::Tab => format!(
            "{defs} \
             <rect x='8' y='12' width='44' height='16' rx='{tr}' fill='{tab}'{out_x}/> \
             <rect x='8' y='22' width='84' height='60' rx='{r}' fill='{back}'{out}/> \
             <rect x='8' y='34' width='84' height='48' rx='{r}' fill='url(#ggfront)'{out}/> \
             {glass}",
            defs = defs,
            tr = (r * 0.7).max(3.0),
            tab = tab,
            out_x = out,
            r = r,
            back = back,
            out = out,
            glass = glass_layers(8.0, 34.0, 84.0, 48.0)
        ),
        // GNOME / Adwaita / Hyprland: detached short notch
        TabStyle::Notch => format!(
            "{defs} \
             <rect x='18' y='13' width='40' height='10' rx='5' fill='{tab}'{out_x}/> \
             <rect x='8' y='20' width='84' height='60' rx='{r}' fill='{back}'{out}/> \
             <rect x='8' y='33' width='84' height='47' rx='{r}' fill='url(#ggfront)'{out}/> \
             {glass}",
            defs = defs,
            tab = tab,
            out_x = out,
            r = r,
            back = back,
            out = out,
            glass = glass_layers(8.0, 33.0, 84.0, 47.0)
        ),
    }
}

fn gloss_overlay(y_top: f32, y_bot: f32) -> String {
    format!(
        "<path d='M8 {:.0} L92 {:.0} L92 {:.0} L8 {:.0} Z' fill='#FFFFFF' opacity='0.12'/>",
        y_top, y_top, y_bot, y_bot
    )
}

/// Polygon-fold emblem for the file page (integer-friendly small shapes)
fn page_emblem(glyph: IconGlyph, st: &PackStyle, accent: &str) -> Option<String> {
    let a = accent;
    let ln = st.line;
    Some(match glyph {
        IconGlyph::PageText | IconGlyph::PageDoc => format!(
            "<rect x='31' y='42' width='38' height='3.4' rx='1.7' fill='{l}'/> \
             <rect x='31' y='50' width='38' height='3.4' rx='1.7' fill='{l}'/> \
             <rect x='31' y='58' width='26' height='3.4' rx='1.7' fill='{l}'/> \
             <rect x='26' y='69' width='48' height='2.6' rx='1.3' fill='{a}' opacity='0.55'/>",
            l = ln,
            a = a
        ),
        IconGlyph::PageSheet => format!(
            "<rect x='29' y='40' width='42' height='30' rx='2.4' fill='none' stroke='{l}' stroke-width='2.6'/> \
             <path d='M29 50 L71 50 M29 60 L71 60 M51 40 L51 70' stroke='{l}' stroke-width='2' opacity='0.75'/> \
             <circle cx='60' cy='55' r='6.5' fill='{a}' opacity='0.85'/>",
            l = ln,
            a = a
        ),
        IconGlyph::PageSlide => format!(
            "<rect x='27' y='42' width='46' height='28' rx='3' fill='none' stroke='{l}' stroke-width='2.6'/> \
             <path d='M43 50 L43 62 L54 56 Z' fill='{a}'/>",
            l = ln,
            a = a
        ),
        IconGlyph::PagePdf => format!(
            "<rect x='26' y='44' width='48' height='13' rx='3' fill='{a}'/> \
             <rect x='26' y='44' width='48' height='4' rx='2' fill='#FFFFFF' opacity='0.28'/> \
             <rect x='34' y='60' width='22' height='2.8' rx='1.4' fill='{l}' opacity='0.55'/>",
            a = a,
            l = ln
        ),
        IconGlyph::PageImage => format!(
            "<rect x='28' y='41' width='44' height='30' rx='3' fill='{w}'/> \
             <circle cx='38.5' cy='49.5' r='3.4' fill='{a}'/> \
             <path d='M30 68 L43 55 L50 62 L57 53 L68 66 L68 68 Z' fill='{a}' opacity='0.85'/>",
            w = if st.page == "#FFFFFF" { "#F1F3F6" } else { st.page },
            a = a
        ),
        IconGlyph::PageVideo => format!(
            "<rect x='28' y='42' width='44' height='27' rx='4' fill='{w}'/> \
             <path d='M44 49 L58 55.5 L44 62 Z' fill='{a}'/>",
            w = if st.page == "#FFFFFF" { "#F1F3F6" } else { st.page },
            a = a
        ),
        IconGlyph::PageAudio => format!(
            "<path d='M46 66 L46 40 L54 44 L54 70' stroke='{a}' stroke-width='3.2' fill='none' stroke-linejoin='round' stroke-linecap='round'/> \
             <ellipse cx='42.4' cy='66.5' rx='4.8' ry='3.6' fill='{a}'/> \
             <ellipse cx='50.4' cy='70.5' rx='4.8' ry='3.6' fill='{a}'/>",
            a = a
        ),
        IconGlyph::PageCode => format!(
            "<path d='M39 45 L30 55.5 L39 66 M61 45 L70 55.5 L61 66' stroke='{a}' stroke-width='4' fill='none' stroke-linecap='round' stroke-linejoin='round'/> \
             <path d='M53 42 L47 70' stroke='{l}' stroke-width='3.2' stroke-linecap='round'/>",
            a = a,
            l = ln
        ),
        IconGlyph::PageConfig => {
            let teeth: String = (0..8)
                .map(|i| {
                    let ang = i as f32 * std::f32::consts::FRAC_PI_4;
                    let (x1, y1) = (50.0 + 12.0 * ang.cos(), 55.5 + 12.0 * ang.sin());
                    let (x2, y2) = (50.0 + 17.0 * ang.cos(), 55.5 + 17.0 * ang.sin());
                    format!(
                        "<path d='M{x1:.1} {y1:.1} L{x2:.1} {y2:.1}' stroke='{l}' stroke-width='4.4' stroke-linecap='round'/>",
                        x1 = x1,
                        y1 = y1,
                        x2 = x2,
                        y2 = y2,
                        l = ln
                    )
                })
                .collect::<Vec<_>>()
                .join(" ");
            format!(
                "{teeth} <circle cx='50' cy='55.5' r='9.5' fill='none' stroke='{l}' stroke-width='3.4'/> \
                 <circle cx='50' cy='55.5' r='3' fill='{a}'/>",
                teeth = teeth,
                l = ln,
                a = a
            )
        }
        IconGlyph::PageArchive => format!(
            "<rect x='44.6' y='30' width='10.8' height='46' rx='2.4' fill='{a}' opacity='0.22'/> \
             <rect x='44.6' y='33' width='10.8' height='3' fill='{a}'/> \
             <rect x='44.6' y='40' width='10.8' height='3' fill='{a}'/> \
             <rect x='44.6' y='47' width='10.8' height='3' fill='{a}'/> \
             <rect x='44.6' y='54' width='10.8' height='3' fill='{a}'/> \
             <rect x='42' y='62' width='16' height='7' rx='2' fill='{a}'/>",
            a = a
        ),
        IconGlyph::PageExe => format!(
            "<rect x='28' y='40' width='44' height='32' rx='4' fill='{w}'/> \
             <rect x='28' y='40' width='44' height='9' rx='4' fill='{a}'/> \
             <circle cx='33' cy='44.5' r='1.7' fill='#FFFFFF'/> \
             <circle cx='38' cy='44.5' r='1.7' fill='#FFFFFF'/> \
             <path d='M40 60 L47 52 L49 56 L52 51 L56 60' stroke='{l}' stroke-width='2.6' fill='none' stroke-linejoin='round'/>",
            w = if st.page == "#FFFFFF" { "#F1F3F6" } else { st.page },
            a = a,
            l = ln
        ),
        IconGlyph::PageLibrary => format!(
            "<circle cx='45' cy='54' r='10' fill='none' stroke='{l}' stroke-width='3' stroke-dasharray='5.5 4.4'/> \
             <circle cx='50' cy='54' r='7.5' fill='none' stroke='{a}' stroke-width='3'/>",
            l = ln,
            a = a
        ),
        _ => format!(
            "<rect x='34' y='46' width='32' height='2.9' rx='1.45' fill='{l}' opacity='0.7'/> \
             <rect x='34' y='55' width='32' height='2.9' rx='1.45' fill='{l}' opacity='0.7'/>",
            l = ln
        ),
    })
}

/// Full 100x100 SVG for a glyph in the requested pack
pub fn svg_for(pack: IconPack, glyph: IconGlyph, accent_rgb: [u8; 3]) -> String {
    let accent = format!(
        "#{:02X}{:02X}{:02X}",
        accent_rgb[0], accent_rgb[1], accent_rgb[2]
    );

    // Official Microsoft Fluent UI System Icons for toolbar, actions, and Fluent pack
    let is_action = matches!(
        glyph,
        IconGlyph::ActionBack
            | IconGlyph::ActionForward
            | IconGlyph::ActionUp
            | IconGlyph::ActionRefresh
            | IconGlyph::ActionNewFolder
            | IconGlyph::ActionOpen
            | IconGlyph::ActionFolderOpen
            | IconGlyph::ActionEdit
            | IconGlyph::ActionDelete
            | IconGlyph::ActionCopy
            | IconGlyph::ActionCut
            | IconGlyph::ActionRename
            | IconGlyph::ActionProperties
            | IconGlyph::ActionTerminal
            | IconGlyph::ActionCheck
            | IconGlyph::ActionSelectAll
            | IconGlyph::ActionPaste
            | IconGlyph::ActionEye
            | IconGlyph::ActionPin
            | IconGlyph::ActionShield
            | IconGlyph::ActionCompress
            | IconGlyph::ActionExtract
            | IconGlyph::ActionShortcut
            | IconGlyph::ActionSort
            | IconGlyph::ActionFilter
            | IconGlyph::ActionMore
            | IconGlyph::ActionColumns
            | IconGlyph::ActionList
    );

    if pack == IconPack::Fluent || is_action {
        if let Some(fluent_svg) = crate::core::fluent_icons::fluent_svg_for_glyph(
            glyph,
            pack == IconPack::Fluent,
            &accent,
        ) {
            return fluent_svg;
        }
    }

    if let Some(sf) = glyph.special_folder() {
        return special_folder_svg(pack, sf);
    }
    let st = pack.style();
    let head =
        "<svg xmlns='http://www.w3.org/2000/svg' width='100' height='100' viewBox='0 0 100 100'>\n";
    match glyph {
        IconGlyph::FolderSpecial(_)
        | IconGlyph::FolderDesktop
        | IconGlyph::FolderDownloads
        | IconGlyph::FolderDocuments
        | IconGlyph::FolderPictures
        | IconGlyph::FolderVideos
        | IconGlyph::FolderMusic
        | IconGlyph::FolderHome
        | IconGlyph::FolderOneDrive
        | IconGlyph::FolderRecycleBin
        | IconGlyph::FolderFavorites
        | IconGlyph::FolderSavedGames
        | IconGlyph::FolderLinks
        | IconGlyph::FolderContacts
        | IconGlyph::FolderSearches
        | IconGlyph::FolderObjects3D
        | IconGlyph::FolderProgramFiles
        | IconGlyph::FolderUsers => {
            if let Some(sf) = glyph.special_folder() {
                special_folder_svg(pack, sf)
            } else {
                special_folder_svg(pack, SpecialFolder::Home)
            }
        }

        // Folders
        IconGlyph::Folder | IconGlyph::FolderRepo | IconGlyph::FolderCode => {
            let shapes = folder_shapes(&st);
            // Neutral folder gets a subtle paper peek; repo/code get their emblem
            let emblem_final = if glyph == IconGlyph::Folder {
                "<rect x='30' y='44' width='34' height='2.4' rx='1.2' fill='#FFFFFF' opacity='0.32'/>".to_string()
            } else {
                emblem_for(glyph, "#FFFFFF", "#FFFFFF")
                    .map(|e| format!("<g>{}</g>", e))
                    .unwrap_or_default()
            };
            format!(
                "{head}{shapes}{gloss}{emblem_final}</svg>",
                head = head,
                shapes = shapes,
                gloss = if st.gloss {
                    gloss_overlay(36.0, 64.0)
                } else {
                    String::new()
                },
                emblem_final = emblem_final
            )
        }

        IconGlyph::Home => format!(
            "{head}<path d='M14 52 L50 24 L86 52' stroke='{a}' stroke-width='7' fill='none' stroke-linecap='round' stroke-linejoin='round'/>\
             <path d='M26 50 L26 76 Q26 79 29 79 L42 79 L42 62 Q42 59 45 59 L55 59 Q58 59 58 62 L58 79 L71 79 Q74 79 74 76 L74 50' fill='{a}' opacity='0.9'/></svg>",
            head = head,
            a = accent
        ),

        IconGlyph::RecycleBin => {
            let body = format!("#{:02X}{:02X}{:02X}", accent_rgb[0], accent_rgb[1], accent_rgb[2]);
            format!(
                "{head}<path d='M34 36 L66 36 L62.5 82 Q62.2 86 58 86 L42 86 Q37.8 86 37.5 82 Z' fill='{b}' opacity='0.9'/>\
                 <rect x='30' y='28' width='40' height='7' rx='3.5' fill='{b}'/>\
                 <rect x='43' y='22' width='14' height='5' rx='2.5' fill='{b}'/>\
                 <path d='M44 44 L44 78 M50 44 L50 78 M56 44 L56 78' stroke='{d}' stroke-width='2.6' opacity='0.55'/></svg>",
                head = head,
                b = body,
                d = if pack == IconPack::Hyprland { "#0B0E18" } else { "#FFFFFF" }
            )
        }

        IconGlyph::Drive | IconGlyph::DriveSystem => {
            let is_sys = glyph == IconGlyph::DriveSystem;
            // A proper 3D-perspective hard drive: angled top face, body, and platter slot
            let top_col = if is_sys { "#5B8DEF" } else { st.front };
            let body_col = if is_sys { "#3D6AD4" } else { st.back };
            let detail_col = if pack == IconPack::Hyprland { "#9DB8CC" } else { "#FFFFFF" };
            let outline_attr = match st.outline {
                Some(c) => format!("stroke='{}' stroke-width='2'", c),
                None => String::new(),
            };
            format!(
                "{head}\
                 <path d='M12 44 L50 30 L88 44 L88 62 Q88 68 50 74 Q12 68 12 62 Z' fill='{body}' {o}/>\
                 <path d='M12 44 Q12 50 50 56 Q88 50 88 44 Q88 38 50 44 Q12 38 12 44 Z' fill='{top}' opacity='0.95'/>\
                 <ellipse cx='50' cy='44' rx='28' ry='6' fill='none' stroke='{d}' stroke-width='2' opacity='0.3'/>\
                 <ellipse cx='50' cy='44' rx='10' ry='2.5' fill='{d}' opacity='0.4'/>\
                 <circle cx='74' cy='58' r='3.5' fill='{d}' opacity='0.8'/>\
                 <rect x='26' y='62' width='20' height='2.4' rx='1.2' fill='{d}' opacity='0.3'/></svg>",
                head = head,
                body = body_col,
                top = top_col,
                o = outline_attr,
                d = detail_col,
            )
        }

        IconGlyph::Cloud => format!(
            "{head}<path d='M30 68 Q18 68 18 56 Q18 45 30 45 Q32 32 46 32 Q58 32 61 43 Q74 42 74 55 Q74 68 62 68 Z' fill='{a}' opacity='0.9'/></svg>",
            head = head,
            a = accent
        ),

        IconGlyph::Search => format!(
            "{head}<circle cx='43' cy='43' r='15.5' fill='none' stroke='{a}' stroke-width='7' stroke-linecap='round'/><path d='M55 55 L74 74' stroke='{a}' stroke-width='7' stroke-linecap='round'/></svg>",
            head = head,
            a = accent
        ),

        IconGlyph::Gear => {
            let teeth: String = (0..8)
                .map(|i| {
                    let ang = i as f32 * std::f32::consts::FRAC_PI_4;
                    let (x1, y1) = (50.0 + 15.0 * ang.cos(), 50.0 + 15.0 * ang.sin());
                    let (x2, y2) = (50.0 + 25.0 * ang.cos(), 50.0 + 25.0 * ang.sin());
                    format!(
                        "<path d='M{x1:.1} {y1:.1} L{x2:.1} {y2:.1}' stroke='{a}' stroke-width='9' stroke-linecap='round'/>",
                        x1 = x1, y1 = y1, x2 = x2, y2 = y2, a = accent
                    )
                })
                .collect::<Vec<_>>()
                .join(" ");
            format!(
                "{head}{teeth}<circle cx='50' cy='50' r='15' fill='none' stroke='{a}' stroke-width='6' /><circle cx='50' cy='50' r='4.5' fill='{a}'/></svg>",
                head = head,
                teeth = teeth,
                a = accent
            )
        }

        IconGlyph::Power => format!(
            "{head}<path d='M50 12 L50 46' stroke='{a}' stroke-width='9' stroke-linecap='round'/>\
             <path d='M28 27 A30 30 0 1 0 72 27' fill='none' stroke='{a}' stroke-width='9' stroke-linecap='round'/></svg>",
            head = head,
            a = accent
        ),

        IconGlyph::Git => format!(
            "{head}<circle cx='37' cy='31' r='7.2' fill='{a}'/><circle cx='37' cy='69' r='7.2' fill='{a}'/><circle cx='63' cy='50' r='7.2' fill='{a}'/>\
             <path d='M37 38 L37 62 M37 38 C37 49 46 50 56 50 M63 43 C63 56 48 55 37 55' stroke='{a}' stroke-width='5' stroke-linecap='round' fill='none'/></svg>",
            head = head,
            a = accent
        ),

        // ── File pages ──
        IconGlyph::PageDefault | IconGlyph::PageText | IconGlyph::PageDoc | IconGlyph::PageSheet
        | IconGlyph::PageSlide | IconGlyph::PagePdf | IconGlyph::PageImage | IconGlyph::PageVideo
        | IconGlyph::PageAudio | IconGlyph::PageCode | IconGlyph::PageConfig
        | IconGlyph::PageArchive | IconGlyph::PageExe | IconGlyph::PageLibrary => {
            // Type-specific accent color for the bottom badge stripe
            let type_color = match glyph {
                IconGlyph::PageText | IconGlyph::PageDoc => "#3B82F6",
                IconGlyph::PageSheet => "#22C55E",
                IconGlyph::PageSlide => "#F59E0B",
                IconGlyph::PagePdf => "#EF4444",
                IconGlyph::PageImage => "#A855F7",
                IconGlyph::PageVideo => "#EC4899",
                IconGlyph::PageAudio => "#F97316",
                IconGlyph::PageCode => "#06B6D4",
                IconGlyph::PageConfig => "#6B7280",
                IconGlyph::PageArchive => "#D97706",
                IconGlyph::PageExe => "#10B981",
                IconGlyph::PageLibrary => "#8B5CF6",
                _ => &accent,
            };
            let em = page_emblem(glyph, &st, &accent).unwrap_or_else(|| {
                format!(
                    "<rect x='34' y='46' width='32' height='2.8' rx='1.4' fill='{l}' opacity='0.7'/> \
                     <rect x='34' y='53' width='32' height='2.8' rx='1.4' fill='{l}' opacity='0.7'/> \
                     <rect x='34' y='60' width='20' height='2.8' rx='1.4' fill='{a}'/>",
                    l = st.line,
                    a = accent,
                )
            });
            // Type badge stripe at bottom of page for visual file-type color coding
            let badge = format!(
                "<rect x='24' y='78' width='52' height='8' rx='3' fill='{tc}' opacity='0.85'/>",
                tc = type_color,
            );
            format!(
                "{head}<path d='M24 14 L62 14 L76 28 L76 82 Q76 86 72 86 L28 86 Q24 86 24 82 Z' fill='{p}' stroke='{e}' stroke-width='2'/>\
                 <path d='M62 14 L62 28 L76 28 Z' fill='{f}'/>\
                 {e2}{badge}</svg>",
                head = head,
                p = st.page,
                e = st.page_edge,
                f = st.fold,
                e2 = em,
                badge = badge,
            )
        }

        // ── Action glyphs ──────────────────────────────────────────────────
        // Monochrome line-art that reads correctly on every pack: the stroke
        // colour follows the pack's line colour with the accent as a tint.
        IconGlyph::ActionOpen => format!(
            "{head}<path d='M40 24 L68 50 L40 76 Z' fill='{a}'/></svg>",
            head = head,
            a = accent
        ),
        IconGlyph::ActionFolderOpen => format!(
            "{head}<path d='M16 32 L42 32 L50 42 L84 42 L84 74 L16 74 Z' fill='{a}' opacity='0.85'/>\
             <path d='M16 44 L90 44 L79 76 L20 76 Z' fill='{a}' opacity='0.55'/></svg>",
            head = head,
            a = accent
        ),
        IconGlyph::ActionEdit => format!(
            "{head}<path d='M70 18 L82 30 L40 72 L22 78 L28 60 Z' fill='{a}'/>\
             <path d='M64 24 L76 36' stroke='#FFFFFF' stroke-width='3' opacity='0.45'/></svg>",
            head = head,
            a = accent
        ),
        IconGlyph::ActionDelete => format!(
            "{head}<path d='M32 34 L68 34 L64 80 Q64 84 60 84 L40 84 Q36 84 36 80 Z' fill='{a}' opacity='0.9'/>\
             <rect x='27' y='25' width='46' height='8' rx='4' fill='{a}'/>\
             <rect x='41' y='17' width='18' height='7' rx='3.5' fill='{a}'/></svg>",
            head = head,
            a = accent
        ),
        IconGlyph::ActionCopy => format!(
            "{head}<rect x='22' y='20' width='40' height='52' rx='5' fill='none' stroke='{a}' stroke-width='6' opacity='0.7'/>\
             <rect x='38' y='34' width='40' height='50' rx='5' fill='none' stroke='{a}' stroke-width='6'/></svg>",
            head = head,
            a = accent
        ),
        IconGlyph::ActionCut => format!(
            "{head}<circle cx='32' cy='72' r='10' fill='none' stroke='{a}' stroke-width='6'/>\
             <circle cx='68' cy='72' r='10' fill='none' stroke='{a}' stroke-width='6'/>\
             <path d='M36 64 L62 22 M64 64 L38 22' stroke='{a}' stroke-width='6' stroke-linecap='round'/></svg>",
            head = head,
            a = accent
        ),
        IconGlyph::ActionRename => format!(
            "{head}<path d='M22 74 L31 42 L58 42 L67 74 Z' fill='none' stroke='{a}' stroke-width='6' stroke-linejoin='round'/>\
             <path d='M28 58 L61 58' stroke='{a}' stroke-width='6'/>\
             <rect x='74' y='26' width='6' height='48' rx='3' fill='{a}'/></svg>",
            head = head,
            a = accent
        ),
        IconGlyph::ActionProperties => format!(
            "{head}<circle cx='50' cy='50' r='32' fill='none' stroke='{a}' stroke-width='7'/>\
             <rect x='46' y='42' width='8' height='26' rx='4' fill='{a}'/>\
             <circle cx='50' cy='32' r='5' fill='{a}'/></svg>",
            head = head,
            a = accent
        ),
        IconGlyph::ActionTerminal => format!(
            "{head}<rect x='14' y='22' width='72' height='56' rx='7' fill='none' stroke='{a}' stroke-width='6'/>\
             <path d='M28 40 L42 50 L28 60' fill='none' stroke='{a}' stroke-width='6' stroke-linecap='round' stroke-linejoin='round'/>\
             <path d='M50 62 L72 62' stroke='{a}' stroke-width='6' stroke-linecap='round'/></svg>",
            head = head,
            a = accent
        ),
        IconGlyph::ActionPrint => format!(
            "{head}<rect x='30' y='16' width='40' height='22' rx='4' fill='none' stroke='{a}' stroke-width='6'/>\
             <rect x='16' y='38' width='68' height='30' rx='6' fill='none' stroke='{a}' stroke-width='6'/>\
             <rect x='30' y='60' width='40' height='26' rx='4' fill='{a}' opacity='0.85'/>\
             <circle cx='72' cy='48' r='4' fill='{a}'/></svg>",
            head = head,
            a = accent
        ),
        IconGlyph::ActionMedia => format!(
            "{head}<circle cx='50' cy='50' r='32' fill='none' stroke='{a}' stroke-width='7'/>\
             <path d='M42 34 L68 50 L42 66 Z' fill='{a}'/></svg>",
            head = head,
            a = accent
        ),
        IconGlyph::ActionShield => format!(
            "{head}<path d='M50 14 L80 26 L80 52 Q80 72 50 86 Q20 72 20 52 L20 26 Z' fill='none' stroke='{a}' stroke-width='7' stroke-linejoin='round'/>\
             <path d='M36 50 L46 60 L66 38' fill='none' stroke='{a}' stroke-width='7' stroke-linecap='round' stroke-linejoin='round'/></svg>",
            head = head,
            a = accent
        ),
        IconGlyph::ActionRefresh => format!(
            "{head}<path d='M76 44 A28 28 0 1 0 78 56' fill='none' stroke='{a}' stroke-width='7' stroke-linecap='round'/>\
             <path d='M78 24 L78 46 L58 44' fill='none' stroke='{a}' stroke-width='7' stroke-linecap='round' stroke-linejoin='round'/></svg>",
            head = head,
            a = accent
        ),
        IconGlyph::ActionSelectAll => format!(
            "{head}<rect x='18' y='18' width='64' height='64' rx='9' fill='none' stroke='{a}' stroke-width='6'/>\
             <path d='M32 50 L45 63 L70 36' fill='none' stroke='{a}' stroke-width='7' stroke-linecap='round' stroke-linejoin='round'/></svg>",
            head = head,
            a = accent
        ),
        IconGlyph::ActionCheck => format!(
            "{head}<path d='M24 52 L42 68 L78 30' fill='none' stroke='{a}' stroke-width='9' stroke-linecap='round' stroke-linejoin='round'/></svg>",
            head = head,
            a = accent
        ),
        IconGlyph::ActionPaste => format!(
            "{head}<rect x='22' y='20' width='56' height='64' rx='7' fill='none' stroke='{a}' stroke-width='6'/>\
             <rect x='36' y='12' width='28' height='14' rx='5' fill='{a}'/>\
             <rect x='34' y='48' width='32' height='6' rx='3' fill='{a}' opacity='0.75'/>\
             <rect x='34' y='62' width='22' height='6' rx='3' fill='{a}' opacity='0.55'/></svg>",
            head = head,
            a = accent
        ),
        IconGlyph::ActionNewFolder => format!(
            "{head}<path d='M16 30 L42 30 L50 40 L84 40 L84 76 L16 76 Z' fill='none' stroke='{a}' stroke-width='6' stroke-linejoin='round'/>\
             <path d='M62 52 L62 72 M52 62 L72 62' stroke='{a}' stroke-width='6' stroke-linecap='round'/></svg>",
            head = head,
            a = accent
        ),
        IconGlyph::ActionWallpaper => format!(
            "{head}<rect x='16' y='22' width='68' height='56' rx='7' fill='none' stroke='{a}' stroke-width='6'/>\
             <circle cx='36' cy='40' r='7' fill='{a}'/>\
             <path d='M22 70 L44 46 L60 62 L70 52 L82 70 Z' fill='{a}' opacity='0.8'/></svg>",
            head = head,
            a = accent
        ),
        IconGlyph::ActionRotate => format!(
            "{head}<path d='M76 42 A28 28 0 1 0 78 58' fill='none' stroke='{a}' stroke-width='7' stroke-linecap='round'/>\
             <path d='M78 20 L78 44 L56 42' fill='none' stroke='{a}' stroke-width='7' stroke-linecap='round' stroke-linejoin='round'/></svg>",
            head = head,
            a = accent
        ),
        IconGlyph::ActionExtract => format!(
            "{head}<rect x='18' y='46' width='64' height='38' rx='7' fill='none' stroke='{a}' stroke-width='6'/>\
             <path d='M50 14 L50 42 M38 30 L50 42 L62 30' fill='none' stroke='{a}' stroke-width='6' stroke-linecap='round' stroke-linejoin='round'/></svg>",
            head = head,
            a = accent
        ),
        IconGlyph::ActionCompress => format!(
            "{head}<rect x='18' y='46' width='64' height='38' rx='7' fill='none' stroke='{a}' stroke-width='6'/>\
             <path d='M50 14 L50 38 M38 26 L50 14 L62 26' fill='none' stroke='{a}' stroke-width='6' stroke-linecap='round' stroke-linejoin='round'/></svg>",
            head = head,
            a = accent
        ),
        IconGlyph::ActionShortcut => format!(
            "{head}<path d='M38 62 L62 38' stroke='{a}' stroke-width='8' stroke-linecap='round'/>\
             <path d='M30 54 A16 16 0 1 1 42 42' fill='none' stroke='{a}' stroke-width='7' stroke-linecap='round'/>\
             <path d='M70 46 A16 16 0 1 1 58 58' fill='none' stroke='{a}' stroke-width='7' stroke-linecap='round'/></svg>",
            head = head,
            a = accent
        ),
        IconGlyph::ActionPath => format!(
            "{head}<rect x='14' y='28' width='44' height='44' rx='7' fill='none' stroke='{a}' stroke-width='6'/>\
             <path d='M24 44 L48 44 M24 56 L40 56' stroke='{a}' stroke-width='5' stroke-linecap='round'/>\
             <path d='M56 50 L78 50 M70 42 L78 50 L70 58' fill='none' stroke='{a}' stroke-width='6' stroke-linecap='round' stroke-linejoin='round'/></svg>",
            head = head,
            a = accent
        ),
        IconGlyph::ActionSort => format!(
            "{head}<path d='M26 24 L26 76 M26 24 L16 36 M26 24 L36 36' stroke='{a}' stroke-width='6' stroke-linecap='round' stroke-linejoin='round'/>\
             <path d='M52 32 L82 32 M52 50 L74 50 M52 68 L66 68' stroke='{a}' stroke-width='6' stroke-linecap='round'/></svg>",
            head = head,
            a = accent
        ),
        IconGlyph::ActionView => format!(
            "{head}<rect x='18' y='18' width='28' height='28' rx='5' fill='{a}' opacity='0.9'/>\
             <rect x='54' y='18' width='28' height='28' rx='5' fill='{a}' opacity='0.6'/>\
             <rect x='18' y='54' width='28' height='28' rx='5' fill='{a}' opacity='0.6'/>\
             <rect x='54' y='54' width='28' height='28' rx='5' fill='{a}' opacity='0.9'/></svg>",
            head = head,
            a = accent
        ),
        IconGlyph::ActionFilter => format!(
            "{head}<path d='M16 22 L84 22 L58 52 L58 78 L42 70 L42 52 Z' fill='none' stroke='{a}' stroke-width='6' stroke-linejoin='round'/></svg>",
            head = head,
            a = accent
        ),
        IconGlyph::ActionMore => format!(
            "{head}<circle cx='28' cy='50' r='7' fill='{a}'/><circle cx='50' cy='50' r='7' fill='{a}'/><circle cx='72' cy='50' r='7' fill='{a}'/></svg>",
            head = head,
            a = accent
        ),
        IconGlyph::ActionHamburger => format!(
            "{head}<line x1='20' y1='30' x2='80' y2='30' stroke='{a}' stroke-width='7' stroke-linecap='round'/>\
             <line x1='20' y1='50' x2='80' y2='50' stroke='{a}' stroke-width='7' stroke-linecap='round'/>\
             <line x1='20' y1='70' x2='80' y2='70' stroke='{a}' stroke-width='7' stroke-linecap='round'/></svg>",
            head = head,
            a = accent
        ),
        IconGlyph::ActionEye => format!(
            "{head}<path d='M12 50 Q28 24 50 24 Q72 24 88 50 Q72 76 50 76 Q28 76 12 50 Z' fill='none' stroke='{a}' stroke-width='6' stroke-linejoin='round'/>\
             <circle cx='50' cy='50' r='11' fill='{a}'/></svg>",
            head = head,
            a = accent
        ),
        IconGlyph::ActionPin => format!(
            "{head}<path d='M62 14 L86 38 L74 44 L60 66 L56 84 L44 72 L28 88 L20 80 L36 64 L28 56 L44 50 L56 26 Z' fill='{a}'/>\
             <path d='M62 14 L86 38' stroke='#FFFFFF' stroke-width='2' opacity='0.35'/></svg>",
            head = head,
            a = accent
        ),
        IconGlyph::ActionMinimize => format!(
            "{head}<path d='M22 50 L78 50' stroke='{a}' stroke-width='7' stroke-linecap='round'/></svg>",
            head = head,
            a = accent
        ),
        IconGlyph::ActionMaximize => format!(
            "{head}<rect x='24' y='24' width='52' height='52' rx='6' fill='none' stroke='{a}' stroke-width='6'/></svg>",
            head = head,
            a = accent
        ),
        IconGlyph::ActionRestore => format!(
            "{head}<rect x='20' y='30' width='42' height='42' rx='6' fill='none' stroke='{a}' stroke-width='6'/>\
             <path d='M36 30 L36 22 Q36 18 40 18 L70 18 Q78 18 78 26 L78 56 Q78 62 72 62 L64 62' fill='none' stroke='{a}' stroke-width='6'/></svg>",
            head = head,
            a = accent
        ),
        IconGlyph::ActionClose => format!(
            "{head}<path d='M28 28 L72 72 M72 28 L28 72' stroke='{a}' stroke-width='7' stroke-linecap='round'/></svg>",
            head = head,
            a = accent
        ),
        IconGlyph::ActionPlus => format!(
            "{head}<path d='M50 24 L50 76 M24 50 L76 50' stroke='{a}' stroke-width='8' stroke-linecap='round'/></svg>",
            head = head,
            a = accent
        ),
        IconGlyph::ActionBack => format!(
            "{head}<path d='M56 22 L30 50 L56 78' fill='none' stroke='{a}' stroke-width='8' stroke-linecap='round' stroke-linejoin='round'/></svg>",
            head = head,
            a = accent
        ),
        IconGlyph::ActionForward => format!(
            "{head}<path d='M44 22 L70 50 L44 78' fill='none' stroke='{a}' stroke-width='8' stroke-linecap='round' stroke-linejoin='round'/></svg>",
            head = head,
            a = accent
        ),
        IconGlyph::ActionUp => format!(
            "{head}<path d='M22 62 L50 34 L78 62' fill='none' stroke='{a}' stroke-width='8' stroke-linecap='round' stroke-linejoin='round'/></svg>",
            head = head,
            a = accent
        ),
        IconGlyph::ActionList => format!(
            "{head}<path d='M32 38 L78 38 M32 50 L78 50 M32 62 L78 62' stroke='{a}' stroke-width='6' stroke-linecap='round'/>\
             <circle cx='22' cy='38' r='4' fill='{a}'/><circle cx='22' cy='50' r='4' fill='{a}'/><circle cx='22' cy='62' r='4' fill='{a}'/></svg>",
            head = head,
            a = accent
        ),
        IconGlyph::ActionColumns => format!(
            "{head}<rect x='18' y='28' width='20' height='44' rx='4' fill='none' stroke='{a}' stroke-width='6'/>\
             <rect x='40' y='28' width='20' height='44' rx='4' fill='none' stroke='{a}' stroke-width='6'/>\
             <rect x='62' y='28' width='20' height='44' rx='4' fill='none' stroke='{a}' stroke-width='6'/></svg>",
            head = head,
            a = accent
        ),
        IconGlyph::ActionSun => format!(
            "{head}<circle cx='50' cy='50' r='16' fill='none' stroke='{a}' stroke-width='6'/>\
             <g stroke='{a}' stroke-width='6' stroke-linecap='round'>\
             <path d='M50 16 L50 28 M50 72 L50 84 M16 50 L28 50 M72 50 L84 50 M26 26 L34 34 M66 66 L74 74 M74 26 L66 34 M34 66 L26 74'/></g></svg>",
            head = head,
            a = accent
        ),
        IconGlyph::ActionMoon => format!(
            "{head}<path d='M62 20 A32 32 0 1 0 62 80 A26 26 0 1 1 62 20 Z' fill='{a}'/></svg>",
            head = head,
            a = accent
        ),
        IconGlyph::ActionPlay => format!(
            "{head}<circle cx='50' cy='50' r='34' fill='none' stroke='{a}' stroke-width='6'/>\
             <path d='M42 34 L68 50 L42 66 Z' fill='{a}'/></svg>",
            head = head,
            a = accent
        ),
        IconGlyph::ActionSidebarToggle => format!(
            "{head}<rect x='18' y='20' width='64' height='60' rx='7' fill='none' stroke='{a}' stroke-width='6'/>\
             <rect x='22' y='24' width='18' height='52' rx='3' fill='{a}' opacity='0.45'/>\
             <line x1='42' y1='20' x2='42' y2='80' stroke='{a}' stroke-width='6'/></svg>",
            head = head,
            a = accent
        ),
        IconGlyph::ActionNewWindow => format!(
            "{head}<rect x='18' y='28' width='54' height='48' rx='6' fill='none' stroke='{a}' stroke-width='6'/>\
             <path d='M36 18 L76 18 Q82 18 82 24 L82 62' fill='none' stroke='{a}' stroke-width='6' stroke-linecap='round'/>\
             <line x1='45' y1='44' x2='45' y2='60' stroke='{a}' stroke-width='6' stroke-linecap='round'/>\
             <line x1='37' y1='52' x2='53' y2='52' stroke='{a}' stroke-width='6' stroke-linecap='round'/></svg>",
            head = head,
            a = accent
        ),
        IconGlyph::ActionMenu => format!(
            "{head}<line x1='20' y1='30' x2='80' y2='30' stroke='{a}' stroke-width='6' stroke-linecap='round'/>\
             <circle cx='62' cy='30' r='6' fill='{a}'/>\
             <line x1='20' y1='50' x2='80' y2='50' stroke='{a}' stroke-width='6' stroke-linecap='round'/>\
             <circle cx='38' cy='50' r='6' fill='{a}'/>\
             <line x1='20' y1='70' x2='80' y2='70' stroke='{a}' stroke-width='6' stroke-linecap='round'/>\
             <circle cx='56' cy='70' r='6' fill='{a}'/></svg>",
            head = head,
            a = accent
        ),
        IconGlyph::ActionSliders => format!(
            "{head}<line x1='18' y1='28' x2='82' y2='28' stroke='{a}' stroke-width='6' stroke-linecap='round'/>\
             <circle cx='36' cy='28' r='7' fill='{a}'/>\
             <line x1='18' y1='50' x2='82' y2='50' stroke='{a}' stroke-width='6' stroke-linecap='round'/>\
             <circle cx='64' cy='50' r='7' fill='{a}'/>\
             <line x1='18' y1='72' x2='82' y2='72' stroke='{a}' stroke-width='6' stroke-linecap='round'/>\
             <circle cx='46' cy='72' r='7' fill='{a}'/></svg>",
            head = head,
            a = accent
        ),
    }
}

// ──────────────────────────────────────────────────────────────────────────
// Rasterization + cache
// ──────────────────────────────────────────────────────────────────────────

type CacheKey = (IconPack, IconGlyph, u32, [u8; 3]);
static ICON_CACHE: std::sync::LazyLock<Mutex<HashMap<CacheKey, Option<TextureHandle>>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

type SpecialFolderCacheKey = (IconPack, SpecialFolder, u32);
static SPECIAL_FOLDER_CACHE: std::sync::LazyLock<
    Mutex<HashMap<SpecialFolderCacheKey, Option<TextureHandle>>>,
> = std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

/// Renders a special folder at `css_px` logical size and caches the texture by `(pack, kind, size)`.
pub fn special_folder_texture(
    ctx: &eframe::egui::Context,
    pack: IconPack,
    kind: SpecialFolder,
    css_px: f32,
) -> Option<TextureHandle> {
    let ppp = ctx.pixels_per_point();
    let phys = ((css_px * ppp).round() as u32).clamp(12, 512);
    let key = (pack, kind, phys);

    if let Ok(cache) = SPECIAL_FOLDER_CACHE.lock() {
        if let Some(hit) = cache.get(&key) {
            return hit.clone();
        }
    }

    let is_small = css_px <= 20.0;
    let svg = special_folder_svg_sized(pack, kind, is_small);
    let tex = rasterize(ctx, &svg, phys, pack).ok();

    if let Ok(mut cache) = SPECIAL_FOLDER_CACHE.lock() {
        cache.insert(key, tex.clone());
        if cache.len() > 4096 {
            cache.clear();
        }
    }
    ctx.request_repaint_after(std::time::Duration::from_millis(1));
    tex
}

/// Convenience painter for special folders.
pub fn paint_special_folder(ui: &Ui, pack: IconPack, kind: SpecialFolder, rect: Rect) {
    let size = rect.width().min(rect.height());
    if size < 4.0 {
        return;
    }
    let Some(tex) = special_folder_texture(ui.ctx(), pack, kind, size) else {
        return;
    };
    let sq = snapped_icon_quad(ui.ctx(), rect, size);
    let uv = Rect::from_min_max(eframe::egui::pos2(0.0, 0.0), eframe::egui::pos2(1.0, 1.0));
    ui.painter().image(tex.id(), sq, uv, Color32::WHITE);
}

/// Renders the glyph at `css_px` logical size (physical px computed internally)
/// and caches the result. Returns None while the rasterizer is busy (rare).
pub fn icon_texture(
    ctx: &eframe::egui::Context,
    pack: IconPack,
    glyph: IconGlyph,
    css_px: f32,
    accent_rgb: [u8; 3],
) -> Option<TextureHandle> {
    if let Some(kind) = glyph.special_folder() {
        return special_folder_texture(ctx, pack, kind, css_px);
    }
    let ppp = ctx.pixels_per_point();
    let phys = ((css_px * ppp).round() as u32).clamp(12, 512);
    let key = (pack, glyph, phys, accent_rgb);

    if let Ok(cache) = ICON_CACHE.lock() {
        if let Some(hit) = cache.get(&key) {
            return hit.clone();
        }
    }

    let svg = svg_for(pack, glyph, accent_rgb);
    let tex = rasterize(ctx, &svg, phys, key.0).ok();

    if let Ok(mut cache) = ICON_CACHE.lock() {
        cache.insert(key, tex.clone());
        // Basic size guard so long sessions with many zoom levels stay bounded
        if cache.len() > 4096 {
            cache.clear();
        }
    }
    ctx.request_repaint_after(std::time::Duration::from_millis(1));
    tex
}

fn rasterize(
    ctx: &eframe::egui::Context,
    svg: &str,
    phys_px: u32,
    _pack: IconPack,
) -> Result<TextureHandle, String> {
    let opt = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_str(svg, &opt).map_err(|e| format!("svg parse: {e:?}"))?;
    let size = tree.size();
    let scale = (phys_px as f32).max(1.0) / size.width().max(1.0);
    let scaled_w = (size.width() * scale).round() as u32;
    let scaled_h = (size.height() * scale).round() as u32;
    let mut pixmap =
        resvg::tiny_skia::Pixmap::new(scaled_w.max(1), scaled_h.max(1)).ok_or("pixmap alloc")?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    // tiny-skia outputs premultiplied RGBA8; egui wants unpremultiplied
    let mut rgba = pixmap.data().to_vec();
    for px in rgba.chunks_exact_mut(4) {
        let a = u32::from(px[3]);
        if a < 255 {
            px[0] = ((u32::from(px[0]) * 255 + a / 2) / a.max(1)) as u8;
            px[1] = ((u32::from(px[1]) * 255 + a / 2) / a.max(1)) as u8;
            px[2] = ((u32::from(px[2]) * 255 + a / 2) / a.max(1)) as u8;
        }
    }
    let img = ColorImage::from_rgba_unmultiplied(
        [pixmap.width() as usize, pixmap.height() as usize],
        &rgba,
    );
    Ok(ctx.load_texture(
        format!("icn_{}_{}_{}", _pack.id(), phys_px, "g"),
        img,
        TextureOptions::LINEAR,
    ))
}

/// Paints a glyph inside `rect` (square centering) at the pack's language.
pub fn paint_icon(ui: &Ui, pack: IconPack, glyph: IconGlyph, rect: Rect, accent_rgb: [u8; 3]) {
    paint_icon_with(ui.painter(), ui.ctx(), pack, glyph, rect, accent_rgb);
}

/// Paints a glyph with transformed geometry: optional rotation, scale, and offset translation.
#[allow(clippy::too_many_arguments)]
pub fn paint_icon_transformed(
    painter: &eframe::egui::Painter,
    ctx: &eframe::egui::Context,
    pack: IconPack,
    glyph: IconGlyph,
    rect: Rect,
    accent_rgb: [u8; 3],
    angle_rad: f32,
    scale: f32,
    offset: Vec2,
) {
    let base_size = rect.width().min(rect.height()) * scale;
    if base_size < 4.0 {
        return;
    }
    let Some(tex) = icon_texture(ctx, pack, glyph, base_size, accent_rgb) else {
        return;
    };
    let center = rect.center() + offset;
    let target_rect = Rect::from_center_size(center, Vec2::splat(base_size));
    let sq = snapped_icon_quad(ctx, target_rect, base_size);

    if angle_rad.abs() > 0.001 {
        let (sin_a, cos_a) = angle_rad.sin_cos();
        let rotate_point = |p: Pos2| -> Pos2 {
            let d = p - center;
            let rx = d.x * cos_a - d.y * sin_a;
            let ry = d.x * sin_a + d.y * cos_a;
            center + Vec2::new(rx, ry)
        };
        let mut mesh = eframe::egui::Mesh::with_texture(tex.id());
        let idx = mesh.vertices.len() as u32;
        mesh.vertices.push(eframe::egui::epaint::Vertex {
            pos: rotate_point(sq.left_top()),
            uv: eframe::egui::pos2(0.0, 0.0),
            color: Color32::WHITE,
        });
        mesh.vertices.push(eframe::egui::epaint::Vertex {
            pos: rotate_point(sq.right_top()),
            uv: eframe::egui::pos2(1.0, 0.0),
            color: Color32::WHITE,
        });
        mesh.vertices.push(eframe::egui::epaint::Vertex {
            pos: rotate_point(sq.right_bottom()),
            uv: eframe::egui::pos2(1.0, 1.0),
            color: Color32::WHITE,
        });
        mesh.vertices.push(eframe::egui::epaint::Vertex {
            pos: rotate_point(sq.left_bottom()),
            uv: eframe::egui::pos2(0.0, 1.0),
            color: Color32::WHITE,
        });
        mesh.add_triangle(idx, idx + 1, idx + 2);
        mesh.add_triangle(idx, idx + 2, idx + 3);
        painter.add(eframe::egui::Shape::Mesh(mesh));
    } else {
        let uv = Rect::from_min_max(eframe::egui::pos2(0.0, 0.0), eframe::egui::pos2(1.0, 1.0));
        painter.image(tex.id(), sq, uv, Color32::WHITE);
    }
}

/// Paints a glyph with an explicit painter — used by widget surfaces that draw
/// straight onto a `Painter` rather than a `Ui`.
pub fn paint_icon_with(
    painter: &eframe::egui::Painter,
    ctx: &eframe::egui::Context,
    pack: IconPack,
    glyph: IconGlyph,
    rect: Rect,
    accent_rgb: [u8; 3],
) {
    let size = rect.width().min(rect.height());
    if size < 4.0 {
        return;
    }
    let Some(tex) = icon_texture(ctx, pack, glyph, size, accent_rgb) else {
        return;
    };
    let sq = snapped_icon_quad(ctx, rect, size);
    let uv = Rect::from_min_max(eframe::egui::pos2(0.0, 0.0), eframe::egui::pos2(1.0, 1.0));
    painter.image(tex.id(), sq, uv, Color32::WHITE);
}

/// Device-exact paint geometry: the returned square matches the
/// icon_texture rasterization of css_px (same formula and clamp) and
/// its min corner lies on integer device pixels, so the texture blits
/// 1:1 with no resampling or edge smear.
pub fn snapped_icon_quad(
    ctx: &eframe::egui::Context,
    rect: eframe::egui::Rect,
    css_px: f32,
) -> eframe::egui::Rect {
    let ppp = ctx.pixels_per_point();
    let phys = ((css_px * ppp).round() as u32).clamp(12, 512) as f32;
    let css = phys / ppp;
    let min = rect.center() - eframe::egui::Vec2::splat(css * 0.5);
    let min = eframe::egui::Pos2::new((min.x * ppp).round() / ppp, (min.y * ppp).round() / ppp);
    eframe::egui::Rect::from_min_size(min, eframe::egui::Vec2::splat(css))
}

/// Convenience: paint with a slightly padded rect
pub fn paint_icon_padded(
    ui: &Ui,
    pack: IconPack,
    glyph: IconGlyph,
    rect: Rect,
    pad: f32,
    accent_rgb: [u8; 3],
) {
    paint_icon(ui, pack, glyph, rect.shrink(pad), accent_rgb)
}

#[cfg(test)]
mod page_svg_tests {
    use super::*;

    /// Every page glyph must produce exactly one well-formed SVG document
    /// (a stray `</svg>` in an emblem once blanked every PDF icon).
    #[test]
    fn page_glyph_svgs_are_well_formed() {
        let pages = [
            IconGlyph::PageDefault,
            IconGlyph::PageText,
            IconGlyph::PageDoc,
            IconGlyph::PageSheet,
            IconGlyph::PageSlide,
            IconGlyph::PagePdf,
            IconGlyph::PageImage,
            IconGlyph::PageVideo,
            IconGlyph::PageAudio,
            IconGlyph::PageCode,
            IconGlyph::PageConfig,
            IconGlyph::PageArchive,
            IconGlyph::PageExe,
            IconGlyph::PageLibrary,
        ];
        for pack in IconPack::all() {
            for g in pages {
                let svg = svg_for(pack, g, [220, 60, 60]);
                assert_eq!(svg.matches("</svg>").count(), 1, "{pack:?} {g:?}");
                let opt = resvg::usvg::Options::default();
                assert!(
                    resvg::usvg::Tree::from_str(&svg, &opt).is_ok(),
                    "{pack:?} {g:?} does not parse"
                );
            }
        }
    }
}
