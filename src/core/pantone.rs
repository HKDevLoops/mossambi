//! Theme System for Mossambi - 13 Named Palettes with Full Semantic Parity
//!
//! A curated launch set of 13 named themes (11 canonical editor palettes
//! plus 2 original Mossambi designs), in catalog order:
//!
//! Dark (11):
//!   1. Gruvbox          (morhetz/gruvbox)
//!   2. Tokyo Night      (folke/tokyonight.nvim, night style)
//!   3. Catppuccin Mocha (catppuccin)
//!   4. Modus Vivendi    (protesilaos modus-themes)
//!   5. Nord             (nordtheme)
//!   6. Dracula          (dracula spec)
//!   7. Kanagawa         (rebelot/kanagawa.nvim)
//!   8. One Dark         (atom/one-dark-syntax)
//!   9. Atom             (Atom editor chrome variant)
//!  10. VS Code Dark+    (microsoft/vscode default themes)
//!  11. Synthwave        (robb0wen/synthwave-vscode)
//!  12. Rangalipi        (Mossambi original; the default theme)
//!
//! Light (1):
//!  13. Mossambi Punch   (Mossambi original true-fruit light palette)
//!
//! Every theme (both Dark and Light variants) defines 16 semantic roles:
//! - bg: Root window canvas & glass tint
//! - surface_1: Sidebar, header toolbar, status bar (Layer 1)
//! - surface_2: Cards, bento tiles, table rows (Layer 2)
//! - surface_3: Flyouts, context menus, tooltips, popovers (Layer 3)
//! - border: Subtle card and divider stroke
//! - text_primary: Primary headings, active labels (WCAG AA >= 4.5:1)
//! - text_secondary: Dates, paths, secondary info (WCAG AA >= 3.0:1)
//! - text_muted: File sizes, hotkey hints, inactive text
//! - accent: Primary brand action, active tab pill
//! - accent_alt: Secondary highlight, search matches, badge background
//! - selection: Row/Card selection background
//! - hover: Cursor hover fill for rows/cards
//! - success / warn / error: System telemetry and health status
//! - tag_code / tag_media / tag_archive / tag_document / tag_image: File categorization

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PantoneColor {
    pub name: &'static str,
    pub code: &'static str,
    pub hex: &'static str,
}

impl PantoneColor {
    pub const fn new(name: &'static str, code: &'static str, hex: &'static str) -> Self {
        Self { name, code, hex }
    }

    /// Converts hex string to standard RGB tuple
    pub fn to_rgb(&self) -> (u8, u8, u8) {
        let clean = self.hex.trim_start_matches('#');
        if clean.len() == 6 {
            let r = u8::from_str_radix(&clean[0..2], 16).unwrap_or(0);
            let g = u8::from_str_radix(&clean[2..4], 16).unwrap_or(0);
            let b = u8::from_str_radix(&clean[4..6], 16).unwrap_or(0);
            (r, g, b)
        } else {
            (128, 128, 128)
        }
    }

    /// Computes WCAG 2.1 relative luminance (0.0 = darkest black, 1.0 = brightest white)
    pub fn relative_luminance(&self) -> f32 {
        let (r, g, b) = self.to_rgb();
        let to_linear = |c: u8| -> f32 {
            let s = c as f32 / 255.0;
            if s <= 0.04045 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * to_linear(r) + 0.7152 * to_linear(g) + 0.0722 * to_linear(b)
    }

    /// Computes WCAG 2.1 contrast ratio against another color (1.0:1 to 21.0:1)
    pub fn contrast_ratio(&self, other: &PantoneColor) -> f32 {
        let l1 = self.relative_luminance();
        let l2 = other.relative_luminance();
        let (lighter, darker) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
        (lighter + 0.05) / (darker + 0.05)
    }
}

pub struct PantonePalette;

impl PantonePalette {
    // Soothing Pantone Accent Colors (100% Eye-Safe, Non-Neon)
    pub const RAW_PEEL_GREEN: PantoneColor =
        PantoneColor::new("Sage Meadow", "Pantone 16-5917 TPG", "#4A855A");
    pub const SWEET_ZEST_LIME: PantoneColor =
        PantoneColor::new("Classic Blue", "Pantone 19-4052 TPG", "#007AFF");
    pub const RIPE_PULP_YELLOW_GREEN: PantoneColor =
        PantoneColor::new("Soft Amber", "Pantone 14-1064 TPG", "#D97706");
    pub const CITRUS_GOLD: PantoneColor =
        PantoneColor::new("Citrus Gold", "Pantone 1235 C", "#FFB81C");
    pub const TANGERINE_SUN: PantoneColor =
        PantoneColor::new("Tangerine Sun", "Pantone 15-1247", "#F88F58");
    pub const DEEP_FOREST_PEEL: PantoneColor =
        PantoneColor::new("Deep Forest Peel", "Pantone 371 C", "#2E4D1E");

