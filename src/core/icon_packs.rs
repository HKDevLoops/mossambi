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

use eframe::egui::{Color32, ColorImage, Rect, TextureHandle, TextureOptions, Ui};
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
    FolderDesktop,
    FolderDownloads,
    FolderDocuments,
    FolderPictures,
    FolderVideos,
    FolderMusic,
    FolderRepo,
    FolderCode,
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
}

impl IconGlyph {
    /// Special folder names follow the existing resolver (case-insensitive)
    fn folder_special(name: &str) -> Option<IconGlyph> {
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
        } else if matches("desktop") {
            Some(IconGlyph::FolderDesktop)
        } else if matches("downloads") || matches("download") {
            Some(IconGlyph::FolderDownloads)
        } else if matches("documents") || matches("document") || matches("my documents") {
            Some(IconGlyph::FolderDocuments)
        } else if matches("pictures") || matches("photos") || matches("wallpaper") {
            Some(IconGlyph::FolderPictures)
        } else if matches("videos") || matches("movies") {
            Some(IconGlyph::FolderVideos)
        } else if matches("music") || matches("audio") {
            Some(IconGlyph::FolderMusic)
        } else {
            None
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
        if is_dir {
            return match Self::folder_special(name) {
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
                back: "#DBA222",
                front: "#FFC844",
                tab: "#B57F0F",
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
                back: "#E4A918",
                front: "#FDCD58",
                tab: "#C28C0B",
                outline: None,
                radius: 3.0,
                tab_style: TabStyle::Tab,
                gloss: false,
                page: "#FFFFFF",
                page_edge: "#DCD5C6",
                fold: "#EFE6D2",
                line: "#8B97A8",
            },
            Self::Windows8 => PackStyle {
                back: "#EDB500",
                front: "#FFCB29",
                tab: "#D19E00",
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
                back: "#E7BE33",
                front: "#FEDB63",
                tab: "#C99F22",
                outline: Some("#A47A1B"),
                radius: 6.0,
                tab_style: TabStyle::Tab,
                gloss: true,
                page: "#FFFFFF",
                page_edge: "#D8CBAF",
                fold: "#EFE3C6",
                line: "#95805A",
            },
            Self::WindowsXp => PackStyle {
                back: "#DFA92A",
                front: "#F9D65C",
                tab: "#B8860B",
                outline: Some("#7A5A14"),
                radius: 2.0,
                tab_style: TabStyle::Tab,
                gloss: true,
                page: "#FFFFFF",
                page_edge: "#8A7B55",
                fold: "#E3D9B9",
                line: "#6E6A4F",
            },
            Self::Fluent => PackStyle {
                back: "#F4A93C",
                front: "#FFC340",
                tab: "#DE8F1E",
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

/// Folder body shapes per pack tab style
fn folder_shapes(st: &PackStyle) -> String {
    let r = st.radius;
    let out = match st.outline {
        Some(c) => format!(
            " stroke='{}' stroke-width='2.2' paint-order='stroke fill'",
            c
        ),
        None => String::new(),
    };
    let (back, front, tab) = (st.back, st.front, st.tab);
    match st.tab_style {
        // Material / Fluent / Android: open-mouth top band, full-width front
        TabStyle::Flush => format!(
            "<rect x='8' y='20' width='84' height='16' rx='{r}' fill='{back}'{out}/> \
             <rect x='8' y='30' width='84' height='52' rx='{r}' fill='{front}'{out}/> \
             <rect x='8' y='30' width='84' height='2.4' fill='#FFFFFF' opacity='0.18'/>",
            r = r,
            back = back,
            front = front,
            out = out
        ),
        // Classic Finder / Win7 / XP style: distinct tab on top-left
        TabStyle::Tab => format!(
            "<rect x='8' y='12' width='44' height='16' rx='{tr}' fill='{tab}'{out_x}/> \
             <rect x='8' y='22' width='84' height='60' rx='{r}' fill='{back}'{out}/> \
             <rect x='8' y='34' width='84' height='48' rx='{r}' fill='{front}'{out}/> \
             <rect x='8' y='34' width='84' height='2.4' fill='#FFFFFF' opacity='0.18'/>",
            tr = (r * 0.7).max(3.0),
            tab = tab,
            out_x = out,
            r = r,
            back = back,
            front = front,
            out = out
        ),
        // GNOME / Adwaita / Hyprland: detached short notch
        TabStyle::Notch => format!(
            "<rect x='18' y='13' width='40' height='10' rx='5' fill='{tab}'{out_x}/> \
             <rect x='8' y='20' width='84' height='60' rx='{r}' fill='{back}'{out}/> \
             <rect x='8' y='33' width='84' height='47' rx='{r}' fill='{front}'{out}/> \
             <rect x='8' y='33' width='84' height='1.6' fill='#FFFFFF' opacity='0.10'/>",
            tab = tab,
            out_x = out,
            r = r,
            back = back,
            front = front,
            out = out
        ),
    }
}

fn gloss_overlay(y_top: f32, y_bot: f32) -> String {
    format!(
        "<path d='M8 {:.0} L92 {:.0} L92 {:.0} L8 {:.0} Z' fill='#FFFFFF' opacity='0.22'/>",
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
             <rect x='34' y='60' width='22' height='2.8' rx='1.4' fill='{l}' opacity='0.55'/></svg>",
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

/// Full 100×100 SVG for a glyph in the requested pack
pub fn svg_for(pack: IconPack, glyph: IconGlyph, accent_rgb: [u8; 3]) -> String {
    let st = pack.style();
    let accent = format!(
        "#{:02X}{:02X}{:02X}",
        accent_rgb[0], accent_rgb[1], accent_rgb[2]
    );
    let head =
        "<svg xmlns='http://www.w3.org/2000/svg' width='100' height='100' viewBox='0 0 100 100'>\n";
    match glyph {
        // ── Folders ──
        IconGlyph::Folder | IconGlyph::FolderDesktop | IconGlyph::FolderDownloads
        | IconGlyph::FolderDocuments | IconGlyph::FolderPictures | IconGlyph::FolderVideos
        | IconGlyph::FolderMusic | IconGlyph::FolderRepo | IconGlyph::FolderCode => {
            let shapes = folder_shapes(&st);
            // Neutral folder gets a subtle paper peek; special folders get a
            // white emblem in the accent's contrast channel
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
            let body = st.front;
            let line = st.back;
            format!(
                "{head}<rect x='12' y='34' width='76' height='34' rx='6' fill='{b}' {o}/>\
                 <rect x='12' y='60' width='76' height='8' rx='2' fill='{l}' opacity='0.55'/>\
                 <circle cx='76' cy='58' r='4' fill='#FFFFFF' opacity='0.9'/>\
                 <path d='M30 44 L58 44 M30 52 L52 52' stroke='{d}' stroke-width='3' stroke-linecap='round' opacity='0.65'/></svg>",
                head = head,
                b = body,
                l = line,
                o = match st.outline { Some(c) => format!("stroke='{}' stroke-width='2'", c), None => String::new() },
                d = if pack == IconPack::Hyprland { "#9DB8CC" } else { "#FFFFFF" }
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
            let em = page_emblem(glyph, &st, &accent).unwrap_or_else(|| {
                format!(
                    "<rect x='34' y='46' width='32' height='2.8' rx='1.4' fill='{l}' opacity='0.7'/> \
                     <rect x='34' y='53' width='32' height='2.8' rx='1.4' fill='{l}' opacity='0.7'/> \
                     <rect x='34' y='60' width='20' height='2.8' rx='1.4' fill='{a}'/>",
                    l = st.line,
                    a = accent
                )
            });
            format!(
                "{head}<path d='M24 14 L62 14 L76 28 L76 82 Q76 86 72 86 L28 86 Q24 86 24 82 Z' fill='{p}' stroke='{e}' stroke-width='2'/>\
                 <path d='M62 14 L62 28 L76 28 Z' fill='{f}'/>\
                 <path d='M62 28 L76 28 L76 42 Z' fill='#000' opacity='0.0'/>\
                 {e2}</svg>",
                head = head,
                p = st.page,
                e = st.page_edge,
                f = st.fold,
                e2 = em
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
    }
}

// ──────────────────────────────────────────────────────────────────────────
// Rasterization + cache
// ──────────────────────────────────────────────────────────────────────────

type CacheKey = (IconPack, IconGlyph, u32, [u8; 3]);
static ICON_CACHE: std::sync::LazyLock<Mutex<HashMap<CacheKey, Option<TextureHandle>>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

/// Renders the glyph at `css_px` logical size (physical px computed internally)
/// and caches the result. Returns None while the rasterizer is busy (rare).
pub fn icon_texture(
    ctx: &eframe::egui::Context,
    pack: IconPack,
    glyph: IconGlyph,
    css_px: f32,
    accent_rgb: [u8; 3],
) -> Option<TextureHandle> {
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
