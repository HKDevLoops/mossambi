//! Finder Chrome Components
//!
//! The shared building blocks of the macOS-Finder-style chrome: traffic lights
//! with real hover glyphs, the merged toolbar row, the bottom path bar, the
//! filter popover, and the standard-files-only sidebar section.
//!
//! These are deliberately surface-agnostic so the Studio explorer, the
//! Preferences pane, and the detached widget side panel all draw the same
//! chrome with the same proportions.
//!
//! All content is pure ASCII.

use crate::core::icon_packs::{IconGlyph, IconPack};
use crate::gui::design::{self, layout, radius, space, type_scale};
use crate::gui::macos_theme::ThemePalette;
use eframe::egui::{self, Color32, FontId, Pos2, Rect, Rounding, Sense, Stroke, Ui, Vec2};

// ---------------------------------------------------------------------------
// Traffic lights
// ---------------------------------------------------------------------------

/// RGB of the macOS close button.
const TL_CLOSE: Color32 = Color32::from_rgb(255, 95, 86);
/// RGB of the macOS minimize button.
const TL_MIN: Color32 = Color32::from_rgb(255, 189, 46);
/// RGB of the macOS zoom button.
const TL_MAX: Color32 = Color32::from_rgb(39, 201, 63);

/// Draws a Finder-style traffic-light control cluster.
///
/// Each dot reveals its glyph only on hover, exactly like macOS: a cross for
/// close, a bar for minimize, and a plus for zoom. The glyphs are drawn as
/// vector strokes (not font glyphs) so they stay crisp at any DPI and never
/// depend on a font shipping the right codepoint.
pub fn render_traffic_lights(
    ui: &mut Ui,
    _palette: &ThemePalette,
    on_close: impl FnOnce(),
    on_min: impl FnOnce(),
    on_zoom: impl FnOnce(),
) {
    let d = layout::TRAFFIC_LIGHT_D;
    let size = Vec2::splat(d);

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(layout::TRAFFIC_LIGHT_GAP, 0.0);

        // Close (red, cross glyph)
        let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
        let painter = ui.painter_at(rect);
        painter.circle_filled(rect.center(), d / 2.0, TL_CLOSE);
        painter.circle_stroke(
            rect.center(),
            d / 2.0,
            Stroke::new(0.5_f32, Color32::from_black_alpha(40)),
        );
        if resp.hovered() {
            draw_x_glyph(&painter, rect.center(), d * 0.36);
        }
        if resp.clicked() {
            on_close();
        }

        // Minimize (yellow, bar glyph)
        let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
        let painter = ui.painter_at(rect);
        painter.circle_filled(rect.center(), d / 2.0, TL_MIN);
        painter.circle_stroke(
            rect.center(),
            d / 2.0,
            Stroke::new(0.0_f32 + 0.5, Color32::from_black_alpha(40)),
        );
        if resp.hovered() {
            let stroke = Stroke::new(1.1_f32, Color32::from_black_alpha(170));
            painter.line_segment(
                [
                    rect.center() - Vec2::new(d * 0.22, 0.0),
                    rect.center() + Vec2::new(d * 0.22, 0.0),
                ],
                stroke,
            );
        }
        if resp.clicked() {
            on_min();
        }

        // Zoom (green, plus glyph)
        let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
        let painter = ui.painter_at(rect);
        painter.circle_filled(rect.center(), d / 2.0, TL_MAX);
        painter.circle_stroke(
            rect.center(),
            d / 2.0,
            Stroke::new(0.5_f32, Color32::from_black_alpha(40)),
        );
        if resp.hovered() {
            let stroke = Stroke::new(1.1_f32, Color32::from_black_alpha(170));
            painter.line_segment(
                [
                    rect.center() - Vec2::new(d * 0.22, 0.0),
                    rect.center() + Vec2::new(d * 0.22, 0.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    rect.center() - Vec2::new(0.0, d * 0.22),
                    rect.center() + Vec2::new(0.0, d * 0.22),
                ],
                stroke,
            );
        }
        if resp.clicked() {
            on_zoom();
        }
    });
}

