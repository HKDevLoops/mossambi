//! Pixel-Accurate 8-Petal Radial Fruit Widget Geometry
//! Perfectly reproduces all 6 visual states from `Image reference.png`:
//! 1. Idle State: Clean light off-white rounded capsule petals, frosted slate center hub with upward arrow "Drop files here".
//! 2. Hover State: Soft glowing illuminated lime-green petal with green icon and dark label.
//! 3. Drag File: Highlighted drop target petal with dragged file detection.
//! 4. Converting State: Clean pure white center hub with large bold "65%" and "Converting...", surrounded by smooth green progress ring.
//! 5. Success State: Clean pure white center hub with green circular checkmark badge and "Converted \n N files".
//! 6. Dark Mode: Deep charcoal/slate matte petals, bright white typography, and vibrant emerald green highlighted active petal.

use eframe::egui::{self, epaint::PathShape, Color32, FontId, Painter, Pos2, Rect, Stroke, Vec2};
use std::f32::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetTheme {
    Light, // States 1-5 in Image reference.png
    Dark,  // State 6 in Image reference.png
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WidgetState {
    Idle,
    Hovered(usize),
    Converting { progress: f32 },
    Success { file_count: usize },
}

// --------------------------------------------------------------------------
// Pie & Marking-Menu Geometry Primitives (Pure Functions)
// --------------------------------------------------------------------------

pub const DEFAULT_DONUT_DIAMETER: f32 = 360.0;
pub const MIN_DONUT_DIAMETER: f32 = 280.0;
pub const MAX_DONUT_DIAMETER: f32 = 480.0;
pub const DEFAULT_WEDGE_GAP_PX: f32 = 2.0;
pub const HOVER_GROW_PX: f32 = 6.0;
pub const MARKING_MENU_MIN_DISTANCE: f32 = 24.0;
pub const MAX_INNER_RING_WEDGES: usize = 8;

/// Open animation progress (120ms ease-out).
/// Returns (scale, alpha) where scale goes 0.85 -> 1.0 and alpha goes 0.0 -> 1.0.
#[inline]
pub fn open_animation_factors(elapsed_sec: f32) -> (f32, f32) {
    const DURATION: f32 = 0.120;
    if elapsed_sec >= DURATION {
        (1.0, 1.0)
    } else {
        let p = (elapsed_sec / DURATION).clamp(0.0, 1.0);
        let ease = 1.0 - (1.0 - p) * (1.0 - p);
        let scale = 0.85 + 0.15 * ease;
        let alpha = ease;
        (scale, alpha)
    }
}

/// Advance hover transition factor toward `target` (0.0 or 1.0) over 80ms.
#[inline]
pub fn advance_hover_factor(current: f32, target: f32, dt: f32) -> f32 {
    const HOVER_DURATION: f32 = 0.080;
    let step = dt / HOVER_DURATION;
    if target > current {
        (current + step).min(target)
    } else {
        (current - step).max(target)
    }
}

/// Computes hover growth in pixels based on the hover factor (0.0..=1.0).
/// Hovered wedge grows by up to HOVER_GROW_PX (6.0px).
#[inline]
pub fn hover_growth(factor: f32) -> f32 {
    HOVER_GROW_PX * factor.clamp(0.0, 1.0)
}

/// 3D perspective flip scale and opacity transition (180ms duration).
/// Returns (scale_x, alpha):
/// - scale_x follows a cosine half-period curve from 1.0 down to near 0.0 at midpoint (90ms)
///   and back up to 1.0 at 180ms.
/// - alpha fades slightly near edge-on transition and recovers.
#[inline]
pub fn face_flip_3d_scale(elapsed_sec: f32) -> (f32, f32) {
    const DURATION: f32 = 0.180;
    if elapsed_sec >= DURATION {
        (1.0, 1.0)
    } else {
        let p = (elapsed_sec / DURATION).clamp(0.0, 1.0);
        let raw_cos = (p * std::f32::consts::PI).cos().abs();
        let scale_x = raw_cos.max(0.04);
        let alpha = 0.4 + 0.6 * raw_cos;
        (scale_x, alpha)
    }
}

/// Face switch transition (180ms smooth rotation and crossfade).
/// Returns (rotation_radians, alpha) where rotation goes -PI/9 -> 0.0 and alpha goes 0.0 -> 1.0.
#[inline]
pub fn face_switch_factors(elapsed_sec: f32) -> (f32, f32) {
    const DURATION: f32 = 0.180;
    if elapsed_sec >= DURATION {
        (0.0, 1.0)
    } else {
        let p = (elapsed_sec / DURATION).clamp(0.0, 1.0);
        let t = p * p * (3.0 - 2.0 * p);
        let rot = (1.0 - t) * (-std::f32::consts::PI / 9.0);
        let alpha = t;
        (rot, alpha)
    }
}

/// Clamps donut diameter to the supported range [280.0, 480.0].
pub fn clamp_donut_diameter(d: f32) -> f32 {
    d.clamp(MIN_DONUT_DIAMETER, MAX_DONUT_DIAMETER)
}

/// Center angle in radians for wedge `idx` out of `count` wedges.
/// Petal 0 is centered at North (-PI/2) and wedges proceed clockwise.
pub fn mid_angle(idx: usize, count: usize) -> f32 {
    let n = count.clamp(1, MAX_INNER_RING_WEDGES) as f32;
    -PI / 2.0 + (idx as f32) * (2.0 * PI / n)
}

/// Angular span of each wedge for `count` wedges.
pub fn wedge_span(count: usize) -> f32 {
    let n = count.clamp(1, MAX_INNER_RING_WEDGES) as f32;
    2.0 * PI / n
}

/// Hit-test whether a radius `r` falls inside the central hub.
pub fn hub_hit(r: f32, hub_r: f32) -> bool {
    r <= hub_r
}

/// Hit-test whether a point `pos` falls inside the central hub centered at `center`.
pub fn hub_hit_point(pos: Pos2, center: Pos2, hub_r: f32) -> bool {
    (pos - center).length() <= hub_r
}

/// Hit-test which wedge (0..count-1) the polar point (angle, r) falls into.
///
/// Features:
/// - Supports 1..=8 wedges (clamped to max 8).
/// - Honors `inner_r` and `outer_r` boundaries (returns None if r < inner_r or r > outer_r).
/// - Honors `gap_px` linear separation between wedges (returns None if the point lies in the gap).
pub fn wedge_at(
    angle: f32,
    r: f32,
    count: usize,
    inner_r: f32,
    outer_r: f32,
    gap_px: f32,
) -> Option<usize> {
    wedge_at_with_hover(angle, r, count, inner_r, outer_r, None, 0.0, gap_px)
}

/// Hit-test with hover-grow support (hovered wedge extends outer radius by `hover_grow_px`).
#[allow(clippy::too_many_arguments)]
pub fn wedge_at_with_hover(
    angle: f32,
    r: f32,
    count: usize,
    inner_r: f32,
    outer_r: f32,
    hovered: Option<usize>,
    hover_grow_px: f32,
    gap_px: f32,
) -> Option<usize> {
    if count == 0 || r < inner_r {
        return None;
    }
    let n = count.min(MAX_INNER_RING_WEDGES);
    let span = 2.0 * PI / (n as f32);

    let offset = angle + (PI / 2.0) + (span / 2.0);
    let rel_angle = offset.rem_euclid(2.0 * PI);
    let idx = ((rel_angle / span).floor() as usize).min(n - 1);

    let max_r = if hovered == Some(idx) {
        outer_r + hover_grow_px
    } else {
        outer_r
    };
    if r > max_r {
        return None;
    }

    if gap_px > 0.0 && n > 1 {
        let half_gap_angle = (gap_px * 0.5) / r.max(1.0);
        let offset_in_wedge = rel_angle - (idx as f32) * span;
        if offset_in_wedge < half_gap_angle || (span - offset_in_wedge) < half_gap_angle {
            return None;
        }
    }

    Some(idx)
}

/// Hit-test which wedge a 2D screen position `pos` falls into relative to `center`.
pub fn wedge_at_point(
    pos: Pos2,
    center: Pos2,
    count: usize,
    inner_r: f32,
    outer_r: f32,
    gap_px: f32,
) -> Option<usize> {
    let delta = pos - center;
    let r = delta.length();
    let angle = delta.y.atan2(delta.x);
    wedge_at(angle, r, count, inner_r, outer_r, gap_px)
}

/// Computes inner and outer radii for an outer-arc submenu attached to an inner ring.
pub fn outer_submenu_radii(inner_ring_outer_r: f32) -> (f32, f32) {
    let sub_inner = inner_ring_outer_r + 13.0;
    let sub_outer = sub_inner + 46.0;
    (sub_inner, sub_outer)
}

/// Hit-test which sub-wedge of an expanded outer-arc submenu the point falls into.
#[allow(clippy::too_many_arguments)]
pub fn sub_wedge_at(
    angle: f32,
    r: f32,
    parent_idx: usize,
    parent_count: usize,
    sub_count: usize,
    sub_inner_r: f32,
    sub_outer_r: f32,
    gap_px: f32,
) -> Option<usize> {
    if sub_count == 0 || r < sub_inner_r - 2.0 || r > sub_outer_r + 8.0 {
        return None;
    }
    let parent_mid = mid_angle(parent_idx, parent_count);
    let sector_span = wedge_span(parent_count);

    let sub_span =
        (sector_span * 1.5 / sub_count as f32).max(44.0 / ((sub_inner_r + sub_outer_r) / 2.0));
    let total_fan = sub_span * sub_count as f32;

    let diff = (angle - parent_mid + PI).rem_euclid(2.0 * PI) - PI;
    if diff.abs() > (total_fan / 2.0) + 0.04 {
        return None;
    }

    let offset = diff + (total_fan / 2.0);
    let idx = ((offset / sub_span).floor() as usize).min(sub_count - 1);

    if gap_px > 0.0 && sub_count > 1 {
        let half_gap_angle = (gap_px * 0.5) / r.max(1.0);
        let offset_in_sub = offset - (idx as f32) * sub_span;
        if offset_in_sub < half_gap_angle || (sub_span - offset_in_sub) < half_gap_angle {
            return None;
        }
    }

    Some(idx)
}

/// Horizontal bounding box for label and icon placement centered strictly inside the wedge.
pub fn wedge_label_bounds(
    center: Pos2,
    mid_angle: f32,
    inner_r: f32,
    outer_r: f32,
    max_text_size: Vec2,
) -> Rect {
    let r_mid = (inner_r + outer_r) / 2.0;
    let mid_pos = Pos2::new(
        center.x + r_mid * mid_angle.cos(),
        center.y + r_mid * mid_angle.sin(),
    );
    let radial_thickness = outer_r - inner_r;
    let w = max_text_size.x.min(radial_thickness * 1.6).max(20.0);
    let h = max_text_size.y.min(radial_thickness * 0.8).max(12.0);
    Rect::from_center_size(mid_pos, Vec2::new(w, h))
}

/// Converts a press-drag-release marking-menu delta vector into a selected wedge index.
/// Returns None if the drag distance is below the `min_threshold` (default 24.0px).
pub fn gesture_direction_to_wedge(delta: Vec2, count: usize, min_threshold: f32) -> Option<usize> {
    if delta.length() < min_threshold || count == 0 {
        return None;
    }
    let n = count.min(MAX_INNER_RING_WEDGES);
    let angle = delta.y.atan2(delta.x);
    let span = 2.0 * PI / (n as f32);
    let rel_angle = (angle + (PI / 2.0) + (span / 2.0)).rem_euclid(2.0 * PI);
    let idx = ((rel_angle / span).floor() as usize).min(n - 1);
    Some(idx)
}

/// Action resulting from keyboard input on the radial donut.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DonutKeyAction {
    SelectWedge(usize),
    HoverWedge(usize),
    Confirm,
    Close,
    CloseSubmenu,
    ToggleFace,
    None,
}

