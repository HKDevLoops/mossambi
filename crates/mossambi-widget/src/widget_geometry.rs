//! Pixel-Accurate 8-Petal Radial Fruit Widget Geometry
//! Perfectly reproduces the 6 visual states from `Image reference.png`:
//! 1. Idle State: Clean light off-white petals, frosted dark center hub with upward arrow "Drop files here".
//! 2. Hover State: Soft glowing illuminated lime-green petal with green icon and dark label.
//! 3. Drag File: Highlighted drop target petal with dragged file detection.
//! 4. Converting State: Clean white center hub with large "65%" and "Converting...", surrounded by smooth green progress ring.
//! 5. Success State: Clean white center hub with green circular checkmark badge and "Converted \n N files".
//! 6. Dark Mode: Deep charcoal/slate matte petals, bright white typography, and vibrant green highlighted active petal.

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
    pub inner_radius: f32, // ~74.0 px
    pub hub_radius: f32,   // ~60.0 px
    pub theme: WidgetTheme,
}

impl Default for RadialWidgetGeometry {
    fn default() -> Self {
        Self {
            center: Pos2::new(180.0, 180.0),
            outer_radius: 165.0,
            inner_radius: 61.0,
            hub_radius: 60.0,
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
            inner_radius: r * 0.33,
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
        let gap = 0.038; // ~2.2 degrees radial gap between petals

        // Determine which petal is highlighted
        let active_petal = match state {
            WidgetState::Hovered(idx) => Some(idx),
            WidgetState::Converting { .. } => drag_hover_target,
            _ => drag_hover_target,
        };

        let is_dark = self.theme == WidgetTheme::Dark;

        // 0. Soft ambient drop shadow around the circular fruit
        let shadow_col1 = if is_dark {
            Color32::from_black_alpha(80)
        } else {
            Color32::from_rgba_premultiplied(20, 30, 50, 45)
        };
        let shadow_col2 = if is_dark {
            Color32::from_black_alpha(45)
        } else {
            Color32::from_rgba_premultiplied(20, 30, 50, 22)
        };
        painter.circle_filled(self.center + Vec2::new(0.0, 5.0), self.outer_radius + 3.0, shadow_col1);
        painter.circle_filled(self.center + Vec2::new(0.0, 8.0), self.outer_radius + 7.0, shadow_col2);

        // 1. Render each of the 8 petals
        for i in 0..8 {
            let label = sectors.get(i).copied().unwrap_or("FMT");
            let is_hovered = active_petal == Some(i);

            if is_hovered {
                let glow_col = if is_dark {
                    Color32::from_rgba_premultiplied(120, 240, 90, 75)
                } else {
                    Color32::from_rgba_premultiplied(120, 190, 32, 60)
                };
                painter.circle_filled(self.petal_center(i) + Vec2::new(0.0, 2.0), 30.0, glow_col);
            }

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

    /// Render an individual petal with curved borders, icon, and label.
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
        // Color palettes from Image reference.png
        let (fill_color, border_stroke, text_color, icon_color) = if is_dark {
            if is_hovered {
                // State 6 active petal: vibrant emerald green (#2E6B2C)
                (
                    Color32::from_rgb(46, 107, 44),
                    Stroke::new(1.5_f32, Color32::from_rgb(122, 224, 110)), // #7AE06E glow
                    Color32::WHITE,
                    Color32::from_rgb(122, 224, 110),
                )
            } else {
                // State 6 idle petal: deep charcoal/slate matte (#2B303C)
                (
                    Color32::from_rgba_premultiplied(43, 48, 60, 245),
                    Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(65, 72, 88, 160)),
                    Color32::from_rgb(240, 242, 246),
                    Color32::from_rgb(195, 202, 215),
                )
            }
        } else if is_hovered {
            // State 2 & 4 hovered petal: soft glowing lime-green (#E7F7DE)
            (
                Color32::from_rgb(231, 247, 222),
                Stroke::new(1.5_f32, Color32::from_rgb(120, 190, 32)), // Pantone 376 C
                Color32::from_rgb(32, 40, 28),
                Color32::from_rgb(60, 164, 44),
            )
        } else {
            // State 1 idle petal: clean off-white / light cream frosted (#F7F7F8)
            (
                Color32::from_rgba_premultiplied(247, 247, 248, 250),
                Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(225, 227, 232, 200)),
                Color32::from_rgb(50, 56, 68),
                Color32::from_rgb(85, 92, 105),
            )
        };

        // Construct rounded annular sector polygon
        let num_arc_steps = 10;
        let mut points = Vec::with_capacity(num_arc_steps * 2 + 2);

        // Outer arc (from start_angle to end_angle)
        for s in 0..=num_arc_steps {
            let t = s as f32 / num_arc_steps as f32;
            let a = start_angle + t * (end_angle - start_angle);
            points.push(Pos2::new(
                self.center.x + self.outer_radius * a.cos(),
                self.center.y + self.outer_radius * a.sin(),
            ));
        }

        // Inner arc (from end_angle back to start_angle)
        for s in 0..=num_arc_steps {
            let t = s as f32 / num_arc_steps as f32;
            let a = end_angle - t * (end_angle - start_angle);
            points.push(Pos2::new(
                self.center.x + self.inner_radius * a.cos(),
                self.center.y + self.inner_radius * a.sin(),
            ));
        }

        // Draw petal background & border
        painter.add(PathShape::convex_polygon(points, fill_color, border_stroke));

        // Compute petal content position (Icon in upper half, Label in lower half)
        let r_icon = self.inner_radius + (self.outer_radius - self.inner_radius) * 0.65;
        let r_text = self.inner_radius + (self.outer_radius - self.inner_radius) * 0.28;

        let icon_pos = Pos2::new(
            self.center.x + r_icon * center_angle.cos(),
            self.center.y + r_icon * center_angle.sin(),
        );

        let text_pos = Pos2::new(
            self.center.x + r_text * center_angle.cos(),
            self.center.y + r_text * center_angle.sin(),
        );

        // Draw format icon glyph matching Image reference.png
        self.render_petal_icon(painter, icon_pos, label, icon_color);

        // Draw format label text (e.g. "JPG", "PDF", "WEBP")
        painter.text(
            text_pos,
            egui::Align2::CENTER_CENTER,
            label,
            FontId::proportional(11.0),
            text_color,
        );
    }