    // Apple-Style Certified Neutrals
    pub const BRILLIANT_WHITE: PantoneColor =
        PantoneColor::new("Brilliant White", "Pantone 11-4001 TPG", "#F4F5F7");
    pub const BLANC_DE_BLANC: PantoneColor =
        PantoneColor::new("Blanc de Blanc", "Pantone 11-4800 TPG", "#F1EFEA");
    pub const GLACIER_GRAY: PantoneColor =
        PantoneColor::new("Glacier Gray", "Pantone 14-4102 TPG", "#C5C6C7");
    pub const COOL_GRAY_6: PantoneColor =
        PantoneColor::new("Cool Gray 6", "Pantone Cool Gray 6 C", "#A1A1A6");
    pub const PEWTER_SLATE: PantoneColor =
        PantoneColor::new("Pewter Slate", "Pantone 18-5203 TPG", "#2A2A2E");
    pub const ANTHRACITE_DARK: PantoneColor =
        PantoneColor::new("Anthracite Dark", "Pantone 19-4007 TPG", "#18181A");
    pub const DEEP_BLACK: PantoneColor =
        PantoneColor::new("Deep Black", "Pantone 19-3911 TPG", "#121214");
    pub const CHARCOAL_TEXT: PantoneColor =
        PantoneColor::new("Charcoal Text", "Pantone 19-3901 TPG", "#1D1D1F");
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CitrusShade {
    RawPeel,
    SweetZest,
    RipePulp,
    CitrusGold,
    DeepForest,
    Tangerine,
}

impl CitrusShade {
    pub fn to_pantone(&self) -> PantoneColor {
        match self {
            Self::RawPeel => PantonePalette::RAW_PEEL_GREEN,
            Self::SweetZest => PantonePalette::SWEET_ZEST_LIME,
            Self::RipePulp => PantonePalette::RIPE_PULP_YELLOW_GREEN,
            Self::CitrusGold => PantonePalette::CITRUS_GOLD,
            Self::DeepForest => PantonePalette::DEEP_FOREST_PEEL,
            Self::Tangerine => PantonePalette::TANGERINE_SUN,
        }
    }
}

/// Named Theme Model (one entry per catalog theme)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PantoneTheme {
    pub id: &'static str,
    pub name: &'static str,
    pub is_dark: bool,
    pub bg: PantoneColor,
    pub surface: PantoneColor,
    pub surface_1: PantoneColor,
    pub surface_2: PantoneColor,
    pub surface_3: PantoneColor,
    pub border: PantoneColor,
    pub text_primary: PantoneColor,
    pub text_secondary: PantoneColor,
    pub text_muted: PantoneColor,
    pub accent: PantoneColor,
    pub highlight: PantoneColor,
    pub accent_alt: PantoneColor,
    pub selection: PantoneColor,
    pub hover: PantoneColor,
    pub success: PantoneColor,
    pub warn: PantoneColor,
    pub error: PantoneColor,
    pub tag_code: PantoneColor,
    pub tag_media: PantoneColor,
    pub tag_archive: PantoneColor,
    pub tag_document: PantoneColor,
    pub tag_image: PantoneColor,
}

// -----------------------------------------------------------------------------
// THE 13 NAMED THEMES (catalog order: 11 dark, then 1 light)
// -----------------------------------------------------------------------------
pub static THEMES: [PantoneTheme; 13] = [
    PantoneTheme {
        id: "gruvbox",
        name: "Gruvbox",
        is_dark: true,
        bg: PantoneColor::new("Dark0", "gruvbox bg0", "#282828"),
        surface: PantoneColor::new("Dark1", "gruvbox bg1", "#3c3836"),
        surface_1: PantoneColor::new("Dark0 Hard", "gruvbox bg0_h", "#1d2021"),
        surface_2: PantoneColor::new("Dark1", "gruvbox bg1", "#3c3836"),
        surface_3: PantoneColor::new("Dark2", "gruvbox bg2", "#504945"),
        border: PantoneColor::new("Dark2", "gruvbox bg2", "#504945"),
        text_primary: PantoneColor::new("Foreground", "gruvbox fg1", "#ebdbb2"),
        text_secondary: PantoneColor::new("Foreground2", "gruvbox fg2", "#d5c4a1"),
        text_muted: PantoneColor::new("Gray", "gruvbox gray", "#928374"),
        accent: PantoneColor::new("Blue", "gruvbox blue", "#83a598"),
        highlight: PantoneColor::new("Yellow", "gruvbox yellow", "#fabd2f"),
        accent_alt: PantoneColor::new("Yellow", "gruvbox yellow", "#fabd2f"),
        selection: PantoneColor::new("Dark3", "gruvbox bg3", "#665c54"),
        hover: PantoneColor::new("Dark0 Soft", "gruvbox bg0_s", "#32302f"),
        success: PantoneColor::new("Green", "gruvbox green", "#b8bb26"),
        warn: PantoneColor::new("Orange", "gruvbox orange", "#fe8019"),
        error: PantoneColor::new("Red", "gruvbox red", "#fb4934"),
        tag_code: PantoneColor::new("Blue", "gruvbox blue", "#83a598"),
        tag_media: PantoneColor::new("Purple", "gruvbox purple", "#d3869b"),
        tag_archive: PantoneColor::new("Yellow", "gruvbox yellow", "#fabd2f"),
        tag_document: PantoneColor::new("Orange", "gruvbox orange", "#fe8019"),
        tag_image: PantoneColor::new("Aqua", "gruvbox aqua", "#8ec07c"),
    },
    PantoneTheme {
        id: "tokyo_night",
        name: "Tokyo Night",
        is_dark: true,
        bg: PantoneColor::new("Night", "tokyonight bg", "#1a1b26"),
        surface: PantoneColor::new("Storm", "tokyonight storm", "#24283b"),
        surface_1: PantoneColor::new("Dark", "tokyonight bg_dark", "#16161e"),
        surface_2: PantoneColor::new("Storm", "tokyonight storm", "#24283b"),
        surface_3: PantoneColor::new("Terminal", "tokyonight terminal", "#2f354f"),
        border: PantoneColor::new("Gutter", "tokyonight gutter", "#292e42"),
        text_primary: PantoneColor::new("Foreground", "tokyonight fg", "#c0caf5"),
        text_secondary: PantoneColor::new("Foreground Dark", "tokyonight fg_dark", "#a9b1d6"),
        text_muted: PantoneColor::new("Comment", "tokyonight comment", "#565f89"),
        accent: PantoneColor::new("Blue", "tokyonight blue", "#7aa2f7"),
        highlight: PantoneColor::new("Yellow", "tokyonight yellow", "#e0af68"),
        accent_alt: PantoneColor::new("Yellow", "tokyonight yellow", "#e0af68"),
        selection: PantoneColor::new("Selection", "tokyonight selection", "#364a82"),
        hover: PantoneColor::new("Highlight", "tokyonight highlight", "#292e42"),
        success: PantoneColor::new("Green", "tokyonight green", "#9ece6a"),
        warn: PantoneColor::new("Orange", "tokyonight orange", "#ff9e64"),
        error: PantoneColor::new("Red", "tokyonight red", "#f7768e"),
        tag_code: PantoneColor::new("Blue", "tokyonight blue", "#7aa2f7"),
        tag_media: PantoneColor::new("Magenta", "tokyonight magenta", "#bb9af7"),
        tag_archive: PantoneColor::new("Orange", "tokyonight orange", "#ff9e64"),
        tag_document: PantoneColor::new("Red", "tokyonight red", "#f7768e"),
        tag_image: PantoneColor::new("Cyan", "tokyonight cyan", "#73daca"),
    },
    PantoneTheme {
        id: "catppuccin_mocha",
        name: "Catppuccin Mocha",
        is_dark: true,
        bg: PantoneColor::new("Base", "catppuccin base", "#1e1e2e"),
        surface: PantoneColor::new("Surface0", "catppuccin surface0", "#313244"),
        surface_1: PantoneColor::new("Mantle", "catppuccin mantle", "#181825"),
        surface_2: PantoneColor::new("Surface0", "catppuccin surface0", "#313244"),
        surface_3: PantoneColor::new("Surface1", "catppuccin surface1", "#45475a"),
        border: PantoneColor::new("Surface1", "catppuccin surface1", "#45475a"),
        text_primary: PantoneColor::new("Text", "catppuccin text", "#cdd6f4"),
        text_secondary: PantoneColor::new("Subtext0", "catppuccin subtext0", "#a6adc8"),
        text_muted: PantoneColor::new("Overlay0", "catppuccin overlay0", "#6c7086"),
        accent: PantoneColor::new("Blue", "catppuccin blue", "#89b4fa"),
        highlight: PantoneColor::new("Pink", "catppuccin pink", "#f5c2e7"),
        accent_alt: PantoneColor::new("Pink", "catppuccin pink", "#f5c2e7"),
        selection: PantoneColor::new("Surface2", "catppuccin surface2", "#585b70"),
        hover: PantoneColor::new("SurfaceHighlight", "catppuccin hover", "#363a4f"),
        success: PantoneColor::new("Green", "catppuccin green", "#a6e3a1"),
        warn: PantoneColor::new("Peach", "catppuccin peach", "#fab387"),
        error: PantoneColor::new("Red", "catppuccin red", "#f38ba8"),
        tag_code: PantoneColor::new("Blue", "catppuccin blue", "#89b4fa"),
        tag_media: PantoneColor::new("Mauve", "catppuccin mauve", "#cba6f7"),
        tag_archive: PantoneColor::new("Peach", "catppuccin peach", "#fab387"),
        tag_document: PantoneColor::new("Red", "catppuccin red", "#f38ba8"),
        tag_image: PantoneColor::new("Teal", "catppuccin teal", "#94e2d5"),
    },
    PantoneTheme {
        id: "modus_vivendi",
        name: "Modus Vivendi",
        is_dark: true,
        bg: PantoneColor::new("Main", "modus-vivendi bg-main", "#000000"),
        surface: PantoneColor::new("Dim", "modus-vivendi bg-dim", "#1e1e1e"),
        surface_1: PantoneColor::new("Darker", "modus-vivendi bg-darker", "#101010"),
        surface_2: PantoneColor::new("Dim", "modus-vivendi bg-dim", "#1e1e1e"),
        surface_3: PantoneColor::new("Inactive", "modus-vivendi bg-inactive", "#2a2a2a"),
        border: PantoneColor::new("Border", "modus-vivendi border", "#383838"),
        text_primary: PantoneColor::new("Main", "modus-vivendi fg-main", "#ffffff"),
        text_secondary: PantoneColor::new("Dim", "modus-vivendi fg-dim", "#b0b0b0"),
        text_muted: PantoneColor::new("Muted", "modus-vivendi muted", "#707070"),
        accent: PantoneColor::new("Blue", "modus-vivendi blue", "#2fafff"),
        highlight: PantoneColor::new("Yellow", "modus-vivendi yellow", "#d0bc00"),
        accent_alt: PantoneColor::new("Yellow", "modus-vivendi yellow", "#d0bc00"),
        selection: PantoneColor::new("Active", "modus-vivendi bg-active", "#303040"),
        hover: PantoneColor::new("Hover", "modus-vivendi hover", "#252525"),
        success: PantoneColor::new("Green", "modus-vivendi green", "#44bc44"),
        warn: PantoneColor::new("Yellow", "modus-vivendi yellow", "#eecc00"),
        error: PantoneColor::new("Red", "modus-vivendi red", "#ff5f59"),
        tag_code: PantoneColor::new("Blue", "modus-vivendi blue", "#2fafff"),
        tag_media: PantoneColor::new("Magenta", "modus-vivendi magenta", "#feacd0"),
        tag_archive: PantoneColor::new("Yellow", "modus-vivendi yellow", "#eecc00"),
        tag_document: PantoneColor::new("Red", "modus-vivendi red", "#ff5f59"),
        tag_image: PantoneColor::new("Cyan", "modus-vivendi cyan", "#00d3d0"),
    },
    PantoneTheme {
        id: "nord",
        name: "Nord",
        is_dark: true,
        bg: PantoneColor::new("Polar Night 0", "nord nord0", "#2e3440"),
        surface: PantoneColor::new("Polar Night 1", "nord nord1", "#3b4252"),
        surface_1: PantoneColor::new("Deep Night", "nord deep", "#272c36"),
        surface_2: PantoneColor::new("Polar Night 1", "nord nord1", "#3b4252"),
        surface_3: PantoneColor::new("Polar Night 2", "nord nord2", "#434c5e"),
        border: PantoneColor::new("Polar Night 2", "nord nord2", "#434c5e"),
        text_primary: PantoneColor::new("Snow Storm 2", "nord nord6", "#eceff4"),
        text_secondary: PantoneColor::new("Snow Storm 1", "nord nord4", "#d8dee9"),
        text_muted: PantoneColor::new("Polar Night 3", "nord nord3", "#616e88"),
        accent: PantoneColor::new("Frost", "nord nord8", "#88c0d0"),
        highlight: PantoneColor::new("Aurora Yellow", "nord nord13", "#ebcb8b"),
        accent_alt: PantoneColor::new("Aurora Yellow", "nord nord13", "#ebcb8b"),
        selection: PantoneColor::new("Polar Night 3", "nord sel", "#434c5e"),
        hover: PantoneColor::new("Polar Night Hover", "nord hov", "#383f4f"),
        success: PantoneColor::new("Aurora Green", "nord nord14", "#a3be8c"),
        warn: PantoneColor::new("Aurora Orange", "nord nord12", "#d08770"),
        error: PantoneColor::new("Aurora Red", "nord nord11", "#bf616a"),
        tag_code: PantoneColor::new("Frost Light", "nord nord8", "#88c0d0"),
        tag_media: PantoneColor::new("Aurora Purple", "nord nord15", "#b48ead"),
        tag_archive: PantoneColor::new("Aurora Yellow", "nord nord13", "#ebcb8b"),
        tag_document: PantoneColor::new("Aurora Red", "nord nord11", "#bf616a"),
        tag_image: PantoneColor::new("Aurora Green", "nord nord14", "#a3be8c"),
    },
    PantoneTheme {
        id: "dracula",
        name: "Dracula",
        is_dark: true,
        bg: PantoneColor::new("Background", "dracula bg", "#282a36"),
        surface: PantoneColor::new("Current Line", "dracula current-line", "#44475a"),
        surface_1: PantoneColor::new("Darker", "dracula bg_dark", "#21222c"),
        surface_2: PantoneColor::new("Current Line", "dracula current-line", "#44475a"),
        surface_3: PantoneColor::new("Selection", "dracula selection", "#52556b"),
        border: PantoneColor::new("Comment", "dracula border", "#6272a4"),
        text_primary: PantoneColor::new("Foreground", "dracula fg", "#f8f8f2"),
        text_secondary: PantoneColor::new("Light Dim", "dracula fg_dim", "#e2e2dc"),
        text_muted: PantoneColor::new("Comment", "dracula comment", "#7a88b8"),
        accent: PantoneColor::new("Purple", "dracula purple", "#bd93f9"),
        highlight: PantoneColor::new("Yellow", "dracula yellow", "#f1fa8c"),
        accent_alt: PantoneColor::new("Yellow", "dracula yellow", "#f1fa8c"),
        selection: PantoneColor::new("Selection", "dracula sel", "#44475a"),
        hover: PantoneColor::new("Hover", "dracula hov", "#383a4a"),
        success: PantoneColor::new("Green", "dracula green", "#50fa7b"),
        warn: PantoneColor::new("Orange", "dracula orange", "#ffb86c"),
        error: PantoneColor::new("Red", "dracula red", "#ff5555"),
        tag_code: PantoneColor::new("Cyan", "dracula cyan", "#8be9fd"),
        tag_media: PantoneColor::new("Pink", "dracula pink", "#ff79c6"),
        tag_archive: PantoneColor::new("Yellow", "dracula yellow", "#f1fa8c"),
        tag_document: PantoneColor::new("Red", "dracula red", "#ff5555"),
        tag_image: PantoneColor::new("Green", "dracula green", "#50fa7b"),
    },
    PantoneTheme {
        id: "kanagawa",
        name: "Kanagawa",
        is_dark: true,
        bg: PantoneColor::new("Sumi Ink 3", "kanagawa sumiInk3", "#1f1f28"),
        surface: PantoneColor::new("Sumi Ink 4", "kanagawa sumiInk4", "#2a2a37"),
        surface_1: PantoneColor::new("Sumi Ink 1", "kanagawa sumiInk1", "#16161d"),
        surface_2: PantoneColor::new("Sumi Ink 4", "kanagawa sumiInk4", "#2a2a37"),
        surface_3: PantoneColor::new("Sumi Ink 5", "kanagawa sumiInk5", "#363646"),
        border: PantoneColor::new("Sumi Ink 5", "kanagawa sumiInk5", "#363646"),
        text_primary: PantoneColor::new("Fuji White", "kanagawa fujiWhite", "#dcd7ba"),
        text_secondary: PantoneColor::new("Old White", "kanagawa oldWhite", "#c8c093"),
        text_muted: PantoneColor::new("Fuji Gray", "kanagawa fujiGray", "#727169"),
        accent: PantoneColor::new("Crystal Blue", "kanagawa crystalBlue", "#7e9cd8"),
        highlight: PantoneColor::new("Carp Yellow", "kanagawa carpYellow", "#e6c384"),
        accent_alt: PantoneColor::new("Carp Yellow", "kanagawa carpYellow", "#e6c384"),
        selection: PantoneColor::new("Wave Blue", "kanagawa waveBlue", "#363646"),
        hover: PantoneColor::new("Sumi Ink Hover", "kanagawa hover", "#2d2d3a"),
        success: PantoneColor::new("Spring Green", "kanagawa springGreen", "#98bb6c"),
        warn: PantoneColor::new("Surimi Orange", "kanagawa surimiOrange", "#ffa066"),
        error: PantoneColor::new("Autumn Red", "kanagawa autumnRed", "#e82424"),
        tag_code: PantoneColor::new("Crystal Blue", "kanagawa crystalBlue", "#7e9cd8"),
        tag_media: PantoneColor::new("Oni Violet", "kanagawa oniViolet", "#957fb8"),
        tag_archive: PantoneColor::new("Carp Yellow", "kanagawa carpYellow", "#e6c384"),
        tag_document: PantoneColor::new("Autumn Red", "kanagawa autumnRed", "#e82424"),
        tag_image: PantoneColor::new("Spring Green", "kanagawa springGreen", "#98bb6c"),
    },
    PantoneTheme {
        id: "one_dark",
        name: "One Dark",
        is_dark: true,
        bg: PantoneColor::new("Editor", "one-dark syntax-bg", "#282c34"),
        surface: PantoneColor::new("Card", "one-dark card", "#2c313a"),
        surface_1: PantoneColor::new("Panel", "one-dark panel", "#21252b"),
        surface_2: PantoneColor::new("Card", "one-dark card", "#2c313a"),
        surface_3: PantoneColor::new("Selection", "one-dark selection", "#3e4451"),
        border: PantoneColor::new("Border", "one-dark border", "#3e4451"),
        text_primary: PantoneColor::new("Mono 1", "one-dark mono-1", "#abb2bf"),
        text_secondary: PantoneColor::new("Mono 2", "one-dark mono-2", "#828997"),
        text_muted: PantoneColor::new("Mono 3", "one-dark mono-3", "#6a7384"),
        accent: PantoneColor::new("Blue", "one-dark hue-2", "#61afef"),
        highlight: PantoneColor::new("Yellow", "one-dark hue-6-2", "#e5c07b"),
        accent_alt: PantoneColor::new("Yellow", "one-dark hue-6-2", "#e5c07b"),
        selection: PantoneColor::new("Selection", "one-dark sel", "#3e4451"),
        hover: PantoneColor::new("Hover", "one-dark hov", "#333842"),
        success: PantoneColor::new("Green", "one-dark green", "#98c379"),
        warn: PantoneColor::new("Orange", "one-dark orange", "#d19a66"),
        error: PantoneColor::new("Red", "one-dark red", "#e06c75"),
        tag_code: PantoneColor::new("Blue", "one-dark blue", "#61afef"),
        tag_media: PantoneColor::new("Purple", "one-dark purple", "#c678dd"),
        tag_archive: PantoneColor::new("Yellow", "one-dark yellow", "#e5c07b"),
        tag_document: PantoneColor::new("Red", "one-dark red", "#e06c75"),
        tag_image: PantoneColor::new("Cyan", "one-dark cyan", "#56b6c2"),
    },
    PantoneTheme {
        id: "atom",
        name: "Atom",
        is_dark: true,
        bg: PantoneColor::new("Chrome", "atom tab-bar", "#21252b"),
        surface: PantoneColor::new("Editor", "atom syntax-bg", "#282c34"),
        surface_1: PantoneColor::new("Deep", "atom deep chrome", "#181a1f"),
        surface_2: PantoneColor::new("Editor", "atom syntax-bg", "#282c34"),
        surface_3: PantoneColor::new("Elevated", "atom elevated", "#3a3f4b"),
        border: PantoneColor::new("Border", "atom border", "#3a3f4b"),
        text_primary: PantoneColor::new("Mono 1", "atom mono-1", "#abb2bf"),
        text_secondary: PantoneColor::new("Mono 2", "atom mono-2", "#828997"),
        text_muted: PantoneColor::new("Mono 3", "atom mono-3", "#6a7384"),
        accent: PantoneColor::new("Cursor Blue", "atom syntax-accent", "#528bff"),
        highlight: PantoneColor::new("Orange", "atom hue-6", "#d19a66"),
        accent_alt: PantoneColor::new("Orange", "atom hue-6", "#d19a66"),
        selection: PantoneColor::new("Selection", "atom sel", "#3a3f4b"),
        hover: PantoneColor::new("Hover", "atom hov", "#2c313a"),
        success: PantoneColor::new("Green", "atom green", "#98c379"),
        warn: PantoneColor::new("Yellow", "atom yellow", "#e5c07b"),
        error: PantoneColor::new("Red", "atom red", "#e06c75"),
        tag_code: PantoneColor::new("Blue", "atom blue", "#528bff"),
        tag_media: PantoneColor::new("Purple", "atom purple", "#c678dd"),
        tag_archive: PantoneColor::new("Orange", "atom orange", "#d19a66"),
        tag_document: PantoneColor::new("Red", "atom red", "#e06c75"),
        tag_image: PantoneColor::new("Teal", "atom teal", "#56b6c2"),
    },
    PantoneTheme {
        id: "dark_plus",
        name: "VS Code Dark+",
        is_dark: true,
        bg: PantoneColor::new("Editor", "vscode editor.bg", "#1e1e1e"),
        surface: PantoneColor::new("Card", "vscode card.bg", "#2d2d2d"),
        surface_1: PantoneColor::new("SideBar", "vscode sidebar.bg", "#252526"),
        surface_2: PantoneColor::new("Card", "vscode card.bg", "#2d2d2d"),
        surface_3: PantoneColor::new("Widget", "vscode widget.bg", "#383838"),
        border: PantoneColor::new("Widget Border", "vscode border", "#404040"),
        text_primary: PantoneColor::new("Editor Fg", "vscode editor.fg", "#d4d4d4"),
        text_secondary: PantoneColor::new("Tab Inactive", "vscode tab.dim", "#a6a6a6"),
        text_muted: PantoneColor::new("Line Numbers", "vscode line-numbers", "#858585"),
        accent: PantoneColor::new("Keyword Blue", "vscode keyword blue", "#569cd6"),
        highlight: PantoneColor::new("Function Yellow", "vscode yellow", "#dcdcaa"),
        accent_alt: PantoneColor::new("Function Yellow", "vscode yellow", "#dcdcaa"),
        selection: PantoneColor::new("Selection", "vscode sel", "#094771"),
        hover: PantoneColor::new("List Hover", "vscode hover", "#333333"),
        success: PantoneColor::new("Cyan", "vscode type", "#4ec9b0"),
        warn: PantoneColor::new("Amber", "vscode string", "#ce9178"),
        error: PantoneColor::new("Error Red", "vscode error", "#f44747"),
        tag_code: PantoneColor::new("Blue", "vscode blue", "#569cd6"),
        tag_media: PantoneColor::new("Pink", "vscode pink", "#c586c0"),
        tag_archive: PantoneColor::new("Yellow", "vscode yellow", "#d7ba7d"),
        tag_document: PantoneColor::new("Red", "vscode red", "#f44747"),
        tag_image: PantoneColor::new("Cyan", "vscode cyan", "#4ec9b0"),
    },
    PantoneTheme {
        id: "synthwave",
        name: "Synthwave '84",
        is_dark: true,
        bg: PantoneColor::new("Neon Grid", "synthwave bg", "#262335"),
        surface: PantoneColor::new("Card", "synthwave card", "#2e2942"),
        surface_1: PantoneColor::new("Chrome", "synthwave chrome", "#241b2f"),
        surface_2: PantoneColor::new("Card", "synthwave card", "#2e2942"),
        surface_3: PantoneColor::new("Elevated", "synthwave elevated", "#3a3250"),
        border: PantoneColor::new("Gridline", "synthwave border", "#495495"),
        text_primary: PantoneColor::new("White", "synthwave fg", "#ffffff"),
        text_secondary: PantoneColor::new("Punctuation", "synthwave dim", "#b6b1b1"),
        text_muted: PantoneColor::new("Comment", "synthwave comment", "#848bbd"),
        accent: PantoneColor::new("Neon Pink", "synthwave neon pink", "#ff7edb"),
        highlight: PantoneColor::new("Neon Yellow", "synthwave yellow", "#fede5d"),
        accent_alt: PantoneColor::new("Neon Yellow", "synthwave yellow", "#fede5d"),
        selection: PantoneColor::new("Selection", "synthwave sel", "#495495"),
        hover: PantoneColor::new("Hover", "synthwave hov", "#342c4b"),
        success: PantoneColor::new("Neon Green", "synthwave green", "#72f1b8"),
        warn: PantoneColor::new("Neon Orange", "synthwave orange", "#fede5d"),
        error: PantoneColor::new("Hot Red", "synthwave red", "#fe4450"),
        tag_code: PantoneColor::new("Neon Cyan", "synthwave cyan", "#36f9f6"),
        tag_media: PantoneColor::new("Neon Pink", "synthwave pink", "#ff7edb"),
        tag_archive: PantoneColor::new("Neon Yellow", "synthwave yellow", "#fede5d"),
        tag_document: PantoneColor::new("Hot Red", "synthwave red", "#fe4450"),
        tag_image: PantoneColor::new("Neon Green", "synthwave green", "#72f1b8"),
    },
    PantoneTheme {
        id: "rangalipi",
        name: "Rangalipi",
        is_dark: true,
        bg: PantoneColor::new("Jamuni Night", "mossambi alpona bg", "#342b3f"),
        surface: PantoneColor::new("Bidi Core", "mossambi alpona card", "#3a2e4b"),
        surface_1: PantoneColor::new("Alpona Tray", "mossambi alpona chrome", "#2c2437"),
        surface_2: PantoneColor::new("Bidi Core", "mossambi alpona card", "#3a2e4b"),
        surface_3: PantoneColor::new("Elevated Tray", "mossambi alpona elevated", "#47385a"),
        border: PantoneColor::new("Sindoor Dust", "mossambi alpona border", "#533f66"),
        text_primary: PantoneColor::new("Bidi Parchment", "mossambi alpona text", "#f8f0e5"),
        text_secondary: PantoneColor::new("Madder Rose", "mossambi alpona dim", "#dbc3d7"),
        text_muted: PantoneColor::new("Dusty Orchid", "mossambi alpona muted", "#ab94b7"),
        accent: PantoneColor::new("Sindoor Vermilion", "mossambi alpona accent", "#e4584f"),
        highlight: PantoneColor::new("Holud Turmeric", "mossambi alpona alt", "#e9a23b"),
        accent_alt: PantoneColor::new("Holud Turmeric", "mossambi alpona alt", "#e9a23b"),
        selection: PantoneColor::new("Shyama Glow", "mossambi alpona sel", "#553553"),
        hover: PantoneColor::new("Alpona Hover", "mossambi alpona hov", "#443454"),
        success: PantoneColor::new("Kacha Kola Green", "mossambi alpona green", "#52a972"),
        warn: PantoneColor::new("Holud Turmeric", "mossambi alpona warn", "#e9a23b"),
        error: PantoneColor::new("Sindoor Red", "mossambi alpona err", "#e4584f"),
        tag_code: PantoneColor::new("Patua Leaf", "mossambi tag code", "#689d6a"),
        tag_media: PantoneColor::new("Madder Bloom", "mossambi tag media", "#b16286"),
        tag_archive: PantoneColor::new("Pitambari", "mossambi tag archive", "#d79921"),
        tag_document: PantoneColor::new("Kumkum Red", "mossambi tag doc", "#cc241d"),
        tag_image: PantoneColor::new("Neel Poddo", "mossambi tag image", "#83a598"),
    },
    PantoneTheme {
        id: "mossambi_punch",
        name: "Mossambi Punch",
        is_dark: false,
        bg: PantoneColor::new("Rind Cream", "mossambi fruit bg", "#e6e0cf"),
        surface: PantoneColor::new("Pulp Linen", "mossambi fruit card", "#efe9dc"),
        surface_1: PantoneColor::new("Pith Tint", "mossambi fruit chrome", "#ded7c4"),
        surface_2: PantoneColor::new("Pulp Linen", "mossambi fruit card", "#efe9dc"),
        surface_3: PantoneColor::new("Zest Tray", "mossambi fruit elevated", "#d5ceba"),
        border: PantoneColor::new("Rind Veil", "mossambi fruit border", "#c8bfab"),
        text_primary: PantoneColor::new("Leaf Noir", "mossambi fruit text", "#1f2415"),
        text_secondary: PantoneColor::new("Sprout Olive", "mossambi fruit dim", "#4d5438"),
        text_muted: PantoneColor::new("Pith Grey", "mossambi fruit muted", "#6d7355"),
        accent: PantoneColor::new("Raw Peel Green", "mossambi fruit accent", "#3d7a4f"),
        highlight: PantoneColor::new("Sun Ripe Gold", "mossambi fruit alt", "#c76e05"),
        accent_alt: PantoneColor::new("Sun Ripe Gold", "mossambi fruit alt", "#c76e05"),
        selection: PantoneColor::new("Zest Meadow", "mossambi fruit sel", "#d6dfc5"),
        hover: PantoneColor::new("Citrus Hover", "mossambi fruit hov", "#e5decb"),
        success: PantoneColor::new("Flavedo Crisp", "mossambi fruit green", "#2e7d32"),
        warn: PantoneColor::new("Tangerine Sun", "mossambi fruit warn", "#c76e05"),
        error: PantoneColor::new("Berry Acid", "mossambi fruit err", "#c62828"),
        tag_code: PantoneColor::new("Ocean Teal", "mossambi tag code", "#0284c7"),
        tag_media: PantoneColor::new("Plum Purple", "mossambi tag media", "#7c3aed"),
        tag_archive: PantoneColor::new("Citrus Gold", "mossambi tag archive", "#b45309"),
        tag_document: PantoneColor::new("Crimson Berry", "mossambi tag doc", "#be123c"),
        tag_image: PantoneColor::new("Emerald Zest", "mossambi tag image", "#059669"),
    },
];

/// Counterpart palettes, index-aligned with `THEMES`: the light variant of
/// every dark theme (official upstream light flavours where they exist) and
/// the dark variant of Mossambi Punch, so every theme works in both
/// appearances.
pub static THEMES_ALT: [PantoneTheme; 13] = [
    PantoneTheme {
        id: "gruvbox_light",
        name: "Gruvbox Light",
        is_dark: false,
        bg: PantoneColor::new("Background", "gruvbox light bg", "#fbf1c7"),
        surface: PantoneColor::new("Surface 2", "gruvbox light s2", "#f7ecc5"),
        surface_1: PantoneColor::new("Surface 1", "gruvbox light s1", "#f2e5bc"),
        surface_2: PantoneColor::new("Surface 2", "gruvbox light s2", "#f7ecc5"),
        surface_3: PantoneColor::new("Surface 3", "gruvbox light s3", "#ebdbb2"),
        border: PantoneColor::new("Border", "gruvbox light border", "#d5c4a1"),
        text_primary: PantoneColor::new("Text", "gruvbox light fg", "#3c3836"),
        text_secondary: PantoneColor::new("Text Dim", "gruvbox light dim", "#504945"),
        text_muted: PantoneColor::new("Muted", "gruvbox light muted", "#665c54"),
        accent: PantoneColor::new("Accent", "gruvbox light blue", "#076678"),
        highlight: PantoneColor::new("Highlight", "gruvbox light yellow", "#b57614"),
        accent_alt: PantoneColor::new("Highlight", "gruvbox light yellow", "#b57614"),
        selection: PantoneColor::new("Selection", "gruvbox light sel", "#ebdbb2"),
        hover: PantoneColor::new("Hover", "gruvbox light hov", "#f4ebd0"),
        success: PantoneColor::new("Green", "gruvbox light green", "#79740e"),
        warn: PantoneColor::new("Orange", "gruvbox light orange", "#af3a03"),
        error: PantoneColor::new("Red", "gruvbox light red", "#9d0006"),
        tag_code: PantoneColor::new("Blue", "gruvbox light blue", "#076678"),
        tag_media: PantoneColor::new("Purple", "gruvbox light purple", "#8f3f71"),
        tag_archive: PantoneColor::new("Yellow", "gruvbox light yellow", "#b57614"),
        tag_document: PantoneColor::new("Orange", "gruvbox light orange", "#af3a03"),
        tag_image: PantoneColor::new("Aqua", "gruvbox light aqua", "#427b58"),
    },
    PantoneTheme {
        id: "tokyo_night_day",
        name: "Tokyo Night Day",
        is_dark: false,
        bg: PantoneColor::new("Background", "tokyonight day bg", "#e1e2e7"),
        surface: PantoneColor::new("Surface 2", "tokyonight day s2", "#e2e6ee"),
        surface_1: PantoneColor::new("Surface 1", "tokyonight day s1", "#d5d9e5"),
        surface_2: PantoneColor::new("Surface 2", "tokyonight day s2", "#e2e6ee"),
        surface_3: PantoneColor::new("Surface 3", "tokyonight day s3", "#d5d9e5"),
        border: PantoneColor::new("Border", "tokyonight day border", "#a8aecb"),
        text_primary: PantoneColor::new("Text", "tokyonight day fg", "#2f55ac"),
        text_secondary: PantoneColor::new("Text Dim", "tokyonight day dim", "#4e5a85"),
        text_muted: PantoneColor::new("Muted", "tokyonight day muted", "#69749e"),
        accent: PantoneColor::new("Accent", "tokyonight day blue", "#2e7de9"),
        highlight: PantoneColor::new("Highlight", "tokyonight day yellow", "#8c6c3e"),
        accent_alt: PantoneColor::new("Highlight", "tokyonight day yellow", "#8c6c3e"),
        selection: PantoneColor::new("Selection", "tokyonight day sel", "#b7c5e3"),
        hover: PantoneColor::new("Hover", "tokyonight day hov", "#d8deec"),
        success: PantoneColor::new("Green", "tokyonight day green", "#587539"),
        warn: PantoneColor::new("Orange", "tokyonight day orange", "#b15c00"),
        error: PantoneColor::new("Red", "tokyonight day red", "#f52a65"),
        tag_code: PantoneColor::new("Blue", "tokyonight day blue", "#2e7de9"),
        tag_media: PantoneColor::new("Purple", "tokyonight day purple", "#9854f1"),
        tag_archive: PantoneColor::new("Orange", "tokyonight day orange", "#b15c00"),
        tag_document: PantoneColor::new("Red", "tokyonight day red", "#f52a65"),
        tag_image: PantoneColor::new("Cyan", "tokyonight day cyan", "#387068"),
    },
    PantoneTheme {
        id: "catppuccin_latte",
        name: "Catppuccin Latte",
        is_dark: false,
        bg: PantoneColor::new("Background", "catppuccin latte bg", "#eff1f5"),
        surface: PantoneColor::new("Crust Pure", "catppuccin latte card", "#e6e9ef"),
        surface_1: PantoneColor::new("Mantle", "catppuccin latte mantle", "#e6e9ef"),
        surface_2: PantoneColor::new("Crust Pure", "catppuccin latte card", "#e6e9ef"),
        surface_3: PantoneColor::new("Surface 0", "catppuccin latte s0", "#dce0e8"),
        border: PantoneColor::new("Border", "catppuccin latte border", "#ccd0da"),
        text_primary: PantoneColor::new("Text", "catppuccin latte fg", "#4c4f69"),
        text_secondary: PantoneColor::new("Text Dim", "catppuccin latte dim", "#5c5f77"),
        text_muted: PantoneColor::new("Muted", "catppuccin latte muted", "#6c6f85"),
        accent: PantoneColor::new("Accent", "catppuccin latte blue", "#1e66f5"),
        highlight: PantoneColor::new("Highlight", "catppuccin latte yellow", "#df8e1d"),
        accent_alt: PantoneColor::new("Highlight", "catppuccin latte yellow", "#df8e1d"),
        selection: PantoneColor::new("Selection", "catppuccin latte sel", "#bcc0cc"),
        hover: PantoneColor::new("Hover", "catppuccin latte hov", "#eaecf2"),
        success: PantoneColor::new("Green", "catppuccin latte green", "#40a02b"),
        warn: PantoneColor::new("Peach", "catppuccin latte peach", "#fe640b"),
        error: PantoneColor::new("Red", "catppuccin latte red", "#d20f39"),
        tag_code: PantoneColor::new("Blue", "catppuccin latte blue", "#1e66f5"),
        tag_media: PantoneColor::new("Mauve", "catppuccin latte mauve", "#8839ef"),
        tag_archive: PantoneColor::new("Peach", "catppuccin latte peach", "#fe640b"),
        tag_document: PantoneColor::new("Red", "catppuccin latte red", "#d20f39"),
        tag_image: PantoneColor::new("Teal", "catppuccin latte teal", "#179299"),
    },
    PantoneTheme {
        id: "modus_operandi",
        name: "Modus Operandi",
        is_dark: false,
        bg: PantoneColor::new("Background", "modus-operandi bg", "#f4f1ea"),
        surface: PantoneColor::new("Surface 2", "modus-operandi s2", "#ece7df"),
        surface_1: PantoneColor::new("Surface 1", "modus-operandi s1", "#e5dfd5"),
        surface_2: PantoneColor::new("Surface 2", "modus-operandi s2", "#ece7df"),
        surface_3: PantoneColor::new("Surface 3", "modus-operandi s3", "#e6e6e6"),
        border: PantoneColor::new("Border", "modus-operandi border", "#c4c4c4"),
        text_primary: PantoneColor::new("Text", "modus-operandi fg", "#000000"),
        text_secondary: PantoneColor::new("Text Dim", "modus-operandi dim", "#444444"),
        text_muted: PantoneColor::new("Muted", "modus-operandi muted", "#666666"),
        accent: PantoneColor::new("Accent", "modus-operandi blue", "#0031a9"),
        highlight: PantoneColor::new("Highlight", "modus-operandi yellow", "#6f5500"),
        accent_alt: PantoneColor::new("Highlight", "modus-operandi yellow", "#6f5500"),
        selection: PantoneColor::new("Selection", "modus-operandi sel", "#c8c8ff"),
        hover: PantoneColor::new("Hover", "modus-operandi hov", "#eeeeee"),
        success: PantoneColor::new("Green", "modus-operandi green", "#006800"),
        warn: PantoneColor::new("Yellow", "modus-operandi yellow", "#6f5500"),
        error: PantoneColor::new("Red", "modus-operandi red", "#a60000"),
        tag_code: PantoneColor::new("Blue", "modus-operandi blue", "#0031a9"),
        tag_media: PantoneColor::new("Magenta", "modus-operandi magenta", "#721045"),
        tag_archive: PantoneColor::new("Yellow", "modus-operandi yellow", "#6f5500"),
        tag_document: PantoneColor::new("Red", "modus-operandi red", "#a60000"),
        tag_image: PantoneColor::new("Cyan", "modus-operandi cyan", "#005e8b"),
    },
    PantoneTheme {
        id: "nord_light",
        name: "Nord Light",
        is_dark: false,
        bg: PantoneColor::new("Background", "nord snow storm 0", "#eceff4"),
        surface: PantoneColor::new("Surface 2", "nord card", "#e2e6ee"),
        surface_1: PantoneColor::new("Surface 1", "nord snow storm 1", "#e5e9f0"),
        surface_2: PantoneColor::new("Surface 2", "nord card", "#e2e6ee"),
        surface_3: PantoneColor::new("Surface 3", "nord snow storm 2", "#d8dee9"),
        border: PantoneColor::new("Border", "nord border", "#c2c9d6"),
        text_primary: PantoneColor::new("Text", "nord polar night 0", "#2e3440"),
        text_secondary: PantoneColor::new("Text Dim", "nord polar night 1", "#3b4252"),
        text_muted: PantoneColor::new("Muted", "nord polar night 3", "#616e88"),
        accent: PantoneColor::new("Accent", "nord frost", "#5e81ac"),
        highlight: PantoneColor::new("Highlight", "nord aurora yellow", "#d08770"),
        accent_alt: PantoneColor::new("Highlight", "nord aurora yellow", "#d08770"),
        selection: PantoneColor::new("Selection", "nord sel", "#d8dee9"),
        hover: PantoneColor::new("Hover", "nord hov", "#e6ebf2"),
        success: PantoneColor::new("Green", "nord green", "#4c7a34"),
        warn: PantoneColor::new("Orange", "nord orange", "#b05f28"),
        error: PantoneColor::new("Red", "nord red", "#a63842"),
        tag_code: PantoneColor::new("Blue", "nord blue", "#5e81ac"),
        tag_media: PantoneColor::new("Purple", "nord purple", "#8a4f80"),
        tag_archive: PantoneColor::new("Orange", "nord orange", "#b05f28"),
        tag_document: PantoneColor::new("Red", "nord red", "#a63842"),
        tag_image: PantoneColor::new("Green", "nord green", "#4c7a34"),
    },
    PantoneTheme {
        id: "alucard",
        name: "Alucard",
        is_dark: false,
        bg: PantoneColor::new("Background", "dracula alucard bg", "#f7f2dc"),
        surface: PantoneColor::new("Surface 2", "dracula alucard s2", "#eee6d2"),
        surface_1: PantoneColor::new("Surface 1", "dracula alucard s1", "#f7f2dc"),
        surface_2: PantoneColor::new("Surface 2", "dracula alucard s2", "#eee6d2"),
        surface_3: PantoneColor::new("Surface 3", "dracula alucard s3", "#e8e2d2"),
        border: PantoneColor::new("Border", "dracula alucard border", "#cfcfde"),
        text_primary: PantoneColor::new("Text", "dracula alucard fg", "#1f1f1f"),
        text_secondary: PantoneColor::new("Text Dim", "dracula alucard dim", "#3d3a4f"),
        text_muted: PantoneColor::new("Muted", "dracula alucard muted", "#5a5482"),
        accent: PantoneColor::new("Accent", "dracula alucard purple", "#644ac9"),
        highlight: PantoneColor::new("Highlight", "dracula alucard yellow", "#846e15"),
        accent_alt: PantoneColor::new("Highlight", "dracula alucard yellow", "#846e15"),
        selection: PantoneColor::new("Selection", "dracula alucard sel", "#e2ddce"),
        hover: PantoneColor::new("Hover", "dracula alucard hov", "#ede6d4"),
        success: PantoneColor::new("Green", "dracula alucard green", "#1b7339"),
        warn: PantoneColor::new("Orange", "dracula alucard orange", "#b35900"),
        error: PantoneColor::new("Red", "dracula alucard red", "#c92a2a"),
        tag_code: PantoneColor::new("Cyan", "dracula alucard cyan", "#006ea6"),
        tag_media: PantoneColor::new("Pink", "dracula alucard pink", "#a32272"),
        tag_archive: PantoneColor::new("Yellow", "dracula alucard yellow", "#846e15"),
        tag_document: PantoneColor::new("Red", "dracula alucard red", "#c92a2a"),
        tag_image: PantoneColor::new("Green", "dracula alucard green", "#1b7339"),
    },
    PantoneTheme {
        id: "kanagawa_lotus",
        name: "Kanagawa Lotus",
        is_dark: false,
        bg: PantoneColor::new("Background", "kanagawa lotus bg", "#f2ecbc"),
        surface: PantoneColor::new("Surface 2", "kanagawa lotus s2", "#eae3b2"),
        surface_1: PantoneColor::new("Surface 1", "kanagawa lotus s1", "#e8e1ad"),
        surface_2: PantoneColor::new("Surface 2", "kanagawa lotus s2", "#eae3b2"),
        surface_3: PantoneColor::new("Surface 3", "kanagawa lotus s3", "#dcd5ac"),
        border: PantoneColor::new("Border", "kanagawa lotus border", "#c9c298"),
        text_primary: PantoneColor::new("Text", "kanagawa lotus fg", "#545464"),
        text_secondary: PantoneColor::new("Text Dim", "kanagawa lotus dim", "#43436c"),
        text_muted: PantoneColor::new("Muted", "kanagawa lotus muted", "#62615a"),
        accent: PantoneColor::new("Accent", "kanagawa lotus blue", "#4d699b"),
        highlight: PantoneColor::new("Highlight", "kanagawa lotus yellow", "#de9800"),
        accent_alt: PantoneColor::new("Highlight", "kanagawa lotus yellow", "#de9800"),
        selection: PantoneColor::new("Selection", "kanagawa lotus sel", "#dcd5ac"),
        hover: PantoneColor::new("Hover", "kanagawa lotus hov", "#e5deab"),
        success: PantoneColor::new("Green", "kanagawa lotus green", "#4a7833"),
        warn: PantoneColor::new("Orange", "kanagawa lotus orange", "#c85e0b"),
        error: PantoneColor::new("Red", "kanagawa lotus red", "#b82424"),
        tag_code: PantoneColor::new("Blue", "kanagawa lotus blue", "#4d699b"),
        tag_media: PantoneColor::new("Violet", "kanagawa lotus violet", "#73568c"),
        tag_archive: PantoneColor::new("Yellow", "kanagawa lotus yellow", "#de9800"),
        tag_document: PantoneColor::new("Red", "kanagawa lotus red", "#b82424"),
        tag_image: PantoneColor::new("Green", "kanagawa lotus green", "#4a7833"),
    },
    PantoneTheme {
        id: "one_light",
        name: "One Light",
        is_dark: false,
        bg: PantoneColor::new("Background", "one-light bg", "#eeebe6"),
        surface: PantoneColor::new("Surface 2", "one-light s2", "#e6e2dc"),
        surface_1: PantoneColor::new("Surface 1", "one-light s1", "#dedad4"),
        surface_2: PantoneColor::new("Surface 2", "one-light s2", "#e6e2dc"),
        surface_3: PantoneColor::new("Surface 3", "one-light s3", "#d6d2cc"),
        border: PantoneColor::new("Border", "one-light border", "#c8c4be"),
        text_primary: PantoneColor::new("Text", "one-light fg", "#383a42"),
        text_secondary: PantoneColor::new("Text Dim", "one-light dim", "#4f525c"),
        text_muted: PantoneColor::new("Muted", "one-light muted", "#696c77"),
        accent: PantoneColor::new("Accent", "one-light blue", "#4078f2"),
        highlight: PantoneColor::new("Highlight", "one-light yellow", "#c18401"),
        accent_alt: PantoneColor::new("Highlight", "one-light yellow", "#c18401"),
        selection: PantoneColor::new("Selection", "one-light sel", "#d6d2cc"),
        hover: PantoneColor::new("Hover", "one-light hov", "#ddd9d3"),
        success: PantoneColor::new("Green", "one-light green", "#50a14f"),
        warn: PantoneColor::new("Orange", "one-light orange", "#986801"),
        error: PantoneColor::new("Red", "one-light red", "#e45649"),
        tag_code: PantoneColor::new("Blue", "one-light blue", "#4078f2"),
        tag_media: PantoneColor::new("Purple", "one-light purple", "#a626a4"),
        tag_archive: PantoneColor::new("Yellow", "one-light yellow", "#c18401"),
        tag_document: PantoneColor::new("Red", "one-light red", "#e45649"),
        tag_image: PantoneColor::new("Cyan", "one-light cyan", "#0184bc"),
    },
    PantoneTheme {
        id: "atom_light",
        name: "Atom Light",
        is_dark: false,
        bg: PantoneColor::new("Background", "atom light bg", "#e2dfdb"),
        surface: PantoneColor::new("Surface 2", "atom light s2", "#dad7d2"),
        surface_1: PantoneColor::new("Surface 1", "atom light s1", "#d2cfca"),
        surface_2: PantoneColor::new("Surface 2", "atom light s2", "#dad7d2"),
        surface_3: PantoneColor::new("Surface 3", "atom light s3", "#cac7c2"),
        border: PantoneColor::new("Border", "atom light border", "#bebbb6"),
        text_primary: PantoneColor::new("Text", "atom light fg", "#383a42"),
        text_secondary: PantoneColor::new("Text Dim", "atom light dim", "#4f525c"),
        text_muted: PantoneColor::new("Muted", "atom light muted", "#696c77"),
        accent: PantoneColor::new("Accent", "atom light blue", "#526fff"),
        highlight: PantoneColor::new("Highlight", "atom light yellow", "#986801"),
        accent_alt: PantoneColor::new("Highlight", "atom light yellow", "#986801"),
        selection: PantoneColor::new("Selection", "atom light sel", "#cac7c2"),
        hover: PantoneColor::new("Hover", "atom light hov", "#d0cdc8"),
        success: PantoneColor::new("Green", "atom light green", "#50a14f"),
        warn: PantoneColor::new("Orange", "atom light orange", "#986801"),
        error: PantoneColor::new("Red", "atom light red", "#e45649"),
        tag_code: PantoneColor::new("Blue", "atom light blue", "#526fff"),
        tag_media: PantoneColor::new("Purple", "atom light purple", "#a626a4"),
        tag_archive: PantoneColor::new("Orange", "atom light orange", "#986801"),
        tag_document: PantoneColor::new("Red", "atom light red", "#e45649"),
        tag_image: PantoneColor::new("Teal", "atom light teal", "#0184bc"),
    },
    PantoneTheme {
        id: "light_plus",
        name: "VS Code Light+",
        is_dark: false,
        bg: PantoneColor::new("Background", "vscode light+ bg", "#f3efe6"),
        surface: PantoneColor::new("Card", "vscode light+ s2", "#ebe6dc"),
        surface_1: PantoneColor::new("SideBar", "vscode light+ s1", "#e2ddd2"),
        surface_2: PantoneColor::new("Card", "vscode light+ s2", "#ebe6dc"),
        surface_3: PantoneColor::new("Widget", "vscode light+ s3", "#d8d3c8"),
        border: PantoneColor::new("Border", "vscode light+ border", "#cfc9be"),
        text_primary: PantoneColor::new("Text", "vscode light+ fg", "#1f1f1f"),
        text_secondary: PantoneColor::new("Text Dim", "vscode light+ dim", "#484848"),
        text_muted: PantoneColor::new("Muted", "vscode light+ muted", "#616161"),
        accent: PantoneColor::new("Accent", "vscode light+ blue", "#005fb8"),
        highlight: PantoneColor::new("Highlight", "vscode light+ yellow", "#795e26"),
        accent_alt: PantoneColor::new("Highlight", "vscode light+ yellow", "#795e26"),
        selection: PantoneColor::new("Selection", "vscode light+ sel", "#add6ff"),
        hover: PantoneColor::new("Hover", "vscode light+ hov", "#e0dbd1"),
        success: PantoneColor::new("Green", "vscode light+ green", "#107c41"),
        warn: PantoneColor::new("Orange", "vscode light+ orange", "#b85c00"),
        error: PantoneColor::new("Red", "vscode light+ red", "#cd3131"),
        tag_code: PantoneColor::new("Blue", "vscode light+ blue", "#005fb8"),
        tag_media: PantoneColor::new("Purple", "vscode light+ purple", "#af00db"),
        tag_archive: PantoneColor::new("Yellow", "vscode light+ yellow", "#795e26"),
        tag_document: PantoneColor::new("Red", "vscode light+ red", "#cd3131"),
        tag_image: PantoneColor::new("Cyan", "vscode light+ cyan", "#0078d4"),
    },
    PantoneTheme {
        id: "synthwave_day",
        name: "Synthwave '84 Day",
        is_dark: false,
        bg: PantoneColor::new("Background", "synthwave day bg", "#f8f1ff"),
        surface: PantoneColor::new("Surface 2", "synthwave day s2", "#efe6fa"),
        surface_1: PantoneColor::new("Surface 1", "synthwave day s1", "#ede3f8"),
        surface_2: PantoneColor::new("Surface 2", "synthwave day s2", "#efe6fa"),
        surface_3: PantoneColor::new("Surface 3", "synthwave day s3", "#e8dcf5"),
        border: PantoneColor::new("Border", "synthwave day border", "#d4c2e8"),
        text_primary: PantoneColor::new("Text", "synthwave day fg", "#2a2139"),
        text_secondary: PantoneColor::new("Text Dim", "synthwave day dim", "#473c5c"),
        text_muted: PantoneColor::new("Muted", "synthwave day muted", "#665882"),
        accent: PantoneColor::new("Accent", "synthwave day pink", "#d13fb0"),
        highlight: PantoneColor::new("Highlight", "synthwave day yellow", "#b38600"),
        accent_alt: PantoneColor::new("Highlight", "synthwave day yellow", "#b38600"),
        selection: PantoneColor::new("Selection", "synthwave day sel", "#e2d5f0"),
        hover: PantoneColor::new("Hover", "synthwave day hov", "#f2e8fc"),
        success: PantoneColor::new("Green", "synthwave day green", "#129468"),
        warn: PantoneColor::new("Orange", "synthwave day orange", "#b38600"),
        error: PantoneColor::new("Red", "synthwave day red", "#cf2038"),
        tag_code: PantoneColor::new("Cyan", "synthwave day cyan", "#088c94"),
        tag_media: PantoneColor::new("Pink", "synthwave day pink", "#d13fb0"),
        tag_archive: PantoneColor::new("Yellow", "synthwave day yellow", "#b38600"),
        tag_document: PantoneColor::new("Red", "synthwave day red", "#cf2038"),
        tag_image: PantoneColor::new("Green", "synthwave day green", "#129468"),
    },
    PantoneTheme {
        id: "rangalipi_day",
        name: "Rangalipi Day",
        is_dark: false,
        bg: PantoneColor::new("Background", "rangalipi day bg", "#fbf4ee"),
        surface: PantoneColor::new("Surface 2", "rangalipi day s2", "#f2e7de"),
        surface_1: PantoneColor::new("Surface 1", "rangalipi day s1", "#ece0d6"),
        surface_2: PantoneColor::new("Surface 2", "rangalipi day s2", "#f2e7de"),
        surface_3: PantoneColor::new("Surface 3", "rangalipi day s3", "#e4d6cc"),
        border: PantoneColor::new("Border", "rangalipi day border", "#dec6bf"),
        text_primary: PantoneColor::new("Text", "rangalipi day fg", "#2b1e33"),
        text_secondary: PantoneColor::new("Text Dim", "rangalipi day dim", "#523a4e"),
        text_muted: PantoneColor::new("Muted", "rangalipi day muted", "#73546e"),
        accent: PantoneColor::new("Accent", "rangalipi day red", "#c8423a"),
        highlight: PantoneColor::new("Highlight", "rangalipi day gold", "#c07a10"),
        accent_alt: PantoneColor::new("Highlight", "rangalipi day gold", "#c07a10"),
        selection: PantoneColor::new("Selection", "rangalipi day sel", "#e8d5ce"),
        hover: PantoneColor::new("Hover", "rangalipi day hov", "#ebdcd2"),
        success: PantoneColor::new("Green", "rangalipi day green", "#35804e"),
        warn: PantoneColor::new("Orange", "rangalipi day warn", "#c07a10"),
        error: PantoneColor::new("Red", "rangalipi day err", "#c8423a"),
        tag_code: PantoneColor::new("Leaf", "rangalipi day code", "#427848"),
        tag_media: PantoneColor::new("Madder", "rangalipi day media", "#8e4566"),
        tag_archive: PantoneColor::new("Pitambari", "rangalipi day archive", "#b87814"),
        tag_document: PantoneColor::new("Kumkum", "rangalipi day doc", "#a32620"),
        tag_image: PantoneColor::new("Poddo", "rangalipi day image", "#487568"),
    },
    PantoneTheme {
        id: "mossambi_punch_night",
        name: "Mossambi Punch Night",
        is_dark: true,
        bg: PantoneColor::new("Background", "punch night bg", "#1b2116"),
        surface: PantoneColor::new("Surface 2", "punch night s2", "#242c1d"),
        surface_1: PantoneColor::new("Surface 1", "punch night s1", "#161c12"),
        surface_2: PantoneColor::new("Surface 2", "punch night s2", "#242c1d"),
        surface_3: PantoneColor::new("Surface 3", "punch night s3", "#2f3926"),
        border: PantoneColor::new("Border", "punch night border", "#354029"),
        text_primary: PantoneColor::new("Text", "punch night fg", "#eef2dc"),
        text_secondary: PantoneColor::new("Text Dim", "punch night dim", "#c3cc9e"),
        text_muted: PantoneColor::new("Muted", "punch night muted", "#85906a"),
        accent: PantoneColor::new("Accent", "punch night lime", "#8fc24f"),
        highlight: PantoneColor::new("Highlight", "punch night gold", "#e5b92c"),
        accent_alt: PantoneColor::new("Highlight", "punch night gold", "#e5b92c"),
        selection: PantoneColor::new("Selection", "punch night sel", "#38462d"),
        hover: PantoneColor::new("Hover", "punch night hov", "#2c3623"),
        success: PantoneColor::new("Green", "punch night green", "#68a635"),
        warn: PantoneColor::new("Orange", "punch night warn", "#e5b92c"),
        error: PantoneColor::new("Red", "punch night red", "#e04f44"),
        tag_code: PantoneColor::new("Blue", "punch night code", "#5cb3e6"),
        tag_media: PantoneColor::new("Purple", "punch night media", "#b37aeb"),
        tag_archive: PantoneColor::new("Gold", "punch night archive", "#e5b92c"),
        tag_document: PantoneColor::new("Red", "punch night doc", "#e04f44"),
        tag_image: PantoneColor::new("Green", "punch night image", "#68a635"),
    },
];

/// Returns all 13 named catalog base themes in catalog order
pub fn all_pantone_themes() -> &'static [PantoneTheme] {
    &THEMES
}

