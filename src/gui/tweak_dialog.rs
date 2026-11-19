//! Tangerine-Inspired Floating Tweak & Preview Modal Dialog
//!
//! Pure ASCII. Provides pre-flight conversion parameter tuning, live output
//! file size estimation, before/after preview, and direct "Save as Preset" integration.

use crate::core::fs_utils::format_file_size;
use crate::core::presets::{save_custom_preset, PresetDefinition, PresetOptions};
use eframe::egui::{self, Color32, Pos2, RichText, Slider, Stroke, Vec2};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct TweakDialogState {
    pub is_open: bool,
    pub input_path: PathBuf,
    pub target_format: String,
    pub options: PresetOptions,
    pub original_size_bytes: u64,
    pub estimated_output_size: u64,
    pub show_save_preset: bool,
    pub preset_name_input: String,
    pub preset_saved_toast: Option<String>,
}

impl Default for TweakDialogState {
    fn default() -> Self {
        Self {
            is_open: false,
            input_path: PathBuf::new(),
            target_format: "WEBP".to_string(),
            options: PresetOptions::default(),
            original_size_bytes: 0,
            estimated_output_size: 0,
            show_save_preset: false,
            preset_name_input: String::new(),
            preset_saved_toast: None,
        }
    }
}

impl TweakDialogState {
    pub fn open_for_file(path: &Path, target_format: &str) -> Self {
        let size = std::fs::metadata(path)
            .map(|m| m.len())
            .unwrap_or(1024 * 500);
        let mut state = Self {
            is_open: true,
            input_path: path.to_path_buf(),
            target_format: target_format.to_ascii_uppercase(),
            options: PresetOptions::default(),
            original_size_bytes: size,
            estimated_output_size: 0,
            show_save_preset: false,
            preset_name_input: format!("Custom {}", target_format.to_ascii_uppercase()),
            preset_saved_toast: None,
        };
        state.recalculate_size_estimate();
        state
    }

    /// Calculates realistic estimated output file size based on input parameters.
    pub fn recalculate_size_estimate(&mut self) {
        let base_size = self.original_size_bytes as f64;
        let quality_factor = self.options.quality.unwrap_or(85) as f64 / 100.0;
        let scale_factor = self.options.scale.unwrap_or(1.0) as f64;
        let area_factor = (scale_factor * scale_factor).clamp(0.05, 4.0);

        // Check if Indian Exam Conformer clamp is active
        if let Some(ref exam) = self.options.exam_preset {
            if exam == "UPSC Photo" || exam == "SSC Photo" {
                self.estimated_output_size = 45 * 1024; // ~45 KB
                return;
            } else if exam == "UPSC Signature" {
                self.estimated_output_size = 18 * 1024; // ~18 KB
                return;
            }
        }

        let estimated = match self.target_format.as_str() {
            "WEBP" => {
                if self.options.lossless.unwrap_or(false) {
                    base_size * area_factor * 0.75
                } else {
                    base_size * area_factor * quality_factor * 0.40
                }
            }
            "AVIF" => base_size * area_factor * quality_factor * 0.30,
            "JPG" | "JPEG" => base_size * area_factor * quality_factor * 0.60,
            "PNG" => base_size * area_factor * 0.85,
            "PDF" => (base_size * area_factor * quality_factor * 0.70).max(15_000.0),
            "DOCX" | "ODT" => (base_size * 0.50).max(8_000.0),
            "ZIP" | "7Z" => base_size * 0.65,
            "MP4" => base_size * quality_factor * 0.75,
            "MP3" => (base_size * 0.20).max(30_000.0),
            _ => base_size * area_factor * quality_factor * 0.60,
        };

        // If metadata strip is enabled, subtract ~4 KB of headers/ICC profile
        let final_est = if self.options.strip_metadata.unwrap_or(false) {
            (estimated - 4096.0).max(512.0)
        } else {
            estimated.max(512.0)
        };

        self.estimated_output_size = final_est as u64;
    }