/// Pure handler for keyboard navigation on the donut:
/// - Keys 1..8 select wedges directly.
/// - Arrow keys cycle hover wedge forward / backward.
/// - Enter confirms the hovered wedge.
/// - Esc closes submenus or dismisses the donut.
/// - Tab toggles face between Primary Formats and Studio Tools.
pub fn handle_donut_key(
    key: eframe::egui::Key,
    current_wedge: usize,
    wedge_count: usize,
    in_sub_menu: bool,
) -> DonutKeyAction {
    use eframe::egui::Key;
    if wedge_count == 0 {
        return DonutKeyAction::None;
    }
    match key {
        Key::Num1 => DonutKeyAction::SelectWedge(0),
        Key::Num2 if wedge_count > 1 => DonutKeyAction::SelectWedge(1),
        Key::Num3 if wedge_count > 2 => DonutKeyAction::SelectWedge(2),
        Key::Num4 if wedge_count > 3 => DonutKeyAction::SelectWedge(3),
        Key::Num5 if wedge_count > 4 => DonutKeyAction::SelectWedge(4),
        Key::Num6 if wedge_count > 5 => DonutKeyAction::SelectWedge(5),
        Key::Num7 if wedge_count > 6 => DonutKeyAction::SelectWedge(6),
        Key::Num8 if wedge_count > 7 => DonutKeyAction::SelectWedge(7),
        Key::ArrowRight | Key::ArrowDown => {
            let next = (current_wedge + 1) % wedge_count;
            DonutKeyAction::HoverWedge(next)
        }
        Key::ArrowLeft | Key::ArrowUp => {
            let prev = (current_wedge + wedge_count - 1) % wedge_count;
            DonutKeyAction::HoverWedge(prev)
        }
        Key::Enter => DonutKeyAction::Confirm,
        Key::Escape => {
            if in_sub_menu {
                DonutKeyAction::CloseSubmenu
            } else {
                DonutKeyAction::Close
            }
        }
        Key::Tab => DonutKeyAction::ToggleFace,
        _ => DonutKeyAction::None,
    }
}

/// DPI scaling helper: computes pixel-snapped dimensions for a given pixels_per_point.
pub fn scale_for_dpi(val: f32, ppp: f32) -> f32 {
    let p = ppp.max(0.1);
    (val * p).round() / p
}

pub struct RadialWidgetGeometry {
    pub center: Pos2,
    pub outer_radius: f32, // ~165.0 px (diameter ~330.0)
    pub inner_radius: f32, // ~68.0 px
    pub hub_radius: f32,   // ~62.0 px
    pub theme: WidgetTheme,
    pub file_count: usize,
    pub category: crate::core::FileCategory,
    pub anim_scale: f32,
    pub anim_alpha: f32,
    pub anim_rotation: f32,
    pub anim_scale_x: f32,
}

impl Default for RadialWidgetGeometry {
    fn default() -> Self {
        Self {
            center: Pos2::new(180.0, 180.0),
            outer_radius: 165.0,
            inner_radius: 68.0,
            hub_radius: 62.0,
            theme: WidgetTheme::Light,
            file_count: 0,
            category: crate::core::FileCategory::Image,
            anim_scale: 1.0,
            anim_alpha: 1.0,
            anim_rotation: 0.0,
            anim_scale_x: 1.0,
        }
    }
}

