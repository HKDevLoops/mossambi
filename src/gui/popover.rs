//! Floating macOS Popover Balloon for Long-Press Quick Settings
//! Anchors to the specific petal sector that was long-pressed (>=350ms)
//! and presents parameter controls for that converter.

use crate::gui::macos_theme::*;
use crate::gui::quick_settings::QuickSettingsState;
use eframe::egui::{self, Color32, Pos2, Rect, Rounding, Stroke, Ui, Vec2};

pub struct PetalPopover;

impl PetalPopover {
    /// Renders a floating popover anchored near the long-pressed petal position.
    pub fn render(
        ui: &mut Ui,
        anchor_pos: Pos2,
        format_name: &str,
        settings: &mut QuickSettingsState,
        on_close: impl FnOnce(),
    ) {
        // Calculate popover box position (offset from petal anchor)
        let popover_w = 260.0f32;
        let popover_h = 240.0f32;

        let left = (anchor_pos.x - popover_w / 2.0).clamp(10.0, 360.0 - popover_w - 10.0);
        let top = if anchor_pos.y < 160.0 {
            anchor_pos.y + 24.0
        } else {
            (anchor_pos.y - popover_h - 24.0).max(10.0)
        };

        let rect = Rect::from_min_size(Pos2::new(left, top), Vec2::new(popover_w, popover_h));
        let painter = ui.painter();

        // 1. Soft Shadow
        painter.rect_filled(rect.expand(6.0), Rounding::same(14.0), Color32::from_black_alpha(45));
        painter.rect_filled(rect.expand(2.0), Rounding::same(12.0), Color32::from_black_alpha(70));

        // 2. Frosted macOS Card Background with 1px border
        painter.rect_filled(rect, Rounding::same(10.0), Color32::from_rgb(28, 28, 32));
        painter.rect_stroke(rect, Rounding::same(10.0), Stroke::new(1.0_f32, PANTONE_FLAVEDO));

        // 3. Anchor Triangle Pointer
        let triangle_pts = if anchor_pos.y < 160.0 {
            vec![
                Pos2::new(anchor_pos.x - 7.0, rect.min.y),
                Pos2::new(anchor_pos.x, rect.min.y - 7.0),
                Pos2::new(anchor_pos.x + 7.0, rect.min.y),
            ]
        } else {
            vec![
                Pos2::new(anchor_pos.x - 7.0, rect.max.y),
                Pos2::new(anchor_pos.x, rect.max.y + 7.0),
                Pos2::new(anchor_pos.x + 7.0, rect.max.y),
            ]
        };
        painter.add(eframe::epaint::Shape::convex_polygon(
            triangle_pts,
            Color32::from_rgb(28, 28, 32),
            Stroke::new(1.0_f32, PANTONE_FLAVEDO),
        ));

        // 4. Popover Content Area
        let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink(10.0)));

        child_ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("⚙ Quick Settings • {}", format_name))
                    .size(11.5)
                    .strong()
                    .color(PANTONE_FLAVEDO),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(egui::RichText::new("✕").size(10.0).color(MAC_TEXT_MUTED)).clicked() {
                    on_close();
                }
            });
        });

        child_ui.add_space(4.0);
        child_ui.separator();
        child_ui.add_space(4.0);

        // Format-specific parameters inside popover
        let fmt = format_name.to_uppercase();

        if fmt == "PDF" || fmt == "TXT" || fmt == "MD" || fmt == "DOCX" {
            child_ui.label(egui::RichText::new("Vector Typography").size(10.5).color(MAC_TEXT_PRIMARY));
            child_ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Page:").size(10.0));
                ui.selectable_value(&mut settings.pdf_page_size, "A4".to_string(), "A4");
                ui.selectable_value(&mut settings.pdf_page_size, "Letter".to_string(), "Letter");
            });

            child_ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Font:").size(10.0));
                ui.selectable_value(&mut settings.pdf_font_family, "Helvetica".to_string(), "Helvetica");
                ui.selectable_value(&mut settings.pdf_font_family, "Times-Roman".to_string(), "Times");
                ui.selectable_value(&mut settings.pdf_font_family, "Courier".to_string(), "Courier");
            });

            child_ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Size:").size(10.0));
                ui.selectable_value(&mut settings.pdf_font_size, 10.0, "10pt");
                ui.selectable_value(&mut settings.pdf_font_size, 11.0, "11pt");
                ui.selectable_value(&mut settings.pdf_font_size, 12.0, "12pt");
            });

            child_ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Margins:").size(10.0));
                ui.selectable_value(&mut settings.pdf_margin, 36.0, "Compact");
                ui.selectable_value(&mut settings.pdf_margin, 54.0, "Normal");
            });

            child_ui.checkbox(&mut settings.pdf_header_footer, "Header & Page No.");
        } else if fmt == "JPG" || fmt == "PNG" || fmt == "WEBP" || fmt == "AVIF" {
            child_ui.label(egui::RichText::new("Image Fidelity").size(10.5).color(MAC_TEXT_PRIMARY));
            child_ui.checkbox(&mut settings.image_lossless, "Strict 0% Lossless");
            if !settings.image_lossless {
                child_ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Quality:").size(10.0));
                    ui.add(egui::Slider::new(&mut settings.image_quality, 10..=100).text("%"));
                });
            }
            child_ui.horizontal(|ui| {
                ui.label(egui::RichText::new("DPI:").size(10.0));
                ui.selectable_value(&mut settings.image_dpi, 72, "72");
                ui.selectable_value(&mut settings.image_dpi, 150, "150");
                ui.selectable_value(&mut settings.image_dpi, 200, "200");
                ui.selectable_value(&mut settings.image_dpi, 300, "300");
            });
        } else if fmt == "MP3" || fmt == "WAV" || fmt == "FLAC" || fmt == "M4A" || fmt == "OGG" {
            child_ui.label(egui::RichText::new("Audio Encoding").size(10.5).color(MAC_TEXT_PRIMARY));
            child_ui.checkbox(&mut settings.audio_strict_lossless, "Strict 0% Bit-Exact (FLAC/PCM)");
            child_ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Bitrate:").size(10.0));
                ui.selectable_value(&mut settings.audio_bitrate, "128k".to_string(), "128k");
                ui.selectable_value(&mut settings.audio_bitrate, "192k".to_string(), "192k");
                ui.selectable_value(&mut settings.audio_bitrate, "256k".to_string(), "256k");
                ui.selectable_value(&mut settings.audio_bitrate, "320k".to_string(), "320k");
            });
        } else if fmt == "MP4" || fmt == "MKV" || fmt == "MOV" || fmt == "WEBM" {
            child_ui.label(egui::RichText::new("Video HW Acceleration").size(10.5).color(MAC_TEXT_PRIMARY));
            child_ui.checkbox(&mut settings.video_hw_accel, "NVENC/AMF/QSV GPU Accel");
            child_ui.horizontal(|ui| {
                ui.label(egui::RichText::new("CRF:").size(10.0));
                ui.selectable_value(&mut settings.video_crf, 16, "16 (Near Lossless)");
                ui.selectable_value(&mut settings.video_crf, 18, "18");
                ui.selectable_value(&mut settings.video_crf, 23, "23");
            });
        } else if fmt == "ZIP" || fmt == "7Z" || fmt == "TAR" || fmt == "EXTRACT" {
            child_ui.label(egui::RichText::new("Archive Suite").size(10.5).color(MAC_TEXT_PRIMARY));
            child_ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Level:").size(10.0));
                ui.selectable_value(&mut settings.archive_compression, "Fast".to_string(), "Fast");
                ui.selectable_value(&mut settings.archive_compression, "Normal".to_string(), "Normal");
                ui.selectable_value(&mut settings.archive_compression, "Ultra".to_string(), "Ultra");
            });
            child_ui.label(egui::RichText::new("✓ Zip-Slip Protection Active").size(9.5).color(PANTONE_FLAVEDO));
        } else if fmt == "ALL" {
            child_ui.label(egui::RichText::new("⭐ Batch ALL Transcoding").size(10.5).strong().color(PANTONE_GOLD));
            child_ui.label(
                egui::RichText::new("Converts source simultaneously to ALL valid formats in domain using Rayon multi-core worker threads.")
                    .size(9.5)
                    .color(MAC_TEXT_SECONDARY),
            );
        }

        child_ui.add_space(4.0);
        child_ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Tap to convert • Hold for options").size(9.0).color(MAC_TEXT_MUTED));
        });
    }
}
