//! Interactive Pre-Conversion Crop Canvas
//! Normalized bounding-box cropping with aspect ratio locks and rule-of-thirds grid.
//! 100% pure ASCII implementation.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CropAspectRatio {
    #[default]
    Free,
    Square,      // 1:1
    FourThree,   // 4:3
    SixteenNine, // 16:9
    ThreeTwo,    // 3:2
    FiveFour,    // 5:4
}

impl CropAspectRatio {
    pub fn ratio(&self) -> Option<f32> {
        match self {
            Self::Free => None,
            Self::Square => Some(1.0),
            Self::FourThree => Some(4.0 / 3.0),
            Self::SixteenNine => Some(16.0 / 9.0),
            Self::ThreeTwo => Some(3.0 / 2.0),
            Self::FiveFour => Some(5.0 / 4.0),
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Free => "Free",
            Self::Square => "1:1 Square",
            Self::FourThree => "4:3 Standard",
            Self::SixteenNine => "16:9 Widescreen",
            Self::ThreeTwo => "3:2 Classic Photo",
            Self::FiveFour => "5:4 Medium Format",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CropHandle {
    TopLeft,
    Top,
    TopRight,
    Right,
    BottomRight,
    Bottom,
    BottomLeft,
    Left,
    CenterPan,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NormalizedCrop {
    pub x: f32, // 0.0 ..= 1.0
    pub y: f32, // 0.0 ..= 1.0
    pub w: f32, // 0.0 ..= 1.0
    pub h: f32, // 0.0 ..= 1.0
}

impl Default for NormalizedCrop {
    fn default() -> Self {
        Self {
            x: 0.05,
            y: 0.05,
            w: 0.90,
            h: 0.90,
        }
    }
}

impl NormalizedCrop {
    pub fn clamp(&mut self) {
        self.w = self.w.clamp(0.05, 1.0);
        self.h = self.h.clamp(0.05, 1.0);
        self.x = self.x.clamp(0.0, (1.0 - self.w).max(0.0));
        self.y = self.y.clamp(0.0, (1.0 - self.h).max(0.0));
    }

    pub fn to_pixel_rect(&self, image_w: u32, image_h: u32) -> (u32, u32, u32, u32) {
        let px = (self.x * image_w as f32).round() as u32;
        let py = (self.y * image_h as f32).round() as u32;
        let pw = (self.w * image_w as f32).round().max(1.0) as u32;
        let ph = (self.h * image_h as f32).round().max(1.0) as u32;
        (
            px.min(image_w.saturating_sub(1)),
            py.min(image_h.saturating_sub(1)),
            pw.min(image_w.saturating_sub(px)),
            ph.min(image_h.saturating_sub(py)),
        )
    }
}

pub struct CropCanvas {
    pub crop: NormalizedCrop,
    pub aspect_ratio: CropAspectRatio,
    pub active_handle: Option<CropHandle>,
    pub is_dragging: bool,
    pub show_grid: bool,
    pub original_dims: (u32, u32),
}

impl Default for CropCanvas {
    fn default() -> Self {
        Self {
            crop: NormalizedCrop::default(),
            aspect_ratio: CropAspectRatio::Free,
            active_handle: None,
            is_dragging: false,
            show_grid: true,
            original_dims: (1920, 1080),
        }
    }
}

impl CropCanvas {
    pub fn new(original_width: u32, original_height: u32) -> Self {
        Self {
            crop: NormalizedCrop::default(),
            aspect_ratio: CropAspectRatio::Free,
            active_handle: None,
            is_dragging: false,
            show_grid: true,
            original_dims: (original_width.max(1), original_height.max(1)),
        }
    }

    pub fn set_aspect_ratio(&mut self, ratio: CropAspectRatio) {
        self.aspect_ratio = ratio;
        if let Some(target_ratio) = ratio.ratio() {
            let img_aspect = self.original_dims.0 as f32 / self.original_dims.1 as f32;
            let norm_ratio = target_ratio / img_aspect;

            if norm_ratio > 1.0 {
                self.crop.w = 0.90;
                self.crop.h = (self.crop.w / norm_ratio).min(0.90);
            } else {
                self.crop.h = 0.90;
                self.crop.w = (self.crop.h * norm_ratio).min(0.90);
            }
            self.crop.x = (1.0 - self.crop.w) / 2.0;
            self.crop.y = (1.0 - self.crop.h) / 2.0;
            self.crop.clamp();
        }
    }

    /// Render interactive crop canvas on top of an image rect.
    pub fn show(&mut self, ui: &mut egui::Ui, image_rect: Rect) -> bool {
        let mut changed = false;
        let painter = ui.painter_at(image_rect);

        let crop_screen_rect = Rect::from_min_size(
            Pos2::new(
                image_rect.left() + self.crop.x * image_rect.width(),
                image_rect.top() + self.crop.y * image_rect.height(),
            ),
            Vec2::new(
                self.crop.w * image_rect.width(),
                self.crop.h * image_rect.height(),
            ),
        );

        let response = ui.allocate_rect(image_rect, egui::Sense::click_and_drag());

        // 1. Dimmed semi-transparent mask around the crop box
        let mask_col = Color32::from_rgba_premultiplied(0, 0, 0, 135);
        // Top band
        painter.rect_filled(
            Rect::from_min_max(
                image_rect.min,
                Pos2::new(image_rect.right(), crop_screen_rect.top()),
            ),
            0.0,
            mask_col,
        );
        // Bottom band
        painter.rect_filled(
            Rect::from_min_max(
                Pos2::new(image_rect.left(), crop_screen_rect.bottom()),
                image_rect.max,
            ),
            0.0,
            mask_col,
        );
        // Left band
        painter.rect_filled(
            Rect::from_min_max(
                Pos2::new(image_rect.left(), crop_screen_rect.top()),
                Pos2::new(crop_screen_rect.left(), crop_screen_rect.bottom()),
            ),
            0.0,
            mask_col,
        );
        // Right band
        painter.rect_filled(
            Rect::from_min_max(
                Pos2::new(crop_screen_rect.right(), crop_screen_rect.top()),
                Pos2::new(image_rect.right(), crop_screen_rect.bottom()),
            ),
            0.0,
            mask_col,
        );

        // 2. Crop box border
        painter.rect_stroke(
            crop_screen_rect,
            0.0,
            Stroke::new(1.5_f32, Color32::from_rgb(255, 255, 255)),
        );

        // 3. Rule of Thirds Guide Lines
        if self.show_grid || self.is_dragging {
            let grid_col = Color32::from_rgba_premultiplied(255, 255, 255, 75);
            let grid_stroke = Stroke::new(1.0_f32, grid_col);

            let w3 = crop_screen_rect.width() / 3.0;
            let h3 = crop_screen_rect.height() / 3.0;

            // Verticals
            painter.line_segment(
                [
                    Pos2::new(crop_screen_rect.left() + w3, crop_screen_rect.top()),
                    Pos2::new(crop_screen_rect.left() + w3, crop_screen_rect.bottom()),
                ],
                grid_stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(crop_screen_rect.left() + 2.0 * w3, crop_screen_rect.top()),
                    Pos2::new(
                        crop_screen_rect.left() + 2.0 * w3,
                        crop_screen_rect.bottom(),
                    ),
                ],
                grid_stroke,
            );

            // Horizontals
            painter.line_segment(
                [
                    Pos2::new(crop_screen_rect.left(), crop_screen_rect.top() + h3),
                    Pos2::new(crop_screen_rect.right(), crop_screen_rect.top() + h3),
                ],
                grid_stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(crop_screen_rect.left(), crop_screen_rect.top() + 2.0 * h3),
                    Pos2::new(crop_screen_rect.right(), crop_screen_rect.top() + 2.0 * h3),
                ],
                grid_stroke,
            );
        }

        // 4. Draggable Boundary Handles
        let handle_size = 9.0;
        let draw_handle = |pos: Pos2, painter: &egui::Painter| {
            let hr = Rect::from_center_size(pos, Vec2::splat(handle_size));
            painter.rect_filled(hr, 2.0, Color32::WHITE);
            painter.rect_stroke(hr, 2.0, Stroke::new(1.0_f32, Color32::from_rgb(40, 45, 55)));
        };

        draw_handle(crop_screen_rect.left_top(), &painter);
        draw_handle(crop_screen_rect.center_top(), &painter);
        draw_handle(crop_screen_rect.right_top(), &painter);
        draw_handle(crop_screen_rect.right_center(), &painter);
        draw_handle(crop_screen_rect.right_bottom(), &painter);
        draw_handle(crop_screen_rect.center_bottom(), &painter);
        draw_handle(crop_screen_rect.left_bottom(), &painter);
        draw_handle(crop_screen_rect.left_center(), &painter);

        // 5. Drag Interaction Handling
        if response.drag_started() {
            if let Some(pos) = response.hover_pos() {
                self.active_handle = self.hit_test_handles(pos, crop_screen_rect, handle_size);
                self.is_dragging = self.active_handle.is_some();
            }
        }

        if response.dragged() {
            if let Some(handle) = self.active_handle {
                let delta = response.drag_delta();
                let dx = delta.x / image_rect.width();
                let dy = delta.y / image_rect.height();

                match handle {
                    CropHandle::CenterPan => {
                        self.crop.x += dx;
                        self.crop.y += dy;
                    }
                    CropHandle::TopLeft => {
                        self.crop.x += dx;
                        self.crop.y += dy;
                        self.crop.w -= dx;
                        self.crop.h -= dy;
                    }
                    CropHandle::Top => {
                        self.crop.y += dy;
                        self.crop.h -= dy;
                    }
                    CropHandle::TopRight => {
                        self.crop.y += dy;
                        self.crop.w += dx;
                        self.crop.h -= dy;
                    }
                    CropHandle::Right => {
                        self.crop.w += dx;
                    }
                    CropHandle::BottomRight => {
                        self.crop.w += dx;
                        self.crop.h += dy;
                    }
                    CropHandle::Bottom => {
                        self.crop.h += dy;
                    }
                    CropHandle::BottomLeft => {
                        self.crop.x += dx;
                        self.crop.w -= dx;
                        self.crop.h += dy;
                    }
                    CropHandle::Left => {
                        self.crop.x += dx;
                        self.crop.w -= dx;
                    }
                }

                self.crop.clamp();
                changed = true;
            }
        }

        if response.drag_stopped() {
            self.active_handle = None;
            self.is_dragging = false;
        }

        // 6. Dimension Badge below Crop Frame
        let (px, py, pw, ph) = self
            .crop
            .to_pixel_rect(self.original_dims.0, self.original_dims.1);
        let badge_text = format!("{} x {} px [{}]", pw, ph, self.aspect_ratio.label());
        let badge_pos = Pos2::new(
            crop_screen_rect.center().x,
            (crop_screen_rect.bottom() + 16.0).min(image_rect.bottom() - 10.0),
        );
        let badge_rect = Rect::from_center_size(badge_pos, Vec2::new(170.0, 20.0));
        painter.rect_filled(
            badge_rect,
            5.0,
            Color32::from_rgba_premultiplied(20, 25, 35, 220),
        );
        painter.rect_stroke(
            badge_rect,
            5.0,
            Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(255, 255, 255, 45)),
        );
        painter.text(
            badge_pos,
            egui::Align2::CENTER_CENTER,
            badge_text,
            egui::FontId::proportional(11.0),
            Color32::from_rgb(230, 235, 245),
        );

        let _ = (px, py);
        changed
    }

    fn hit_test_handles(&self, pos: Pos2, rect: Rect, size: f32) -> Option<CropHandle> {
        let tolerance = size * 1.5;
        let is_near = |p: Pos2| (pos - p).length() <= tolerance;

        if is_near(rect.left_top()) {
            Some(CropHandle::TopLeft)
        } else if is_near(rect.center_top()) {
            Some(CropHandle::Top)
        } else if is_near(rect.right_top()) {
            Some(CropHandle::TopRight)
        } else if is_near(rect.right_center()) {
            Some(CropHandle::Right)
        } else if is_near(rect.right_bottom()) {
            Some(CropHandle::BottomRight)
        } else if is_near(rect.center_bottom()) {
            Some(CropHandle::Bottom)
        } else if is_near(rect.left_bottom()) {
            Some(CropHandle::BottomLeft)
        } else if is_near(rect.left_center()) {
            Some(CropHandle::Left)
        } else if rect.contains(pos) {
            Some(CropHandle::CenterPan)
        } else {
            None
        }
    }
}

/// Applies the normalized crop rectangle to a DynamicImage.
pub fn apply_crop_to_image(
    img: &image::DynamicImage,
    crop: &NormalizedCrop,
) -> image::DynamicImage {
    let (x, y, w, h) = crop.to_pixel_rect(img.width(), img.height());
    image::imageops::crop_imm(img, x, y, w, h).to_image().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crop_pixel_rect_calculation() {
        let crop = NormalizedCrop {
            x: 0.10,
            y: 0.20,
            w: 0.50,
            h: 0.40,
        };
        let (x, y, w, h) = crop.to_pixel_rect(1000, 1000);
        assert_eq!(x, 100);
        assert_eq!(y, 200);
        assert_eq!(w, 500);
        assert_eq!(h, 400);
    }

    #[test]
    fn test_crop_clamping() {
        let mut crop = NormalizedCrop {
            x: -0.5,
            y: -0.5,
            w: 1.5,
            h: 1.5,
        };
        crop.clamp();
        assert!(crop.x >= 0.0);
        assert!(crop.y >= 0.0);
        assert!(crop.w <= 1.0);
        assert!(crop.h <= 1.0);
    }

    #[test]
    fn test_aspect_ratio_presets() {
        let mut canvas = CropCanvas::new(1920, 1080);
        canvas.set_aspect_ratio(CropAspectRatio::Square);
        assert_eq!(canvas.aspect_ratio, CropAspectRatio::Square);
        assert!(canvas.crop.w > 0.0 && canvas.crop.h > 0.0);
    }
}
