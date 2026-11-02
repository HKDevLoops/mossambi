//! Apple macOS Design System & Theme Engine for Mossambi
//! Implements:
//! - 4 Distinct Apple-Inspired Themes:
//!   1. Liquid Glass (macOS Sequoia / VisionOS refraction, high translucency, specular highlights)
//!   2. Transparent Frosted Glass (macOS Sonoma / Aqua acrylic blur, matte frosted finish)
//!   3. Dark Anthracite (macOS Pro Dark #161618)
//!   4. Light Aqua (macOS Crisp Light #F5F5F7)
//! - Authentic Traffic Light window controls (Close, Minimize, Zoom) with subtle inner borders and hover glyphs
//! - Translucent frosted sidebar with pill navigation items
//! - Segmented capsule controls and clean Apple system typography
//! - Certified Pantone Citrus Palette tokens (Flavedo 376 C, Gold 1235 C, Core 102 C)

use eframe::egui::{self, Color32, FontId, Pos2, Rect, Rounding, Sense, Stroke, Ui, Vec2};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Certified Pantone Citrus Tokens
// ---------------------------------------------------------------------------
pub const PANTONE_FLAVEDO: Color32 = Color32::from_rgb(120, 190, 32); // Pantone 376 C
pub const PANTONE_ALBEDO: Color32 = Color32::from_rgb(244, 245, 240); // Pantone 11-0601 TPG
pub const PANTONE_CARPEL: Color32 = Color32::from_rgb(255, 199, 44); // Pantone 123 C
pub const PANTONE_HOVER: Color32 = Color32::from_rgb(255, 163, 0); // Pantone 137 C
pub const PANTONE_GOLD: Color32 = Color32::from_rgb(255, 184, 28); // Pantone 1235 C
pub const PANTONE_CORE: Color32 = Color32::from_rgb(252, 227, 0); // Pantone 102 C

// Traffic Light Window Controls
pub const TRAFFIC_CLOSE: Color32 = Color32::from_rgb(255, 95, 86); // #FF5F56
pub const TRAFFIC_MIN: Color32 = Color32::from_rgb(255, 189, 46); // #FFBD2E
pub const TRAFFIC_MAX: Color32 = Color32::from_rgb(39, 201, 63); // #27C93F

// ---------------------------------------------------------------------------
// Theme Modes
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThemeStyle {
    // 10 Certified Dark Themes
    #[default]
    #[serde(alias = "dark", alias = "system", alias = "anthracite_obsidian")]
    DarkAnthracite,
    #[serde(alias = "dark_forest", alias = "deep_forest_emerald")]
    DarkForest,
    #[serde(alias = "dark_eclipse", alias = "midnight_eclipse")]
    DarkEclipse,
    #[serde(alias = "dark_pewter", alias = "pewter_slate")]
    DarkPewter,
    #[serde(alias = "dark_tangerine", alias = "tangerine_nightfall")]
    DarkTangerine,
    #[serde(alias = "dark_caviar", alias = "citrus_caviar")]
    DarkCaviar,
    #[serde(alias = "dark_indigo", alias = "abyssal_indigo")]
    DarkIndigo,
    #[serde(alias = "dark_carbon", alias = "carbon_noir")]
    DarkCarbon,
    #[serde(alias = "dark_merlot", alias = "vintage_merlot")]
    DarkMerlot,
    #[serde(alias = "dark_gunmetal", alias = "gunmetal_steel")]
    DarkGunmetal,

    // 10 Certified Light Themes
    #[serde(alias = "light", alias = "light_brilliant", alias = "brilliant_lime")]
    LightBrilliant,
    #[serde(alias = "light_zest", alias = "sweet_zest_sunlight")]
    LightZest,
    #[serde(alias = "light_blanc", alias = "blanc_de_blanc")]
    LightBlanc,
    #[serde(alias = "light_glacier", alias = "glacier_aqua")]
    LightGlacier,
    #[serde(alias = "light_tangerine", alias = "tangerine_sorbet")]
    LightTangerine,
    #[serde(alias = "light_albedo", alias = "albedo_pure_ivory")]
    LightAlbedo,
    #[serde(alias = "light_quartz", alias = "cool_quartz")]
    LightQuartz,
    #[serde(alias = "light_pistachio", alias = "pistachio_cream")]
    LightPistachio,
    #[serde(alias = "light_amber", alias = "vanilla_amber")]
    LightAmber,
    #[serde(alias = "light_silver", alias = "silver_pearl")]
    LightSilver,

    // Code compatibility variants
    #[serde(alias = "liquidglass")]
    LiquidGlass,
    #[serde(alias = "frostedglass")]
    FrostedGlass,
    #[serde(alias = "lightaqua")]
    LightAqua,
}

