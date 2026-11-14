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
use crate::gui::explorer_view::ViewMode;
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

/// Paints the thin Finder-style chevron-right separator drawn between
/// path-bar segments.
///
/// Vector strokes stay crisp at any DPI and never depend on a font
/// shipping a codepoint; the hover-sensed slot keeps the separator
/// from eating clicks and is centered vertically by the horizontal
/// layout, like the text separators it replaces.
pub fn paint_path_chevron(ui: &mut Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(5.5, 10.0), Sense::hover());
    ui.painter().add(egui::Shape::line(
        vec![
            rect.left_top() + Vec2::new(0.9, 1.1),
            rect.right_center() - Vec2::new(0.9, 0.0),
            rect.left_bottom() + Vec2::new(0.9, -1.1),
        ],
        Stroke::new(1.8_f32, color),
    ));
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
    let side = 29.0;
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(side), Sense::click());
    let painter = ui.painter_at(rect);

    if active {
        painter.rect_filled(rect, Rounding::same(radius::MD), palette.card_hover);
    } else if resp.hovered() {
        painter.rect_filled(
            rect,
            Rounding::same(radius::MD),
            // Straight-alpha hover tint: premultiplied with raw rgb renders
            // near-opaque and drowns the glyph (premul-vs-unmul semantics).
            Color32::from_rgba_unmultiplied(
                palette.text_primary.r(),
                palette.text_primary.g(),
                palette.text_primary.b(),
                22,
            ),
        );
    }

    let icon_rect = Rect::from_center_size(rect.center(), Vec2::splat(22.0));
    crate::core::icon_packs::paint_icon(ui, pack, glyph, icon_rect, accent_rgb);

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
            Color32::from_rgba_unmultiplied(
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

/// Renders an authentic macOS Finder connected segmented view switcher control:
/// [ Icons | List | Columns ] in a unified pill container with active highlight.
pub fn render_segmented_view_switcher(
    ui: &mut Ui,
    palette: &ThemePalette,
    pack: IconPack,
    current_mode: ViewMode,
    mut on_select: impl FnMut(ViewMode),
) {
    let height = 28.0_f32;
    let seg_w = 34.0_f32;
    let total_w = seg_w * 3.0;
    let (container_rect, _) = ui.allocate_exact_size(Vec2::new(total_w, height), Sense::hover());
    let painter = ui.painter_at(container_rect);

    // Subtle background pill & border (straight alpha: premul with raw
    // rgb renders near-opaque and would glare in dark themes)
    let bg_col = if palette.is_dark {
        Color32::from_rgba_unmultiplied(255, 255, 255, 14)
    } else {
        Color32::from_rgba_unmultiplied(0, 0, 0, 10)
    };
    painter.rect_filled(container_rect, Rounding::same(radius::MD), bg_col);
    painter.rect_stroke(
        container_rect,
        Rounding::same(radius::MD),
        Stroke::new(1.0_f32, palette.border),
    );

    let modes = [
        (
            ViewMode::Grid,
            IconGlyph::ActionView,
            "Icon Grid View (Cmd+1)",
        ),
        (
            ViewMode::Details,
            IconGlyph::ActionList,
            "Details List View (Cmd+2)",
        ),
        (
            ViewMode::Columns,
            IconGlyph::ActionColumns,
            "Miller Columns View (Cmd+3)",
        ),
    ];

    for (idx, (mode, glyph, tip)) in modes.into_iter().enumerate() {
        let seg_rect = Rect::from_min_size(
            Pos2::new(
                container_rect.min.x + idx as f32 * seg_w,
                container_rect.min.y,
            ),
            Vec2::new(seg_w, height),
        );
        let resp = ui.interact(
            seg_rect,
            ui.make_persistent_id(("view_switcher_seg", idx)),
            Sense::click(),
        );
        let is_active = current_mode == mode;

        if is_active {
            let active_rect = seg_rect.shrink2(Vec2::new(2.0, 2.0));
            painter.rect_filled(active_rect, Rounding::same(radius::SM), palette.accent);
        } else if resp.hovered() {
            let hover_rect = seg_rect.shrink2(Vec2::new(2.0, 2.0));
            painter.rect_filled(
                hover_rect,
                Rounding::same(radius::SM),
                Color32::from_rgba_unmultiplied(
                    palette.text_primary.r(),
                    palette.text_primary.g(),
                    palette.text_primary.b(),
                    18,
                ),
            );
        }

        // Ring highlight system: soft accent on hover, full accent on the
        // active segment, strongest while pressed or keyboard-focused. Painted
        // with the full ui painter so the ring may breathe past the container.
        let ring_state = design::pill_state(&resp, is_active);
        design::paint_pill_ring(
            ui.painter(),
            seg_rect.shrink2(Vec2::new(2.0, 2.0)),
            Rounding::same(radius::SM),
            ring_state,
            palette.accent,
        );

        // Dividers between inactive items
        if idx > 0 && !is_active && current_mode != modes[idx - 1].0 {
            let div_x = seg_rect.min.x;
            painter.line_segment(
                [
                    Pos2::new(div_x, container_rect.min.y + 6.0),
                    Pos2::new(div_x, container_rect.max.y - 6.0),
                ],
                Stroke::new(1.0_f32, palette.border),
            );
        }

        let icon_rect = Rect::from_center_size(seg_rect.center(), Vec2::splat(18.0));
        let tint = if is_active {
            [255, 255, 255]
        } else {
            [
                palette.text_secondary.r(),
                palette.text_secondary.g(),
                palette.text_secondary.b(),
            ]
        };
        crate::core::icon_packs::paint_icon(ui, pack, glyph, icon_rect, tint);

        let clicked = resp.clicked();
        resp.on_hover_text(tip);
        if clicked {
            on_select(mode);
        }
    }
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
                    Color32::from_rgba_unmultiplied(
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
/// A stacked sidebar zone header with an identity boundary: a faint 1px
/// hairline above the header (Finder group boundary), then the disclosure
/// chevron and uppercase caption label. Returns true on click; callers
/// toggle the zone and persist the new state in the config.
pub fn render_sidebar_zone_header(
    ui: &mut Ui,
    palette: &ThemePalette,
    label: &str,
    expanded: bool,
) -> bool {
    let style = crate::core::MossambiConfig::load().sidebar_style;
    if style != crate::core::SidebarStyle::Thunar {
        ui.add_space(space::SM);
        // Zone identity boundary: hairline across the content width.
        let cursor = ui.cursor();
        let line_col = Color32::from_rgba_unmultiplied(
            palette.border.r(),
            palette.border.g(),
            palette.border.b(),
            90,
        );
        ui.painter().line_segment(
            [
                Pos2::new(cursor.min.x + 2.0, cursor.top() + 0.5),
                Pos2::new(cursor.max.x - 2.0, cursor.top() + 0.5),
            ],
            Stroke::new(1.0_f32, line_col),
        );
    }
    ui.add_space(space::XS);
    let row_h = if style == crate::core::SidebarStyle::Thunar {
        20.0
    } else {
        22.0
    };
    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), row_h), Sense::click());
    let painter = ui.painter_at(rect);
    let col = if resp.hovered() {
        palette.text_primary
    } else {
        palette.text_muted
    };
    let text_x = if style == crate::core::SidebarStyle::Windows {
        rect.min.x + 16.0
    } else {
        rect.min.x + 4.0
    };
    painter.text(
        Pos2::new(text_x, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        FontId::proportional(type_scale::CAPTION + 0.5),
        col,
    );
    let c = if style == crate::core::SidebarStyle::Windows {
        Pos2::new(rect.min.x + 6.0, rect.center().y)
    } else {
        Pos2::new(rect.max.x - 10.0, rect.center().y)
    };
    let s = 3.5_f32;
    let chevron_col = if resp.hovered() {
        palette.text_secondary
    } else {
        Color32::from_rgba_unmultiplied(
            palette.text_muted.r(),
            palette.text_muted.g(),
            palette.text_muted.b(),
            140,
        )
    };
    let pts = if expanded {
        [
            Pos2::new(c.x - s, c.y - s * 0.5),
            Pos2::new(c.x, c.y + s * 0.5),
            Pos2::new(c.x + s, c.y - s * 0.5),
        ]
    } else {
        [
            Pos2::new(c.x - s * 0.5, c.y - s),
            Pos2::new(c.x + s * 0.5, c.y),
            Pos2::new(c.x - s * 0.5, c.y + s),
        ]
    };
    painter.line_segment([pts[0], pts[1]], Stroke::new(1.4_f32, chevron_col));
    painter.line_segment([pts[1], pts[2]], Stroke::new(1.4_f32, chevron_col));
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

/// What the user clicked on a sidebar row.
///
/// The pin strip is its own hit region, so a row action and a pin toggle
/// can never fire from the same click.
#[derive(Debug, Clone, Default)]
pub struct SidebarRowClick {
    /// The row body was clicked (navigate/activate).
    pub row: bool,
    /// The pin strip was clicked (toggle the pin).
    pub pin: bool,
    /// Dragged files dropped onto this row.
    pub dropped: Option<crate::gui::explorer_view::FileDragPayload>,
}

/// A standard sidebar row with a real vector icon, a label, and an
/// interactive pin strip.
///
/// The label elides to the available width instead of overflowing into the
/// next column, which is the "Custom Conforme" truncation bug fixed at source.
/// When `show_pin` is set, the right edge of the row is a dedicated pin hit
/// region - a real control, not a decorative glyph.
pub fn render_sidebar_row(
    ui: &mut Ui,
    palette: &ThemePalette,
    spec: &SidebarRowSpec<'_>,
) -> SidebarRowClick {
    let cfg = crate::core::MossambiConfig::load();
    let style = cfg.sidebar_style;
    let row_h = style.sidebar_row_height();
    let desired_size = Vec2::new(ui.available_width(), row_h);
    let (rect, hover_resp) = ui.allocate_exact_size(desired_size, Sense::hover());

    // Non-overlapping hit regions: the row action covers everything left of
    // the pin strip; the pin strip is its own click target on the right edge.
    let pin_strip_w = if spec.show_pin { 26.0 } else { 0.0 };
    let row_hit_rect =
        Rect::from_min_max(rect.min, Pos2::new(rect.max.x - pin_strip_w, rect.max.y));
    let row_id = egui::Id::new(("mossambi_sidebar_row", rect.min.x as i32, rect.min.y as i32));
    let row_resp = ui.interact(row_hit_rect, row_id, Sense::click());
    let is_hovered = hover_resp.hovered() || row_resp.hovered();
    let painter = ui.painter_at(rect);

    // Straight-alpha fill. Feeding raw accent rgb into from_rgba_premultiplied
    // renders the selection close to full opacity, drowning the accent label
    // (blue-on-blue selected rows, first seen on the Drives sidebar rows).
    let bg_color = if spec.selected {
        Color32::from_rgba_unmultiplied(
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

    let (rounding, draw_accent_bar) = match style {
        crate::core::SidebarStyle::Finder => (Rounding::same(radius::SM), true),
        crate::core::SidebarStyle::Windows => (Rounding::same(radius::XS), false),
        crate::core::SidebarStyle::Nautilus => (Rounding::same(8.0), false),
        crate::core::SidebarStyle::Thunar => (Rounding::same(2.0), false),
    };

    if bg_color != Color32::TRANSPARENT {
        painter.rect_filled(rect, rounding, bg_color);
        if style == crate::core::SidebarStyle::Thunar && spec.selected {
            painter.rect_stroke(rect, rounding, Stroke::new(1.0_f32, palette.accent));
        }
    }

    let is_drop_hovered = row_resp
        .dnd_hover_payload::<crate::gui::explorer_view::FileDragPayload>()
        .is_some();
    if is_drop_hovered {
        painter.rect_stroke(rect, rounding, Stroke::new(1.5_f32, palette.accent));
    }
    let dropped = row_resp
        .dnd_release_payload::<crate::gui::explorer_view::FileDragPayload>()
        .map(|p| (*p).clone());

    if spec.selected && draw_accent_bar {
        let bar_rect = Rect::from_min_size(
            rect.min + Vec2::new(2.0, 4.0),
            Vec2::new(3.0, (row_h - 8.0).max(4.0)),
        );
        painter.rect_filled(bar_rect, Rounding::same(1.5), palette.accent);
    }

    let icon_side = 24.0;
    let icon_rect = Rect::from_center_size(
        Pos2::new(rect.min.x + 10.0 + icon_side / 2.0, rect.center().y),
        Vec2::splat(icon_side),
    );
    crate::core::icon_packs::paint_icon(ui, spec.pack, spec.glyph, icon_rect, spec.accent_rgb);

    let label_x = rect.min.x + 10.0 + icon_side + space::MD;
    let label_max_w = (rect.max.x - label_x - pin_strip_w - space::SM).max(space::LG);
    let font_id = FontId::proportional(14.0);
    let display_label = design::elide_end(ui, spec.label, &font_id, label_max_w);
    // High-contrast labels: the 3px accent bar already marks selection, so the
    // label itself always keeps primary contrast. Painting the label in
    // palette.accent over an accent-tinted row read as blue-on-blue in dark
    // themes (the Drives sidebar rows defect).
    let text_color = if spec.selected || is_hovered {
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

    let mut pin_clicked = false;
    if spec.show_pin {
        let pin_rect =
            Rect::from_min_max(Pos2::new(rect.max.x - pin_strip_w, rect.min.y), rect.max);
        let pin_id = egui::Id::new(("mossambi_sidebar_pin", rect.min.x as i32, rect.min.y as i32));
        let pin_resp = ui.interact(pin_rect, pin_id, Sense::click());
        let pin_rect_glyph = Rect::from_center_size(
            Pos2::new(rect.max.x - pin_strip_w / 2.0, rect.center().y),
            Vec2::splat(16.0),
        );
        let pin_color = if pin_resp.hovered() {
            // Direct pin hover: the brightest affordance tier.
            [
                palette.text_primary.r(),
                palette.text_primary.g(),
                palette.text_primary.b(),
            ]
        } else if is_hovered {
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
        };
        crate::core::icon_packs::paint_icon(
            ui,
            spec.pack,
            IconGlyph::ActionPin,
            pin_rect_glyph,
            pin_color,
        );
        pin_clicked = pin_resp.clicked();
    }

    SidebarRowClick {
        row: row_resp.clicked(),
        pin: pin_clicked,
        dropped,
    }
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
