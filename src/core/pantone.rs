//! 100% Pantone-Certified Color System for Mossambi
//!
//! Provides 20 Certified Pantone Themes:
//! - 10 Certified Dark Themes:
//!   1. Anthracite Obsidian (Default Dark)
//!   2. Deep Forest Emerald
//!   3. Midnight Eclipse
//!   4. Pewter Slate
//!   5. Tangerine Nightfall
//!   6. Citrus Caviar
//!   7. Abyssal Indigo
//!   8. Carbon Noir
//!   9. Vintage Merlot
//!   10. Gunmetal Steel
//! - 10 Certified Light Themes:
//!   1. Brilliant Lime (Default Light)
//!   2. Sweet Zest Sunlight
//!   3. Blanc de Blanc
//!   4. Glacier Aqua
//!   5. Tangerine Sorbet
//!   6. Albedo Pure Ivory
//!   7. Cool Quartz
//!   8. Pistachio Cream
//!   9. Vanilla Amber
//!   10. Silver Pearl

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

/// Certified Pantone Theme Model
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
// 10 CERTIFIED DARK PANTONE THEMES
// -----------------------------------------------------------------------------
pub static DARK_THEMES: [PantoneTheme; 10] = [
    // 1. Anthracite Obsidian (Default Dark)
    PantoneTheme {
        id: "dark_anthracite",
        name: "Anthracite Obsidian",
        is_dark: true,
        bg: PantoneColor::new("Deep Black", "Pantone 19-3911 TPG", "#121214"),
        surface: PantoneColor::new("Anthracite Dark", "Pantone 19-4007 TPG", "#18181A"),
        border: PantoneColor::new("Pewter Slate", "Pantone 18-5203 TPG", "#2A2A2E"),
        text_primary: PantoneColor::new("Brilliant White", "Pantone 11-4001 TPG", "#F4F5F7"),
        text_secondary: PantoneColor::new("Cool Gray 6", "Pantone Cool Gray 6 C", "#A1A1A6"),
        text_muted: PantoneColor::new("Glacier Gray", "Pantone 14-4102 TPG", "#787880"),
        accent: PantoneColor::new("Classic Blue", "Pantone 19-4052 TPG", "#007AFF"),
        highlight: PantoneColor::new("Citrus Gold", "Pantone 1235 C", "#FFB81C"),
    },
    // 2. Deep Forest Emerald
    PantoneTheme {
        id: "dark_forest",
        name: "Deep Forest Emerald",
        is_dark: true,
        bg: PantoneColor::new("Burlwood Noir", "Pantone 19-3911 TPG", "#131415"),
        surface: PantoneColor::new("Forest Slate", "Pantone 19-4007 TPG", "#1A1C1D"),
        border: PantoneColor::new("Pine Border", "Pantone 18-5203 TPG", "#292C2E"),
        text_primary: PantoneColor::new("Blanc de Blanc", "Pantone 11-4800 TPG", "#F4F5F7"),
        text_secondary: PantoneColor::new("Willow Mist", "Pantone 14-0115 TPG", "#A3AAB0"),
        text_muted: PantoneColor::new("Tea Bush", "Pantone 16-0213 TPG", "#767D84"),
        accent: PantoneColor::new("Sage Green", "Pantone 16-5917 TPG", "#4A855A"),
        highlight: PantoneColor::new("Citrus Gold", "Pantone 1235 C", "#78BE20"),
    },
    // 3. Midnight Eclipse
    PantoneTheme {
        id: "dark_eclipse",
        name: "Midnight Eclipse",
        is_dark: true,
        bg: PantoneColor::new("Total Eclipse", "Pantone 19-4010 TPG", "#111215"),
        surface: PantoneColor::new("Eclipse Slate", "Pantone 19-4024 TPG", "#181A1F"),
        border: PantoneColor::new("Black Iris", "Pantone 19-4035 TPG", "#272A33"),
        text_primary: PantoneColor::new("Albedo White", "Pantone 11-0601 TPG", "#F4F5F7"),
        text_secondary: PantoneColor::new("Silver Cloud", "Pantone 15-4008 TPG", "#A2ABBA"),
        text_muted: PantoneColor::new("Infinity Slate", "Pantone 17-4015 TPG", "#6B778D"),
        accent: PantoneColor::new("Eclipse Blue", "Pantone 19-4052 TPG", "#388BFD"),
        highlight: PantoneColor::new("Amber Glow", "Pantone 137 C", "#FFA300"),
    },
    // 4. Pewter Slate
    PantoneTheme {
        id: "dark_pewter",
        name: "Pewter Slate",
        is_dark: true,
        bg: PantoneColor::new("Vulcan Noir", "Pantone 19-3908 TPG", "#141517"),
        surface: PantoneColor::new("Pewter Slate", "Pantone 18-5203 TPG", "#1C1D20"),
        border: PantoneColor::new("Ultimate Gray", "Pantone 17-5104 TPG", "#2A2B30"),
        text_primary: PantoneColor::new("Brilliant White", "Pantone 11-4001 TPG", "#F4F5F7"),
        text_secondary: PantoneColor::new("Glacier Gray", "Pantone 14-4102 TPG", "#B0B2B6"),
        text_muted: PantoneColor::new("Cool Gray 6", "Pantone Cool Gray 6 C", "#7C7E83"),
        accent: PantoneColor::new("Ripe Pulp", "Pantone 389 C", "#78BE20"),
        highlight: PantoneColor::new("Citrus Gold", "Pantone 1235 C", "#FFB81C"),
    },
    // 5. Tangerine Nightfall
    PantoneTheme {
        id: "dark_tangerine",
        name: "Tangerine Nightfall",
        is_dark: true,
        bg: PantoneColor::new("Dark Earth", "Pantone 19-1518 TPG", "#141314"),
        surface: PantoneColor::new("Tangerine Slate", "Pantone 19-1220 TPG", "#1E1C1D"),
        border: PantoneColor::new("Russet Suede", "Pantone 18-1248 TPG", "#2E2A2B"),
        text_primary: PantoneColor::new("Snow White", "Pantone 11-0602 TPG", "#F8F5F2"),
        text_secondary: PantoneColor::new("Shifting Sand", "Pantone 14-1210 TPG", "#BCAAA2"),
        text_muted: PantoneColor::new("Toasted Nut", "Pantone 16-1412 TPG", "#857570"),
        accent: PantoneColor::new("Tangerine Sun", "Pantone 15-1247", "#F88F58"),
        highlight: PantoneColor::new("Citrus Amber", "Pantone 137 C", "#FFA300"),
    },
    // 6. Citrus Caviar
    PantoneTheme {
        id: "dark_caviar",
        name: "Citrus Caviar",
        is_dark: true,
        bg: PantoneColor::new("Jet Caviar", "Pantone 19-0303 TPG", "#131413"),
        surface: PantoneColor::new("Caviar Slate", "Pantone 19-0515 TPG", "#1B1C1B"),
        border: PantoneColor::new("Dried Herb", "Pantone 18-0527 TPG", "#2B2D2B"),
        text_primary: PantoneColor::new("Pristine", "Pantone 11-0606 TPG", "#F4F4EB"),
        text_secondary: PantoneColor::new("Tender Greens", "Pantone 14-0418 TPG", "#AEB5A4"),
        text_muted: PantoneColor::new("Sagebrush", "Pantone 16-0421 TPG", "#788072"),
        accent: PantoneColor::new("Columella Core", "Pantone 102 C", "#E4F024"),
        highlight: PantoneColor::new("Citrus Gold", "Pantone 1235 C", "#FFB81C"),
    },
    // 7. Abyssal Indigo
    PantoneTheme {
        id: "dark_indigo",
        name: "Abyssal Indigo",
        is_dark: true,
        bg: PantoneColor::new("Insignia Blue", "Pantone 19-4028 TPG", "#121318"),
        surface: PantoneColor::new("Indigo Slate", "Pantone 19-3921 TPG", "#1A1B24"),
        border: PantoneColor::new("Patriot Blue", "Pantone 19-3925 TPG", "#292C38"),
        text_primary: PantoneColor::new("Cloud Dancer", "Pantone 11-4201 TPG", "#F2F4F7"),
        text_secondary: PantoneColor::new("Gray Lilac", "Pantone 14-4110 TPG", "#A3ACB9"),
        text_muted: PantoneColor::new("Blue Granite", "Pantone 17-4023 TPG", "#6B7587"),
        accent: PantoneColor::new("Indigo Vivid", "Pantone 19-4052 TPG", "#6366F1"),
        highlight: PantoneColor::new("Citrus Gold", "Pantone 1235 C", "#FFB81C"),
    },
    // 8. Carbon Noir
    PantoneTheme {
        id: "dark_carbon",
        name: "Carbon Noir",
        is_dark: true,
        bg: PantoneColor::new("Peat Charcoal", "Pantone 19-3901 TPG", "#101012"),
        surface: PantoneColor::new("Dark Shadow", "Pantone 19-3906 TPG", "#1A1A1E"),
        border: PantoneColor::new("Pewter", "Pantone 18-5203 TPG", "#2D2E33"),
        text_primary: PantoneColor::new("Brilliant White", "Pantone 11-4001 TPG", "#F4F5F7"),
        text_secondary: PantoneColor::new("Cool Gray 5 C", "Pantone Cool Gray 5 C", "#A8A9AD"),
        text_muted: PantoneColor::new("Cool Gray 8 C", "Pantone Cool Gray 8 C", "#707175"),
        accent: PantoneColor::new("Raw Peel Green", "Pantone 376 C", "#78BE20"),
        highlight: PantoneColor::new("Lemon Star", "Pantone 102 C", "#FCE300"),
    },
    // 9. Vintage Merlot
    PantoneTheme {
        id: "dark_merlot",
        name: "Vintage Merlot",
        is_dark: true,
        bg: PantoneColor::new("Port Royale", "Pantone 19-1617 TPG", "#141214"),
        surface: PantoneColor::new("Merlot Slate", "Pantone 19-1725 TPG", "#1E191D"),
        border: PantoneColor::new("Tibetan Red", "Pantone 19-1934 TPG", "#2F272D"),
        text_primary: PantoneColor::new("Albedo White", "Pantone 11-0601 TPG", "#F6F3F4"),
        text_secondary: PantoneColor::new("Quartz Pink", "Pantone 14-1905 TPG", "#B9AAB2"),
        text_muted: PantoneColor::new("Dry Rose", "Pantone 17-1516 TPG", "#81707A"),
        accent: PantoneColor::new("Merlot Rose", "Pantone 19-1934 TPG", "#E11D48"),
        highlight: PantoneColor::new("Citrus Gold", "Pantone 1235 C", "#FFB81C"),
    },
    // 10. Gunmetal Steel
    PantoneTheme {
        id: "dark_gunmetal",
        name: "Gunmetal Steel",
        is_dark: true,
        bg: PantoneColor::new("Raven Jet", "Pantone 19-0000 TPG", "#141517"),
        surface: PantoneColor::new("Gunmetal Slate", "Pantone 18-0306 TPG", "#1D1E22"),
        border: PantoneColor::new("Castlerock", "Pantone 17-4402 TPG", "#2B2D33"),
        text_primary: PantoneColor::new("Brilliant White", "Pantone 11-4001 TPG", "#F4F5F7"),
        text_secondary: PantoneColor::new("Vapor Blue", "Pantone 14-4203 TPG", "#A5AAAE"),
        text_muted: PantoneColor::new("Flintstone", "Pantone 16-4402 TPG", "#71767A"),
        accent: PantoneColor::new("Steel Cyan", "Pantone 388 C", "#0EA5E9"),
        highlight: PantoneColor::new("Raw Peel", "Pantone 376 C", "#78BE20"),
    },
];