impl RadialWidgetGeometry {
    pub fn new(center: Pos2, size: f32) -> Self {
        let r = size / 2.0;
        Self {
            center,
            outer_radius: r * 0.88,
            inner_radius: r * 0.36,
            hub_radius: r * 0.32,
            theme: WidgetTheme::Light,
            file_count: 0,
            category: crate::core::FileCategory::Image,
            anim_scale: 1.0,
            anim_alpha: 1.0,
            anim_rotation: 0.0,
            anim_scale_x: 1.0,
        }
    }

    /// Hit-test which petal index (0..7) the point lies within.
    pub fn hit_test_petal(&self, point: Pos2) -> Option<usize> {
        wedge_at_point(
            point,
            self.center,
            8,
            self.inner_radius,
            self.outer_radius + HOVER_GROW_PX,
            0.0,
        )
    }

    /// Hit-test if cursor is inside the central hub.
    pub fn hit_test_hub(&self, point: Pos2) -> bool {
        hub_hit_point(point, self.center, self.hub_radius)
    }

    /// Get midpoint coordinates for a petal (useful for popovers or tooltips).
    pub fn petal_center(&self, index: usize) -> Pos2 {
        let angle = mid_angle(index, 8);
        let r_mid = (self.inner_radius + self.outer_radius) / 2.0;
        Pos2::new(
            self.center.x + r_mid * angle.cos(),
            self.center.y + r_mid * angle.sin(),
        )
    }

    /// Calculate inner and outer radii for the group fan-out sub-ring.
    pub fn sub_radii(&self) -> (f32, f32) {
        outer_submenu_radii(self.outer_radius)
    }

    /// Hit-test which sub-petal of an expanded group (0..sub_count - 1) the point lies within.
    pub fn hit_test_sub_petal(
        &self,
        point: Pos2,
        group_idx: usize,
        sub_count: usize,
    ) -> Option<usize> {
        let delta = point - self.center;
        let dist = delta.length();
        let angle = delta.y.atan2(delta.x);
        let (sub_inner, sub_outer) = self.sub_radii();
        sub_wedge_at(
            angle, dist, group_idx, 8, sub_count, sub_inner, sub_outer, 0.0,
        )
    }

    /// Render the complete radial widget according to the visual specification of `Image reference.png`.
    pub fn render(
        &self,
        painter: &Painter,
        sectors: &[&str],
        state: WidgetState,
        drag_hover_target: Option<usize>,
    ) {
        self.render_with_hint(painter, sectors, state, drag_hover_target, None);
    }