/// Returns all 26 named themes (13 Dark + 13 Eye-Care Light counterparts)
pub fn all_26_pantone_themes() -> Vec<&'static PantoneTheme> {
    THEMES.iter().chain(THEMES_ALT.iter()).collect()
}

/// Resolves a PantoneTheme by its identifier or human-readable name across all 26 themes
pub fn get_pantone_theme(id_or_name: &str) -> &'static PantoneTheme {
    let query = id_or_name.to_lowercase().replace([' ', '-', '_'], "");
    for theme in THEMES.iter().chain(THEMES_ALT.iter()) {
        let tid = theme.id.to_lowercase().replace([' ', '-', '_'], "");
        let tname = theme.name.to_lowercase().replace([' ', '-', '_'], "");
        if tid == query || tname == query {
            return theme;
        }
    }
    // Backwards compatibility fallbacks
    if query.contains("light") || query.contains("aqua") || query.contains("white") {
        &THEMES[12] // Mossambi Punch - the catalog light theme
    } else {
        &THEMES[11] // Rangalipi - the default dark theme
    }
}

#[cfg(test)]
mod alt_theme_tests {
    use super::*;

    #[test]
    fn every_theme_has_an_opposite_appearance_counterpart() {
        for (base, alt) in THEMES.iter().zip(THEMES_ALT.iter()) {
            assert_ne!(base.is_dark, alt.is_dark, "{} / {}", base.name, alt.name);
            for c in [
                alt.bg,
                alt.surface,
                alt.surface_1,
                alt.surface_2,
                alt.surface_3,
                alt.border,
                alt.text_primary,
                alt.text_secondary,
                alt.text_muted,
                alt.accent,
                alt.accent_alt,
                alt.selection,
                alt.hover,
                alt.success,
                alt.warn,
                alt.error,
                alt.tag_code,
                alt.tag_media,
                alt.tag_archive,
                alt.tag_document,
                alt.tag_image,
            ] {
                assert_eq!(c.hex.len(), 7, "{} role {}", alt.name, c.name);
                assert!(c.hex.starts_with('#'), "hex must start with #");
            }
        }
    }