/// Draws a small vector "x" centered at `center`.
fn draw_x_glyph(painter: &egui::Painter, center: egui::Pos2, size: f32) {
    let stroke = Stroke::new(1.1_f32, Color32::from_black_alpha(170));
    let h = Vec2::new(size * 0.5, size * 0.5);
    painter.line_segment([center - h, center + h], stroke);
    painter.line_segment(
        [center + Vec2::new(-h.x, h.y), center + Vec2::new(h.x, -h.y)],
        stroke,
    );
}

// ---------------------------------------------------------------------------
// Toolbar
// ---------------------------------------------------------------------------

/// A square, icon-only toolbar button in Finder style.
///
/// Returns true when clicked. The button carries no text, so it can never
/// collide with a neighbouring label.
pub fn toolbar_icon_button(
    ui: &mut Ui,
    palette: &ThemePalette,
    pack: IconPack,
    glyph: IconGlyph,
    accent_rgb: [u8; 3],
    tooltip: &str,
    active: bool,
) -> bool {
    let side = layout::TOOLBAR_H - space::MD * 2.0;
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(side), Sense::click());
    let painter = ui.painter_at(rect);

    if active {
        painter.rect_filled(rect, Rounding::same(radius::MD), palette.card_hover);
    } else if resp.hovered() {
        painter.rect_filled(
            rect,
            Rounding::same(radius::MD),
            Color32::from_rgba_premultiplied(
                palette.text_primary.r(),
                palette.text_primary.g(),
                palette.text_primary.b(),
                22,
            ),
        );
    }

    crate::core::icon_packs::paint_icon(ui, pack, glyph, rect, accent_rgb);

    let clicked = resp.clicked();
    resp.on_hover_ui(|ui| show_tooltip(ui, tooltip.to_owned()));
    clicked
}

/// Renders the tooltip body.
fn show_tooltip(ui: &mut Ui, text: String) {
    egui::Frame::popup(ui.style())
        .fill(Color32::from_rgb(38, 38, 42))
        .stroke(Stroke::new(1.0_f32, Color32::from_black_alpha(90)))
        .rounding(Rounding::same(radius::SM))
        .inner_margin(egui::Margin::symmetric(space::MD, space::SM))
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(text)
                    .size(type_scale::SMALL)
                    .color(Color32::WHITE),
            );
        });
}

/// A labeled toolbar button used for text-bearing actions (e.g. "Group").
pub fn toolbar_text_button(ui: &mut Ui, palette: &ThemePalette, label: &str, active: bool) -> bool {
    let text = egui::RichText::new(label)
        .size(type_scale::BODY)
        .color(if active {
            palette.text_primary
        } else {
            palette.text_secondary
        });
    let btn = egui::Button::new(text)
        .fill(if active {
            palette.card_hover
        } else {
            Color32::TRANSPARENT
        })
        .stroke(Stroke::NONE)
        .rounding(Rounding::same(radius::MD))
        .min_size(Vec2::new(0.0, layout::TOOLBAR_H - space::MD * 2.0));
    let resp = ui.add(btn);
    if resp.hovered() {
        let painter = ui.painter();
        let rect = resp.rect;
        painter.rect_filled(
            rect,
            Rounding::same(radius::MD),
            Color32::from_rgba_premultiplied(
                palette.text_primary.r(),
                palette.text_primary.g(),
                palette.text_primary.b(),
                18,
            ),
        );
    }
    resp.clicked()
}

/// Splits the toolbar into a leading group and a trailing group with a flexible
/// spacer between them, so the layout stays balanced at any window width.
pub fn toolbar_row(ui: &mut Ui, build: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(space::XS, 0.0);
        build(ui);
    });
}

// ---------------------------------------------------------------------------
// Path bar (bottom, Finder style)
// ---------------------------------------------------------------------------

/// Renders the Finder-style bottom path bar.
///
/// Segments are separated by a chevron; the last segment is emphasized. Long
/// paths elide from the middle so the leaf folder name always stays visible.
pub fn render_path_bar(
    ui: &mut Ui,
    palette: &ThemePalette,
    segments: &[String],
    on_segment: impl Fn(usize),
) {
    let font_id = FontId::proportional(type_scale::SMALL);
    let frame = egui::Frame::none()
        .fill(palette.sidebar_bg)
        .inner_margin(egui::Margin::symmetric(space::MD, space::XS));
    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(space::XS, 0.0);
            let last = segments.len().saturating_sub(1);
            for (i, seg) in segments.iter().enumerate() {
                if i > 0 {
                    ui.label(
                        egui::RichText::new(">")
                            .size(type_scale::CAPTION)
                            .color(palette.text_muted),
                    );
                }
                let is_last = i == last;
                let label = design::elide_middle(ui, seg, &font_id, 220.0);
                let text = egui::RichText::new(label)
                    .size(type_scale::SMALL)
                    .color(if is_last {
                        palette.text_primary
                    } else {
                        palette.text_secondary
                    });
                if ui
                    .add(
                        egui::Button::new(text)
                            .fill(Color32::TRANSPARENT)
                            .stroke(Stroke::NONE),
                    )
                    .clicked()
                {
                    on_segment(i);
                }
            }
        });
    });
}

