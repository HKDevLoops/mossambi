//! Custom Liquid Glass & Pantone Progress Bar Widget
//!
//! Provides smooth, anti-aliased pill progress bars with:
//! - Specular top refraction highlight line
//! - Active Pantone accent gradient fill
//! - Centered high-contrast percentage / status text
//! - Configurable height, rounding, and tooltip

use crate::gui::macos_theme::ThemePalette;
use eframe::egui::{self, Color32, Rect, Rounding, Stroke, Ui, Vec2};

pub fn render_custom_progress_bar(
    ui: &mut Ui,
    fraction: f32,
    text: Option<&str>,
    palette: &ThemePalette,
    height: f32,
    width: Option<f32>,
) -> egui::Response {
    let clamped = fraction.clamp(0.0, 1.0);
    let target_w = width.unwrap_or_else(|| ui.available_width());
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(target_w, height), egui::Sense::hover());

    let painter = ui.painter_at(rect);
    let rounding = Rounding::same(height * 0.5);

    // 1. Background Pill Container
    painter.rect_filled(rect, rounding, palette.card_bg);
    painter.rect_stroke(rect, rounding, Stroke::new(1.0_f32, palette.border));

    // 2. Active Filled Bar with Pantone Accent
    if clamped > 0.001 {
        let fill_w = (rect.width() * clamped).max(height);
        let fill_rect = Rect::from_min_size(rect.min, Vec2::new(fill_w, rect.height()));
        painter.rect_filled(fill_rect, rounding, palette.accent);

        // Specular Liquid Glass Top Highlight
        let spec_rect = Rect::from_min_size(
            fill_rect.min + Vec2::new(2.0, 1.0),
            Vec2::new((fill_w - 4.0).max(0.0), (height * 0.4).max(1.0)),
        );
        painter.rect_filled(
            spec_rect,
            Rounding::same(height * 0.25),
            Color32::from_rgba_premultiplied(255, 255, 255, 60),
        );
    }

    // 3. Centered Progress Text
    let label = text
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("{:.0}%", clamped * 100.0));
    let font_size = (height * 0.68).clamp(8.5, 12.0);
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(font_size),
        if clamped > 0.55 {
            Color32::WHITE
        } else {
            palette.text_primary
        },
    );

    resp
}
