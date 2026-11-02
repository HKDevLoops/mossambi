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

pub struct RadialWidgetGeometry {
    pub center: Pos2,
    pub outer_radius: f32, // ~165.0 px (diameter ~330.0)
    pub inner_radius: f32, // ~68.0 px
    pub hub_radius: f32,   // ~62.0 px
    pub theme: WidgetTheme,
}

impl Default for RadialWidgetGeometry {
    fn default() -> Self {
        Self {
            center: Pos2::new(180.0, 180.0),
            outer_radius: 165.0,
            inner_radius: 68.0,
            hub_radius: 62.0,
            theme: WidgetTheme::Light,
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
        }
    }

    /// Hit-test which petal index (0..7) the point lies within.
    pub fn hit_test_petal(&self, point: Pos2) -> Option<usize> {
        let delta = point - self.center;
        let dist = delta.length();

        if dist < self.inner_radius || dist > self.outer_radius + 6.0 {
            return None;
        }

        // Angle in radians: -PI..PI (0 is +X, -PI/2 is top/North)
        let angle = delta.y.atan2(delta.x);
        // Normalize angle so that Petal 0 is centered at the top North (-PI/2)
        // Petal 0: [-PI/2 - PI/8, -PI/2 + PI/8]
        let sector_span = (2.0 * PI) / 8.0;
        let offset = angle + (PI / 2.0) + (sector_span / 2.0);
        let mut normalized = offset % (2.0 * PI);
        if normalized < 0.0 {
            normalized += 2.0 * PI;
        }

        let idx = (normalized / sector_span).floor() as usize;
        if idx < 8 {
            Some(idx)
        } else {
            None
        }
    }

    /// Hit-test if cursor is inside the central hub.
    pub fn hit_test_hub(&self, point: Pos2) -> bool {
        (point - self.center).length() <= self.hub_radius
    }

    /// Get midpoint coordinates for a petal (useful for popovers or tooltips).
    pub fn petal_center(&self, index: usize) -> Pos2 {
        let sector_span = (2.0 * PI) / 8.0;
        let angle = -PI / 2.0 + (index as f32) * sector_span;
        let r_mid = (self.inner_radius + self.outer_radius) / 2.0;
        Pos2::new(
            self.center.x + r_mid * angle.cos(),
            self.center.y + r_mid * angle.sin(),
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
        let sector_span = (2.0 * PI) / 8.0;
        // Aesthetic radial gap between petals (~3.0 degrees)
        let gap = 0.052;

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

            // Angle of sector: 0 is North (-PI/2)
            let center_angle = -PI / 2.0 + (i as f32) * sector_span;
            let start_angle = center_angle - (sector_span / 2.0) + gap;
            let end_angle = center_angle + (sector_span / 2.0) - gap;

            self.render_single_petal(
                painter,
                start_angle,
                end_angle,
                center_angle,
                label,
                is_hovered,
            );
        }

        // 2. Render Central Hub matching Image reference.png States 1, 4, 5, 6
        self.render_hub(painter, state);
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
    ) {
        let is_dark = self.theme == WidgetTheme::Dark;

        // Colors matching Image reference.png
        let (fill_color, border_stroke, text_color, icon_color) = if is_dark {
            if is_hovered {
                // State 6 active petal: vibrant emerald green (#266228) with #52B846 border
                (
                    Color32::from_rgb(38, 98, 40),
                    Stroke::new(1.8_f32, Color32::from_rgb(95, 215, 80)),
                    Color32::WHITE,
                    Color32::from_rgb(115, 235, 95),
                )
            } else {
                // State 6 idle petal: deep charcoal/slate matte (#222733)
                (
                    Color32::from_rgba_premultiplied(34, 39, 51, 250),
                    Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(62, 70, 88, 180)),
                    Color32::from_rgb(240, 242, 248),
                    Color32::from_rgb(190, 198, 212),
                )
            }
        } else if is_hovered {
            // State 2 & 4 hovered petal: soft illuminated lime-green (#E2F7D6)
            (
                Color32::from_rgb(226, 247, 214),
                Stroke::new(1.6_f32, Color32::from_rgb(120, 190, 32)), // Pantone 376 C
                Color32::from_rgb(32, 44, 26),
                Color32::from_rgb(45, 105, 30),
            )
        } else {
            // State 1 idle petal: clean off-white / pearl frosted (#F8F9FA)
            (
                Color32::from_rgba_premultiplied(248, 249, 250, 255),
                Stroke::new(
                    1.0_f32,
                    Color32::from_rgba_premultiplied(222, 226, 234, 220),
                ),
                Color32::from_rgb(45, 52, 64),
                Color32::from_rgb(75, 84, 100),
            )
        };

