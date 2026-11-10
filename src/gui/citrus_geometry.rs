//! Authentic Mossambi Citrus Fruit Slice Radial Geometry
//! Renders the botanical anatomy of a sweet lime cross-section:
//! 1. Flavedo Outer Peel Rind (Pantone 376 C `#78BE20`)
//! 2. Albedo Inner Pith Membrane (Pantone 11-0601 TPG `#F4F5F0`)
//! 3. 8 Radial Carpel Pulp Wedges (Pantone 123 C `#FFC72C` / Pantone 137 C `#FFA300`)
//! 4. Golden ALL Sector (Pantone 1235 C `#FFB81C`)
//! 5. Central Columella Hub (Pantone 102 C `#FCE300`)
//!
//! Supports 4 certified botanical varieties (Sweet Lime, Tangerine, Ruby Grapefruit, Persian Lime) and custom hex tuning.

use eframe::egui::{Color32, FontId, Painter, Pos2, Stroke};
use serde::{Deserialize, Serialize};
use std::f32::consts::PI;

pub const FLAVEDO_GREEN: Color32 = Color32::from_rgb(120, 190, 32); // Pantone 376 C
pub const ALBEDO_WHITE: Color32 = Color32::from_rgb(244, 245, 240); // Pantone 11-0601 TPG
pub const CARPEL_YELLOW: Color32 = Color32::from_rgb(255, 199, 44); // Pantone 123 C
pub const CARPEL_HOVER: Color32 = Color32::from_rgb(255, 163, 0); // Pantone 137 C
pub const ALL_GOLD: Color32 = Color32::from_rgb(255, 184, 28); // Pantone 1235 C
pub const COLUMELLA_CORE: Color32 = Color32::from_rgb(252, 227, 0); // Pantone 102 C
pub const TEXT_DARK: Color32 = Color32::from_rgb(33, 43, 22);
pub const SUCCESS_GREEN: Color32 = Color32::from_rgb(46, 204, 113); // Emerald Celebration

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CitrusVariety {
    #[default]
    SweetLime, // Sweet Lime / Mossambi (Pantone 376 C peel / 123 C pulp)
    MandarinTangerine, // Mandarin / Tangerine (Pantone 15-1247 peel / Amber pulp)
    RubyGrapefruit,    // Ruby Red Grapefruit (Pantone 18-1662 peel / Crimson pulp)
    PersianLime,       // Persian / Kaffir Lime (Pantone 371 C peel / 389 C pulp)
    CustomHex,         // Custom user-defined colors
}