    pub fn render_with_hint(
        &self,
        painter: &Painter,
        sectors: &[&str],
        state: WidgetState,
        drag_hover_target: Option<usize>,
        hub_hint: Option<&str>,
    ) {
        self.render_with_full_details(
            painter,
            sectors,
            state,
            drag_hover_target,
            hub_hint,
            0.0,
            None,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_with_full_details(
        &self,
        painter: &Painter,
        sectors: &[&str],
        state: WidgetState,
        drag_hover_target: Option<usize>,
        hub_hint: Option<&str>,
        hover_pop: f32,
        expanded_group: Option<(usize, &[&str], f32, Option<usize>)>,
    ) {
        let sector_span = (2.0 * PI) / 8.0;
        let r_mid = (self.inner_radius + self.outer_radius) * 0.5;
        // 2px linear gap between petals
        let gap = (DEFAULT_WEDGE_GAP_PX / r_mid).clamp(0.01, 0.04);

        // Determine which petal is active/hovered
        let active_petal = match state {
            WidgetState::Hovered(idx) => Some(idx),
            WidgetState::Converting { .. } => drag_hover_target.or(Some(0)),
            _ => drag_hover_target,
        };

        let is_dark = self.theme == WidgetTheme::Dark;

        // 0. Soft ambient background shadow for the entire fruit cluster
        let shadow_col1 = if is_dark {
            Color32::from_black_alpha(60)
        } else {
            Color32::from_rgba_premultiplied(15, 25, 40, 35)
        };
        let shadow_col2 = if is_dark {
            Color32::from_black_alpha(30)
        } else {
            Color32::from_rgba_premultiplied(15, 25, 40, 18)
        };
        painter.circle_filled(
            self.center + Vec2::new(0.0, 5.0),
            self.outer_radius + 4.0,
            shadow_col1,
        );
        painter.circle_filled(
            self.center + Vec2::new(0.0, 9.0),
            self.outer_radius + 8.0,
            shadow_col2,
        );

        // 1. Render each of the 8 organic rounded capsule petals
        for i in 0..8 {
            let label = sectors.get(i).copied().unwrap_or("FMT");
            let is_hovered = active_petal == Some(i);

            // Angle of sector: 0 is North (-PI/2) + animation rotation
            let center_angle = -PI / 2.0 + (i as f32) * sector_span + self.anim_rotation;
            let start_angle = center_angle - (sector_span / 2.0) + gap * 0.5;
            let end_angle = center_angle + (sector_span / 2.0) - gap * 0.5;

            self.render_single_petal(
                painter,
                start_angle,
                end_angle,
                center_angle,
                label,
                is_hovered,
                if is_hovered { hover_pop } else { 0.0 },
            );
        }

        // 2. Render Sub-Ring Fan-Out if a group slice is expanded
        if let Some((grp_idx, sub_items, expansion, sub_hover)) = expanded_group {
            self.render_sub_ring(painter, grp_idx, sub_items, expansion, sub_hover);
        }

        // 3. Render Central Hub matching Image reference.png States 1, 4, 5, 6
        self.render_hub(painter, state, hub_hint);
    }

    /// Render outer sub-ring fan-out for group slices (Filee DonutMenu.Groups parity)
    pub fn render_sub_ring(
        &self,
        painter: &Painter,
        group_idx: usize,
        sub_items: &[&str],
        expansion: f32,
        hovered_sub: Option<usize>,
    ) {
        if sub_items.is_empty() || expansion <= 0.01 {
            return;
        }
        let is_dark = self.theme == WidgetTheme::Dark;
        let (sub_inner, sub_outer) = self.sub_radii();
        let sector_span = (2.0 * PI) / 8.0;
        let parent_mid = -PI / 2.0 + (group_idx as f32) * sector_span;

        let sub_span = (sector_span * 1.5 / sub_items.len() as f32)
            .max(44.0 / ((sub_inner + sub_outer) / 2.0));
        let gap = 0.04_f32;

        for (sub_idx, &label) in sub_items.iter().enumerate() {
            let is_sub_hovered = hovered_sub == Some(sub_idx);
            let sub_mid =
                parent_mid + (sub_idx as f32 - (sub_items.len() - 1) as f32 / 2.0) * sub_span;
            let start_a = sub_mid - (sub_span / 2.0) + gap;
            let end_a = sub_mid + (sub_span / 2.0) - gap;

            // Animate expansion radius
            let cur_inner = self.outer_radius + (sub_inner - self.outer_radius) * expansion;
            let cur_outer = cur_inner + (sub_outer - sub_inner) * expansion;

            let (fill, stroke, text_col) = if is_dark {
                if is_sub_hovered {
                    (
                        crate::core::glass_fill(
                            Color32::from_rgb(38, 98, 40),
                            crate::core::GlassEffect::Acrylic,
                            crate::core::Surface::Donut,
                            true,
                        ),
                        Stroke::new(1.8_f32, Color32::from_rgb(95, 215, 80)),
                        Color32::WHITE,
                    )
                } else {
                    (
                        crate::core::glass_fill(
                            Color32::from_rgb(28, 33, 44),
                            crate::core::GlassEffect::Acrylic,
                            crate::core::Surface::Donut,
                            true,
                        ),
                        Stroke::new(
                            1.0_f32,
                            Color32::from_rgba_premultiplied(
                                70,
                                80,
                                100,
                                (180.0 * expansion) as u8,
                            ),
                        ),
                        Color32::from_rgb(230, 234, 245),
                    )
                }
            } else if is_sub_hovered {
                (
                    crate::core::glass_fill(
                        Color32::from_rgb(226, 247, 214),
                        crate::core::GlassEffect::Acrylic,
                        crate::core::Surface::Donut,
                        false,
                    ),
                    Stroke::new(1.6_f32, Color32::from_rgb(120, 190, 32)),
                    Color32::from_rgb(32, 44, 26),
                )
            } else {
                (
                    crate::core::glass_fill(
                        Color32::from_rgb(245, 247, 250),
                        crate::core::GlassEffect::Acrylic,
                        crate::core::Surface::Donut,
                        false,
                    ),
                    Stroke::new(
                        1.0_f32,
                        Color32::from_rgba_premultiplied(215, 220, 230, (200.0 * expansion) as u8),
                    ),
                    Color32::from_rgb(40, 48, 60),
                )
            };

            let pts =
                self.generate_petal_capsule_points(cur_inner, cur_outer, start_a, end_a, 9.0, 6.5);

            // Drop shadow
            let shadow_pts = offset_points(&pts, Vec2::new(0.0, 2.5));
            painter.add(PathShape::convex_polygon(
                shadow_pts,
                Color32::from_black_alpha((35.0 * expansion) as u8),
                Stroke::NONE,
            ));

            if is_sub_hovered {
                let glow_pts = offset_points(&pts, Vec2::new(0.0, 1.0));
                painter.add(PathShape::convex_polygon(
                    glow_pts,
                    Color32::TRANSPARENT,
                    Stroke::new(3.0_f32, Color32::from_rgba_premultiplied(120, 210, 50, 70)),
                ));
            }

            painter.add(PathShape::convex_polygon(pts, fill, stroke));

            // Content position
            let r_mid = (cur_inner + cur_outer) / 2.0;
            let content_pos = Pos2::new(
                self.center.x + r_mid * sub_mid.cos(),
                self.center.y + r_mid * sub_mid.sin(),
            );
            painter.text(
                content_pos,
                egui::Align2::CENTER_CENTER,
                label,
                FontId::proportional(11.0),
                text_col,
            );
        }
    }

    /// Render an individual petal with pill-rounded corners, soft drop shadow, icon, and label.
    #[allow(clippy::too_many_arguments)]
    fn render_single_petal(
        &self,
        painter: &Painter,
        start_angle: f32,
        end_angle: f32,
        center_angle: f32,
        label: &str,
        is_hovered: bool,
        hover_pop: f32,
    ) {
        let is_dark = self.theme == WidgetTheme::Dark;

        // Colors matching Image reference.png
        let (fill_color, border_stroke, text_color, icon_color) = if is_dark {
            if is_hovered {
                // State 6 active petal: vibrant emerald green (#266228) with #52B846 border
                let fill = crate::core::glass_fill(
                    Color32::from_rgb(38, 98, 40),
                    crate::core::GlassEffect::Acrylic,
                    crate::core::Surface::Donut,
                    true,
                );
                (
                    fill,
                    Stroke::new(1.8_f32, Color32::from_rgb(95, 215, 80)),
                    Color32::WHITE,
                    Color32::from_rgb(115, 235, 95),
                )
            } else {
                // State 6 idle petal: deep charcoal/slate matte (#222733)
                let fill = crate::core::glass_fill(
                    Color32::from_rgb(34, 39, 51),
                    crate::core::GlassEffect::Acrylic,
                    crate::core::Surface::Donut,
                    true,
                );
                (
                    fill,
                    Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(62, 70, 88, 180)),
                    Color32::from_rgb(240, 242, 248),
                    Color32::from_rgb(190, 198, 212),
                )
            }
        } else if is_hovered {
            // State 2 & 4 hovered petal: soft illuminated lime-green (#E2F7D6)
            let fill = crate::core::glass_fill(
                Color32::from_rgb(226, 247, 214),
                crate::core::GlassEffect::Acrylic,
                crate::core::Surface::Donut,
                false,
            );
            (
                fill,
                Stroke::new(1.6_f32, Color32::from_rgb(120, 190, 32)), // Pantone 376 C
                Color32::from_rgb(32, 44, 26),
                Color32::from_rgb(45, 105, 30),
            )
        } else {
            // State 1 idle petal: clean off-white / pearl frosted (#F8F9FA)
            let fill = crate::core::glass_fill(
                Color32::from_rgb(248, 249, 250),
                crate::core::GlassEffect::Acrylic,
                crate::core::Surface::Donut,
                false,
            );
            (
                fill,
                Stroke::new(
                    1.0_f32,
                    Color32::from_rgba_premultiplied(222, 226, 234, 220),
                ),
                Color32::from_rgb(45, 52, 64),
                Color32::from_rgb(75, 84, 100),
            )
        };

        let alpha_mul = self.anim_alpha.clamp(0.0, 1.0);
        let scale_alpha = |c: Color32| -> Color32 {
            Color32::from_rgba_premultiplied(
                ((c.r() as f32) * alpha_mul) as u8,
                ((c.g() as f32) * alpha_mul) as u8,
                ((c.b() as f32) * alpha_mul) as u8,
                ((c.a() as f32) * alpha_mul) as u8,
            )
        };
        let fill_color = scale_alpha(fill_color);
        let border_stroke = Stroke::new(border_stroke.width, scale_alpha(border_stroke.color));
        let text_color = scale_alpha(text_color);
        let icon_color = scale_alpha(icon_color);

        // Construct 100% organic rounded capsule polygon points
        let outer_corner_r = 13.0_f32;
        let inner_corner_r = 8.5_f32;
        let cur_outer = if is_hovered {
            self.outer_radius + hover_pop.min(HOVER_GROW_PX)
        } else {
            self.outer_radius
        };
        let base_pts = self.generate_petal_capsule_points(
            self.inner_radius,
            cur_outer,
            start_angle,
            end_angle,
            outer_corner_r,
            inner_corner_r,
        );

        // Apply dynamic radial pop-out translation along spoke angle if hovered
        let pop_vec = if is_hovered && hover_pop > 0.0 {
            Vec2::new(
                center_angle.cos() * hover_pop,
                center_angle.sin() * hover_pop,
            )
        } else {
            Vec2::ZERO
        };
        let petal_pts = offset_points(&base_pts, pop_vec);
        let petal_pts = if (self.anim_scale_x - 1.0).abs() > 0.001 {
            petal_pts
                .into_iter()
                .map(|p| {
                    Pos2::new(
                        self.center.x + (p.x - self.center.x) * self.anim_scale_x,
                        p.y,
                    )
                })
                .collect()
        } else {
            petal_pts
        };

        // 1. Soft Ambient Drop Shadows under the individual petal
        let shadow_pts_deep = offset_points(&petal_pts, Vec2::new(0.0, 3.5));
        let shadow_deep_col = if is_dark {
            Color32::from_black_alpha(55)
        } else {
            Color32::from_rgba_premultiplied(18, 25, 38, 30)
        };
        painter.add(PathShape::convex_polygon(
            shadow_pts_deep,
            shadow_deep_col,
            Stroke::NONE,
        ));

        let shadow_pts_soft = offset_points(&petal_pts, Vec2::new(0.0, 1.5));
        let shadow_soft_col = if is_dark {
            Color32::from_black_alpha(30)
        } else {
            Color32::from_rgba_premultiplied(18, 25, 38, 20)
        };
        painter.add(PathShape::convex_polygon(
            shadow_pts_soft,
            shadow_soft_col,
            Stroke::NONE,
        ));

        // 2. Hovered Luminous Glow Halo
        if is_hovered {
            let glow_col = if is_dark {
                Color32::from_rgba_premultiplied(110, 230, 85, 70)
            } else {
                Color32::from_rgba_premultiplied(120, 205, 35, 60)
            };
            let glow_pts = offset_points(&petal_pts, Vec2::new(0.0, 1.0));
            painter.add(PathShape::convex_polygon(
                glow_pts,
                Color32::TRANSPARENT,
                Stroke::new(3.5_f32, glow_col),
            ));
        }

        // 3. Draw Petal Body & Crisp Border
        painter.add(PathShape::convex_polygon(
            petal_pts,
            fill_color,
            border_stroke,
        ));

        // Compute petal content position (Icon in upper half, Label in lower half)
        let r_icon = self.inner_radius + (self.outer_radius - self.inner_radius) * 0.66;
        let r_text = self.inner_radius + (self.outer_radius - self.inner_radius) * 0.28;

        let icon_pos = Pos2::new(
            self.center.x + r_icon * center_angle.cos(),
            self.center.y + r_icon * center_angle.sin(),
        ) + pop_vec;

        let text_pos = Pos2::new(
            self.center.x + r_text * center_angle.cos(),
            self.center.y + r_text * center_angle.sin(),
        ) + pop_vec;

        let (icon_pos, text_pos) = if (self.anim_scale_x - 1.0).abs() > 0.001 {
            (
                Pos2::new(
                    self.center.x + (icon_pos.x - self.center.x) * self.anim_scale_x,
                    icon_pos.y,
                ),
                Pos2::new(
                    self.center.x + (text_pos.x - self.center.x) * self.anim_scale_x,
                    text_pos.y,
                ),
            )
        } else {
            (icon_pos, text_pos)
        };

        // 4. Draw Authentic Vector Glyph Icon matching Image reference.png
        self.render_petal_icon(painter, icon_pos, label, icon_color);

        // 5. Draw Format Label Text (e.g. "JPG", "WEBP", "HEIC", "PDF")
        let (display_label, font_size) = if label.len() > 6 {
            if label.contains(' ') {
                (label.replacen(' ', "\n", 1), 9.5)
            } else if label.len() > 8 {
                (label[..8].to_string(), 10.0)
            } else {
                (label.to_string(), 10.0)
            }
        } else {
            (label.to_string(), 12.0)
        };

        painter.text(
            text_pos,
            egui::Align2::CENTER_CENTER,
            display_label,
            FontId::proportional(font_size),
            text_color,
        );
    }

    /// Generates a smooth, closed, organic rounded capsule polygon for a single petal.
    /// Bridges the outer arc, left radial edge, inner arc, and right radial edge with smooth corner fillets.
    fn generate_petal_capsule_points(
        &self,
        inner_r: f32,
        outer_r: f32,
        start_angle: f32,
        end_angle: f32,
        corner_r_outer: f32,
        corner_r_inner: f32,
    ) -> Vec<Pos2> {
        generate_capsule_polygon(
            self.center,
            inner_r,
            outer_r,
            start_angle,
            end_angle,
            corner_r_outer,
            corner_r_inner,
        )
    }

    /// Renders vector glyph icons for each petal (Image, Document, Camera, Stack, Video, Audio)
    /// Exact representation of Image reference.png
    fn render_petal_icon(&self, painter: &Painter, pos: Pos2, ext: &str, color: Color32) {
        let stroke = Stroke::new(1.7_f32, color);
        let ext_u = ext.to_uppercase();

        match ext_u.as_str() {
            "JPG" | "JPEG" | "PNG" | "WEBP" | "BMP" | "GIF" => {
                // Picture / Photo frame with sun & mountain peak
                let frame_w = 17.0;
                let frame_h = 13.0;
                let frame_rect = Rect::from_center_size(pos, Vec2::new(frame_w, frame_h));
                painter.rect_stroke(frame_rect, 2.5, stroke);

                // Sun dot (top right)
                let sun_pos = Pos2::new(frame_rect.right() - 4.5, frame_rect.top() + 4.0);
                painter.circle_filled(sun_pos, 1.4, color);

                // Mountain peaks inside frame
                let p_left = Pos2::new(frame_rect.left() + 2.5, frame_rect.bottom() - 2.5);
                let p_peak1 = Pos2::new(frame_rect.left() + 6.5, frame_rect.top() + 5.5);
                let p_mid = Pos2::new(frame_rect.left() + 9.5, frame_rect.bottom() - 3.5);
                let p_peak2 = Pos2::new(frame_rect.left() + 11.5, frame_rect.top() + 6.5);
                let p_right = Pos2::new(frame_rect.right() - 2.5, frame_rect.bottom() - 2.5);

                painter.line_segment([p_left, p_peak1], stroke);
                painter.line_segment([p_peak1, p_mid], stroke);
                painter.line_segment([p_mid, p_peak2], stroke);
                painter.line_segment([p_peak2, p_right], stroke);
            }
            "HEIC" => {
                // Camera aperture & body with top flash
                let cam_w = 17.0;
                let cam_h = 12.0;
                let cam_rect =
                    Rect::from_center_size(pos + Vec2::new(0.0, 1.0), Vec2::new(cam_w, cam_h));
                painter.rect_stroke(cam_rect, 3.0, stroke);
                // Center camera lens
                painter.circle_stroke(pos + Vec2::new(0.0, 1.0), 3.4, stroke);
                // Top flash bar
                let flash_w = 6.0;
                let flash_rect = Rect::from_center_size(
                    Pos2::new(pos.x, cam_rect.top() - 1.2),
                    Vec2::new(flash_w, 2.2),
                );
                painter.rect_stroke(flash_rect, 1.0, stroke);
            }
            "TIFF" => {
                // 3 Stacked isometric sheet diamonds
                let top_diamond = vec![
                    pos + Vec2::new(0.0, -4.5),
                    pos + Vec2::new(7.0, -1.5),
                    pos + Vec2::new(0.0, 1.5),
                    pos + Vec2::new(-7.0, -1.5),
                ];
                painter.add(PathShape::closed_line(top_diamond, stroke));

                // Middle layer
                let mid_layer = vec![
                    pos + Vec2::new(-7.0, 2.0),
                    pos + Vec2::new(0.0, 5.0),
                    pos + Vec2::new(7.0, 2.0),
                ];
                painter.add(PathShape::line(mid_layer, stroke));

                // Bottom layer
                let bot_layer = vec![
                    pos + Vec2::new(-7.0, 5.5),
                    pos + Vec2::new(0.0, 8.5),
                    pos + Vec2::new(7.0, 5.5),
                ];
                painter.add(PathShape::line(bot_layer, stroke));
            }
            "AVIF" => {
                // Stylized geometric delta prism / split triangle
                let p_top = pos + Vec2::new(0.0, -7.0);
                let p_left = pos + Vec2::new(-7.0, 6.0);
                let p_right = pos + Vec2::new(7.0, 6.0);
                painter.line_segment([p_top, p_left], stroke);
                painter.line_segment([p_left, p_right], stroke);
                painter.line_segment([p_right, p_top], stroke);
                // Inner refraction line
                painter.line_segment([p_top, pos + Vec2::new(2.0, 6.0)], stroke);
            }
            "PDF" | "DOCX" | "TXT" | "MD" | "HTML" => {
                // Document page with clean 3 horizontal lines
                let page_w = 13.0;
                let page_h = 17.0;
                let page_rect = Rect::from_center_size(pos, Vec2::new(page_w, page_h));
                painter.rect_stroke(page_rect, 2.2, stroke);

                // 3 clean horizontal text lines inside page
                let line_stroke = Stroke::new(1.3_f32, color);
                painter.line_segment(
                    [
                        Pos2::new(page_rect.left() + 3.0, pos.y - 3.5),
                        Pos2::new(page_rect.right() - 3.0, pos.y - 3.5),
                    ],
                    line_stroke,
                );
                painter.line_segment(
                    [
                        Pos2::new(page_rect.left() + 3.0, pos.y),
                        Pos2::new(page_rect.right() - 3.0, pos.y),
                    ],
                    line_stroke,
                );
                painter.line_segment(
                    [
                        Pos2::new(page_rect.left() + 3.0, pos.y + 3.5),
                        Pos2::new(page_rect.right() - 3.0, pos.y + 3.5),
                    ],
                    line_stroke,
                );
            }
            "WAV" | "FLAC" | "MP3" | "M4A" | "OGG" | "AAC" | "EXTRACT AUDIO" | "AUDIO" => {
                // Audio waveform equalizer bars
                let x = pos.x;
                let y = pos.y;
                painter.line_segment(
                    [Pos2::new(x - 6.0, y - 2.5), Pos2::new(x - 6.0, y + 2.5)],
                    stroke,
                );
                painter.line_segment(
                    [Pos2::new(x - 3.0, y - 5.5), Pos2::new(x - 3.0, y + 5.5)],
                    stroke,
                );
                painter.line_segment([Pos2::new(x, y - 7.5), Pos2::new(x, y + 7.5)], stroke);
                painter.line_segment(
                    [Pos2::new(x + 3.0, y - 5.5), Pos2::new(x + 3.0, y + 5.5)],
                    stroke,
                );
                painter.line_segment(
                    [Pos2::new(x + 6.0, y - 2.5), Pos2::new(x + 6.0, y + 2.5)],
                    stroke,
                );
            }
            "MP4" | "MKV" | "MOV" | "WEBM" => {
                // Video film play triangle
                let p1 = pos + Vec2::new(-4.5, -6.0);
                let p2 = pos + Vec2::new(-4.5, 6.0);
                let p3 = pos + Vec2::new(6.0, 0.0);
                painter.add(PathShape::convex_polygon(
                    vec![p1, p2, p3],
                    color,
                    Stroke::NONE,
                ));
            }
            "CROP" => {
                // Pre-conversion crop box with corner boundary tick marks
                let box_w = 14.0;
                let box_h = 14.0;
                let rect = Rect::from_center_size(pos, Vec2::new(box_w, box_h));
                painter.rect_stroke(rect, 1.5, stroke);
                painter.line_segment(
                    [pos + Vec2::new(-9.0, -7.0), pos + Vec2::new(-7.0, -7.0)],
                    stroke,
                );
                painter.line_segment(
                    [pos + Vec2::new(7.0, -7.0), pos + Vec2::new(9.0, -7.0)],
                    stroke,
                );
                painter.line_segment(
                    [pos + Vec2::new(-9.0, 7.0), pos + Vec2::new(-7.0, 7.0)],
                    stroke,
                );
                painter.line_segment(
                    [pos + Vec2::new(7.0, 7.0), pos + Vec2::new(9.0, 7.0)],
                    stroke,
                );
            }
            "EXAM" => {
                // Graduation mortarboard cap & diploma envelope
                let p_top = pos + Vec2::new(0.0, -6.5);
                let p_right = pos + Vec2::new(7.5, -2.5);
                let p_bottom = pos + Vec2::new(0.0, 1.5);
                let p_left = pos + Vec2::new(-7.5, -2.5);
                painter.add(PathShape::closed_line(
                    vec![p_top, p_right, p_bottom, p_left],
                    stroke,
                ));
                let cap_base = vec![
                    pos + Vec2::new(-5.0, -0.5),
                    pos + Vec2::new(-5.0, 4.0),
                    pos + Vec2::new(0.0, 6.5),
                    pos + Vec2::new(5.0, 4.0),
                    pos + Vec2::new(5.0, -0.5),
                ];
                painter.add(PathShape::line(cap_base, stroke));
                painter.line_segment(
                    [p_right, pos + Vec2::new(8.0, 5.0)],
                    Stroke::new(1.2_f32, color),
                );
            }
            "META" => {
                // Information circular badge with 'i' glyph
                painter.circle_stroke(pos, 7.0, stroke);
                painter.circle_filled(pos - Vec2::new(0.0, 3.0), 1.2, color);
                painter.line_segment(
                    [pos - Vec2::new(0.0, 0.5), pos + Vec2::new(0.0, 4.0)],
                    stroke,
                );
            }
            "RENAME" => {
                // Pencil angled cursor
                let p_tip = pos + Vec2::new(-6.0, 6.0);
                let p_back = pos + Vec2::new(6.0, -6.0);
                painter.line_segment([p_tip, p_back], stroke);
                painter.line_segment([p_tip, p_tip + Vec2::new(3.0, 0.0)], stroke);
                painter.line_segment([p_tip, p_tip + Vec2::new(0.0, -3.0)], stroke);
            }
            "AI OCR" => {
                // 4-Point AI Sparkle Star
                let p_top = pos + Vec2::new(0.0, -7.5);
                let p_bottom = pos + Vec2::new(0.0, 7.5);
                let p_left = pos + Vec2::new(-7.5, 0.0);
                let p_right = pos + Vec2::new(7.5, 0.0);
                painter.line_segment([p_top, p_bottom], stroke);
                painter.line_segment([p_left, p_right], stroke);
                let d_top = pos + Vec2::new(0.0, -3.0);
                let d_right = pos + Vec2::new(3.0, 0.0);
                let d_bottom = pos + Vec2::new(0.0, 3.0);
                let d_left = pos + Vec2::new(-3.0, 0.0);
                painter.add(PathShape::closed_line(
                    vec![d_top, d_right, d_bottom, d_left],
                    stroke,
                ));
            }
            "ZIP" => {
                // Archive box with horizontal zipper teeth
                let box_w = 15.0;
                let box_h = 14.0;
                let box_rect = Rect::from_center_size(pos, Vec2::new(box_w, box_h));
                painter.rect_stroke(box_rect, 2.0, stroke);
                painter.line_segment(
                    [pos + Vec2::new(-2.0, -4.0), pos + Vec2::new(2.0, -4.0)],
                    stroke,
                );
                painter.line_segment(
                    [pos + Vec2::new(-2.0, 0.0), pos + Vec2::new(2.0, 0.0)],
                    stroke,
                );
                painter.line_segment(
                    [pos + Vec2::new(-2.0, 4.0), pos + Vec2::new(2.0, 4.0)],
                    stroke,
                );
            }
            "EXPLORE" => {
                // macOS Finder / Universal Explorer Dual Pane Window
                let win_w = 16.0;
                let win_h = 13.0;
                let win_rect = Rect::from_center_size(pos, Vec2::new(win_w, win_h));
                painter.rect_stroke(win_rect, 2.0, stroke);
                painter.line_segment(
                    [
                        Pos2::new(win_rect.left() + 5.0, win_rect.top()),
                        Pos2::new(win_rect.left() + 5.0, win_rect.bottom()),
                    ],
                    Stroke::new(1.2_f32, color),
                );
            }
            _ => {
                // Generic circular badge
                painter.circle_stroke(pos, 4.5, stroke);
            }
        }
    }

    /// Renders Central Hub matching Image reference.png States 1, 4, 5, 6.
    fn render_hub(&self, painter: &Painter, state: WidgetState, hub_hint: Option<&str>) {
        let is_dark = self.theme == WidgetTheme::Dark;

        match state {
            WidgetState::Converting { progress } => {
                // State 4: Clean pure white center circle with progress arc & "65% \n Converting..."
                // Drop shadow under white hub
                painter.circle_filled(
                    self.center + Vec2::new(0.0, 3.5),
                    self.hub_radius + 2.0,
                    Color32::from_rgba_premultiplied(15, 25, 40, 45),
                );

                // Immaculate White Disc
                painter.circle(
                    self.center,
                    self.hub_radius,
                    Color32::WHITE,
                    Stroke::new(1.0_f32, Color32::from_rgb(228, 231, 238)),
                );

                // Circular green progress arc
                let pct = progress.clamp(0.0, 1.0);
                let arc_r = self.hub_radius - 4.0;

                // Background track
                painter.circle_stroke(
                    self.center,
                    arc_r,
                    Stroke::new(3.5_f32, Color32::from_rgb(238, 241, 246)),
                );

                // Active progress arc (sweeps clockwise from top North -PI/2)
                let arc_steps = (50.0 * pct).max(2.0) as usize;
                let mut arc_pts = Vec::with_capacity(arc_steps + 1);
                let start_a = -PI / 2.0;
                let end_a = start_a + pct * 2.0 * PI;

                for s in 0..=arc_steps {
                    let t = s as f32 / arc_steps as f32;
                    let a = start_a + t * (end_a - start_a);
                    arc_pts.push(Pos2::new(
                        self.center.x + arc_r * a.cos(),
                        self.center.y + arc_r * a.sin(),
                    ));
                }

                painter.add(PathShape::line(
                    arc_pts,
                    Stroke::new(4.5_f32, Color32::from_rgb(60, 195, 45)), // Vibrant Pantone Lime Green
                ));

                // Large bold percentage text (e.g. "65%")
                painter.text(
                    self.center - Vec2::new(0.0, 6.0),
                    egui::Align2::CENTER_CENTER,
                    format!("{:.0}%", pct * 100.0),
                    FontId::proportional(22.0),
                    Color32::from_rgb(30, 36, 48),
                );

                // Subtext "Converting..."
                painter.text(
                    self.center + Vec2::new(0.0, 15.0),
                    egui::Align2::CENTER_CENTER,
                    "Converting...",
                    FontId::proportional(12.0),
                    Color32::from_rgb(115, 122, 136),
                );
            }
            WidgetState::Success { file_count } => {
                // State 5: Clean pure white center circle with green checkmark & "Converted \n N files"
                // Drop shadow under white hub
                painter.circle_filled(
                    self.center + Vec2::new(0.0, 3.5),
                    self.hub_radius + 2.0,
                    Color32::from_rgba_premultiplied(15, 25, 40, 45),
                );

                // Immaculate White Disc
                painter.circle(
                    self.center,
                    self.hub_radius,
                    Color32::WHITE,
                    Stroke::new(1.0_f32, Color32::from_rgb(225, 228, 236)),
                );

                // Circular green checkmark badge
                let badge_center = self.center - Vec2::new(0.0, 16.0);
                painter.circle_filled(badge_center, 13.0, Color32::from_rgb(52, 188, 48));

                // White checkmark glyph inside badge
                let p1 = badge_center + Vec2::new(-4.5, -0.5);
                let p2 = badge_center + Vec2::new(-1.0, 3.5);
                let p3 = badge_center + Vec2::new(5.0, -3.5);
                painter.line_segment([p1, p2], Stroke::new(2.4_f32, Color32::WHITE));
                painter.line_segment([p2, p3], Stroke::new(2.4_f32, Color32::WHITE));

                // "Converted" text
                painter.text(
                    self.center + Vec2::new(0.0, 5.0),
                    egui::Align2::CENTER_CENTER,
                    "Converted",
                    FontId::proportional(14.0),
                    Color32::from_rgb(30, 36, 48),
                );

                // File count subtext
                let file_str = if file_count == 1 {
                    "1 file".to_string()
                } else {
                    format!("{} files", file_count)
                };
                painter.text(
                    self.center + Vec2::new(0.0, 20.0),
                    egui::Align2::CENTER_CENTER,
                    file_str,
                    FontId::proportional(12.0),
                    Color32::from_rgb(115, 122, 136),
                );
            }
            _ => {
                // State 1, 2, 3, 6: Frosted disc with upward arrow "Drop files here"
                let (hub_fill, hub_stroke, arrow_color, text_color) = if is_dark {
                    (
                        Color32::from_rgba_premultiplied(22, 26, 36, 245), // Deep slate disc
                        Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(55, 64, 82, 180)),
                        Color32::WHITE,
                        Color32::from_rgb(220, 224, 235),
                    )
                } else {
                    (
                        Color32::from_rgba_premultiplied(142, 149, 165, 230), // Translucent frosted slate
                        Stroke::new(
                            1.0_f32,
                            Color32::from_rgba_premultiplied(255, 255, 255, 140),
                        ),
                        Color32::WHITE,
                        Color32::from_rgb(255, 255, 255),
                    )
                };

                // Drop shadow under hub
                painter.circle_filled(
                    self.center + Vec2::new(0.0, 3.0),
                    self.hub_radius + 2.0,
                    Color32::from_black_alpha(35),
                );

                painter.circle(self.center, self.hub_radius, hub_fill, hub_stroke);

                if self.file_count > 0 {
                    // Show domain icon + file count ("1 file" or "N files")
                    let icon_pos = self.center - Vec2::new(0.0, 10.0);
                    let cat_label = match self.category {
                        crate::core::FileCategory::Image => "JPG",
                        crate::core::FileCategory::Audio => "MP3",
                        crate::core::FileCategory::Video => "MP4",
                        crate::core::FileCategory::Document
                        | crate::core::FileCategory::Spreadsheet
                        | crate::core::FileCategory::Presentation => "PDF",
                        crate::core::FileCategory::Archive => "ZIP",
                        crate::core::FileCategory::Cad | crate::core::FileCategory::Models3D => {
                            "3D"
                        }
                        crate::core::FileCategory::Fonts => "FONT",
                        crate::core::FileCategory::Data
                        | crate::core::FileCategory::Email
                        | crate::core::FileCategory::Geo
                        | crate::core::FileCategory::Subtitle
                        | crate::core::FileCategory::Unknown => "FILE",
                    };
                    self.render_petal_icon(painter, icon_pos, cat_label, text_color);

                    let count_str = if self.file_count == 1 {
                        "1 file".to_string()
                    } else {
                        format!("{} files", self.file_count)
                    };
                    painter.text(
                        self.center + Vec2::new(0.0, 12.0),
                        egui::Align2::CENTER_CENTER,
                        count_str,
                        FontId::proportional(12.5),
                        text_color,
                    );
                } else {
                    // Draw crisp upward arrow matching Image reference.png
                    let arrow_center = self.center - Vec2::new(0.0, 10.0);
                    let shaft_top = arrow_center - Vec2::new(0.0, 8.5);
                    let shaft_bottom = arrow_center + Vec2::new(0.0, 7.5);
                    let arrow_stroke = Stroke::new(2.4_f32, arrow_color);

                    painter.line_segment([shaft_top, shaft_bottom], arrow_stroke);
                    painter.line_segment(
                        [shaft_top, arrow_center + Vec2::new(-5.0, -3.5)],
                        arrow_stroke,
                    );
                    painter.line_segment(
                        [shaft_top, arrow_center + Vec2::new(5.0, -3.5)],
                        arrow_stroke,
                    );

                    // Subtext "Drop files \n here"
                    painter.text(
                        self.center + Vec2::new(0.0, 10.0),
                        egui::Align2::CENTER_CENTER,
                        "Drop files",
                        FontId::proportional(13.0),
                        text_color,
                    );
                    painter.text(
                        self.center + Vec2::new(0.0, 23.5),
                        egui::Align2::CENTER_CENTER,
                        "here",
                        FontId::proportional(12.0),
                        text_color,
                    );
                }

                if let Some(hint) = hub_hint {
                    let pill_rect = Rect::from_center_size(
                        self.center + Vec2::new(0.0, 40.0),
                        Vec2::new(96.0, 15.0),
                    );
                    let pill_fill = if is_dark {
                        Color32::from_rgba_premultiplied(35, 42, 58, 210)
                    } else {
                        Color32::from_rgba_premultiplied(220, 226, 238, 220)
                    };
                    let pill_text_col = if is_dark {
                        Color32::from_rgb(180, 200, 225)
                    } else {
                        Color32::from_rgb(60, 72, 95)
                    };
                    painter.rect_filled(pill_rect, 4.0, pill_fill);
                    painter.text(
                        pill_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        hint,
                        FontId::proportional(9.5),
                        pill_text_col,
                    );
                }
            }
        }
    }
}