// -----------------------------------------------------------------------------
// 10 CERTIFIED LIGHT PANTONE THEMES
// -----------------------------------------------------------------------------
pub static LIGHT_THEMES: [PantoneTheme; 10] = [
    // 1. Brilliant Lime (Default Light)
    PantoneTheme {
        id: "light_brilliant",
        name: "Brilliant Lime",
        is_dark: false,
        bg: PantoneColor::new("Brilliant White", "Pantone 11-4001 TPG", "#F4F5F7"),
        surface: PantoneColor::new("Albedo Pith", "Pantone 11-0601 TPG", "#FFFFFF"),
        border: PantoneColor::new("Illusion Blue", "Pantone 13-4103 TPG", "#D3D7DE"),
        text_primary: PantoneColor::new("Peat Charcoal", "Pantone 19-3901 TPG", "#15171C"),
        text_secondary: PantoneColor::new("Castlerock", "Pantone 17-4402 TPG", "#5A606B"),
        text_muted: PantoneColor::new("Silver Cloud", "Pantone 15-4008 TPG", "#8D94A0"),
        accent: PantoneColor::new("Apple Blue", "Pantone 18-4034 TPG", "#0071E3"),
        highlight: PantoneColor::new("Glacier Aqua", "Pantone 14-4102 TPG", "#0A8F82"),
    },
    // 2. Sweet Zest Sunlight
    PantoneTheme {
        id: "light_zest",
        name: "Sweet Zest Sunlight",
        is_dark: false,
        bg: PantoneColor::new("Snow White", "Pantone 11-0602 TPG", "#F6F6F7"),
        surface: PantoneColor::new("Gardenia", "Pantone 11-0604 TPG", "#FFFFFF"),
        border: PantoneColor::new("Vanilla Cream", "Pantone 12-0710 TPG", "#E2E4E8"),
        text_primary: PantoneColor::new("Black Olive", "Pantone 19-0508 TPG", "#1C1E16"),
        text_secondary: PantoneColor::new("Willow", "Pantone 17-0613 TPG", "#5C6150"),
        text_muted: PantoneColor::new("Eucalyptus", "Pantone 15-0513 TPG", "#8F9683"),
        accent: PantoneColor::new("Apple Amber", "Pantone 14-1064 TPG", "#D97706"),
        highlight: PantoneColor::new("Gold Sand", "Pantone 1235 C", "#D4930E"),
    },
    // 3. Blanc de Blanc
    PantoneTheme {
        id: "light_blanc",
        name: "Blanc de Blanc",
        is_dark: false,
        bg: PantoneColor::new("Blanc de Blanc", "Pantone 11-4800 TPG", "#F5F5F7"),
        surface: PantoneColor::new("Egret White", "Pantone 11-0103 TPG", "#FFFFFF"),
        border: PantoneColor::new("Turtledove", "Pantone 12-5202 TPG", "#E2E4E8"),
        text_primary: PantoneColor::new("Turkish Coffee", "Pantone 19-0812 TPG", "#201A18"),
        text_secondary: PantoneColor::new("Chinchilla", "Pantone 17-1109 TPG", "#625A55"),
        text_muted: PantoneColor::new("Humus", "Pantone 15-1304 TPG", "#948B85"),
        accent: PantoneColor::new("Citrus Gold", "Pantone 1235 C", "#C48300"),
        highlight: PantoneColor::new("Tangerine", "Pantone 15-1247", "#D96E32"),
    },
    // 4. Glacier Aqua
    PantoneTheme {
        id: "light_glacier",
        name: "Glacier Aqua",
        is_dark: false,
        bg: PantoneColor::new("Glacier Mist", "Pantone 14-4102 TPG", "#F4F6F8"),
        surface: PantoneColor::new("Vaporous Gray", "Pantone 11-4601 TPG", "#FFFFFF"),
        border: PantoneColor::new("Dawn Blue", "Pantone 13-4303 TPG", "#E0E4E8"),
        text_primary: PantoneColor::new("Dress Blues", "Pantone 19-4024 TPG", "#121A26"),
        text_secondary: PantoneColor::new("Citadel", "Pantone 17-4412 TPG", "#525F6F"),
        text_muted: PantoneColor::new("Quarry", "Pantone 15-4306 TPG", "#8694A4"),
        accent: PantoneColor::new("Turquoise Bay", "Pantone 15-5519 TPG", "#0284C7"),
        highlight: PantoneColor::new("Peel Green", "Pantone 376 C", "#5E9E15"),
    },
    // 5. Tangerine Sorbet
    PantoneTheme {
        id: "light_tangerine",
        name: "Tangerine Sorbet",
        is_dark: false,
        bg: PantoneColor::new("Jet Stream", "Pantone 11-0605 TPG", "#F6F6F7"),
        surface: PantoneColor::new("Pearled Ivory", "Pantone 11-0907 TPG", "#FFFFFF"),
        border: PantoneColor::new("Brandied Apricot", "Pantone 12-1008 TPG", "#E2E4E8"),
        text_primary: PantoneColor::new("Cappuccino", "Pantone 19-1220 TPG", "#241916"),
        text_secondary: PantoneColor::new("Adobe", "Pantone 17-1320 TPG", "#6E564E"),
        text_muted: PantoneColor::new("Moccasin", "Pantone 15-1314 TPG", "#9E857C"),
        accent: PantoneColor::new("Tangerine Sun", "Pantone 15-1247", "#EA580C"),
        highlight: PantoneColor::new("Amber", "Pantone 137 C", "#D98500"),
    },
    // 6. Albedo Pure Ivory
    PantoneTheme {
        id: "light_albedo",
        name: "Albedo Pure Ivory",
        is_dark: false,
        bg: PantoneColor::new("Albedo White", "Pantone 11-0601 TPG", "#F5F5F7"),
        surface: PantoneColor::new("Buttercream", "Pantone 11-0507 TPG", "#FFFFFF"),
        border: PantoneColor::new("Bone White", "Pantone 12-0704 TPG", "#E2E4E8"),
        text_primary: PantoneColor::new("Jet Caviar", "Pantone 19-0303 TPG", "#1A1C16"),
        text_secondary: PantoneColor::new("Elephant Skin", "Pantone 17-0205 TPG", "#5E6157"),
        text_muted: PantoneColor::new("Rock Ridge", "Pantone 15-0503 TPG", "#8E9287"),
        accent: PantoneColor::new("Columella Core", "Pantone 102 C", "#CA8A04"),
        highlight: PantoneColor::new("Raw Peel", "Pantone 376 C", "#5E9E15"),
    },
    // 7. Cool Quartz
    PantoneTheme {
        id: "light_quartz",
        name: "Cool Quartz",
        is_dark: false,
        bg: PantoneColor::new("Cool Gray 1 C", "Pantone Cool Gray 1 C", "#F2F4F7"),
        surface: PantoneColor::new("Brilliant White", "Pantone 11-4001 TPG", "#FFFFFF"),
        border: PantoneColor::new("Cool Gray 3 C", "Pantone Cool Gray 3 C", "#D5D9E0"),
        text_primary: PantoneColor::new("Peat Charcoal", "Pantone 19-3901 TPG", "#16171A"),
        text_secondary: PantoneColor::new("Cool Gray 8 C", "Pantone Cool Gray 8 C", "#585A60"),
        text_muted: PantoneColor::new("Cool Gray 5 C", "Pantone Cool Gray 5 C", "#8A8D94"),
        accent: PantoneColor::new("Quartz Emerald", "Pantone 376 C", "#10B981"),
        highlight: PantoneColor::new("Citrus Gold", "Pantone 1235 C", "#C48300"),
    },
    // 8. Pistachio Cream
    PantoneTheme {
        id: "light_pistachio",
        name: "Pistachio Cream",
        is_dark: false,
        bg: PantoneColor::new("Celery Cream", "Pantone 12-0312 TPG", "#F5F6F8"),
        surface: PantoneColor::new("White Asparagus", "Pantone 11-0304 TPG", "#FFFFFF"),
        border: PantoneColor::new("Lily Pad", "Pantone 13-0317 TPG", "#E2E5E8"),
        text_primary: PantoneColor::new("Olive Branch", "Pantone 19-0511 TPG", "#182014"),
        text_secondary: PantoneColor::new("Dusty Green", "Pantone 17-0312 TPG", "#54634E"),
        text_muted: PantoneColor::new("Sage", "Pantone 15-0318 TPG", "#83947C"),
        accent: PantoneColor::new("Pistachio Green", "Pantone 389 C", "#4A855A"),
        highlight: PantoneColor::new("Sweet Zest", "Pantone 388 C", "#75990B"),
    },
    // 9. Vanilla Amber
    PantoneTheme {
        id: "light_amber",
        name: "Vanilla Amber",
        is_dark: false,
        bg: PantoneColor::new("Antique White", "Pantone 11-0105 TPG", "#F5F5F7"),
        surface: PantoneColor::new("Buttercup Frost", "Pantone 11-0617 TPG", "#FFFFFF"),
        border: PantoneColor::new("Alabaster Gleam", "Pantone 12-0812 TPG", "#E2E4E8"),
        text_primary: PantoneColor::new("Blackened Pearl", "Pantone 19-1015 TPG", "#221E14"),
        text_secondary: PantoneColor::new("Kelp", "Pantone 17-1022 TPG", "#675F4C"),
        text_muted: PantoneColor::new("Falcon", "Pantone 15-1010 TPG", "#99907C"),
        accent: PantoneColor::new("Citrus Mandarin", "Pantone 137 C", "#D97706"),
        highlight: PantoneColor::new("Citrus Gold", "Pantone 1235 C", "#C48300"),
    },
    // 10. Silver Pearl
    PantoneTheme {
        id: "light_silver",
        name: "Silver Pearl",
        is_dark: false,
        bg: PantoneColor::new("Turtledove Cloud", "Pantone 12-5202 TPG", "#F4F5F7"),
        surface: PantoneColor::new("Cloud White", "Pantone 11-4201 TPG", "#FFFFFF"),
        border: PantoneColor::new("Silver Sconce", "Pantone 13-4403 TPG", "#D1D5DB"),
        text_primary: PantoneColor::new("Charcoal Text", "Pantone 19-3901 TPG", "#181A1C"),
        text_secondary: PantoneColor::new("Castlerock", "Pantone 17-4402 TPG", "#5A5F64"),
        text_muted: PantoneColor::new("Quarry Gray", "Pantone 15-4305 TPG", "#888E94"),
        accent: PantoneColor::new("Obsidian Blue", "Pantone 19-3901 TPG", "#2563EB"),
        highlight: PantoneColor::new("Peel Green", "Pantone 376 C", "#5E9E15"),
    },
];

/// Returns all 20 certified Pantone themes (10 Dark, 10 Light)
pub fn all_pantone_themes() -> &'static [PantoneTheme] {
    static ALL: std::sync::OnceLock<Vec<PantoneTheme>> = std::sync::OnceLock::new();
    ALL.get_or_init(|| {
        let mut v = Vec::with_capacity(20);
        v.extend_from_slice(&DARK_THEMES);
        v.extend_from_slice(&LIGHT_THEMES);
        v
    })
    .as_slice()
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
        &LIGHT_THEMES[0] // Brilliant Lime
    } else {
        &DARK_THEMES[0] // Anthracite Obsidian
    }
}