impl CitrusVariety {
    pub fn all() -> &'static [CitrusVariety] {
        &[
            CitrusVariety::SweetLime,
            CitrusVariety::MandarinTangerine,
            CitrusVariety::RubyGrapefruit,
            CitrusVariety::PersianLime,
            CitrusVariety::CustomHex,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            CitrusVariety::SweetLime => "Sweet Lime / Mossambi (Pantone 376 C / 123 C)",
            CitrusVariety::MandarinTangerine => "Mandarin Tangerine (Pantone 15-1247)",
            CitrusVariety::RubyGrapefruit => "Ruby Red Grapefruit (Pantone 18-1662)",
            CitrusVariety::PersianLime => "Persian Lime (Pantone 371 C / 389 C)",
            CitrusVariety::CustomHex => "Custom Hex Swatch",
        }
    }

    pub fn colors(&self, custom: Option<&CitrusColorPalette>) -> CitrusColorPalette {
        match self {
            CitrusVariety::SweetLime => CitrusColorPalette {
                flavedo_peel: Color32::from_rgb(120, 190, 32), // Pantone 376 C
                albedo_pith: Color32::from_rgb(244, 245, 240), // Pantone 11-0601 TPG
                carpel_pulp: Color32::from_rgb(255, 199, 44),  // Pantone 123 C
                carpel_hover: Color32::from_rgb(255, 163, 0),  // Pantone 137 C
                all_sector: Color32::from_rgb(255, 184, 28),   // Pantone 1235 C
                columella_core: Color32::from_rgb(252, 227, 0), // Pantone 102 C
                text_dark: Color32::from_rgb(33, 43, 22),
            },
            CitrusVariety::MandarinTangerine => CitrusColorPalette {
                flavedo_peel: Color32::from_rgb(226, 109, 92), // Pantone 15-1247
                albedo_pith: Color32::from_rgb(253, 246, 235),
                carpel_pulp: Color32::from_rgb(243, 156, 18), // Amber Pulp
                carpel_hover: Color32::from_rgb(230, 126, 34),
                all_sector: Color32::from_rgb(255, 184, 28),
                columella_core: Color32::from_rgb(241, 196, 15),
                text_dark: Color32::from_rgb(45, 25, 15),
            },
            CitrusVariety::RubyGrapefruit => CitrusColorPalette {
                flavedo_peel: Color32::from_rgb(200, 16, 46), // Pantone 18-1662
                albedo_pith: Color32::from_rgb(250, 240, 242),
                carpel_pulp: Color32::from_rgb(217, 56, 58), // Crimson Pulp
                carpel_hover: Color32::from_rgb(180, 20, 35),
                all_sector: Color32::from_rgb(255, 184, 28),
                columella_core: Color32::from_rgb(235, 100, 110),
                text_dark: Color32::from_rgb(50, 10, 15),
            },
            CitrusVariety::PersianLime => CitrusColorPalette {
                flavedo_peel: Color32::from_rgb(92, 118, 51), // Pantone 371 C
                albedo_pith: Color32::from_rgb(240, 248, 235),
                carpel_pulp: Color32::from_rgb(208, 223, 0), // Pantone 389 C
                carpel_hover: Color32::from_rgb(175, 195, 0),
                all_sector: Color32::from_rgb(255, 184, 28),
                columella_core: Color32::from_rgb(225, 235, 30),
                text_dark: Color32::from_rgb(25, 40, 15),
            },
            CitrusVariety::CustomHex => custom
                .copied()
                .unwrap_or_else(|| CitrusVariety::SweetLime.colors(None)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CitrusColorPalette {
    pub flavedo_peel: Color32,
    pub albedo_pith: Color32,
    pub carpel_pulp: Color32,
    pub carpel_hover: Color32,
    pub all_sector: Color32,
    pub columella_core: Color32,
    pub text_dark: Color32,
}

pub struct CarpelSlice {
    pub index: usize,
    pub format_name: String,
    pub is_all: bool,
    pub start_angle: f32,
    pub end_angle: f32,
    pub is_hovered: bool,
}

pub struct CitrusGeometry {
    pub center: Pos2,
    pub outer_radius: f32, // 150.0
    pub pith_outer: f32,   // 142.0
    pub pith_inner: f32,   // 135.0
    pub carpel_outer: f32, // 133.0
    pub carpel_inner: f32, // 46.0
    pub hub_radius: f32,   // 42.0
    pub variety: CitrusVariety,
    pub custom_palette: Option<CitrusColorPalette>,
}

impl Default for CitrusGeometry {
    fn default() -> Self {
        Self {
            center: Pos2::new(160.0, 160.0),
            outer_radius: 148.0,
            pith_outer: 140.0,
            pith_inner: 134.0,
            carpel_outer: 132.0,
            carpel_inner: 46.0,
            hub_radius: 42.0,
            variety: CitrusVariety::SweetLime,
            custom_palette: None,
        }
    }
}

impl CitrusGeometry {
    pub fn new(center: Pos2, size: f32) -> Self {
        let scale = size / 320.0;
        Self {
            center,
            outer_radius: 148.0 * scale,
            pith_outer: 140.0 * scale,
            pith_inner: 134.0 * scale,
            carpel_outer: 132.0 * scale,
            carpel_inner: 46.0 * scale,
            hub_radius: 42.0 * scale,
            variety: CitrusVariety::SweetLime,
            custom_palette: None,
        }
    }

    /// Determines which carpel sector (0..7) a point falls into, or None if in hub/outside.
    pub fn hit_test_carpel(&self, point: Pos2) -> Option<usize> {
        let dx = point.x - self.center.x;
        let dy = point.y - self.center.y;
        let dist = (dx * dx + dy * dy).sqrt();

        if dist < self.carpel_inner || dist > self.outer_radius {
            return None;
        }

        // Angle relative to -PI/2 (top 12 o'clock)
        let mut angle = dy.atan2(dx);
        // Normalize angle to [0, 2*PI) starting from -PI/2
        angle += PI / 2.0;
        if angle < 0.0 {
            angle += 2.0 * PI;
        }

        let sector_angle = (2.0 * PI) / 8.0;
        let idx = (angle / sector_angle).floor() as usize;
        if idx < 8 {
            Some(idx)
        } else {
            None
        }
    }

    /// Determines if a point is within the central Columella hub.
    pub fn hit_test_hub(&self, point: Pos2) -> bool {
        let dx = point.x - self.center.x;
        let dy = point.y - self.center.y;
        (dx * dx + dy * dy).sqrt() <= self.hub_radius
    }

    /// Renders the complete botanical Mossambi fruit cross-section.
    pub fn render(
        &self,
        painter: &Painter,
        sectors: &[&str],
        hovered_carpel: Option<usize>,
        hub_status: &str,
        is_converting: bool,
        progress_angle: f32,
    ) {
        let c = self.center;
        let pal = self.variety.colors(self.custom_palette.as_ref());

        // 1. Shadow glow behind the fruit
        painter.circle_filled(c, self.outer_radius + 4.0, Color32::from_black_alpha(25));
        painter.circle_filled(c, self.outer_radius + 1.0, Color32::from_black_alpha(35));

        // 2. Flavedo (Outer Peel Rind) - Botanical Variety Peel
        painter.circle_filled(c, self.outer_radius, pal.flavedo_peel);
        painter.circle_stroke(
            c,
            self.outer_radius,
            Stroke::new(1.5_f32, Color32::from_rgba_premultiplied(0, 0, 0, 40)),
        );

        // Subtle peel oil glands (botanical pores)
        let num_glands = 32;
        for i in 0..num_glands {
            let theta = (i as f32) * (2.0 * PI / num_glands as f32);
            let r = self.outer_radius - 4.5;
            let gx = c.x + r * theta.cos();
            let gy = c.y + r * theta.sin();
            painter.circle_filled(
                Pos2::new(gx, gy),
                1.0,
                Color32::from_rgba_unmultiplied(255, 255, 255, 60),
            );
        }

        // 3. Albedo (Inner Pith Ring) - Soft White
        painter.circle_filled(c, self.pith_outer, pal.albedo_pith);

        // 4. 8 Carpel Pulp Wedges
        let num_sectors = 8;
        let sector_span = (2.0 * PI) / (num_sectors as f32);
        let gap_rad = 0.055; // Natural separation membrane (~3 degrees)

        for i in 0..num_sectors {
            let base_angle = -PI / 2.0 + (i as f32) * sector_span;
            let start_theta = base_angle + gap_rad;
            let end_theta = base_angle + sector_span - gap_rad;
            let is_hover = hovered_carpel == Some(i);
            let is_all = i == 7 || sectors.get(i).map(|s| *s == "ALL").unwrap_or(false);

            // Construct Carpel polygon mesh
            let steps = 14;
            let mut points: Vec<Pos2> = Vec::with_capacity(steps * 2 + 2);

            // Outer arc (from start_theta to end_theta)
            for s in 0..=steps {
                let t = start_theta + (end_theta - start_theta) * (s as f32 / steps as f32);
                let r = if is_hover {
                    self.carpel_outer + 3.0
                } else {
                    self.carpel_outer
                };
                points.push(Pos2::new(c.x + r * t.cos(), c.y + r * t.sin()));
            }

            // Inner arc (from end_theta down to start_theta)
            for s in 0..=steps {
                let t = end_theta - (end_theta - start_theta) * (s as f32 / steps as f32);
                let r = if is_hover {
                    self.carpel_inner - 2.0
                } else {
                    self.carpel_inner
                };
                points.push(Pos2::new(c.x + r * t.cos(), c.y + r * t.sin()));
            }

            // Color selection based on state and ALL sector
            let fill_color = if is_all {
                if is_hover {
                    Color32::from_rgb(255, 210, 70) // Bright Gold Hover
                } else {
                    pal.all_sector // Pantone 1235 C
                }
            } else if is_hover {
                pal.carpel_hover // Glowing Hover
            } else {
                pal.carpel_pulp // Natural Citrus Pulp
            };

            let stroke_color = if is_hover {
                Color32::from_rgb(255, 255, 255)
            } else if is_all {
                Color32::from_rgb(230, 160, 10)
            } else {
                Color32::from_rgba_premultiplied(0, 0, 0, 30)
            };

            painter.add(eframe::epaint::Shape::convex_polygon(
                points,
                fill_color,
                Stroke::new(if is_hover { 2.0_f32 } else { 1.0_f32 }, stroke_color),
            ));

            // Render format label inside carpel
            let mid_theta = (start_theta + end_theta) / 2.0;
            let r_mid = (self.carpel_outer + self.carpel_inner) / 2.0;
            let label_r = r_mid + 4.0;
            let text_pos = Pos2::new(
                c.x + label_r * mid_theta.cos(),
                c.y + label_r * mid_theta.sin(),
            );
            let fmt_name = sectors
                .get(i)
                .copied()
                .unwrap_or(if is_all { "ALL" } else { "FMT" });

            let font = if is_all {
                FontId::proportional(16.0)
            } else if is_hover {
                FontId::proportional(16.5)
            } else {
                FontId::proportional(15.0)
            };

            let text_color = if is_all {
                Color32::from_rgb(60, 40, 0)
            } else if is_hover {
                Color32::from_rgb(30, 15, 0)
            } else {
                pal.text_dark
            };

            let display_text = if is_all { "ALL" } else { fmt_name };

            painter.text(
                text_pos,
                eframe::egui::Align2::CENTER_CENTER,
                display_text,
                font,
                text_color,
            );
        }

        // 5. Radial Septa (Albedo separation walls connecting to core)
        for i in 0..num_sectors {
            let angle = -PI / 2.0 + (i as f32) * sector_span;
            let p1 = Pos2::new(
                c.x + self.carpel_inner * angle.cos(),
                c.y + self.carpel_inner * angle.sin(),
            );
            let p2 = Pos2::new(
                c.x + (self.outer_radius - 2.0) * angle.cos(),
                c.y + (self.outer_radius - 2.0) * angle.sin(),
            );
            painter.line_segment([p1, p2], Stroke::new(2.5_f32, pal.albedo_pith));
        }

        // 6. Central Columella Hub (Center Core)
        painter.circle_filled(c, self.hub_radius, pal.albedo_pith);

        if hub_status == "DONE" {
            // Celebration Morph: Emerald Green glow & checkmark
            painter.circle_filled(c, self.hub_radius - 1.0, SUCCESS_GREEN);
            painter.circle_stroke(
                c,
                self.hub_radius - 1.0,
                Stroke::new(2.0_f32, Color32::WHITE),
            );
            painter.text(
                Pos2::new(c.x, c.y - 6.0),
                eframe::egui::Align2::CENTER_CENTER,
                "DONE",
                FontId::proportional(23.0),
                Color32::WHITE,
            );
        } else {
            painter.circle_filled(c, self.hub_radius - 3.0, pal.columella_core);
            painter.circle_stroke(
                c,
                self.hub_radius - 3.0,
                Stroke::new(1.5_f32, Color32::from_rgba_premultiplied(0, 0, 0, 40)),
            );

            // Central pith star radiating rays
            for i in 0..8 {
                let a = (i as f32) * (2.0 * PI / 8.0);
                let ray_tip = Pos2::new(c.x + 14.0 * a.cos(), c.y + 14.0 * a.sin());
                painter.line_segment(
                    [c, ray_tip],
                    Stroke::new(1.5_f32, Color32::from_rgb(255, 255, 230)),
                );
            }

            // Central center dot
            painter.circle_filled(c, 3.5, Color32::from_rgba_premultiplied(0, 0, 0, 80));

            // 7. Hub Status & Converting Spinner
            if is_converting {
                // Rotating progress ring
                let arc_len = 1.2 * PI;
                let steps = 18;
                for s in 0..steps {
                    let t1 = progress_angle + (s as f32 / steps as f32) * arc_len;
                    let t2 = progress_angle + ((s + 1) as f32 / steps as f32) * arc_len;
                    let r = self.hub_radius - 1.5;
                    let p1 = Pos2::new(c.x + r * t1.cos(), c.y + r * t1.sin());
                    let p2 = Pos2::new(c.x + r * t2.cos(), c.y + r * t2.sin());
                    painter.line_segment([p1, p2], Stroke::new(3.0_f32, pal.flavedo_peel));
                }
                painter.text(
                    Pos2::new(c.x, c.y + 2.0),
                    eframe::egui::Align2::CENTER_CENTER,
                    "CONVERT",
                    FontId::proportional(15.0),
                    pal.text_dark,
                );
            } else {
                // Idle or Hover instruction
                painter.text(
                    Pos2::new(c.x, c.y - 6.0),
                    eframe::egui::Align2::CENTER_CENTER,
                    "READY",
                    FontId::proportional(20.0),
                    pal.text_dark,
                );
                painter.text(
                    Pos2::new(c.x, c.y + 10.0),
                    eframe::egui::Align2::CENTER_CENTER,
                    hub_status,
                    FontId::proportional(13.5),
                    pal.text_dark,
                );
            }
        }
    }
}
