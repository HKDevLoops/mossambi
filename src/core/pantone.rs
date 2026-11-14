//! Theme System for Mossambi - 13 Named Palettes
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
//! The `code` field of each PantoneColor records the source provenance of
//! the value (for example "tokyonight blue"). The full research report
//! with upstream sources lives at:
//! docs/superpowers/plans/2026-11-12-theme-research.md

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
    pub border: PantoneColor,
    pub text_primary: PantoneColor,
    pub text_secondary: PantoneColor,
    pub text_muted: PantoneColor,
    pub accent: PantoneColor,
    pub highlight: PantoneColor,
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
        border: PantoneColor::new("Dark2", "gruvbox bg2", "#504945"),
        text_primary: PantoneColor::new("Foreground", "gruvbox fg1", "#ebdbb2"),
        text_secondary: PantoneColor::new("Foreground2", "gruvbox fg2", "#d5c4a1"),
        text_muted: PantoneColor::new("Gray", "gruvbox gray", "#928374"),
        accent: PantoneColor::new("Blue", "gruvbox blue", "#83a598"),
        highlight: PantoneColor::new("Yellow", "gruvbox yellow", "#fabd2f"),
    },
    PantoneTheme {
        id: "tokyo_night",
        name: "Tokyo Night",
        is_dark: true,
        bg: PantoneColor::new("Night", "tokyonight bg", "#1a1b26"),
        surface: PantoneColor::new("Highlight", "tokyonight bg_highlight", "#292e42"),
        border: PantoneColor::new("Gutter", "tokyonight fg_gutter", "#3b4261"),
        text_primary: PantoneColor::new("Foreground", "tokyonight fg", "#c0caf5"),
        text_secondary: PantoneColor::new("Foreground Dark", "tokyonight fg_dark", "#a9b1d6"),
        text_muted: PantoneColor::new("Comment", "tokyonight comment", "#565f89"),
        accent: PantoneColor::new("Blue", "tokyonight blue", "#7aa2f7"),
        highlight: PantoneColor::new("Yellow", "tokyonight yellow", "#e0af68"),
    },
    PantoneTheme {
        id: "catppuccin_mocha",
        name: "Catppuccin Mocha",
        is_dark: true,
        bg: PantoneColor::new("Base", "catppuccin base", "#1e1e2e"),
        surface: PantoneColor::new("Surface0", "catppuccin surface0", "#313244"),
        border: PantoneColor::new("Surface1", "catppuccin surface1", "#45475a"),
        text_primary: PantoneColor::new("Text", "catppuccin text", "#cdd6f4"),
        text_secondary: PantoneColor::new("Subtext0", "catppuccin subtext0", "#a6adc8"),
        text_muted: PantoneColor::new("Overlay0", "catppuccin overlay0", "#6c7086"),
        accent: PantoneColor::new("Blue", "catppuccin blue", "#89b4fa"),
        highlight: PantoneColor::new("Yellow", "catppuccin yellow", "#f9e2af"),
    },
    PantoneTheme {
        id: "modus_vivendi",
        name: "Modus Vivendi",
        is_dark: true,
        bg: PantoneColor::new("Main", "modus-vivendi bg-main", "#000000"),
        surface: PantoneColor::new("Dim", "modus-vivendi bg-dim", "#1e1e1e"),
        border: PantoneColor::new("Inactive", "modus-vivendi bg-inactive", "#303030"),
        text_primary: PantoneColor::new("Main", "modus-vivendi fg-main", "#ffffff"),
        text_secondary: PantoneColor::new("Dim", "modus-vivendi fg-dim", "#989898"),
        text_muted: PantoneColor::new("Border", "modus-vivendi border", "#646464"),
        accent: PantoneColor::new("Blue", "modus-vivendi blue", "#2fafff"),
        highlight: PantoneColor::new("Yellow", "modus-vivendi yellow", "#d0bc00"),
    },
    PantoneTheme {
        id: "nord",
        name: "Nord",
        is_dark: true,
        bg: PantoneColor::new("Polar Night 0", "nord nord0", "#2e3440"),
        surface: PantoneColor::new("Polar Night 1", "nord nord1", "#3b4252"),
        border: PantoneColor::new("Polar Night 2", "nord nord2", "#434c5e"),
        text_primary: PantoneColor::new("Snow Storm 2", "nord nord6", "#eceff4"),
        text_secondary: PantoneColor::new("Snow Storm 1", "nord nord4", "#d8dee9"),
        text_muted: PantoneColor::new("Polar Night 3", "nord nord3", "#4c566a"),
        accent: PantoneColor::new("Frost", "nord nord8", "#88c0d0"),
        highlight: PantoneColor::new("Aurora Yellow", "nord nord13", "#ebcb8b"),
    },
    PantoneTheme {
        id: "dracula",
        name: "Dracula",
        is_dark: true,
        bg: PantoneColor::new("Background", "dracula background", "#282a36"),
        surface: PantoneColor::new("Current Line", "dracula current-line", "#44475a"),
        border: PantoneColor::new("Comment", "dracula comment", "#6272a4"),
        text_primary: PantoneColor::new("Foreground", "dracula fg", "#f8f8f2"),
        text_secondary: PantoneColor::new("Comment", "dracula comment", "#6272a4"),
        text_muted: PantoneColor::new("Comment", "dracula comment", "#6272a4"),
        accent: PantoneColor::new("Purple", "dracula purple", "#bd93f9"),
        highlight: PantoneColor::new("Yellow", "dracula yellow", "#f1fa8c"),
    },
    PantoneTheme {
        id: "kanagawa",
        name: "Kanagawa",
        is_dark: true,
        bg: PantoneColor::new("Sumi Ink 3", "kanagawa sumiInk3", "#1f1f28"),
        surface: PantoneColor::new("Sumi Ink 4", "kanagawa sumiInk4", "#2a2a37"),
        border: PantoneColor::new("Sumi Ink 5", "kanagawa sumiInk5", "#363646"),
        text_primary: PantoneColor::new("Fuji White", "kanagawa fujiWhite", "#dcd7ba"),
        text_secondary: PantoneColor::new("Old White", "kanagawa oldWhite", "#c8c093"),
        text_muted: PantoneColor::new("Fuji Gray", "kanagawa fujiGray", "#727169"),
        accent: PantoneColor::new("Crystal Blue", "kanagawa crystalBlue", "#7e9cd8"),
        highlight: PantoneColor::new("Carp Yellow", "kanagawa carpYellow", "#e6c384"),
    },
    PantoneTheme {
        id: "one_dark",
        name: "One Dark",
        is_dark: true,
        bg: PantoneColor::new("Editor", "one-dark syntax-bg", "#282c34"),
        surface: PantoneColor::new("Selection", "one-dark selection", "#3e4451"),
        border: PantoneColor::new("Panel", "one-dark panel", "#21252b"),
        text_primary: PantoneColor::new("Mono 1", "one-dark mono-1", "#abb2bf"),
        text_secondary: PantoneColor::new("Mono 2", "one-dark mono-2", "#828997"),
        text_muted: PantoneColor::new("Mono 3", "one-dark mono-3", "#5c6370"),
        accent: PantoneColor::new("Blue", "one-dark hue-2", "#61afef"),
        highlight: PantoneColor::new("Yellow", "one-dark hue-6-2", "#e5c07b"),
    },
    PantoneTheme {
        id: "atom",
        name: "Atom",
        is_dark: true,
        bg: PantoneColor::new("Chrome", "atom tab-bar", "#21252b"),
        surface: PantoneColor::new("Editor", "atom syntax-bg", "#282c34"),
        border: PantoneColor::new("Deep", "atom deep chrome", "#181a1f"),
        text_primary: PantoneColor::new("Mono 1", "atom mono-1", "#abb2bf"),
        text_secondary: PantoneColor::new("Mono 2", "atom mono-2", "#828997"),
        text_muted: PantoneColor::new("Mono 3", "atom mono-3", "#5c6370"),
        accent: PantoneColor::new("Cursor Blue", "atom syntax-accent", "#528bff"),
        highlight: PantoneColor::new("Orange", "atom hue-6", "#d19a66"),
    },
    PantoneTheme {
        id: "dark_plus",
        name: "VS Code Dark+",
        is_dark: true,
        bg: PantoneColor::new("Editor", "vscode editor.background", "#1e1e1e"),
        surface: PantoneColor::new("Menu", "vscode menu.background", "#252526"),
        border: PantoneColor::new("Widget", "vscode widget border", "#303031"),
        text_primary: PantoneColor::new("Editor Fg", "vscode editor.foreground", "#d4d4d4"),
        text_secondary: PantoneColor::new(
            "Tab Inactive",
            "vscode tab.inactiveForeground",
            "#a6a6a6",
        ),
        text_muted: PantoneColor::new("Line Numbers", "vscode line numbers", "#858585"),
        accent: PantoneColor::new("Keyword Blue", "vscode keyword blue", "#569cd6"),
        highlight: PantoneColor::new("Function Yellow", "vscode function yellow", "#dcdcaa"),
    },
    PantoneTheme {
        id: "synthwave",
        name: "Synthwave '84",
        is_dark: true,
        bg: PantoneColor::new("Neon Grid", "synthwave editor.background", "#262335"),
        surface: PantoneColor::new("Chrome", "synthwave sideBar.background", "#241b2f"),
        border: PantoneColor::new("Gridline", "synthwave editorGroup.border", "#495495"),
        text_primary: PantoneColor::new("White", "synthwave foreground", "#ffffff"),
        text_secondary: PantoneColor::new("Punctuation", "synthwave punctuation", "#b6b1b1"),
        text_muted: PantoneColor::new("Comment", "synthwave comment", "#848bbd"),
        accent: PantoneColor::new("Neon Pink", "synthwave neon pink", "#ff7edb"),
        highlight: PantoneColor::new("Neon Yellow", "synthwave neon yellow", "#fede5d"),
    },
    PantoneTheme {
        id: "rangalipi",
        name: "Rangalipi",
        is_dark: true,
        bg: PantoneColor::new("Jamuni Night", "mossambi original alpona", "#1e1726"),
        surface: PantoneColor::new("Alpona Tray", "mossambi original alpona", "#271d33"),
        border: PantoneColor::new("Sindoor Dust", "mossambi original alpona", "#3c2b49"),
        text_primary: PantoneColor::new("Bidi Parchment", "mossambi original alpona", "#f2e9dc"),
        text_secondary: PantoneColor::new("Madder Rose", "mossambi original alpona", "#c7aec3"),
        text_muted: PantoneColor::new("Dusty Orchid", "mossambi original alpona", "#8e7699"),
        accent: PantoneColor::new("Sindoor Vermilion", "mossambi original alpona", "#e4584f"),
        highlight: PantoneColor::new("Holud Turmeric", "mossambi original alpona", "#e9a23b"),
    },
    PantoneTheme {
        id: "mossambi_punch",
        name: "Mossambi Punch",
        is_dark: false,
        bg: PantoneColor::new("Rind Cream", "mossambi original fruit", "#f8f7e6"),
        surface: PantoneColor::new("Pulp Ivory", "mossambi original fruit", "#ffffff"),
        border: PantoneColor::new("Rind Veil", "mossambi original fruit", "#e4e4c9"),
        text_primary: PantoneColor::new("Leaf Noir", "mossambi original fruit", "#26301c"),
        text_secondary: PantoneColor::new("Sprout Olive", "mossambi original fruit", "#5a6a44"),
        text_muted: PantoneColor::new("Pith Grey", "mossambi original fruit", "#97a183"),
        accent: PantoneColor::new("Zest Green", "mossambi original fruit", "#5c8b2f"),
        highlight: PantoneColor::new("Sun Ripe", "mossambi original fruit", "#e5b92c"),
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
        bg: PantoneColor::new("Background", "gruvbox light", "#fbf1c7"),
        surface: PantoneColor::new("Surface", "gruvbox light", "#ebdbb2"),
        border: PantoneColor::new("Border", "gruvbox light", "#d5c4a1"),
        text_primary: PantoneColor::new("Text", "gruvbox light", "#3c3836"),
        text_secondary: PantoneColor::new("Text Dim", "gruvbox light", "#504945"),
        text_muted: PantoneColor::new("Muted", "gruvbox light", "#928374"),
        accent: PantoneColor::new("Accent", "gruvbox light", "#076678"),
        highlight: PantoneColor::new("Highlight", "gruvbox light", "#b57614"),
    },
    PantoneTheme {
        id: "tokyo_night_day",
        name: "Tokyo Night Day",
        is_dark: false,
        bg: PantoneColor::new("Background", "tokyonight day", "#e1e2e7"),
        surface: PantoneColor::new("Surface", "tokyonight day", "#d0d5e3"),
        border: PantoneColor::new("Border", "tokyonight day", "#a8aecb"),
        text_primary: PantoneColor::new("Text", "tokyonight day", "#3760bf"),
        text_secondary: PantoneColor::new("Text Dim", "tokyonight day", "#6172b0"),
        text_muted: PantoneColor::new("Muted", "tokyonight day", "#848cb5"),
        accent: PantoneColor::new("Accent", "tokyonight day", "#2e7de9"),
        highlight: PantoneColor::new("Highlight", "tokyonight day", "#8c6c3e"),
    },
    PantoneTheme {
        id: "catppuccin_latte",
        name: "Catppuccin Latte",
        is_dark: false,
        bg: PantoneColor::new("Background", "catppuccin latte", "#eff1f5"),
        surface: PantoneColor::new("Surface", "catppuccin latte", "#e6e9ef"),
        border: PantoneColor::new("Border", "catppuccin latte", "#ccd0da"),
        text_primary: PantoneColor::new("Text", "catppuccin latte", "#4c4f69"),
        text_secondary: PantoneColor::new("Text Dim", "catppuccin latte", "#6c6f85"),
        text_muted: PantoneColor::new("Muted", "catppuccin latte", "#9ca0b0"),
        accent: PantoneColor::new("Accent", "catppuccin latte", "#1e66f5"),
        highlight: PantoneColor::new("Highlight", "catppuccin latte", "#df8e1d"),
    },
    PantoneTheme {
        id: "modus_operandi",
        name: "Modus Operandi",
        is_dark: false,
        bg: PantoneColor::new("Background", "modus-operandi", "#ffffff"),
        surface: PantoneColor::new("Surface", "modus-operandi", "#f2f2f2"),
        border: PantoneColor::new("Border", "modus-operandi", "#c4c4c4"),
        text_primary: PantoneColor::new("Text", "modus-operandi", "#000000"),
        text_secondary: PantoneColor::new("Text Dim", "modus-operandi", "#595959"),
        text_muted: PantoneColor::new("Muted", "modus-operandi", "#7a7a7a"),
        accent: PantoneColor::new("Accent", "modus-operandi", "#0031a9"),
        highlight: PantoneColor::new("Highlight", "modus-operandi", "#6f5500"),
    },
    PantoneTheme {
        id: "nord_light",
        name: "Nord Light",
        is_dark: false,
        bg: PantoneColor::new("Background", "nord snow storm", "#eceff4"),
        surface: PantoneColor::new("Surface", "nord snow storm", "#e5e9f0"),
        border: PantoneColor::new("Border", "nord snow storm", "#d8dee9"),
        text_primary: PantoneColor::new("Text", "nord snow storm", "#2e3440"),
        text_secondary: PantoneColor::new("Text Dim", "nord snow storm", "#3b4252"),
        text_muted: PantoneColor::new("Muted", "nord snow storm", "#7b88a1"),
        accent: PantoneColor::new("Accent", "nord snow storm", "#5e81ac"),
        highlight: PantoneColor::new("Highlight", "nord snow storm", "#d08770"),
    },
    PantoneTheme {
        id: "alucard",
        name: "Alucard",
        is_dark: false,
        bg: PantoneColor::new("Background", "dracula alucard", "#fffbeb"),
        surface: PantoneColor::new("Surface", "dracula alucard", "#f3eee0"),
        border: PantoneColor::new("Border", "dracula alucard", "#cfcfde"),
        text_primary: PantoneColor::new("Text", "dracula alucard", "#1f1f1f"),
        text_secondary: PantoneColor::new("Text Dim", "dracula alucard", "#4d4a63"),
        text_muted: PantoneColor::new("Muted", "dracula alucard", "#635d97"),
        accent: PantoneColor::new("Accent", "dracula alucard", "#644ac9"),
        highlight: PantoneColor::new("Highlight", "dracula alucard", "#846e15"),
    },
    PantoneTheme {
        id: "kanagawa_lotus",
        name: "Kanagawa Lotus",
        is_dark: false,
        bg: PantoneColor::new("Background", "kanagawa lotus", "#f2ecbc"),
        surface: PantoneColor::new("Surface", "kanagawa lotus", "#e7dba0"),
        border: PantoneColor::new("Border", "kanagawa lotus", "#dcd5ac"),
        text_primary: PantoneColor::new("Text", "kanagawa lotus", "#545464"),
        text_secondary: PantoneColor::new("Text Dim", "kanagawa lotus", "#43436c"),
        text_muted: PantoneColor::new("Muted", "kanagawa lotus", "#8a8980"),
        accent: PantoneColor::new("Accent", "kanagawa lotus", "#4d699b"),
        highlight: PantoneColor::new("Highlight", "kanagawa lotus", "#de9800"),
    },
    PantoneTheme {
        id: "one_light",
        name: "One Light",
        is_dark: false,
        bg: PantoneColor::new("Background", "one-light syntax", "#fafafa"),
        surface: PantoneColor::new("Surface", "one-light syntax", "#f0f0f1"),
        border: PantoneColor::new("Border", "one-light syntax", "#e5e5e6"),
        text_primary: PantoneColor::new("Text", "one-light syntax", "#383a42"),
        text_secondary: PantoneColor::new("Text Dim", "one-light syntax", "#696c77"),
        text_muted: PantoneColor::new("Muted", "one-light syntax", "#a0a1a7"),
        accent: PantoneColor::new("Accent", "one-light syntax", "#4078f2"),
        highlight: PantoneColor::new("Highlight", "one-light syntax", "#c18401"),
    },
    PantoneTheme {
        id: "atom_light",
        name: "Atom Light",
        is_dark: false,
        bg: PantoneColor::new("Background", "atom one-light chrome", "#eaeaeb"),
        surface: PantoneColor::new("Surface", "atom one-light chrome", "#fafafa"),
        border: PantoneColor::new("Border", "atom one-light chrome", "#dbdbdc"),
        text_primary: PantoneColor::new("Text", "atom one-light chrome", "#383a42"),
        text_secondary: PantoneColor::new("Text Dim", "atom one-light chrome", "#696c77"),
        text_muted: PantoneColor::new("Muted", "atom one-light chrome", "#a0a1a7"),
        accent: PantoneColor::new("Accent", "atom one-light chrome", "#526fff"),
        highlight: PantoneColor::new("Highlight", "atom one-light chrome", "#986801"),
    },
    PantoneTheme {
        id: "light_plus",
        name: "VS Code Light+",
        is_dark: false,
        bg: PantoneColor::new("Background", "vscode light+", "#ffffff"),
        surface: PantoneColor::new("Surface", "vscode light+", "#f3f3f3"),
        border: PantoneColor::new("Border", "vscode light+", "#e5e5e5"),
        text_primary: PantoneColor::new("Text", "vscode light+", "#1f1f1f"),
        text_secondary: PantoneColor::new("Text Dim", "vscode light+", "#616161"),
        text_muted: PantoneColor::new("Muted", "vscode light+", "#6e7681"),
        accent: PantoneColor::new("Accent", "vscode light+", "#005fb8"),
        highlight: PantoneColor::new("Highlight", "vscode light+", "#795e26"),
    },
    PantoneTheme {
        id: "synthwave_day",
        name: "Synthwave '84 Day",
        is_dark: false,
        bg: PantoneColor::new("Background", "mossambi derived synthwave", "#f8f1ff"),
        surface: PantoneColor::new("Surface", "mossambi derived synthwave", "#ffffff"),
        border: PantoneColor::new("Border", "mossambi derived synthwave", "#e2d5f0"),
        text_primary: PantoneColor::new("Text", "mossambi derived synthwave", "#2a2139"),
        text_secondary: PantoneColor::new("Text Dim", "mossambi derived synthwave", "#5a4d72"),
        text_muted: PantoneColor::new("Muted", "mossambi derived synthwave", "#8e86a8"),
        accent: PantoneColor::new("Accent", "mossambi derived synthwave", "#d13fb0"),
        highlight: PantoneColor::new("Highlight", "mossambi derived synthwave", "#b38600"),
    },
    PantoneTheme {
        id: "rangalipi_day",
        name: "Rangalipi Day",
        is_dark: false,
        bg: PantoneColor::new("Background", "mossambi original alpona", "#fbf4ee"),
        surface: PantoneColor::new("Surface", "mossambi original alpona", "#ffffff"),
        border: PantoneColor::new("Border", "mossambi original alpona", "#ead9d6"),
        text_primary: PantoneColor::new("Text", "mossambi original alpona", "#2b1e33"),
        text_secondary: PantoneColor::new("Text Dim", "mossambi original alpona", "#6b4f66"),
        text_muted: PantoneColor::new("Muted", "mossambi original alpona", "#9a8496"),
        accent: PantoneColor::new("Accent", "mossambi original alpona", "#c8423a"),
        highlight: PantoneColor::new("Highlight", "mossambi original alpona", "#c07a10"),
    },
    PantoneTheme {
        id: "mossambi_punch_night",
        name: "Mossambi Punch Night",
        is_dark: true,
        bg: PantoneColor::new("Background", "mossambi original fruit", "#1b2116"),
        surface: PantoneColor::new("Surface", "mossambi original fruit", "#242c1d"),
        border: PantoneColor::new("Border", "mossambi original fruit", "#354029"),
        text_primary: PantoneColor::new("Text", "mossambi original fruit", "#eef2dc"),
        text_secondary: PantoneColor::new("Text Dim", "mossambi original fruit", "#c3cc9e"),
        text_muted: PantoneColor::new("Muted", "mossambi original fruit", "#85906a"),
        accent: PantoneColor::new("Accent", "mossambi original fruit", "#8fc24f"),
        highlight: PantoneColor::new("Highlight", "mossambi original fruit", "#e5b92c"),
    },
];

/// Returns all 13 named themes (11 Dark, 1 Light) in catalog order
pub fn all_pantone_themes() -> &'static [PantoneTheme] {
    &THEMES
}

/// Resolves a PantoneTheme by its identifier or human-readable name
pub fn get_pantone_theme(id_or_name: &str) -> &'static PantoneTheme {
    let query = id_or_name.to_lowercase().replace([' ', '-', '_'], "");
    for theme in all_pantone_themes() {
        let tid = theme.id.to_lowercase().replace([' ', '-', '_'], "");
        let tname = theme.name.to_lowercase().replace([' ', '-', '_'], "");
        if tid == query || tname == query {
            return theme;
        }
    }
    // Backwards compatibility fallbacks
    if query.contains("light") || query.contains("aqua") || query.contains("white") {
        &THEMES[12] // Mossambi Punch - the one light theme
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
                alt.border,
                alt.text_primary,
                alt.accent,
            ] {
                assert_eq!(c.hex.len(), 7, "{}", alt.name);
            }
        }
    }
}
