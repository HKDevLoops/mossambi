//! Apple macOS Design System & Theme Tokens for Mossambi
//! Implements macOS Sonoma/Sequoia window styling:
//! - Traffic light controls (Close, Minimize, Zoom) with subtle inner borders and hover glyphs
//! - Translucent frosted sidebar with pill navigation items
//! - Segmented capsule controls and clean system typography
//! - Certified Pantone Citrus Palette tokens

use eframe::egui::{self, Color32, FontId, Pos2, Rect, Rounding, Sense, Stroke, Ui, Vec2};

// ---------------------------------------------------------------------------
// macOS Design Palette & Pantone Citrus Tokens
// ---------------------------------------------------------------------------
pub const MAC_WINDOW_BG: Color32 = Color32::from_rgb(22, 22, 24);         // Deep Anthracite
pub const MAC_SIDEBAR_BG: Color32 = Color32::from_rgb(29, 29, 32);        // Translucent Sidebar
pub const MAC_CARD_BG: Color32 = Color32::from_rgb(38, 38, 43);           // Card Surface
pub const MAC_CARD_HOVER: Color32 = Color32::from_rgb(48, 48, 54);
pub const MAC_BORDER: Color32 = Color32::from_rgba_premultiplied(255, 255, 255, 20); // 1px Hairline Border
pub const MAC_DIVIDER: Color32 = Color32::from_rgba_premultiplied(255, 255, 255, 14);

pub const MAC_TEXT_PRIMARY: Color32 = Color32::from_rgb(245, 245, 247);
pub const MAC_TEXT_SECONDARY: Color32 = Color32::from_rgb(161, 161, 166);
pub const MAC_TEXT_MUTED: Color32 = Color32::from_rgb(120, 120, 126);

// Certified Pantone Tokens
pub const PANTONE_FLAVEDO: Color32 = Color32::from_rgb(120, 190, 32);     // Pantone 376 C
pub const PANTONE_ALBEDO: Color32 = Color32::from_rgb(244, 245, 240);     // Pantone 11-0601 TPG
pub const PANTONE_CARPEL: Color32 = Color32::from_rgb(255, 199, 44);      // Pantone 123 C
pub const PANTONE_HOVER: Color32 = Color32::from_rgb(255, 163, 0);        // Pantone 137 C
pub const PANTONE_GOLD: Color32 = Color32::from_rgb(255, 184, 28);        // Pantone 1235 C
pub const PANTONE_CORE: Color32 = Color32::from_rgb(252, 227, 0);         // Pantone 102 C

// Traffic Light Window Controls
pub const TRAFFIC_CLOSE: Color32 = Color32::from_rgb(255, 95, 86);        // #FF5F56
pub const TRAFFIC_MIN: Color32 = Color32::from_rgb(255, 189, 46);         // #FFBD2E
pub const TRAFFIC_MAX: Color32 = Color32::from_rgb(39, 201, 63);          // #27C93F

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
        painter.circle_stroke(rect_close.center(), 6.0, Stroke::new(0.5_f32, Color32::from_black_alpha(40)));
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
        painter.circle_stroke(rect_min.center(), 6.0, Stroke::new(0.5_f32, Color32::from_black_alpha(40)));
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
        painter.circle_stroke(rect_zoom.center(), 6.0, Stroke::new(0.5_f32, Color32::from_black_alpha(40)));
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

/// Renders an authentic macOS sidebar item with smooth pill highlight.
pub fn render_sidebar_item(ui: &mut Ui, label: &str, icon: &str, is_selected: bool) -> bool {
    let desired_size = Vec2::new(ui.available_width(), 32.0);
    let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());
    let painter = ui.painter_at(rect);

    let is_hovered = response.hovered();

    let bg_color = if is_selected {
        Color32::from_rgba_premultiplied(120, 190, 32, 45) // Pantone 376 C translucent pill
    } else if is_hovered {
        Color32::from_white_alpha(12)
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
        MAC_TEXT_PRIMARY
    } else {
        MAC_TEXT_SECONDARY
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

/// Renders a macOS-style pill segmented control switch.
pub fn render_segmented_switch(ui: &mut Ui, selected: &mut String, options: &[(&str, &str)]) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);
        let track_frame = egui::Frame::none()
            .fill(Color32::from_rgb(32, 32, 36))
            .stroke(Stroke::new(1.0_f32, MAC_BORDER))
            .rounding(Rounding::same(8.0))
            .inner_margin(egui::Margin::same(2.0));

        track_frame.show(ui, |ui| {
            for (val, label) in options {
                let is_active = selected.as_str() == *val;
                let btn = ui.add(
                    egui::Button::new(
                        egui::RichText::new(*label)
                            .size(11.0)
                            .color(if is_active { Color32::WHITE } else { MAC_TEXT_MUTED }),
                    )
                    .fill(if is_active { PANTONE_FLAVEDO } else { Color32::TRANSPARENT })
                    .rounding(Rounding::same(6.0)),
                );
                if btn.clicked() {
                    *selected = val.to_string();
                }
            }
        });
    });
}
