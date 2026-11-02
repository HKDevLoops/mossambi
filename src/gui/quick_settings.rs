//! Contextual Quick Settings Menu for Mossambi
//! Displays a compact macOS-styled floating parameter panel for the active format/converter
//! (PDF typography & margins, image quality/lossless, audio bitrate, video HW accel, and Indian exam constraints).

use eframe::egui::{self, Color32, Frame, Margin, Rounding, Stroke, Ui};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickSettingsState {
    pub is_open: bool,
    pub active_format: String,

    // Document / PDF settings
    pub pdf_page_size: String,
    pub pdf_font_family: String,
    pub pdf_font_size: f32,
    pub pdf_margin: f32,
    pub pdf_header_footer: bool,

    // Image settings
    pub image_quality: u8,
    pub image_lossless: bool,
    pub image_dpi: u32,

    // Audio settings
    pub audio_bitrate: String,
    pub audio_strict_lossless: bool,

    // Video settings
    pub video_hw_accel: bool,
    pub video_crf: u8,
    pub video_preset: String,

    // Archive settings
    pub archive_compression: String,

    // Exam Mode settings
    pub exam_mode_enabled: bool,
    pub exam_preset_name: String,
    pub exam_width: u32,
    pub exam_height: u32,
    pub exam_dpi: u32,
    pub exam_min_kb: u64,
    pub exam_max_kb: u64,
    pub exam_whiten: bool,
}

impl Default for QuickSettingsState {
    fn default() -> Self {
        Self {
            is_open: true, // Default open as requested: "By default a small menu should popup for quick small settings"
            active_format: "PDF".to_string(),

            pdf_page_size: "A4".to_string(),
            pdf_font_family: "Helvetica".to_string(),
            pdf_font_size: 11.0,
            pdf_margin: 54.0,
            pdf_header_footer: true,

            image_quality: 95,
            image_lossless: false,
            image_dpi: 150,

            audio_bitrate: "256k".to_string(),
            audio_strict_lossless: true,

            video_hw_accel: true,
            video_crf: 16,
            video_preset: "High Quality".to_string(),

            archive_compression: "Ultra".to_string(),

            exam_mode_enabled: false,
            exam_preset_name: "Custom Specification".to_string(),
            exam_width: 350,
            exam_height: 350,
            exam_dpi: 200,
            exam_min_kb: 20,
            exam_max_kb: 100,
            exam_whiten: true,
        }
    }
}

impl QuickSettingsState {
    pub fn set_active_format(&mut self, fmt: &str) {
        self.active_format = fmt.to_uppercase();
        if self.active_format.contains("EXAM") {
            self.exam_mode_enabled = true;
        }
    }

    pub fn apply_exam_preset(&mut self, constraint: &crate::exam::ExamConstraint) {
        self.exam_mode_enabled = true;
        self.exam_preset_name = constraint.name.clone();
        self.exam_width = constraint.width_px;
        self.exam_height = constraint.height_px;
        self.exam_dpi = constraint.dpi;
        self.exam_min_kb = constraint.min_bytes / 1024;
        self.exam_max_kb = constraint.max_bytes / 1024;
        self.exam_whiten = constraint.normalize_white_balance;
    }