impl ThemeStyle {
    pub fn all() -> &'static [ThemeStyle] {
        static ALL: [ThemeStyle; 20] = [
            ThemeStyle::DarkAnthracite,
            ThemeStyle::DarkForest,
            ThemeStyle::DarkEclipse,
            ThemeStyle::DarkPewter,
            ThemeStyle::DarkTangerine,
            ThemeStyle::DarkCaviar,
            ThemeStyle::DarkIndigo,
            ThemeStyle::DarkCarbon,
            ThemeStyle::DarkMerlot,
            ThemeStyle::DarkGunmetal,
            ThemeStyle::LightBrilliant,
            ThemeStyle::LightZest,
            ThemeStyle::LightBlanc,
            ThemeStyle::LightGlacier,
            ThemeStyle::LightTangerine,
            ThemeStyle::LightAlbedo,
            ThemeStyle::LightQuartz,
            ThemeStyle::LightPistachio,
            ThemeStyle::LightAmber,
            ThemeStyle::LightSilver,
        ];
        &ALL
    }

    pub fn dark_themes() -> &'static [ThemeStyle] {
        &Self::all()[0..10]
    }

    pub fn light_themes() -> &'static [ThemeStyle] {
        &Self::all()[10..20]
    }

    pub fn to_pantone(&self) -> &'static mossambi_core::pantone::PantoneTheme {
        match self {
            Self::DarkAnthracite | Self::LiquidGlass => &mossambi_core::pantone::DARK_THEMES[0],
            Self::DarkForest => &mossambi_core::pantone::DARK_THEMES[1],
            Self::DarkEclipse => &mossambi_core::pantone::DARK_THEMES[2],
            Self::DarkPewter | Self::FrostedGlass => &mossambi_core::pantone::DARK_THEMES[3],
            Self::DarkTangerine => &mossambi_core::pantone::DARK_THEMES[4],
            Self::DarkCaviar => &mossambi_core::pantone::DARK_THEMES[5],
            Self::DarkIndigo => &mossambi_core::pantone::DARK_THEMES[6],
            Self::DarkCarbon => &mossambi_core::pantone::DARK_THEMES[7],
            Self::DarkMerlot => &mossambi_core::pantone::DARK_THEMES[8],
            Self::DarkGunmetal => &mossambi_core::pantone::DARK_THEMES[9],

            Self::LightBrilliant => &mossambi_core::pantone::LIGHT_THEMES[0],
            Self::LightZest => &mossambi_core::pantone::LIGHT_THEMES[1],
            Self::LightBlanc => &mossambi_core::pantone::LIGHT_THEMES[2],
            Self::LightGlacier | Self::LightAqua => &mossambi_core::pantone::LIGHT_THEMES[3],
            Self::LightTangerine => &mossambi_core::pantone::LIGHT_THEMES[4],
            Self::LightAlbedo => &mossambi_core::pantone::LIGHT_THEMES[5],
            Self::LightQuartz => &mossambi_core::pantone::LIGHT_THEMES[6],
            Self::LightPistachio => &mossambi_core::pantone::LIGHT_THEMES[7],
            Self::LightAmber => &mossambi_core::pantone::LIGHT_THEMES[8],
            Self::LightSilver => &mossambi_core::pantone::LIGHT_THEMES[9],
        }
    }

    pub fn name(&self) -> &'static str {
        self.to_pantone().name
    }

    pub fn is_dark(&self) -> bool {
        self.to_pantone().is_dark
    }

    pub fn colors(&self) -> ThemePalette {
        let cfg = mossambi_core::MossambiConfig::load();
        self.colors_with_glass(cfg.glass_effect, cfg.window_opacity)
    }

    pub fn colors_with_glass(&self, effect: mossambi_core::GlassEffect, opacity: f32) -> ThemePalette {
        let pt = self.to_pantone();
        let (bg_r, bg_g, bg_b) = pt.bg.to_rgb();
        let (surf_r, surf_g, surf_b) = pt.surface.to_rgb();
        let (bord_r, bord_g, bord_b) = pt.border.to_rgb();
        let (tp_r, tp_g, tp_b) = pt.text_primary.to_rgb();
        let (ts_r, ts_g, ts_b) = pt.text_secondary.to_rgb();
        let (tm_r, tm_g, tm_b) = pt.text_muted.to_rgb();
        let (acc_r, acc_g, acc_b) = pt.accent.to_rgb();
        let (hi_r, hi_g, hi_b) = pt.highlight.to_rgb();

        let clamped_opacity = opacity.clamp(0.20, 1.00);

        match effect {
            mossambi_core::GlassEffect::Opaque => {
                if pt.is_dark {
                    let card_hover_r = surf_r.saturating_add(14);
                    let card_hover_g = surf_g.saturating_add(14);
                    let card_hover_b = surf_b.saturating_add(14);
                    ThemePalette {
                        window_bg: Color32::from_rgb(bg_r, bg_g, bg_b),
                        sidebar_bg: Color32::from_rgb(
                            bg_r.saturating_add(6),
                            bg_g.saturating_add(6),
                            bg_b.saturating_add(6),
                        ),
                        card_bg: Color32::from_rgb(surf_r, surf_g, surf_b),
                        card_hover: Color32::from_rgb(card_hover_r, card_hover_g, card_hover_b),
                        border: Color32::from_rgb(bord_r, bord_g, bord_b),
                        specular: Color32::from_rgba_premultiplied(255, 255, 255, 18),
                        divider: Color32::from_rgb(
                            bord_r.saturating_sub(10),
                            bord_g.saturating_sub(10),
                            bord_b.saturating_sub(10),
                        ),
                        text_primary: Color32::from_rgb(tp_r, tp_g, tp_b),
                        text_secondary: Color32::from_rgb(ts_r, ts_g, ts_b),
                        text_muted: Color32::from_rgb(tm_r, tm_g, tm_b),
                        accent: Color32::from_rgb(acc_r, acc_g, acc_b),
                        highlight: Color32::from_rgb(hi_r, hi_g, hi_b),
                        is_translucent: false,
                        is_dark: true,
                    }
                } else {
                    let card_hover_r = surf_r.saturating_sub(10);
                    let card_hover_g = surf_r.saturating_sub(10);
                    let card_hover_b = surf_b.saturating_sub(10);
                    ThemePalette {
                        window_bg: Color32::from_rgb(bg_r, bg_g, bg_b),
                        sidebar_bg: Color32::from_rgb(
                            bg_r.saturating_sub(6),
                            bg_g.saturating_sub(6),
                            bg_b.saturating_sub(6),
                        ),
                        card_bg: Color32::from_rgb(surf_r, surf_g, surf_b),
                        card_hover: Color32::from_rgb(card_hover_r, card_hover_g, card_hover_b),
                        border: Color32::from_rgb(bord_r, bord_g, bord_b),
                        specular: Color32::from_rgba_premultiplied(255, 255, 255, 120),
                        divider: Color32::from_rgb(bord_r, bord_g, bord_b),
                        text_primary: Color32::from_rgb(tp_r, tp_g, tp_b),
                        text_secondary: Color32::from_rgb(ts_r, ts_g, ts_b),
                        text_muted: Color32::from_rgb(tm_r, tm_g, tm_b),
                        accent: Color32::from_rgb(acc_r, acc_g, acc_b),
                        highlight: Color32::from_rgb(hi_r, hi_g, hi_b),
                        is_translucent: false,
                        is_dark: false,
                    }
                }
            }
            mossambi_core::GlassEffect::FrostedGlass => {
                let win_a = (clamped_opacity * 235.0) as u8;
                let side_a = (clamped_opacity * 245.0) as u8;
                let card_a = (clamped_opacity * 215.0) as u8;
                let hover_a = (clamped_opacity * 230.0) as u8;
                let bord_a = (clamped_opacity * 160.0).clamp(50.0, 255.0) as u8;

                ThemePalette {
                    window_bg: Color32::from_rgba_premultiplied(bg_r, bg_g, bg_b, win_a),
                    sidebar_bg: Color32::from_rgba_premultiplied(
                        bg_r.saturating_add(6),
                        bg_g.saturating_add(6),
                        bg_b.saturating_add(6),
                        side_a,
                    ),
                    card_bg: Color32::from_rgba_premultiplied(surf_r, surf_g, surf_b, card_a),
                    card_hover: Color32::from_rgba_premultiplied(
                        surf_r.saturating_add(10),
                        surf_g.saturating_add(10),
                        surf_b.saturating_add(10),
                        hover_a,
                    ),
                    border: Color32::from_rgba_premultiplied(bord_r, bord_g, bord_b, bord_a),
                    specular: if pt.is_dark {
                        Color32::from_rgba_premultiplied(255, 255, 255, 40)
                    } else {
                        Color32::from_rgba_premultiplied(255, 255, 255, 140)
                    },
                    divider: Color32::from_rgba_premultiplied(bord_r, bord_g, bord_b, (bord_a / 2).max(20)),
                    text_primary: Color32::from_rgb(tp_r, tp_g, tp_b),
                    text_secondary: Color32::from_rgb(ts_r, ts_g, ts_b),
                    text_muted: Color32::from_rgb(tm_r, tm_g, tm_b),
                    accent: Color32::from_rgb(acc_r, acc_g, acc_b),
                    highlight: Color32::from_rgb(hi_r, hi_g, hi_b),
                    is_translucent: true,
                    is_dark: pt.is_dark,
                }
            }
            mossambi_core::GlassEffect::LiquidGlass => {
                let win_a = (clamped_opacity * 180.0) as u8;
                let side_a = (clamped_opacity * 195.0) as u8;
                let card_a = (clamped_opacity * 160.0) as u8;
                let hover_a = (clamped_opacity * 185.0) as u8;
                let bord_a = (clamped_opacity * 200.0).clamp(60.0, 255.0) as u8;

                ThemePalette {
                    window_bg: Color32::from_rgba_premultiplied(bg_r, bg_g, bg_b, win_a),
                    sidebar_bg: Color32::from_rgba_premultiplied(
                        bg_r.saturating_add(4),
                        bg_g.saturating_add(4),
                        bg_b.saturating_add(4),
                        side_a,
                    ),
                    card_bg: Color32::from_rgba_premultiplied(surf_r, surf_g, surf_b, card_a),
                    card_hover: Color32::from_rgba_premultiplied(
                        surf_r.saturating_add(15),
                        surf_g.saturating_add(15),
                        surf_b.saturating_add(15),
                        hover_a,
                    ),
                    border: Color32::from_rgba_premultiplied(bord_r, bord_g, bord_b, bord_a),
                    specular: if pt.is_dark {
                        Color32::from_rgba_premultiplied(255, 255, 255, 85)
                    } else {
                        Color32::from_rgba_premultiplied(255, 255, 255, 190)
                    },
                    divider: Color32::from_rgba_premultiplied(bord_r, bord_g, bord_b, (bord_a / 2).max(25)),
                    text_primary: Color32::from_rgb(tp_r, tp_g, tp_b),
                    text_secondary: Color32::from_rgb(ts_r, ts_g, ts_b),
                    text_muted: Color32::from_rgb(tm_r, tm_g, tm_b),
                    accent: Color32::from_rgb(acc_r, acc_g, acc_b),
                    highlight: Color32::from_rgb(hi_r, hi_g, hi_b),
                    is_translucent: true,
                    is_dark: pt.is_dark,
                }
            }
        }
    }
}