    /// Renders vector glyph icons for each petal (Image, Document, Camera, Stack, Video, Audio)
    fn render_petal_icon(&self, painter: &Painter, pos: Pos2, ext: &str, color: Color32) {
        let stroke = Stroke::new(1.6_f32, color);
        let ext_u = ext.to_uppercase();

        match ext_u.as_str() {
            "JPG" | "JPEG" | "PNG" | "WEBP" | "BMP" => {
                // Picture / Photo frame with mountain peak & sun
                let frame_rect = Rect::from_center_size(pos, Vec2::new(14.0, 12.0));
                painter.rect_stroke(frame_rect, 2.0, stroke);
                // Mini mountain inside
                let p1 = Pos2::new(frame_rect.left() + 2.0, frame_rect.bottom() - 2.0);
                let p2 = Pos2::new(frame_rect.left() + 6.0, frame_rect.top() + 4.0);
                let p3 = Pos2::new(frame_rect.left() + 9.0, frame_rect.bottom() - 2.0);
                painter.line_segment([p1, p2], stroke);
                painter.line_segment([p2, p3], stroke);
            }
            "HEIC" => {
                // Camera aperture / body
                let cam_rect = Rect::from_center_size(pos + Vec2::new(0.0, 1.0), Vec2::new(14.0, 11.0));
                painter.rect_stroke(cam_rect, 2.0, stroke);
                painter.circle_stroke(pos + Vec2::new(0.0, 1.0), 3.0, stroke);
                // Top flash nub
                painter.line_segment(
                    [
                        Pos2::new(pos.x - 3.0, cam_rect.top() - 1.5),
                        Pos2::new(pos.x + 3.0, cam_rect.top() - 1.5),
                    ],
                    stroke,
                );
            }
            "TIFF" => {
                // Multi-layer stack icon
                let w = 12.0;
                let h = 4.0;
                painter.rect_stroke(Rect::from_center_size(pos - Vec2::new(0.0, 4.0), Vec2::new(w, h)), 1.0, stroke);
                painter.rect_stroke(Rect::from_center_size(pos, Vec2::new(w, h)), 1.0, stroke);
                painter.rect_stroke(Rect::from_center_size(pos + Vec2::new(0.0, 4.0), Vec2::new(w, h)), 1.0, stroke);
            }
            "AVIF" => {
                // Vector diamond / delta triangle
                let p_top = pos - Vec2::new(0.0, 6.0);
                let p_left = pos + Vec2::new(-6.0, 5.0);
                let p_right = pos + Vec2::new(6.0, 5.0);
                painter.line_segment([p_top, p_left], stroke);
                painter.line_segment([p_left, p_right], stroke);
                painter.line_segment([p_right, p_top], stroke);
            }
            "PDF" | "DOCX" | "TXT" | "MD" | "HTML" => {
                // Document page with top-right fold
                let page_rect = Rect::from_center_size(pos, Vec2::new(11.0, 14.0));
                painter.rect_stroke(page_rect, 1.5, stroke);
                // Line markers inside page
                painter.line_segment(
                    [
                        Pos2::new(page_rect.left() + 2.5, pos.y - 2.0),
                        Pos2::new(page_rect.right() - 2.5, pos.y - 2.0),
                    ],
                    Stroke::new(1.2_f32, color),
                );
                painter.line_segment(
                    [
                        Pos2::new(page_rect.left() + 2.5, pos.y + 2.0),
                        Pos2::new(page_rect.right() - 2.5, pos.y + 2.0),
                    ],
                    Stroke::new(1.2_f32, color),
                );
            }
            "WAV" | "FLAC" | "MP3" | "M4A" | "OGG" | "AAC" => {
                // Audio waveform bars
                let x = pos.x;
                let y = pos.y;
                painter.line_segment([Pos2::new(x - 5.0, y - 2.0), Pos2::new(x - 5.0, y + 2.0)], stroke);
                painter.line_segment([Pos2::new(x - 2.5, y - 5.0), Pos2::new(x - 2.5, y + 5.0)], stroke);
                painter.line_segment([Pos2::new(x, y - 7.0), Pos2::new(x, y + 7.0)], stroke);
                painter.line_segment([Pos2::new(x + 2.5, y - 5.0), Pos2::new(x + 2.5, y + 5.0)], stroke);
                painter.line_segment([Pos2::new(x + 5.0, y - 2.0), Pos2::new(x + 5.0, y + 2.0)], stroke);
            }
            "MP4" | "MKV" | "MOV" | "WEBM" => {
                // Video film clapper / play triangle
                let p1 = pos + Vec2::new(-4.0, -5.0);
                let p2 = pos + Vec2::new(-4.0, 5.0);
                let p3 = pos + Vec2::new(5.0, 0.0);
                painter.add(PathShape::convex_polygon(vec![p1, p2, p3], color, Stroke::NONE));
            }
            _ => {
                // Generic circle/sparkle
                painter.circle_stroke(pos, 4.0, stroke);
            }
        }
    }