// ---------------------------------------------------------------------------
// Filter popover
// ---------------------------------------------------------------------------

/// The category filters available in the filter popover.
pub const FILTER_CATEGORIES: [&str; 8] = [
    "All Files",
    "Images",
    "Audio",
    "Video",
    "Documents",
    "Archives",
    "Code",
    "Folders",
];

/// Renders the compact filter popover body.
///
/// Replaces the old always-visible chip row: the funnel button opens this,
/// and the counts live here instead of stealing a full toolbar row.
pub fn render_filter_popover(
    ui: &mut Ui,
    palette: &ThemePalette,
    categories: &[(&str, usize)],
    selected: usize,
    on_select: impl Fn(usize),
) {
    let frame = egui::Frame::popup(ui.style())
        .fill(palette.card_bg)
        .stroke(Stroke::new(1.0_f32, palette.border))
        .rounding(Rounding::same(radius::LG))
        .inner_margin(egui::Margin::symmetric(space::SM, space::MD));
    frame.show(ui, |ui| {
        ui.set_min_width(196.0);
        ui.label(
            egui::RichText::new("Filter by Kind")
                .size(type_scale::SMALL)
                .strong()
                .color(palette.text_primary),
        );
        ui.add_space(space::XS);
        for (i, (name, count)) in categories.iter().enumerate() {
            let is_active = i == selected;
            let row = egui::Frame::none()
                .fill(if is_active {
                    Color32::from_rgba_premultiplied(
                        palette.accent.r(),
                        palette.accent.g(),
                        palette.accent.b(),
                        46,
                    )
                } else {
                    Color32::TRANSPARENT
                })
                .rounding(Rounding::same(radius::SM))
                .inner_margin(egui::Margin::symmetric(space::SM, space::XS));
            row.show(ui, |ui| {
                ui.horizontal(|ui| {
                    let text =
                        egui::RichText::new(*name)
                            .size(type_scale::BODY)
                            .color(if is_active {
                                palette.text_primary
                            } else {
                                palette.text_secondary
                            });
                    if ui
                        .add(egui::Label::new(text).sense(Sense::click()))
                        .clicked()
                    {
                        on_select(i);
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(count.to_string())
                                .size(type_scale::SMALL)
                                .color(palette.text_muted),
                        );
                    });
                });
            });
        }
    });
}

// ---------------------------------------------------------------------------
// Sidebar
// ---------------------------------------------------------------------------

/// A standard macOS-Finder sidebar section header (uppercase, small, muted).
pub fn render_sidebar_section_header(ui: &mut Ui, palette: &ThemePalette, label: &str) {
    ui.add_space(space::MD);
    ui.label(
        egui::RichText::new(label.to_ascii_uppercase())
            .size(type_scale::CAPTION)
            .strong()
            .color(palette.text_muted),
    );
    ui.add_space(space::XXS);
}

/// A collapsible sidebar section header with ASCII chevron and hover feedback.
pub fn render_sidebar_collapsible_header(
    ui: &mut Ui,
    palette: &ThemePalette,
    label: &str,
    expanded: bool,
) -> bool {
    ui.add_space(space::SM);
    let row_h = 20.0;
    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), row_h), Sense::click());
    let painter = ui.painter_at(rect);
    let chevron = if expanded { "v" } else { ">" };
    let text = format!("{} {}", chevron, label.to_ascii_uppercase());
    let is_hovered = resp.hovered();
    let col = if is_hovered {
        palette.text_primary
    } else {
        palette.text_muted
    };
    painter.text(
        Pos2::new(rect.min.x + 2.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        text,
        FontId::proportional(type_scale::CAPTION),
        col,
    );
    ui.add_space(space::XXS);
    resp.clicked()
}

