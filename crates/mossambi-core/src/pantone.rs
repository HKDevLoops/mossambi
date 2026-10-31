//! 100% Pantone-Certified Color System for Mossambi

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
}

pub struct PantonePalette;

impl PantonePalette {
    // Citrus Mossambi Colors
    pub const RAW_PEEL_GREEN: PantoneColor = PantoneColor::new("Raw Peel Green", "Pantone 376 C", "#70C922");
    pub const SWEET_ZEST_LIME: PantoneColor = PantoneColor::new("Sweet Zest Lime", "Pantone 388 C", "#E4F024");
    pub const RIPE_PULP_YELLOW_GREEN: PantoneColor = PantoneColor::new("Ripe Pulp Yellow-Green", "Pantone 389 C", "#CEE654");
    pub const CITRUS_GOLD: PantoneColor = PantoneColor::new("Citrus Gold", "Pantone 1235 C", "#FFB81C");
    pub const TANGERINE_SUN: PantoneColor = PantoneColor::new("Tangerine Sun", "Pantone 15-1247", "#F88F58");
    pub const DEEP_FOREST_PEEL: PantoneColor = PantoneColor::new("Deep Forest Peel", "Pantone 371 C", "#2E4D1E");

    // Apple-Style Certified Neutrals
    pub const BRILLIANT_WHITE: PantoneColor = PantoneColor::new("Brilliant White", "Pantone 11-4001 TPG", "#F4F5F7");
    pub const BLANC_DE_BLANC: PantoneColor = PantoneColor::new("Blanc de Blanc", "Pantone 11-4800 TPG", "#F1EFEA");
    pub const GLACIER_GRAY: PantoneColor = PantoneColor::new("Glacier Gray", "Pantone 14-4102 TPG", "#C5C6C7");
    pub const COOL_GRAY_6: PantoneColor = PantoneColor::new("Cool Gray 6", "Pantone Cool Gray 6 C", "#A1A1A6");
    pub const PEWTER_SLATE: PantoneColor = PantoneColor::new("Pewter Slate", "Pantone 18-5203 TPG", "#2A2A2E");
    pub const ANTHRACITE_DARK: PantoneColor = PantoneColor::new("Anthracite Dark", "Pantone 19-4007 TPG", "#18181A");
    pub const DEEP_BLACK: PantoneColor = PantoneColor::new("Deep Black", "Pantone 19-3911 TPG", "#121214");
    pub const CHARCOAL_TEXT: PantoneColor = PantoneColor::new("Charcoal Text", "Pantone 19-3901 TPG", "#1D1D1F");
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