    /// Renders Central Hub matching Image reference.png States 1, 4, 5, 6.
    fn render_hub(&self, painter: &Painter, state: WidgetState) {
        let is_dark = self.theme == WidgetTheme::Dark;
        match state {
            WidgetState::Converting { progress } => {
                // State 4: Clean white center circle with progress arc & "65% \n Converting..."
                painter.circle(
                    self.center,
                    self.hub_radius,
                    Color32::WHITE,
                    Stroke::new(1.0_f32, Color32::from_rgb(220, 222, 228)),
                );

                // Circular green progress arc
                let pct = progress.clamp(0.0, 1.0);
                let arc_steps = (40.0 * pct).max(2.0) as usize;
                let mut arc_pts = Vec::with_capacity(arc_steps + 1);
                let arc_r = self.hub_radius - 4.0;
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
                    Stroke::new(4.5_f32, Color32::from_rgb(72, 192, 56)), // Bright vibrant green
                ));

                // Percentage text
                painter.text(
                    self.center - Vec2::new(0.0, 6.0),
                    egui::Align2::CENTER_CENTER,
                    format!("{:.0}%", pct * 100.0),
                    FontId::proportional(20.0),
                    Color32::from_rgb(35, 40, 50),
                );

                // Subtext "Converting..."
                painter.text(
                    self.center + Vec2::new(0.0, 14.0),
                    egui::Align2::CENTER_CENTER,
                    "Converting...",
                    FontId::proportional(10.0),
                    Color32::from_rgb(120, 126, 138),
                );
            }
            WidgetState::Success { file_count } => {
                // State 5: Clean white center circle with green checkmark & "Converted \n N files"
                painter.circle(
                    self.center,
                    self.hub_radius,
                    Color32::WHITE,
                    Stroke::new(1.0_f32, Color32::from_rgb(215, 220, 228)),
                );

                // Circular green checkmark badge
                let badge_center = self.center - Vec2::new(0.0, 16.0);
                painter.circle_filled(badge_center, 12.0, Color32::from_rgb(56, 180, 52));
                // White checkmark glyph
                let p1 = badge_center + Vec2::new(-4.5, 0.0);
                let p2 = badge_center + Vec2::new(-1.0, 3.5);
                let p3 = badge_center + Vec2::new(5.0, -3.5);
                painter.line_segment([p1, p2], Stroke::new(2.2_f32, Color32::WHITE));
                painter.line_segment([p2, p3], Stroke::new(2.2_f32, Color32::WHITE));

                // "Converted" text
                painter.text(
                    self.center + Vec2::new(0.0, 4.0),
                    egui::Align2::CENTER_CENTER,
                    "Converted",
                    FontId::proportional(12.0),
                    Color32::from_rgb(35, 40, 50),
                );

                // File count subtext
                let file_str = if file_count == 1 {
                    "1 file".to_string()
                } else {
                    format!("{} files", file_count)
                };
                painter.text(
                    self.center + Vec2::new(0.0, 18.0),
                    egui::Align2::CENTER_CENTER,
                    file_str,
                    FontId::proportional(10.0),
                    Color32::from_rgb(120, 126, 138),
                );
            }
            _ => {
                // State 1, 2, 3, 6: Frosted disc with upward arrow "Drop files here"
                let (hub_fill, hub_stroke, arrow_color, text_color) = if is_dark {
                    (
                        Color32::from_rgba_premultiplied(28, 32, 40, 240), // Deep slate disc
                        Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(60, 68, 85, 180)),
                        Color32::WHITE,
                        Color32::from_rgb(220, 224, 232),
                    )
                } else {
                    (
                        Color32::from_rgba_premultiplied(135, 140, 152, 190), // Translucent frosted slate
                        Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(255, 255, 255, 120)),
                        Color32::WHITE,
                        Color32::from_rgb(255, 255, 255),
                    )
                };

                painter.circle(self.center, self.hub_radius, hub_fill, hub_stroke);

                // Draw crisp upward arrow `↑` matching Image reference.png
                let arrow_center = self.center - Vec2::new(0.0, 10.0);
                let shaft_top = arrow_center - Vec2::new(0.0, 8.0);
                let shaft_bottom = arrow_center + Vec2::new(0.0, 8.0);
                let arrow_stroke = Stroke::new(2.2_f32, arrow_color);

                painter.line_segment([shaft_top, shaft_bottom], arrow_stroke);
                painter.line_segment([shaft_top, arrow_center + Vec2::new(-5.0, -3.0)], arrow_stroke);
                painter.line_segment([shaft_top, arrow_center + Vec2::new(5.0, -3.0)], arrow_stroke);

                // Subtext "Drop files \n here"
                painter.text(
                    self.center + Vec2::new(0.0, 10.0),
                    egui::Align2::CENTER_CENTER,
                    "Drop files",
                    FontId::proportional(11.0),
                    text_color,
                );
                painter.text(
                    self.center + Vec2::new(0.0, 23.0),
                    egui::Align2::CENTER_CENTER,
                    "here",
                    FontId::proportional(11.0),
                    text_color,
                );
            }
        }
    }
}
