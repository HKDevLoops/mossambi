//! Apple macOS Design System & Theme Engine for Mossambi
//! Implements:
//! - 13 Named Palettes (11 canonical editor palettes + 2 Mossambi originals):
//!   Rangalipi is the signature default dark theme, Mossambi Punch is the
//!   true-fruit light theme. Glass remains config-driven via GlassEffect.
//! - Authentic Traffic Light window controls (Close, Minimize, Zoom) with subtle inner borders and hover glyphs
//! - Translucent frosted sidebar with pill navigation items
//! - Segmented capsule controls and clean Apple system typography
//! - Citrus Shade fruit tokens (CitrusShade -> PantonePalette) for accents

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
    // 11 Named Dark Themes (10 canonical + Rangalipi, the Mossambi signature)
    Gruvbox,
    TokyoNight,
    CatppuccinMocha,
    ModusVivendi,
    Nord,
    Dracula,
    Kanagawa,
    OneDark,
    Atom,
    DarkPlus,
    Synthwave,
    // Rangalipi: the default theme. Every legacy dark config id migrates
    // onto it so old config.json files keep loading.
    #[default]
    #[serde(alias = "dark")]
    #[serde(alias = "system")]
    #[serde(alias = "liquid_glass")]
    #[serde(alias = "liquidglass")]
    #[serde(alias = "frosted_glass")]
    #[serde(alias = "frostedglass")]
    #[serde(alias = "dark_anthracite")]
    #[serde(alias = "anthracite_obsidian")]
    #[serde(alias = "dark_forest")]
    #[serde(alias = "deep_forest_emerald")]
    #[serde(alias = "dark_eclipse")]
    #[serde(alias = "midnight_eclipse")]
    #[serde(alias = "dark_pewter")]
    #[serde(alias = "pewter_slate")]
    #[serde(alias = "dark_tangerine")]
    #[serde(alias = "tangerine_nightfall")]
    #[serde(alias = "dark_caviar")]
    #[serde(alias = "citrus_caviar")]
    #[serde(alias = "dark_indigo")]
    #[serde(alias = "abyssal_indigo")]
    #[serde(alias = "dark_carbon")]
    #[serde(alias = "carbon_noir")]
    #[serde(alias = "dark_merlot")]
    #[serde(alias = "vintage_merlot")]
    #[serde(alias = "dark_gunmetal")]
    #[serde(alias = "gunmetal_steel")]
    Rangalipi,
    // Mossambi Punch: the true sweet-lime fruit light theme. Every legacy
    // light config id migrates onto it.
    #[serde(alias = "light")]
    #[serde(alias = "light_brilliant")]
    #[serde(alias = "brilliant_lime")]
    #[serde(alias = "light_zest")]
    #[serde(alias = "sweet_zest_sunlight")]
    #[serde(alias = "light_blanc")]
    #[serde(alias = "blanc_de_blanc")]
    #[serde(alias = "light_glacier")]
    #[serde(alias = "glacier_aqua")]
    #[serde(alias = "light_tangerine")]
    #[serde(alias = "tangerine_sorbet")]
    #[serde(alias = "light_albedo")]
    #[serde(alias = "albedo_pure_ivory")]
    #[serde(alias = "light_quartz")]
    #[serde(alias = "cool_quartz")]
    #[serde(alias = "light_pistachio")]
    #[serde(alias = "pistachio_cream")]
    #[serde(alias = "light_amber")]
    #[serde(alias = "vanilla_amber")]
    #[serde(alias = "light_silver")]
    #[serde(alias = "silver_pearl")]
    #[serde(alias = "light_aqua")]
    #[serde(alias = "lightaqua")]
    MossambiPunch,
}