    /// Renders the Tangerine-style floating glass modal dialog.
    /// Returns Some(PresetOptions) if the user clicked "Convert Now".
    pub fn render(&mut self, ctx: &egui::Context) -> Option<PresetOptions> {
        if !self.is_open {
            return None;
        }

        let mut convert_triggered = false;
        let mut close_dialog = false;

        let screen_rect = ctx.screen_rect();
        let center = screen_rect.center();

        let window = egui::Window::new("Tangerine Tuning & Live Preview")
            .fixed_pos(Pos2::new(center.x - 220.0, center.y - 240.0))
            .fixed_size(Vec2::new(440.0, 480.0))
            .resizable(false)
            .collapsible(false)
            .title_bar(true);

        window.show(ctx, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(8.0, 10.0);

            // 1. File & Format Header
            let file_name = self
                .input_path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "Selected File".to_string());

            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Target Format:")
                        .strong()
                        .color(Color32::from_rgb(180, 180, 180)),
                );
                ui.label(
                    RichText::new(&self.target_format)
                        .strong()
                        .size(16.0)
                        .color(Color32::from_rgb(90, 200, 120)),
                );
            });

            ui.label(
                RichText::new(format!("Input: {}", file_name))
                    .small()
                    .color(Color32::from_rgb(160, 160, 160)),
            );

            ui.separator();

            // 2. Live Size Delta Estimator Banner
            let orig_str = format_file_size(self.original_size_bytes);
            let est_str = format_file_size(self.estimated_output_size);
            let delta_percent = if self.original_size_bytes > 0 {
                let diff = (self.estimated_output_size as f64 - self.original_size_bytes as f64)
                    / self.original_size_bytes as f64
                    * 100.0;
                format!("{:+0.1}%", diff)
            } else {
                "0%".to_string()
            };

            egui::Frame::none()
                .fill(Color32::from_rgba_premultiplied(30, 35, 45, 220))
                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(60, 70, 90)))
                .rounding(6.0)
                .inner_margin(egui::Margin::same(10.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Estimated Size:").strong());
                        ui.label(
                            RichText::new(format!("{} -> {}", orig_str, est_str))
                                .strong()
                                .color(Color32::from_rgb(100, 220, 255)),
                        );
                        ui.label(
                            RichText::new(format!("({})", delta_percent))
                                .strong()
                                .color(if delta_percent.starts_with('-') {
                                    Color32::from_rgb(120, 230, 120)
                                } else {
                                    Color32::from_rgb(255, 160, 80)
                                }),
                        );
                    });
                });

            ui.add_space(4.0);

            // 3. Tuning Sliders & Parameter Controls
            let mut changed = false;

            // Quality Slider (if applicable to format)
            let is_raster = matches!(
                self.target_format.as_str(),
                "WEBP" | "JPG" | "JPEG" | "AVIF" | "MP4"
            );
            if is_raster {
                let mut q = self.options.quality.unwrap_or(85);
                ui.horizontal(|ui| {
                    ui.label("Quality:");
                    if ui.add(Slider::new(&mut q, 1..=100).text("%")).changed() {
                        self.options.quality = Some(q);
                        changed = true;
                    }
                });
            }

            // Lossless Toggle
            if self.target_format == "WEBP" || self.target_format == "PNG" {
                let mut lossless = self.options.lossless.unwrap_or(false);
                if ui
                    .checkbox(&mut lossless, "Lossless Compression (0.00% Zero-Loss)")
                    .changed()
                {
                    self.options.lossless = Some(lossless);
                    changed = true;
                }
            }

            // Dimension Scaling Presets (50%, 75%, 100%, 150%)
            let mut scale = self.options.scale.unwrap_or(1.0);
            ui.horizontal(|ui| {
                ui.label("Scale:");
                if ui
                    .add(Slider::new(&mut scale, 0.25..=2.0).step_by(0.05))
                    .changed()
                {
                    self.options.scale = Some(scale);
                    changed = true;
                }
                if ui.button("50%").clicked() {
                    self.options.scale = Some(0.5);
                    changed = true;
                }
                if ui.button("100%").clicked() {
                    self.options.scale = Some(1.0);
                    changed = true;
                }
            });

            // Target DPI
            let mut dpi = self.options.dpi.unwrap_or(300);
            ui.horizontal(|ui| {
                ui.label("Target DPI:");
                if ui
                    .add(Slider::new(&mut dpi, 72..=600).step_by(1.0))
                    .changed()
                {
                    self.options.dpi = Some(dpi);
                    changed = true;
                }
            });

            // Strip Metadata (EXIF, GPS, camera serials)
            let mut strip_meta = self.options.strip_metadata.unwrap_or(false);
            if ui
                .checkbox(
                    &mut strip_meta,
                    "Strip Metadata (EXIF, GPS, camera serials)",
                )
                .changed()
            {
                self.options.strip_metadata = Some(strip_meta);
                changed = true;
            }

            // Indian Exam Conformer Dropdown
            ui.horizontal(|ui| {
                ui.label("Exam Conformer:");
                let current_exam = self
                    .options
                    .exam_preset
                    .as_deref()
                    .unwrap_or("None")
                    .to_string();
                egui::ComboBox::from_id_salt("exam_preset_combo")
                    .selected_text(&current_exam)
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_label(self.options.exam_preset.is_none(), "None")
                            .clicked()
                        {
                            self.options.exam_preset = None;
                            changed = true;
                        }
                        if ui
                            .selectable_label(
                                self.options.exam_preset.as_deref() == Some("UPSC Photo"),
                                "UPSC Photo (350x450, 20-50 KB)",
                            )
                            .clicked()
                        {
                            self.options.exam_preset = Some("UPSC Photo".to_string());
                            self.options.target_width = Some(350);
                            self.options.target_height = Some(450);
                            self.options.min_kb = Some(20);
                            self.options.max_kb = Some(50);
                            self.options.dpi = Some(300);
                            self.options.whiten_background = Some(true);
                            changed = true;
                        }
                        if ui
                            .selectable_label(
                                self.options.exam_preset.as_deref() == Some("UPSC Signature"),
                                "UPSC Signature (1000x550, 10-20 KB)",
                            )
                            .clicked()
                        {
                            self.options.exam_preset = Some("UPSC Signature".to_string());
                            self.options.target_width = Some(1000);
                            self.options.target_height = Some(550);
                            self.options.min_kb = Some(10);
                            self.options.max_kb = Some(20);
                            self.options.whiten_background = Some(true);
                            changed = true;
                        }
                        if ui
                            .selectable_label(
                                self.options.exam_preset.as_deref() == Some("SSC Photo"),
                                "SSC Photo (20-50 KB)",
                            )
                            .clicked()
                        {
                            self.options.exam_preset = Some("SSC Photo".to_string());
                            self.options.min_kb = Some(20);
                            self.options.max_kb = Some(50);
                            self.options.whiten_background = Some(true);
                            changed = true;
                        }
                    });
            });

            if changed {
                self.recalculate_size_estimate();
            }

            ui.separator();

            // 4. Save as Custom Preset Section
            if self.show_save_preset {
                ui.horizontal(|ui| {
                    ui.label("Preset Name:");
                    ui.text_edit_singleline(&mut self.preset_name_input);
                    if ui.button("Save").clicked() && !self.preset_name_input.trim().is_empty() {
                        let id = self
                            .preset_name_input
                            .to_lowercase()
                            .replace(' ', "-")
                            .replace(|c: char| !c.is_alphanumeric() && c != '-', "");
                        let preset = PresetDefinition {
                            schema_version: "1.0".to_string(),
                            id,
                            name: self.preset_name_input.trim().to_string(),
                            description: format!(
                                "Custom tuned preset for {} conversion",
                                self.target_format
                            ),
                            category: "Custom".to_string(),
                            input_formats: vec!["*".to_string()],
                            output_format: self.target_format.clone(),
                            options: self.options.clone(),
                            icon: "sliders".to_string(),
                            author: "User".to_string(),
                            tags: vec!["custom".to_string()],
                            is_builtin: false,
                        };
                        if let Ok(p) = save_custom_preset(&preset) {
                            self.preset_saved_toast = Some(format!(
                                "Saved preset to {:?}",
                                p.file_name().unwrap_or_default()
                            ));
                            self.show_save_preset = false;
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        self.show_save_preset = false;
                    }
                });
            } else if ui.button("+ Save Parameters as New Preset...").clicked() {
                self.show_save_preset = true;
            }

            if let Some(ref toast) = self.preset_saved_toast {
                ui.label(
                    RichText::new(toast)
                        .small()
                        .color(Color32::from_rgb(100, 255, 100)),
                );
            }

            ui.add_space(8.0);

            // 5. Action Buttons (Cancel / Convert Now)
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    close_dialog = true;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let conv_btn = ui.add(
                        egui::Button::new(
                            RichText::new("  Convert Now  ")
                                .strong()
                                .color(Color32::BLACK),
                        )
                        .fill(Color32::from_rgb(90, 220, 120)),
                    );
                    if conv_btn.clicked() {
                        convert_triggered = true;
                        close_dialog = true;
                    }
                });
            });
        });

        if close_dialog {
            self.is_open = false;
        }

        if convert_triggered {
            Some(self.options.clone())
        } else {
            None
        }
    }
}