/// Computes a point on a quadratic Bezier curve: B(t) = (1-t)^2 P0 + 2(1-t)t P1 + t^2 P2
#[inline]
fn quadratic_bezier(p0: Pos2, p1: Pos2, p2: Pos2, t: f32) -> Pos2 {
    let u = 1.0 - t;
    Pos2::new(
        u * u * p0.x + 2.0 * u * t * p1.x + t * t * p2.x,
        u * u * p0.y + 2.0 * u * t * p1.y + t * t * p2.y,
    )
}

/// Offsets an entire slice of polygon points by a 2D delta (used for multi-stage drop shadows)
#[inline]
fn offset_points(pts: &[Pos2], delta: Vec2) -> Vec<Pos2> {
    pts.iter().map(|&p| p + delta).collect()
}

/// Generates a smooth, closed, organic rounded capsule polygon for a single petal.
/// Bridges the outer arc, left radial edge, inner arc, and right radial edge with smooth corner fillets.
pub fn generate_capsule_polygon(
    center: Pos2,
    inner_r: f32,
    outer_r: f32,
    start_angle: f32,
    end_angle: f32,
    corner_r_outer: f32,
    corner_r_inner: f32,
) -> Vec<Pos2> {
    let mut pts = Vec::with_capacity(36);

    let d_theta_outer = corner_r_outer / outer_r;
    let d_rad_outer = corner_r_outer;

    let d_theta_inner = corner_r_inner / inner_r;
    let d_rad_inner = corner_r_inner;

    // 1. Left Radial Edge (from inner to outer)
    let rad_left_in = Pos2::new(
        center.x + (inner_r + d_rad_inner) * start_angle.cos(),
        center.y + (inner_r + d_rad_inner) * start_angle.sin(),
    );
    let rad_left_out = Pos2::new(
        center.x + (outer_r - d_rad_outer) * start_angle.cos(),
        center.y + (outer_r - d_rad_outer) * start_angle.sin(),
    );
    pts.push(rad_left_in);
    pts.push(rad_left_out);

    // 2. Outer-Left Corner Fillet
    let corner_ol_v = Pos2::new(
        center.x + outer_r * start_angle.cos(),
        center.y + outer_r * start_angle.sin(),
    );
    let arc_ol_start = Pos2::new(
        center.x + outer_r * (start_angle + d_theta_outer).cos(),
        center.y + outer_r * (start_angle + d_theta_outer).sin(),
    );
    pts.push(quadratic_bezier(
        rad_left_out,
        corner_ol_v,
        arc_ol_start,
        0.45,
    ));
    pts.push(quadratic_bezier(
        rad_left_out,
        corner_ol_v,
        arc_ol_start,
        0.85,
    ));

    // 3. Outer Circular Arc
    let num_outer_arc = 6;
    for s in 0..=num_outer_arc {
        let t = s as f32 / num_outer_arc as f32;
        let a = (start_angle + d_theta_outer)
            + t * (end_angle - d_theta_outer - (start_angle + d_theta_outer));
        pts.push(Pos2::new(
            center.x + outer_r * a.cos(),
            center.y + outer_r * a.sin(),
        ));
    }

    // 4. Outer-Right Corner Fillet
    let arc_or_end = Pos2::new(
        center.x + outer_r * (end_angle - d_theta_outer).cos(),
        center.y + outer_r * (end_angle - d_theta_outer).sin(),
    );
    let corner_or_v = Pos2::new(
        center.x + outer_r * end_angle.cos(),
        center.y + outer_r * end_angle.sin(),
    );
    let rad_right_out = Pos2::new(
        center.x + (outer_r - d_rad_outer) * end_angle.cos(),
        center.y + (outer_r - d_rad_outer) * end_angle.sin(),
    );
    pts.push(quadratic_bezier(
        arc_or_end,
        corner_or_v,
        rad_right_out,
        0.45,
    ));
    pts.push(quadratic_bezier(
        arc_or_end,
        corner_or_v,
        rad_right_out,
        0.85,
    ));

    // 5. Right Radial Edge (from outer to inner)
    let rad_right_in = Pos2::new(
        center.x + (inner_r + d_rad_inner) * end_angle.cos(),
        center.y + (inner_r + d_rad_inner) * end_angle.sin(),
    );
    pts.push(rad_right_out);
    pts.push(rad_right_in);

    // 6. Inner-Right Corner Fillet
    let corner_ir_v = Pos2::new(
        center.x + inner_r * end_angle.cos(),
        center.y + inner_r * end_angle.sin(),
    );
    let arc_ir_start = Pos2::new(
        center.x + inner_r * (end_angle - d_theta_inner).cos(),
        center.y + inner_r * (end_angle - d_theta_inner).sin(),
    );
    pts.push(quadratic_bezier(
        rad_right_in,
        corner_ir_v,
        arc_ir_start,
        0.45,
    ));
    pts.push(quadratic_bezier(
        rad_right_in,
        corner_ir_v,
        arc_ir_start,
        0.85,
    ));

    // 7. Inner Circular Arc
    let num_inner_arc = 4;
    for s in 0..=num_inner_arc {
        let t = s as f32 / num_inner_arc as f32;
        let a = (end_angle - d_theta_inner)
            - t * ((end_angle - d_theta_inner) - (start_angle + d_theta_inner));
        pts.push(Pos2::new(
            center.x + inner_r * a.cos(),
            center.y + inner_r * a.sin(),
        ));
    }

    // 8. Inner-Left Corner Fillet
    let arc_il_end = Pos2::new(
        center.x + inner_r * (start_angle + d_theta_inner).cos(),
        center.y + inner_r * (start_angle + d_theta_inner).sin(),
    );
    let corner_il_v = Pos2::new(
        center.x + inner_r * start_angle.cos(),
        center.y + inner_r * start_angle.sin(),
    );
    pts.push(quadratic_bezier(arc_il_end, corner_il_v, rad_left_in, 0.45));
    pts.push(quadratic_bezier(arc_il_end, corner_il_v, rad_left_in, 0.85));

    pts
}