impl From<mossambi_core::ThemeMode> for ThemeStyle {
    fn from(m: mossambi_core::ThemeMode) -> Self {
        match m {
            mossambi_core::ThemeMode::DarkAnthracite | mossambi_core::ThemeMode::LiquidGlass => ThemeStyle::DarkAnthracite,
            mossambi_core::ThemeMode::DarkForest => ThemeStyle::DarkForest,
            mossambi_core::ThemeMode::DarkEclipse => ThemeStyle::DarkEclipse,
            mossambi_core::ThemeMode::DarkPewter | mossambi_core::ThemeMode::FrostedGlass => ThemeStyle::DarkPewter,
            mossambi_core::ThemeMode::DarkTangerine => ThemeStyle::DarkTangerine,
            mossambi_core::ThemeMode::DarkCaviar => ThemeStyle::DarkCaviar,
            mossambi_core::ThemeMode::DarkIndigo => ThemeStyle::DarkIndigo,
            mossambi_core::ThemeMode::DarkCarbon => ThemeStyle::DarkCarbon,
            mossambi_core::ThemeMode::DarkMerlot => ThemeStyle::DarkMerlot,
            mossambi_core::ThemeMode::DarkGunmetal => ThemeStyle::DarkGunmetal,

            mossambi_core::ThemeMode::LightBrilliant => ThemeStyle::LightBrilliant,
            mossambi_core::ThemeMode::LightZest => ThemeStyle::LightZest,
            mossambi_core::ThemeMode::LightBlanc => ThemeStyle::LightBlanc,
            mossambi_core::ThemeMode::LightGlacier | mossambi_core::ThemeMode::LightAqua => ThemeStyle::LightGlacier,
            mossambi_core::ThemeMode::LightTangerine => ThemeStyle::LightTangerine,
            mossambi_core::ThemeMode::LightAlbedo => ThemeStyle::LightAlbedo,
            mossambi_core::ThemeMode::LightQuartz => ThemeStyle::LightQuartz,
            mossambi_core::ThemeMode::LightPistachio => ThemeStyle::LightPistachio,
            mossambi_core::ThemeMode::LightAmber => ThemeStyle::LightAmber,
            mossambi_core::ThemeMode::LightSilver => ThemeStyle::LightSilver,
        }
    }
}