    #[test]
    fn every_theme_meets_wcag_contrast_floors() {
        for t in THEMES.iter().chain(THEMES_ALT.iter()) {
            // Normal text contrast >= 4.5:1 against surface_2 (cards) and surface_3 (popovers)
            let cr_surf2 = t.text_primary.contrast_ratio(&t.surface_2);
            assert!(
                cr_surf2 >= 4.5,
                "Theme '{}' text_primary ({}) vs surface_2 ({}) contrast ratio {:.2} < 4.5",
                t.name,
                t.text_primary.hex,
                t.surface_2.hex,
                cr_surf2
            );

            let cr_surf3 = t.text_primary.contrast_ratio(&t.surface_3);
            assert!(
                cr_surf3 >= 4.5,
                "Theme '{}' text_primary ({}) vs surface_3 ({}) contrast ratio {:.2} < 4.5",
                t.name,
                t.text_primary.hex,
                t.surface_3.hex,
                cr_surf3
            );

            // Secondary text contrast >= 3.0:1 against surface_2
            let cr_sec = t.text_secondary.contrast_ratio(&t.surface_2);
            assert!(
                cr_sec >= 3.0,
                "Theme '{}' text_secondary ({}) vs surface_2 ({}) contrast ratio {:.2} < 3.0",
                t.name,
                t.text_secondary.hex,
                t.surface_2.hex,
                cr_sec
            );
        }
    }
}