/// Everything a sidebar row needs to draw itself.
///
/// Grouping these into one struct keeps the call sites readable and keeps the
/// argument count below the clippy ceiling without resorting to an allow
/// attribute.
#[derive(Debug, Clone)]
pub struct SidebarRowSpec<'a> {
    /// The active icon pack.
    pub pack: IconPack,
    /// Which vector glyph to draw.
    pub glyph: IconGlyph,
    /// RGB tint applied to the glyph.
    pub accent_rgb: [u8; 3],
    /// The row label (elided to fit the available width).
    pub label: &'a str,
    /// Whether the row is the active location.
    pub selected: bool,
    /// Whether to draw the pin affordance on the right.
    pub show_pin: bool,
}

/// A standard sidebar row with a real vector icon, a label, and optional pin.
///
/// The label elides to the available width instead of overflowing into the
/// next column, which is the "Custom Conforme" truncation bug fixed at source.
pub fn render_sidebar_row(ui: &mut Ui, palette: &ThemePalette, spec: &SidebarRowSpec<'_>) -> bool {
    let row_h = 26.0;
    let desired_size = Vec2::new(ui.available_width(), row_h);
    let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());
    let is_hovered = response.hovered();
    let painter = ui.painter_at(rect);

    let bg_color = if spec.selected {
        Color32::from_rgba_premultiplied(
            palette.accent.r(),
            palette.accent.g(),
            palette.accent.b(),
            if palette.is_dark { 45 } else { 26 },
        )
    } else if is_hovered {
        palette.card_hover
    } else {
        Color32::TRANSPARENT
    };

    if bg_color != Color32::TRANSPARENT {
        painter.rect_filled(rect, Rounding::same(radius::SM), bg_color);
    }

    if spec.selected {
        let bar_rect = Rect::from_min_size(
            rect.min + Vec2::new(2.0, 4.0),
            Vec2::new(3.0, (row_h - 8.0).max(4.0)),
        );
        painter.rect_filled(bar_rect, Rounding::same(1.5), palette.accent);
    }

    let icon_side = 16.0;
    let pin_side = if spec.show_pin { 14.0 } else { 0.0 };
    let icon_rect = Rect::from_center_size(
        Pos2::new(rect.min.x + 10.0 + icon_side / 2.0, rect.center().y),
        Vec2::splat(icon_side),
    );
    crate::core::icon_packs::paint_icon(ui, spec.pack, spec.glyph, icon_rect, spec.accent_rgb);

    let label_x = rect.min.x + 10.0 + icon_side + space::MD;
    let label_max_w = (rect.max.x - label_x - pin_side - space::SM).max(space::LG);
    let font_id = FontId::proportional(type_scale::BODY_SM);
    let display_label = design::elide_end(ui, spec.label, &font_id, label_max_w);
    let text_color = if spec.selected {
        if palette.is_dark {
            palette.accent
        } else {
            palette.text_primary
        }
    } else if is_hovered {
        palette.text_primary
    } else {
        palette.text_secondary
    };

    painter.text(
        Pos2::new(label_x, rect.center().y),
        egui::Align2::LEFT_CENTER,
        display_label,
        font_id,
        text_color,
    );

    if spec.show_pin {
        let pin_rect = Rect::from_center_size(
            Pos2::new(rect.max.x - 12.0, rect.center().y),
            Vec2::splat(pin_side),
        );
        crate::core::icon_packs::paint_icon(
            ui,
            spec.pack,
            IconGlyph::ActionPin,
            pin_rect,
            if is_hovered {
                [
                    palette.text_secondary.r(),
                    palette.text_secondary.g(),
                    palette.text_secondary.b(),
                ]
            } else {
                [
                    palette.text_muted.r(),
                    palette.text_muted.g(),
                    palette.text_muted.b(),
                ]
            },
        );
    }

    response.clicked()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_categories_are_complete() {
        assert_eq!(FILTER_CATEGORIES.len(), 8);
        assert_eq!(FILTER_CATEGORIES[0], "All Files");
    }

    #[test]
    fn traffic_light_colors_are_distinct() {
        assert_ne!(TL_CLOSE, TL_MIN);
        assert_ne!(TL_MIN, TL_MAX);
        assert_ne!(TL_CLOSE, TL_MAX);
    }
}