impl ThemeStyle {
    pub fn all() -> &'static [ThemeStyle] {
        static ALL: [ThemeStyle; 13] = [
            ThemeStyle::Gruvbox,
            ThemeStyle::TokyoNight,
            ThemeStyle::CatppuccinMocha,
            ThemeStyle::ModusVivendi,
            ThemeStyle::Nord,
            ThemeStyle::Dracula,
            ThemeStyle::Kanagawa,
            ThemeStyle::OneDark,
            ThemeStyle::Atom,
            ThemeStyle::DarkPlus,
            ThemeStyle::Synthwave,
            ThemeStyle::Rangalipi,
            ThemeStyle::MossambiPunch,
        ];
        &ALL
    }

    pub fn dark_themes() -> &'static [ThemeStyle] {
        &Self::all()[0..12]
    }

    pub fn light_themes() -> &'static [ThemeStyle] {
        &Self::all()[12..13]
    }

    pub fn to_pantone(&self) -> &'static crate::core::pantone::PantoneTheme {
        match self {
            Self::Gruvbox => &crate::core::pantone::THEMES[0],
            Self::TokyoNight => &crate::core::pantone::THEMES[1],
            Self::CatppuccinMocha => &crate::core::pantone::THEMES[2],
            Self::ModusVivendi => &crate::core::pantone::THEMES[3],
            Self::Nord => &crate::core::pantone::THEMES[4],
            Self::Dracula => &crate::core::pantone::THEMES[5],
            Self::Kanagawa => &crate::core::pantone::THEMES[6],
            Self::OneDark => &crate::core::pantone::THEMES[7],
            Self::Atom => &crate::core::pantone::THEMES[8],
            Self::DarkPlus => &crate::core::pantone::THEMES[9],
            Self::Synthwave => &crate::core::pantone::THEMES[10],
            Self::Rangalipi => &crate::core::pantone::THEMES[11],
            Self::MossambiPunch => &crate::core::pantone::THEMES[12],
        }
    }

    pub fn name(&self) -> &'static str {
        self.to_pantone().name
    }

    pub fn is_dark(&self) -> bool {
        self.to_pantone().is_dark
    }

    pub fn colors(&self) -> ThemePalette {
        let cfg = crate::core::MossambiConfig::load();
        self.colors_with_glass(cfg.glass_effect, cfg.window_opacity)
    }

    pub fn colors_with_glass(
        &self,
        effect: crate::core::GlassEffect,
        opacity: f32,
    ) -> ThemePalette {
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
            crate::core::GlassEffect::Opaque => {
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
                    let card_hover_g = surf_g.saturating_sub(10);
                    let card_hover_b = surf_b.saturating_sub(10);
                    ThemePalette {
                        window_bg: Color32::from_rgb(bg_r, bg_g, bg_b),
                        sidebar_bg: Color32::from_rgb(
                            bg_r.saturating_sub(6),
                            bg_g.saturating_sub(6),
                            bg_b.saturating_sub(6),
                        ),
                        card_bg: Color32::from_rgb(255, 255, 255), // Pure crisp white cards for Fluent light aesthetic
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
            crate::core::GlassEffect::FrostedGlass => {
                let effective_op = clamped_opacity.clamp(0.92, 1.00);
                let _win_a = (effective_op * 252.0) as u8;
                let side_a = (effective_op * 240.0) as u8;
                let card_a = (effective_op * 250.0) as u8;
                let hover_a = (effective_op * 252.0) as u8;
                let bord_a = (effective_op * 170.0).clamp(60.0, 255.0) as u8;

                ThemePalette {
                    // Solid, crisp window canvas (Apple HIG: main content canvas never bleeds through)
                    window_bg: Color32::from_rgb(bg_r, bg_g, bg_b),
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
                    divider: Color32::from_rgba_premultiplied(
                        bord_r,
                        bord_g,
                        bord_b,
                        (bord_a / 2).max(20),
                    ),
                    text_primary: Color32::from_rgb(tp_r, tp_g, tp_b),
                    text_secondary: Color32::from_rgb(ts_r, ts_g, ts_b),
                    text_muted: Color32::from_rgb(tm_r, tm_g, tm_b),
                    accent: Color32::from_rgb(acc_r, acc_g, acc_b),
                    highlight: Color32::from_rgb(hi_r, hi_g, hi_b),
                    is_translucent: true,
                    is_dark: pt.is_dark,
                }
            }
            crate::core::GlassEffect::LiquidGlass => {
                let effective_op = clamped_opacity.clamp(0.90, 1.00);
                let _win_a = (effective_op * 250.0) as u8;
                let side_a = (effective_op * 236.0) as u8;
                let card_a = (effective_op * 248.0) as u8;
                let hover_a = (effective_op * 250.0) as u8;
                let bord_a = (effective_op * 190.0).clamp(70.0, 255.0) as u8;

                ThemePalette {
                    // Solid, crisp window canvas (Apple HIG: main content canvas never bleeds through)
                    window_bg: Color32::from_rgb(bg_r, bg_g, bg_b),
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
                    divider: Color32::from_rgba_premultiplied(
                        bord_r,
                        bord_g,
                        bord_b,
                        (bord_a / 2).max(25),
                    ),
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

impl From<crate::core::ThemeMode> for ThemeStyle {
    fn from(m: crate::core::ThemeMode) -> Self {
        match m {
            crate::core::ThemeMode::Gruvbox => ThemeStyle::Gruvbox,
            crate::core::ThemeMode::TokyoNight => ThemeStyle::TokyoNight,
            crate::core::ThemeMode::CatppuccinMocha => ThemeStyle::CatppuccinMocha,
            crate::core::ThemeMode::ModusVivendi => ThemeStyle::ModusVivendi,
            crate::core::ThemeMode::Nord => ThemeStyle::Nord,
            crate::core::ThemeMode::Dracula => ThemeStyle::Dracula,
            crate::core::ThemeMode::Kanagawa => ThemeStyle::Kanagawa,
            crate::core::ThemeMode::OneDark => ThemeStyle::OneDark,
            crate::core::ThemeMode::Atom => ThemeStyle::Atom,
            crate::core::ThemeMode::DarkPlus => ThemeStyle::DarkPlus,
            crate::core::ThemeMode::Synthwave => ThemeStyle::Synthwave,
            crate::core::ThemeMode::Rangalipi => ThemeStyle::Rangalipi,
            crate::core::ThemeMode::MossambiPunch => ThemeStyle::MossambiPunch,
        }
    }
}

impl From<ThemeStyle> for crate::core::ThemeMode {
    fn from(s: ThemeStyle) -> Self {
        match s {
            ThemeStyle::Gruvbox => crate::core::ThemeMode::Gruvbox,
            ThemeStyle::TokyoNight => crate::core::ThemeMode::TokyoNight,
            ThemeStyle::CatppuccinMocha => crate::core::ThemeMode::CatppuccinMocha,
            ThemeStyle::ModusVivendi => crate::core::ThemeMode::ModusVivendi,
            ThemeStyle::Nord => crate::core::ThemeMode::Nord,
            ThemeStyle::Dracula => crate::core::ThemeMode::Dracula,
            ThemeStyle::Kanagawa => crate::core::ThemeMode::Kanagawa,
            ThemeStyle::OneDark => crate::core::ThemeMode::OneDark,
            ThemeStyle::Atom => crate::core::ThemeMode::Atom,
            ThemeStyle::DarkPlus => crate::core::ThemeMode::DarkPlus,
            ThemeStyle::Synthwave => crate::core::ThemeMode::Synthwave,
            ThemeStyle::Rangalipi => crate::core::ThemeMode::Rangalipi,
            ThemeStyle::MossambiPunch => crate::core::ThemeMode::MossambiPunch,
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
// Hairline strokes used by popovers (dark themes would render raw-white
// premul near-opaque). Hand-premultiplied white (const fn limitation:
// from_rgba_unmultiplied is not const-callable) - premul(255,255,255,20)
// is exactly (20,20,20,20); premul(255,255,255,14) is (14,14,14,14).
pub const MAC_BORDER: Color32 = Color32::from_rgba_premultiplied(20, 20, 20, 20);
pub const MAC_DIVIDER: Color32 = Color32::from_rgba_premultiplied(14, 14, 14, 14);
pub const MAC_TEXT_PRIMARY: Color32 = Color32::from_rgb(245, 245, 247);
pub const MAC_TEXT_SECONDARY: Color32 = Color32::from_rgb(161, 161, 166);
pub const MAC_TEXT_MUTED: Color32 = Color32::from_rgb(120, 120, 126);

// ---------------------------------------------------------------------------
// macOS UI Component Renderers
// ---------------------------------------------------------------------------

/// Renders an authentic Fluent & macOS sidebar item with smooth pill highlight and theme awareness.
pub fn render_sidebar_item(
    ui: &mut Ui,
    theme: &ThemePalette,
    label: &str,
    icon: &str,
    is_selected: bool,
) -> bool {
    render_sidebar_item_with_pin(ui, theme, label, icon, is_selected, false)
}

/// Renders a Fluent & macOS sidebar item with optional pin indicator.
pub fn render_sidebar_item_with_pin(
    ui: &mut Ui,
    theme: &ThemePalette,
    label: &str,
    icon: &str,
    is_selected: bool,
    show_pin: bool,
) -> bool {
    let desired_size = Vec2::new(ui.available_width(), 26.0);
    let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());
    let painter = ui.painter_at(rect);

    let is_hovered = response.hovered();

    // Straight-alpha fill (premultiplied with raw rgb renders near-opaque and
    // drowns the label - same class as the render_sidebar_row defect).
    let bg_color = if is_selected {
        Color32::from_rgba_unmultiplied(
            theme.accent.r(),
            theme.accent.g(),
            theme.accent.b(),
            if theme.is_dark { 45 } else { 26 },
        )
    } else if is_hovered {
        theme.card_hover
    } else {
        Color32::TRANSPARENT
    };

    painter.rect_filled(rect, Rounding::same(6.0), bg_color);

    // Ring highlight system: soft accent on hover, full accent while
    // selected, strongest while pressed or keyboard-focused.
    let ring_state = crate::gui::design::pill_state(&response, is_selected);
    crate::gui::design::paint_pill_ring(
        ui.painter(),
        rect,
        Rounding::same(6.0),
        ring_state,
        theme.accent,
    );

    if is_selected {
        // Active vertical indicator bar in accent color
        let bar_rect = Rect::from_min_size(rect.min + Vec2::new(2.0, 5.0), Vec2::new(3.0, 16.0));
        painter.rect_filled(bar_rect, Rounding::same(1.5), theme.accent);
    }

    // The accent bar marks selection; the label keeps primary contrast in
    // every theme (accent-on-accent-tint read as blue-on-blue in dark).
    let text_color = if is_selected || is_hovered {
        theme.text_primary
    } else {
        theme.text_secondary
    };

    // Render Icon
    let icon_pos = Pos2::new(rect.min.x + 14.0, rect.center().y);
    painter.text(
        icon_pos,
        egui::Align2::LEFT_CENTER,
        icon,
        FontId::proportional(14.0),
        text_color,
    );

    // Render Label
    let pin_w = if show_pin { 18.0 } else { 0.0 };
    let label_x = rect.min.x + 36.0;
    let label_max_w = (rect.max.x - label_x - pin_w - 8.0).max(24.0);
    let label_font = FontId::proportional(13.0);
    let display_label = crate::gui::design::elide_end(ui, label, &label_font, label_max_w);
    let label_pos = Pos2::new(label_x, rect.center().y);
    painter.text(
        label_pos,
        egui::Align2::LEFT_CENTER,
        display_label,
        label_font,
        text_color,
    );

    if show_pin {
        // A real vector pin glyph. The previous implementation painted an
        // empty string here, so the pin column was invisible.
        let pin_rect = Rect::from_center_size(
            Pos2::new(rect.max.x - 13.0, rect.center().y),
            Vec2::splat(15.0),
        );
        crate::core::icon_packs::paint_icon(
            ui,
            crate::core::icon_packs::IconPack::MacosFinder,
            crate::core::icon_packs::IconGlyph::ActionPin,
            pin_rect,
            if is_hovered {
                [
                    theme.text_secondary.r(),
                    theme.text_secondary.g(),
                    theme.text_secondary.b(),
                ]
            } else {
                [
                    theme.text_muted.r(),
                    theme.text_muted.g(),
                    theme.text_muted.b(),
                ]
            },
        );
    }

    response.clicked()
}

/// Renders native Windows 11 Fluent style window control buttons (Minimize, Maximize, Close).
pub fn render_fluent_window_controls(
    ui: &mut Ui,
    theme: &ThemePalette,
    is_max: bool,
    on_min: impl FnOnce(),
    on_max: impl FnOnce(),
    on_close: impl FnOnce(),
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = Vec2::ZERO;

        // Minimize Button
        let min_btn = egui::Button::new(
            egui::RichText::new("-")
                .size(13.0)
                .color(theme.text_secondary),
        )
        .min_size(Vec2::new(42.0, 28.0))
        .fill(Color32::TRANSPARENT)
        .rounding(Rounding::ZERO);
        if ui.add(min_btn).on_hover_text("Minimize").clicked() {
            on_min();
        }

        // Maximize / Restore Button
        let max_glyph = if is_max { "[=]" } else { "[]" };
        let max_btn = egui::Button::new(
            egui::RichText::new(max_glyph)
                .size(11.5)
                .color(theme.text_secondary),
        )
        .min_size(Vec2::new(42.0, 28.0))
        .fill(Color32::TRANSPARENT)
        .rounding(Rounding::ZERO);
        if ui
            .add(max_btn)
            .on_hover_text(if is_max { "Restore" } else { "Maximize" })
            .clicked()
        {
            on_max();
        }

        // Close Button
        let close_btn = egui::Button::new(
            egui::RichText::new("x")
                .size(12.0)
                .color(theme.text_secondary),
        )
        .min_size(Vec2::new(44.0, 28.0))
        .fill(Color32::TRANSPARENT)
        .rounding(Rounding::ZERO);
        let resp = ui.add(close_btn).on_hover_text("Close");
        if resp.hovered() {
            ui.painter()
                .rect_filled(resp.rect, Rounding::ZERO, Color32::from_rgb(232, 17, 35));
            ui.painter().text(
                resp.rect.center(),
                egui::Align2::CENTER_CENTER,
                "x",
                egui::FontId::proportional(12.0),
                Color32::WHITE,
            );
        }
        if resp.clicked() {
            on_close();
        }
    });
}

/// Applies eye-safe theme colors to global egui context visuals.
pub fn apply_theme_visuals(ctx: &egui::Context, palette: &ThemePalette) {
    let mut visuals = if palette.is_dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };

    // Finder-width context menus: egui defaults spacing.menu_width to 400,
    // which makes every menu (root, submenu, context) about 400 px wide.
    // macOS Finder menus run about 230 px; all menu Areas share this spacing
    // value, so one setting narrows every level including submenus.
    ctx.style_mut(|style| style.spacing.menu_width = 230.0);

    if palette.is_translucent && palette.window_bg.a() == 255 {
        // Acrylic menu material: menus and popups paint window_fill through
        // Frame::menu / Frame::popup. The main window canvas stays solid
        // while floating menus render translucent acrylic at alpha 232.
        visuals.window_fill = Color32::from_rgba_unmultiplied(
            palette.window_bg.r(),
            palette.window_bg.g(),
            palette.window_bg.b(),
            232,
        );
        visuals.menu_rounding = Rounding::same(10.0);
    } else {
        visuals.window_fill = palette.window_bg;
    }
    visuals.panel_fill = palette.sidebar_bg;
    visuals.faint_bg_color = palette.card_bg;
    visuals.extreme_bg_color = palette.window_bg;

    visuals.widgets.noninteractive.bg_fill = palette.card_bg;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, palette.text_secondary);
    visuals.widgets.noninteractive.rounding = Rounding::same(6.0);

    visuals.widgets.inactive.bg_fill = palette.card_bg;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, palette.text_primary);
    visuals.widgets.inactive.rounding = Rounding::same(6.0);

    visuals.widgets.hovered.bg_fill = palette.card_hover;
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.2_f32, palette.text_primary);
    visuals.widgets.hovered.rounding = Rounding::same(6.0);

    visuals.widgets.active.bg_fill = palette.accent;
    visuals.widgets.active.fg_stroke = Stroke::new(1.2_f32, Color32::WHITE);
    visuals.widgets.active.rounding = Rounding::same(6.0);

    visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(
        palette.accent.r(),
        palette.accent.g(),
        palette.accent.b(),
        if palette.is_dark { 45 } else { 26 },
    );
    visuals.selection.stroke = Stroke::new(1.0_f32, palette.accent);

    ctx.set_visuals(visuals);
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
                    egui::Button::new(egui::RichText::new(*label).size(19.0).color(if is_active {
                        Color32::WHITE
                    } else {
                        theme.text_muted
                    }))
                    .fill(if is_active {
                        theme.accent
                    } else {
                        Color32::TRANSPARENT
                    })
                    .rounding(Rounding::same(6.0)),
                );
                crate::gui::design::paint_pill_ring(
                    ui.painter(),
                    btn.rect,
                    Rounding::same(6.0),
                    crate::gui::design::pill_state(&btn, is_active),
                    theme.accent,
                );
                if btn.clicked() {
                    *selected = val.to_string();
                }
            }
        });
    });
}