impl From<ThemeStyle> for mossambi_core::ThemeMode {
    fn from(s: ThemeStyle) -> Self {
        match s {
            ThemeStyle::DarkAnthracite | ThemeStyle::LiquidGlass => mossambi_core::ThemeMode::DarkAnthracite,
            ThemeStyle::DarkForest => mossambi_core::ThemeMode::DarkForest,
            ThemeStyle::DarkEclipse => mossambi_core::ThemeMode::DarkEclipse,
            ThemeStyle::DarkPewter | ThemeStyle::FrostedGlass => mossambi_core::ThemeMode::DarkPewter,
            ThemeStyle::DarkTangerine => mossambi_core::ThemeMode::DarkTangerine,
            ThemeStyle::DarkCaviar => mossambi_core::ThemeMode::DarkCaviar,
            ThemeStyle::DarkIndigo => mossambi_core::ThemeMode::DarkIndigo,
            ThemeStyle::DarkCarbon => mossambi_core::ThemeMode::DarkCarbon,
            ThemeStyle::DarkMerlot => mossambi_core::ThemeMode::DarkMerlot,
            ThemeStyle::DarkGunmetal => mossambi_core::ThemeMode::DarkGunmetal,

            ThemeStyle::LightBrilliant => mossambi_core::ThemeMode::LightBrilliant,
            ThemeStyle::LightZest => mossambi_core::ThemeMode::LightZest,
            ThemeStyle::LightBlanc => mossambi_core::ThemeMode::LightBlanc,
            ThemeStyle::LightGlacier | ThemeStyle::LightAqua => mossambi_core::ThemeMode::LightGlacier,
            ThemeStyle::LightTangerine => mossambi_core::ThemeMode::LightTangerine,
            ThemeStyle::LightAlbedo => mossambi_core::ThemeMode::LightAlbedo,
            ThemeStyle::LightQuartz => mossambi_core::ThemeMode::LightQuartz,
            ThemeStyle::LightPistachio => mossambi_core::ThemeMode::LightPistachio,
            ThemeStyle::LightAmber => mossambi_core::ThemeMode::LightAmber,
            ThemeStyle::LightSilver => mossambi_core::ThemeMode::LightSilver,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ThemePalette {
    pub window_bg: Color32,
    pub sidebar_bg: Color32,
    pub card_bg: Color32,
    pub card_hover: Color32,
    pub border: Color32,
    pub specular: Color32,
    pub divider: Color32,
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub text_muted: Color32,
    pub accent: Color32,
    pub highlight: Color32,
    pub is_translucent: bool,
    pub is_dark: bool,
}

// ---------------------------------------------------------------------------
// Static Fallback Constants (for legacy code references)
// ---------------------------------------------------------------------------
pub const MAC_WINDOW_BG: Color32 = Color32::from_rgb(22, 22, 24);
pub const MAC_SIDEBAR_BG: Color32 = Color32::from_rgb(29, 29, 32);
pub const MAC_CARD_BG: Color32 = Color32::from_rgb(38, 38, 43);
pub const MAC_CARD_HOVER: Color32 = Color32::from_rgb(48, 48, 54);
pub const MAC_BORDER: Color32 = Color32::from_rgba_premultiplied(255, 255, 255, 20);
pub const MAC_DIVIDER: Color32 = Color32::from_rgba_premultiplied(255, 255, 255, 14);
pub const MAC_TEXT_PRIMARY: Color32 = Color32::from_rgb(245, 245, 247);
pub const MAC_TEXT_SECONDARY: Color32 = Color32::from_rgb(161, 161, 166);
pub const MAC_TEXT_MUTED: Color32 = Color32::from_rgb(120, 120, 126);

// ---------------------------------------------------------------------------
// macOS UI Component Renderers
// ---------------------------------------------------------------------------

/// Renders authentic macOS traffic light window controls (Close, Minimize, Zoom).
pub fn render_traffic_lights(
    ui: &mut Ui,
    on_close: impl FnOnce(),
    on_min: impl FnOnce(),
    on_zoom: impl FnOnce(),
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(8.0, 0.0);
        let size = Vec2::splat(12.0);

        // Close
        let (rect_close, resp_close) = ui.allocate_exact_size(size, Sense::click());
        let painter = ui.painter_at(rect_close);
        let hovered_close = resp_close.hovered();
        painter.circle_filled(rect_close.center(), 6.0, TRAFFIC_CLOSE);
        painter.circle_stroke(
            rect_close.center(),
            6.0,
            Stroke::new(0.5_f32, Color32::from_black_alpha(40)),
        );
        if hovered_close {
            painter.text(
                rect_close.center(),
                egui::Align2::CENTER_CENTER,
                "✕",
                FontId::proportional(7.5),
                Color32::from_black_alpha(160),
            );
            if resp_close.clicked() {
                on_close();
            }
        }

        // Minimize
        let (rect_min, resp_min) = ui.allocate_exact_size(size, Sense::click());
        let painter = ui.painter_at(rect_min);
        let hovered_min = resp_min.hovered();
        painter.circle_filled(rect_min.center(), 6.0, TRAFFIC_MIN);
        painter.circle_stroke(
            rect_min.center(),
            6.0,
            Stroke::new(0.5_f32, Color32::from_black_alpha(40)),
        );
        if hovered_min {
            painter.text(
                rect_min.center(),
                egui::Align2::CENTER_CENTER,
                "—",
                FontId::proportional(7.5),
                Color32::from_black_alpha(160),
            );
            if resp_min.clicked() {
                on_min();
            }
        }

        // Zoom / Maximize
        let (rect_zoom, resp_zoom) = ui.allocate_exact_size(size, Sense::click());
        let painter = ui.painter_at(rect_zoom);
        let hovered_zoom = resp_zoom.hovered();
        painter.circle_filled(rect_zoom.center(), 6.0, TRAFFIC_MAX);
        painter.circle_stroke(
            rect_zoom.center(),
            6.0,
            Stroke::new(0.5_f32, Color32::from_black_alpha(40)),
        );
        if hovered_zoom {
            painter.text(
                rect_zoom.center(),
                egui::Align2::CENTER_CENTER,
                "⤢",
                FontId::proportional(7.5),
                Color32::from_black_alpha(160),
            );
            if resp_zoom.clicked() {
                on_zoom();
            }
        }
    });
}

/// Renders an authentic macOS sidebar item with smooth pill highlight and theme awareness.
pub fn render_sidebar_item(
    ui: &mut Ui,
    theme: &ThemePalette,
    label: &str,
    icon: &str,
    is_selected: bool,
) -> bool {
    let desired_size = Vec2::new(ui.available_width(), 32.0);
    let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());
    let painter = ui.painter_at(rect);