        // Construct 100% organic rounded capsule polygon points
        let outer_corner_r = 13.0_f32;
        let inner_corner_r = 8.5_f32;
        let petal_pts = self.generate_petal_capsule_points(
            self.inner_radius,
            self.outer_radius,
            start_angle,
            end_angle,
            outer_corner_r,
            inner_corner_r,
        );

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
        );

        let text_pos = Pos2::new(
            self.center.x + r_text * center_angle.cos(),
            self.center.y + r_text * center_angle.sin(),
        );

        // 4. Draw Authentic Vector Glyph Icon matching Image reference.png
        self.render_petal_icon(painter, icon_pos, label, icon_color);

        // 5. Draw Format Label Text (e.g. "JPG", "WEBP", "HEIC", "PDF")
        painter.text(
            text_pos,
            egui::Align2::CENTER_CENTER,
            label,
            FontId::proportional(11.5),
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
        let mut pts = Vec::with_capacity(36);

        let d_theta_outer = corner_r_outer / outer_r;
        let d_rad_outer = corner_r_outer;

        let d_theta_inner = corner_r_inner / inner_r;
        let d_rad_inner = corner_r_inner;

        // 1. Left Radial Edge (from inner to outer)
        let rad_left_in = Pos2::new(
            self.center.x + (inner_r + d_rad_inner) * start_angle.cos(),
            self.center.y + (inner_r + d_rad_inner) * start_angle.sin(),
        );
        let rad_left_out = Pos2::new(
            self.center.x + (outer_r - d_rad_outer) * start_angle.cos(),
            self.center.y + (outer_r - d_rad_outer) * start_angle.sin(),
        );
        pts.push(rad_left_in);
        pts.push(rad_left_out);

        // 2. Outer-Left Corner Fillet
        let corner_ol_v = Pos2::new(
            self.center.x + outer_r * start_angle.cos(),
            self.center.y + outer_r * start_angle.sin(),
        );
        let arc_ol_start = Pos2::new(
            self.center.x + outer_r * (start_angle + d_theta_outer).cos(),
            self.center.y + outer_r * (start_angle + d_theta_outer).sin(),
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
                self.center.x + outer_r * a.cos(),
                self.center.y + outer_r * a.sin(),
            ));
        }

        // 4. Outer-Right Corner Fillet
        let arc_or_end = Pos2::new(
            self.center.x + outer_r * (end_angle - d_theta_outer).cos(),
            self.center.y + outer_r * (end_angle - d_theta_outer).sin(),
        );
        let corner_or_v = Pos2::new(
            self.center.x + outer_r * end_angle.cos(),
            self.center.y + outer_r * end_angle.sin(),
        );
        let rad_right_out = Pos2::new(
            self.center.x + (outer_r - d_rad_outer) * end_angle.cos(),
            self.center.y + (outer_r - d_rad_outer) * end_angle.sin(),
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
            self.center.x + (inner_r + d_rad_inner) * end_angle.cos(),
            self.center.y + (inner_r + d_rad_inner) * end_angle.sin(),
        );
        pts.push(rad_right_out);
        pts.push(rad_right_in);

        // 6. Inner-Right Corner Fillet
        let corner_ir_v = Pos2::new(
            self.center.x + inner_r * end_angle.cos(),
            self.center.y + inner_r * end_angle.sin(),
        );
        let arc_ir_start = Pos2::new(
            self.center.x + inner_r * (end_angle - d_theta_inner).cos(),
            self.center.y + inner_r * (end_angle - d_theta_inner).sin(),
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
                self.center.x + inner_r * a.cos(),
                self.center.y + inner_r * a.sin(),
            ));
        }

        // 8. Inner-Left Corner Fillet
        let arc_il_end = Pos2::new(
            self.center.x + inner_r * (start_angle + d_theta_inner).cos(),
            self.center.y + inner_r * (start_angle + d_theta_inner).sin(),
        );
        let corner_il_v = Pos2::new(
            self.center.x + inner_r * start_angle.cos(),
            self.center.y + inner_r * start_angle.sin(),
        );
        pts.push(quadratic_bezier(arc_il_end, corner_il_v, rad_left_in, 0.45));
        pts.push(quadratic_bezier(arc_il_end, corner_il_v, rad_left_in, 0.85));

        pts
    }

    /// Renders vector glyph icons for each petal (Image, Document, Camera, Stack, Video, Audio)
    /// Exact representation of Image reference.png
    fn render_petal_icon(&self, painter: &Painter, pos: Pos2, ext: &str, color: Color32) {
        let stroke = Stroke::new(1.7_f32, color);
        let ext_u = ext.to_uppercase();

        match ext_u.as_str() {
            "JPG" | "JPEG" | "PNG" | "WEBP" | "BMP" => {
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
            "WAV" | "FLAC" | "MP3" | "M4A" | "OGG" | "AAC" => {
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
            _ => {
                // Generic circular badge
                painter.circle_stroke(pos, 4.5, stroke);
            }
        }
    }

    /// Renders Central Hub matching Image reference.png States 1, 4, 5, 6.
    fn render_hub(&self, painter: &Painter, state: WidgetState) {
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
                    FontId::proportional(10.5),
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
                    FontId::proportional(12.5),
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
                    FontId::proportional(10.0),
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

                // Draw crisp upward arrow `↑` matching Image reference.png
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
                    FontId::proportional(11.5),
                    text_color,
                );
                painter.text(
                    self.center + Vec2::new(0.0, 23.5),
                    egui::Align2::CENTER_CENTER,
                    "here",
                    FontId::proportional(11.5),
                    text_color,
                );
            }
        }
    }
}

/// Computes a point on a quadratic Bézier curve: B(t) = (1-t)^2 P0 + 2(1-t)t P1 + t^2 P2
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