    /// Renders the sleek macOS-styled floating quick settings panel.
    pub fn render_ui(&mut self, ui: &mut Ui) {
        let frame = Frame::none()
            .fill(Color32::from_rgba_premultiplied(30, 36, 28, 240))
            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(120, 190, 32))) // Pantone 376 C border
            .rounding(Rounding::same(12.0))
            .inner_margin(Margin::same(14.0));

        frame.show(ui, |ui| {
            ui.set_width(290.0);

            // Title Bar
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("⚙ Quick Settings")
                        .size(13.0)
                        .strong()
                        .color(Color32::from_rgb(120, 190, 32)),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(format!("[{}]", self.active_format))
                            .size(11.0)
                            .strong()
                            .color(Color32::from_rgb(255, 199, 44)),
                    );
                });
            });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(6.0);

            let fmt = self.active_format.as_str();

            // 1. Document / PDF Settings
            if fmt == "PDF" || fmt == "TXT" || fmt == "MD" || fmt == "DOCX" || fmt == "HTML" {
                ui.label(
                    egui::RichText::new("Vector PDF Typography & Layout")
                        .size(11.0)
                        .color(Color32::from_rgb(220, 230, 220)),
                );
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Page Size:").size(11.0));
                    ui.selectable_value(&mut self.pdf_page_size, "A4".to_string(), "A4");
                    ui.selectable_value(&mut self.pdf_page_size, "Letter".to_string(), "Letter");
                });

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Font:").size(11.0));
                    ui.selectable_value(&mut self.pdf_font_family, "Helvetica".to_string(), "Helvetica");
                    ui.selectable_value(&mut self.pdf_font_family, "Times-Roman".to_string(), "Times");
                    ui.selectable_value(&mut self.pdf_font_family, "Courier".to_string(), "Courier");
                });

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Font Size:").size(11.0));
                    ui.selectable_value(&mut self.pdf_font_size, 10.0, "10pt");
                    ui.selectable_value(&mut self.pdf_font_size, 11.0, "11pt");
                    ui.selectable_value(&mut self.pdf_font_size, 12.0, "12pt");
                    ui.selectable_value(&mut self.pdf_font_size, 14.0, "14pt");
                });

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Margins:").size(11.0));
                    ui.selectable_value(&mut self.pdf_margin, 36.0, "Compact");
                    ui.selectable_value(&mut self.pdf_margin, 54.0, "Normal");
                    ui.selectable_value(&mut self.pdf_margin, 72.0, "Wide");
                });

                ui.checkbox(&mut self.pdf_header_footer, "Include Header & Page Numbers");
            }
            // 2. Image Settings
            else if fmt == "JPG" || fmt == "JPEG" || fmt == "PNG" || fmt == "WEBP" || fmt == "AVIF" || fmt == "TIFF" {
                ui.label(
                    egui::RichText::new("Image Quality & Encoding")
                        .size(11.0)
                        .color(Color32::from_rgb(220, 230, 220)),
                );
                ui.add_space(4.0);

                ui.checkbox(&mut self.image_lossless, "Strict 0.00% Lossless");

                if !self.image_lossless {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Quality:").size(11.0));
                        ui.add(egui::Slider::new(&mut self.image_quality, 10..=100).text("%"));
                    });
                }

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Target DPI:").size(11.0));
                    ui.selectable_value(&mut self.image_dpi, 72, "72");
                    ui.selectable_value(&mut self.image_dpi, 150, "150");
                    ui.selectable_value(&mut self.image_dpi, 200, "200");
                    ui.selectable_value(&mut self.image_dpi, 300, "300");
                });
            }
            // 3. Audio Settings
            else if fmt == "MP3" || fmt == "WAV" || fmt == "FLAC" || fmt == "M4A" || fmt == "OGG" || fmt == "AAC" || fmt == "OPUS" {
                ui.label(
                    egui::RichText::new("Audio Quality & Bitrate")
                        .size(11.0)
                        .color(Color32::from_rgb(220, 230, 220)),
                );
                ui.add_space(4.0);

                ui.checkbox(&mut self.audio_strict_lossless, "Strict 0.00% Bit-Exact (FLAC/WAV)");

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Bitrate:").size(11.0));
                    ui.selectable_value(&mut self.audio_bitrate, "128k".to_string(), "128k");
                    ui.selectable_value(&mut self.audio_bitrate, "192k".to_string(), "192k");
                    ui.selectable_value(&mut self.audio_bitrate, "256k".to_string(), "256k");
                    ui.selectable_value(&mut self.audio_bitrate, "320k".to_string(), "320k");
                });
            }
            // 4. Video Settings
            else if fmt == "MP4" || fmt == "MOV" || fmt == "MKV" || fmt == "WEBM" || fmt == "GIF" {
                ui.label(
                    egui::RichText::new("Video Hardware Acceleration")
                        .size(11.0)
                        .color(Color32::from_rgb(220, 230, 220)),
                );
                ui.add_space(4.0);

                ui.checkbox(&mut self.video_hw_accel, "Hardware Acceleration (NVENC/AMF/QSV)");

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("CRF Quality:").size(11.0));
                    ui.selectable_value(&mut self.video_crf, 16, "16 (Near Lossless)");
                    ui.selectable_value(&mut self.video_crf, 18, "18 (Visually Lossless)");
                    ui.selectable_value(&mut self.video_crf, 23, "23 (Standard)");
                });
            }
            // 5. Archive Settings
            else if fmt == "ZIP" || fmt == "7Z" || fmt == "TAR" || fmt == "TAR.GZ" || fmt == "EXTRACT" {
                ui.label(
                    egui::RichText::new("Archive & Compression Suite")
                        .size(11.0)
                        .color(Color32::from_rgb(220, 230, 220)),
                );
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Compression:").size(11.0));
                    ui.selectable_value(&mut self.archive_compression, "Fast".to_string(), "Fast");
                    ui.selectable_value(&mut self.archive_compression, "Normal".to_string(), "Normal");
                    ui.selectable_value(&mut self.archive_compression, "Ultra".to_string(), "Ultra");
                });
                ui.label(egui::RichText::new("✓ Zip-Slip Traversal Protection Active").size(10.0).color(Color32::from_rgb(120, 190, 32)));
            }
            // 6. ALL Sector
            else if fmt == "ALL" {
                ui.label(
                    egui::RichText::new("⭐ Batch ALL Transcoding Mode")
                        .size(11.0)
                        .strong()
                        .color(Color32::from_rgb(255, 184, 28)),
                );
                ui.label(
                    egui::RichText::new("Converts source file simultaneously to ALL valid formats in domain using Rayon multi-core concurrency.")
                        .size(10.0)
                        .color(Color32::from_rgb(200, 210, 200)),
                );
            }

            ui.add_space(6.0);
            ui.separator();

            // Custom Specification & Form Conformer Parameters
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.exam_mode_enabled, "🎯 Custom Parameter Envelope");
            });

            if self.exam_mode_enabled {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.add(egui::TextEdit::singleline(&mut self.exam_preset_name).desired_width(170.0));
                });

                ui.horizontal(|ui| {
                    ui.label("Dimensions:");
                    ui.add(egui::DragValue::new(&mut self.exam_width).speed(1).prefix("W: ").suffix("px"));
                    ui.add(egui::DragValue::new(&mut self.exam_height).speed(1).prefix("H: ").suffix("px"));
                });

                ui.horizontal(|ui| {
                    ui.label("Size Limit:");
                    ui.add(egui::DragValue::new(&mut self.exam_min_kb).speed(1).prefix("Min: ").suffix("KB"));
                    ui.add(egui::DragValue::new(&mut self.exam_max_kb).speed(1).prefix("Max: ").suffix("KB"));
                });

                ui.horizontal(|ui| {
                    ui.label("DPI:");
                    ui.add(egui::DragValue::new(&mut self.exam_dpi).speed(1));
                    ui.checkbox(&mut self.exam_whiten, "Whiten Background");
                });

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(format!("Envelope: {}x{} px | {}-{} KB @ {} DPI", self.exam_width, self.exam_height, self.exam_min_kb, self.exam_max_kb, self.exam_dpi)).size(10.0).color(Color32::from_rgb(255, 199, 44)));
                });
            }
        });
    }
}