    let is_hovered = response.hovered();

    let bg_color = if is_selected {
        Color32::from_rgba_premultiplied(120, 190, 32, 45) // Pantone 376 C translucent pill
    } else if is_hovered {
        theme.card_hover
    } else {
        Color32::TRANSPARENT
    };

    painter.rect_filled(rect, Rounding::same(8.0), bg_color);

    if is_selected {
        // Active vertical indicator bar
        let bar_rect = Rect::from_min_size(rect.min + Vec2::new(2.0, 7.0), Vec2::new(3.0, 18.0));
        painter.rect_filled(bar_rect, Rounding::same(2.0), PANTONE_FLAVEDO);
    }

    let text_color = if is_selected {
        PANTONE_FLAVEDO
    } else if is_hovered {
        theme.text_primary
    } else {
        theme.text_secondary
    };

    // Render Icon
    let icon_pos = Pos2::new(rect.min.x + 16.0, rect.center().y);
    painter.text(
        icon_pos,
        egui::Align2::LEFT_CENTER,
        icon,
        FontId::proportional(14.0),
        text_color,
    );

    // Render Label
    let label_pos = Pos2::new(rect.min.x + 40.0, rect.center().y);
    painter.text(
        label_pos,
        egui::Align2::LEFT_CENTER,
        label,
        FontId::proportional(12.5),
        text_color,
    );

    response.clicked()
}

/// Renders a macOS-style pill segmented control switch with theme awareness.
pub fn render_segmented_switch(
    ui: &mut Ui,
    theme: &ThemePalette,
    selected: &mut String,
    options: &[(&str, &str)],
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);
        let track_frame = egui::Frame::none()
            .fill(theme.card_bg)
            .stroke(Stroke::new(1.0_f32, theme.border))
            .rounding(Rounding::same(8.0))
            .inner_margin(egui::Margin::same(2.0));

        track_frame.show(ui, |ui| {
            for (val, label) in options {
                let is_active = selected.as_str() == *val;
                let btn = ui.add(
                    egui::Button::new(
                        egui::RichText::new(*label)
                            .size(11.0)
                            .color(if is_active {
                                Color32::WHITE
                            } else {
                                theme.text_muted
                            }),
                    )
                    .fill(if is_active {
                        PANTONE_FLAVEDO
                    } else {
                        Color32::TRANSPARENT
                    })
                    .rounding(Rounding::same(6.0)),
                );
                if btn.clicked() {
                    *selected = val.to_string();
                }
            }
        });
    });
}
