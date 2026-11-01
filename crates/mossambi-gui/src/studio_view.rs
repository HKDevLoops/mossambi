//! 100% Apple macOS-Styled Mossambi Studio GUI Application
//! Features:
//! - macOS window chrome with authentic traffic lights (Close, Minimize, Zoom)
//! - Translucent frosted sidebar with pill navigation items
//! - Theme Selector: Liquid Glass, Transparent Frosted Glass, Dark Anthracite, Light Aqua
//! - Universal Transcoder & 200+ format matrix
//! - Indian Exam Form Studio (USP: 500x300 px, 200 DPI, 100-500 KB envelope, DirectML scan whitening)
//! - Archive suite & Zip-Slip protection
//! - Hardware & AI telemetry (NVENC, AMF, QSV, DirectML NPU)
//! - Certified Pantone Citrus Palette tokens

use crate::macos_theme::*;
use crate::quick_settings::QuickSettingsState;
use eframe::egui::{self, Color32, Pos2, Rect, Rounding, Stroke, Ui, Vec2, ViewportCommand};
use mossambi_core::UniversalRegistry;
use mossambi_exam::{optimize_exam_media, ExamConstraint};
use mossambi_hwaccel::detect_hardware_capabilities;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StudioTab {
    Transcoder,
    ExamStudio,
    ArchiveSuite,
    HardwareTelemetry,
    Preferences,
}

pub struct StudioView {
    pub current_tab: StudioTab,
    pub exam_status: String,
    pub last_exam_output: Option<PathBuf>,
}

impl Default for StudioView {
    fn default() -> Self {
        Self {
            current_tab: StudioTab::Transcoder,
            exam_status: "Ready to optimize for portal boundaries.".to_string(),
            last_exam_output: None,
        }
    }
}

impl StudioView {
    /// Renders the complete macOS Studio window with sidebar and content area.
    pub fn render(
        &mut self,
        ui: &mut Ui,
        ctx: &egui::Context,
        current_theme: &mut ThemeStyle,
        current_file: &Option<PathBuf>,
        quick_settings: &mut QuickSettingsState,
        on_pick_file: impl FnOnce(),
        on_convert: impl FnMut(String),
        on_open_folder: impl FnMut(&Path),
        on_switch_to_widget: impl FnOnce(),
    ) {
        let palette = current_theme.colors();
        let full_rect = ui.available_rect_before_wrap();
        let mut req_switch_to_widget = false;

        // 1. macOS Window Backdrop
        ui.painter().rect_filled(full_rect, Rounding::same(12.0), palette.window_bg);
        let border_stroke = if palette.is_translucent {
            Stroke::new(1.2_f32, palette.border)
        } else {
            Stroke::new(1.0_f32, palette.border)
        };
        ui.painter().rect_stroke(full_rect, Rounding::same(12.0), border_stroke);

        // Specular highlight line along top edge for Liquid Glass
        if palette.is_translucent {
            let top_line_start = Pos2::new(full_rect.min.x + 12.0, full_rect.min.y + 0.5);
            let top_line_end = Pos2::new(full_rect.max.x - 12.0, full_rect.min.y + 0.5);
            ui.painter().line_segment([top_line_start, top_line_end], Stroke::new(1.0_f32, palette.specular));
        }

        ui.horizontal(|ui| {
            // 2. Left Sidebar (Width 220px)
            let sidebar_rect = Rect::from_min_size(full_rect.min, Vec2::new(220.0, full_rect.height()));

            let mut sidebar_ui = ui.new_child(egui::UiBuilder::new().max_rect(sidebar_rect.shrink(14.0)));
            sidebar_ui.painter().rect_filled(sidebar_rect, Rounding::ZERO, palette.sidebar_bg);
            sidebar_ui.painter().line_segment(
                [Pos2::new(sidebar_rect.max.x, sidebar_rect.min.y), Pos2::new(sidebar_rect.max.x, sidebar_rect.max.y)],
                Stroke::new(1.0_f32, palette.divider),
            );

            // Top Traffic Lights
            render_traffic_lights(
                &mut sidebar_ui,
                || ctx.send_viewport_cmd(ViewportCommand::Close),
                || ctx.send_viewport_cmd(ViewportCommand::Minimized(true)),
                || req_switch_to_widget = true,
            );

            sidebar_ui.add_space(18.0);

            // Brand Header with Pantone 376 C
            sidebar_ui.horizontal(|ui| {
                ui.label(egui::RichText::new("🍋").size(18.0));
                ui.label(
                    egui::RichText::new("Mossambi")
                        .size(16.0)
                        .strong()
                        .color(PANTONE_FLAVEDO),
                );
                ui.label(
                    egui::RichText::new("v2.2")
                        .size(10.0)
                        .color(palette.text_muted),
                );
            });

            sidebar_ui.add_space(16.0);
            sidebar_ui.separator();
            sidebar_ui.add_space(12.0);

            // Sidebar Navigation Pills
            if render_sidebar_item(&mut sidebar_ui, &palette, "Transcoder", "⚡", self.current_tab == StudioTab::Transcoder) {
                self.current_tab = StudioTab::Transcoder;
            }
            sidebar_ui.add_space(2.0);

            if render_sidebar_item(&mut sidebar_ui, &palette, "Exam Studio", "🇮🇳", self.current_tab == StudioTab::ExamStudio) {
                self.current_tab = StudioTab::ExamStudio;
                quick_settings.exam_mode_enabled = true;
            }
            sidebar_ui.add_space(2.0);

            if render_sidebar_item(&mut sidebar_ui, &palette, "Archive Suite", "📦", self.current_tab == StudioTab::ArchiveSuite) {
                self.current_tab = StudioTab::ArchiveSuite;
            }
            sidebar_ui.add_space(2.0);

            if render_sidebar_item(&mut sidebar_ui, &palette, "Hardware & AI", "🏎️", self.current_tab == StudioTab::HardwareTelemetry) {
                self.current_tab = StudioTab::HardwareTelemetry;
            }
            sidebar_ui.add_space(2.0);

            if render_sidebar_item(&mut sidebar_ui, &palette, "Preferences", "⚙️", self.current_tab == StudioTab::Preferences) {
                self.current_tab = StudioTab::Preferences;
            }

            sidebar_ui.add_space(28.0);
            sidebar_ui.separator();
            sidebar_ui.add_space(10.0);

            // Quick Switch to Floating Widget
            if sidebar_ui.button(egui::RichText::new("⮌ Radial Citrus Wheel").size(11.0).color(PANTONE_GOLD)).clicked() {
                req_switch_to_widget = true;
            }

            // 3. Right Content Area (Remaining Width)
            let content_rect = Rect::from_min_size(
                Pos2::new(sidebar_rect.max.x + 1.0, full_rect.min.y),
                Vec2::new(full_rect.width() - 221.0, full_rect.height()),
            );

            let mut content_ui = ui.new_child(egui::UiBuilder::new().max_rect(content_rect.shrink(20.0)));
            match self.current_tab {
                StudioTab::Transcoder => {
                    self.render_transcoder_tab(&mut content_ui, &palette, current_file, quick_settings, on_pick_file, on_convert);
                }
                StudioTab::ExamStudio => {
                    self.render_exam_studio_tab(&mut content_ui, &palette, current_file, quick_settings, on_pick_file, on_open_folder);
                }
                StudioTab::ArchiveSuite => {
                    self.render_archive_tab(&mut content_ui, &palette, current_file, quick_settings, on_pick_file, on_convert);
                }
                StudioTab::HardwareTelemetry => {
                    self.render_telemetry_tab(&mut content_ui, &palette);
                }
                StudioTab::Preferences => {
                    self.render_preferences_tab(&mut content_ui, &palette, current_theme, quick_settings);
                }
            }
        });

        if req_switch_to_widget {
            on_switch_to_widget();
        }
    }

    /// Tab 1: Universal Transcoder & 200+ Format Matrix
    fn render_transcoder_tab(
        &mut self,
        ui: &mut Ui,
        palette: &ThemePalette,
        current_file: &Option<PathBuf>,
        settings: &mut QuickSettingsState,
        on_pick_file: impl FnOnce(),
        mut on_convert: impl FnMut(String),
    ) {
        ui.label(egui::RichText::new("Universal File Transcoder").size(18.0).strong().color(palette.text_primary));
        ui.label(egui::RichText::new("0.00% Zero-Loss Standard • 200+ Formats • Multi-Threaded Rayon Concurrency").size(11.5).color(palette.text_secondary));
        ui.add_space(12.0);

        // Staged File Card
        let card_frame = egui::Frame::none()
            .fill(palette.card_bg)
            .stroke(Stroke::new(1.0_f32, palette.border))
            .rounding(Rounding::same(10.0))
            .inner_margin(egui::Margin::same(14.0));

        card_frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                if let Some(ref path) = current_file {
                    let (cat, ext) = UniversalRegistry::classify_file(path);
                    let meta = std::fs::metadata(path).ok();
                    let size_kb = meta.map(|m| m.len() as f64 / 1024.0).unwrap_or(0.0);

                    ui.label(egui::RichText::new("📄").size(24.0));
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new(path.file_name().unwrap_or_default().to_string_lossy()).size(13.5).strong().color(palette.text_primary));
                        ui.label(egui::RichText::new(format!("Category: {:?} | Extension: .{} | Size: {:.1} KB", cat, ext, size_kb)).size(11.0).color(palette.text_muted));
                    });
                } else {
                    ui.label(egui::RichText::new("📥").size(24.0));
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("No File Staged").size(13.5).strong().color(palette.text_primary));
                        ui.label(egui::RichText::new("Drag and drop any file from Explorer or click Browse").size(11.0).color(palette.text_muted));
                    });
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(egui::RichText::new("📁 Browse File").size(12.0)).clicked() {
                        on_pick_file();
                    }
                });
            });
        });

        ui.add_space(16.0);

        // Target Formats Grid
        if let Some(ref path) = current_file {
            let (cat, in_ext) = UniversalRegistry::classify_file(path);
            let available = UniversalRegistry::get_all_supported_outputs(cat);

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Select Target Output Format:").size(13.0).strong().color(palette.text_primary));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Golden ALL Button
                    let all_btn = ui.add(
                        egui::Button::new(egui::RichText::new("⭐ Convert to ALL Formats").size(11.5).strong().color(Color32::from_rgb(50, 35, 0)))
                            .fill(PANTONE_GOLD)
                            .rounding(Rounding::same(8.0)),
                    );
                    if all_btn.clicked() {
                        on_convert("ALL".to_string());
                        return;
                    }
                });
            });

            ui.add_space(8.0);

            // Format Chips
            ui.horizontal_wrapped(|ui| {
                for fmt in available {
                    if fmt == in_ext.as_str() {
                        continue;
                    }
                    let is_sel = settings.active_format == fmt;
                    let chip = ui.add(
                        egui::Button::new(egui::RichText::new(fmt).size(11.5).color(if is_sel { Color32::WHITE } else { palette.text_secondary }))
                            .fill(if is_sel { PANTONE_FLAVEDO } else { palette.card_bg })
                            .stroke(Stroke::new(1.0_f32, if is_sel { PANTONE_FLAVEDO } else { palette.border }))
                            .rounding(Rounding::same(6.0)),
                    );
                    if chip.clicked() {
                        settings.set_active_format(fmt);
                    }
                }
            });

            ui.add_space(16.0);

            // Quick Parameters Panel for Selected Format
            let fmt_box = egui::Frame::none()
                .fill(palette.card_bg)
                .stroke(Stroke::new(1.0_f32, palette.border))
                .rounding(Rounding::same(8.0))
                .inner_margin(egui::Margin::same(12.0));

            fmt_box.show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(format!("Quick Tuning for {}:", settings.active_format)).strong().color(PANTONE_FLAVEDO));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let convert_btn = ui.add(
                            egui::Button::new(egui::RichText::new(format!("Convert to {} ➔", settings.active_format)).size(12.5).strong().color(Color32::WHITE))
                                .fill(PANTONE_FLAVEDO)
                                .rounding(Rounding::same(8.0)),
                        );
                        if convert_btn.clicked() {
                            on_convert(settings.active_format.clone());
                        }
                    });
                });

                ui.add_space(6.0);
                match settings.active_format.as_str() {
                    "PDF" => {
                        ui.horizontal(|ui| {
                            ui.label("Page Size:");
                            ui.selectable_value(&mut settings.pdf_page_size, "A4".to_string(), "A4");
                            ui.selectable_value(&mut settings.pdf_page_size, "Letter".to_string(), "Letter");
                            ui.add_space(16.0);
                            ui.label("Font Size:");
                            ui.add(egui::Slider::new(&mut settings.pdf_font_size, 8.0..=18.0).suffix(" pt"));
                        });
                    }
                    "MP3" | "AAC" | "OGG" => {
                        ui.horizontal(|ui| {
                            ui.label("Bitrate:");
                            ui.selectable_value(&mut settings.audio_bitrate, "128k".to_string(), "128 kbps");
                            ui.selectable_value(&mut settings.audio_bitrate, "192k".to_string(), "192 kbps");
                            ui.selectable_value(&mut settings.audio_bitrate, "320k".to_string(), "320 kbps (Max)");
                        });
                    }
                    "JPG" | "WEBP" | "AVIF" => {
                        ui.horizontal(|ui| {
                            ui.label("Quality:");
                            ui.add(egui::Slider::new(&mut settings.image_quality, 50..=100).suffix("%"));
                            ui.checkbox(&mut settings.image_lossless, "Strict Lossless (Bit-Exact)");
                        });
                    }
                    _ => {
                        ui.label(egui::RichText::new("Lossless stream conversion active.").size(11.0).color(palette.text_muted));
                    }
                }
            });
        }
    }

    /// Tab 2: Indian Competitive Exam & Recruitment Portal Form Tuner
    fn render_exam_studio_tab(
        &mut self,
        ui: &mut Ui,
        palette: &ThemePalette,
        current_file: &Option<PathBuf>,
        settings: &mut QuickSettingsState,
        on_pick_file: impl FnOnce(),
        mut on_open_folder: impl FnMut(&Path),
    ) {
        ui.label(egui::RichText::new("🇮🇳 Indian Competitive Exam & Recruitment Form Studio").size(18.0).strong().color(palette.text_primary));
        ui.label(
            egui::RichText::new("Solves rejection errors on UPSC, SSC, IBPS, GATE, and State PSC application portals with micro-converged binary search bounds.")
                .size(11.5)
                .color(palette.text_secondary),
        );
        ui.add_space(12.0);

        // Preset Chips
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Official Exam Presets:").size(12.0).strong().color(palette.text_primary));
            if ui.button(egui::RichText::new("UPSC Photo (350x450, 300DPI, 20-100KB)").size(11.0)).clicked() {
                settings.apply_exam_preset(&ExamConstraint::preset_upsc_photo());
            }
            if ui.button(egui::RichText::new("UPSC Sign (350x150, 300DPI, 10-40KB)").size(11.0)).clicked() {
                settings.apply_exam_preset(&ExamConstraint::preset_upsc_signature());
            }
            if ui.button(egui::RichText::new("SSC CGL (100x120, 20-50KB)").size(11.0)).clicked() {
                settings.apply_exam_preset(&ExamConstraint::preset_ssc_photo());
            }
            if ui.button(egui::RichText::new("IBPS Bank (200x230, 20-50KB)").size(11.0)).clicked() {
                settings.apply_exam_preset(&ExamConstraint::preset_ibps_photo());
            }
        });

        ui.add_space(10.0);

        // Tuning Envelope Card
        let envelope_frame = egui::Frame::none()
            .fill(palette.card_bg)
            .stroke(Stroke::new(1.0_f32, palette.border))
            .rounding(Rounding::same(10.0))
            .inner_margin(egui::Margin::same(16.0));

        envelope_frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Target Envelope Constraints").strong().color(PANTONE_FLAVEDO));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(format!("Active Preset: {}", settings.exam_preset_name)).size(11.0).color(palette.text_muted));
                });
            });

            ui.add_space(8.0);

            ui.columns(2, |cols| {
                cols[0].vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("Width (px):");
                        ui.add(egui::DragValue::new(&mut settings.exam_width).speed(1));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Height (px):");
                        ui.add(egui::DragValue::new(&mut settings.exam_height).speed(1));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Calibrated DPI:");
                        ui.add(egui::DragValue::new(&mut settings.exam_dpi).speed(1));
                    });
                });

                cols[1].vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("Min Size (KB):");
                        ui.add(egui::DragValue::new(&mut settings.exam_min_kb).speed(1));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Max Size (KB):");
                        ui.add(egui::DragValue::new(&mut settings.exam_max_kb).speed(1));
                    });
                    ui.checkbox(&mut settings.exam_whiten, "Whiten Scanner Shadows (DirectML)");
                });
            });

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);

            // Execute Exam Optimization
            ui.horizontal(|ui| {
                if let Some(ref path) = current_file {
                    let optimize_btn = ui.add(
                        egui::Button::new(
                            egui::RichText::new("🎯 Optimize & Generate Portal-Ready JPG")
                                .size(13.0)
                                .strong()
                                .color(Color32::WHITE),
                        )
                        .fill(PANTONE_FLAVEDO)
                        .rounding(Rounding::same(8.0)),
                    );

                    if optimize_btn.clicked() {
                        let constraint = ExamConstraint {
                            name: settings.exam_preset_name.clone(),
                            target_format: "JPG".to_string(),
                            min_bytes: settings.exam_min_kb * 1024,
                            max_bytes: settings.exam_max_kb * 1024,
                            width_px: settings.exam_width,
                            height_px: settings.exam_height,
                            dpi: settings.exam_dpi,
                            normalize_white_balance: settings.exam_whiten,
                        };

                        let parent = path.parent().unwrap_or_else(|| Path::new("."));
                        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("exam_output");
                        let out_file = parent.join(format!("{}_portal_ready.jpg", stem));

                        match optimize_exam_media(path, &out_file, &constraint) {
                            Ok(res) => {
                                let len = std::fs::metadata(&res).map(|m| m.len()).unwrap_or(0);
                                self.exam_status = format!(
                                    "✨ PASS: Generated {} ({:.1} KB) fitting [{} - {} KB] @ {} DPI",
                                    res.file_name().unwrap_or_default().to_string_lossy(),
                                    len as f64 / 1024.0,
                                    settings.exam_min_kb,
                                    settings.exam_max_kb,
                                    settings.exam_dpi
                                );
                                self.last_exam_output = Some(res);
                            }
                            Err(e) => {
                                self.exam_status = format!("❌ Exam optimization failed: {}", e);
                            }
                        }
                    }

                    if let Some(ref out) = self.last_exam_output {
                        if ui.button(egui::RichText::new("📂 Open in Explorer").size(12.0)).clicked() {
                            on_open_folder(out);
                        }
                    }
                } else {
                    ui.label(egui::RichText::new("Drag and drop photo or signature file to begin.").color(palette.text_muted));
                    if ui.button("📁 Select Photo").clicked() {
                        on_pick_file();
                    }
                }
            });

            ui.add_space(8.0);
            ui.label(egui::RichText::new(&self.exam_status).size(11.5).color(PANTONE_GOLD));
        });
    }

    /// Tab 3: Archive Suite & Safe Extraction
    fn render_archive_tab(
        &mut self,
        ui: &mut Ui,
        palette: &ThemePalette,
        current_file: &Option<PathBuf>,
        _settings: &mut QuickSettingsState,
        on_pick_file: impl FnOnce(),
        mut on_convert: impl FnMut(String),
    ) {
        ui.label(egui::RichText::new("Archive Suite & Compression").size(18.0).strong().color(palette.text_primary));
        ui.label(egui::RichText::new("Zip-Slip Traversal Protection • Multi-Threaded Deflate / Zstandard").size(11.5).color(palette.text_secondary));
        ui.add_space(14.0);

        if let Some(ref path) = current_file {
            ui.label(format!("Target Input: {}", path.display()));
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                if ui.button(egui::RichText::new("📦 Pack to ZIP").size(12.0)).clicked() {
                    on_convert("ZIP".to_string());
                }
                if ui.button(egui::RichText::new("📦 Pack to TAR.GZ").size(12.0)).clicked() {
                    on_convert("TAR.GZ".to_string());
                }
                if ui.button(egui::RichText::new("📦 Pack to 7Z").size(12.0)).clicked() {
                    on_convert("7Z".to_string());
                }
                if ui.button(egui::RichText::new("📂 Safe Extract").size(12.0).color(PANTONE_FLAVEDO)).clicked() {
                    on_convert("EXTRACT".to_string());
                }
            });
        } else {
            ui.label(egui::RichText::new("Select a file or folder to compress or extract.").color(palette.text_muted));
            if ui.button("📁 Select Input").clicked() {
                on_pick_file();
            }
        }
    }

    /// Tab 4: Hardware & AI Telemetry
    fn render_telemetry_tab(&mut self, ui: &mut Ui, palette: &ThemePalette) {
        let hw = detect_hardware_capabilities();
        ui.label(egui::RichText::new("Hardware Acceleration & Neural Telemetry").size(18.0).strong().color(palette.text_primary));
        ui.label(egui::RichText::new("GPU Encoder Pipelines • Windows DirectML • Multi-Core Concurrency").size(11.5).color(palette.text_secondary));
        ui.add_space(14.0);

        let card_frame = egui::Frame::none()
            .fill(palette.card_bg)
            .stroke(Stroke::new(1.0_f32, palette.border))
            .rounding(Rounding::same(10.0))
            .inner_margin(egui::Margin::same(16.0));

        card_frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("GPU Transcoder:").size(13.0).strong().color(PANTONE_FLAVEDO));
                ui.label(format!("{:?} (H.264: {} | HEVC: {})", hw.vendor, hw.h264_encoder, hw.hevc_encoder));
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("NPU AI Accelerator:").size(13.0).strong().color(PANTONE_FLAVEDO));
                ui.label(format!("{} (Active)", hw.npu_name));
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("CPU Concurrency:").size(13.0).strong().color(PANTONE_FLAVEDO));
                ui.label(format!("{} Concurrent Rayon Worker Threads", hw.optimal_thread_count));
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("SIMD Vector Units:").size(13.0).strong().color(PANTONE_FLAVEDO));
                ui.label(format!("AVX2: {} | NEON: {}", if hw.supports_avx2_simd { "Active" } else { "Off" }, if hw.supports_neon_simd { "Active" } else { "Off" }));
            });
        });
    }

    /// Tab 5: Preferences & Certified Pantone Swatches & Themes
    fn render_preferences_tab(
        &mut self,
        ui: &mut Ui,
        palette: &ThemePalette,
        current_theme: &mut ThemeStyle,
        _settings: &mut QuickSettingsState,
    ) {
        ui.label(egui::RichText::new("Preferences & Appearance").size(18.0).strong().color(palette.text_primary));
        ui.label(egui::RichText::new("macOS Sequoia Themes, Gestures, and Certified Pantone Swatches").size(11.5).color(palette.text_secondary));
        ui.add_space(14.0);

        // 1. Theme Selector Card
        let theme_frame = egui::Frame::none()
            .fill(palette.card_bg)
            .stroke(Stroke::new(1.0_f32, palette.border))
            .rounding(Rounding::same(10.0))
            .inner_margin(egui::Margin::same(16.0));

        theme_frame.show(ui, |ui| {
            ui.label(egui::RichText::new("macOS Window Theme & Aesthetics").strong().color(palette.text_primary));
            ui.label(egui::RichText::new("Choose your appearance across the GUI Studio, Radial Citrus Widget, and System Tray:").size(11.5).color(palette.text_muted));
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                for theme_opt in ThemeStyle::all() {
                    let is_active = *current_theme == *theme_opt;
                    let btn = ui.add(
                        egui::Button::new(
                            egui::RichText::new(theme_opt.name())
                                .size(11.5)
                                .color(if is_active { Color32::WHITE } else { palette.text_secondary }),
                        )
                        .fill(if is_active { PANTONE_FLAVEDO } else { Color32::TRANSPARENT })
                        .stroke(Stroke::new(1.0_f32, if is_active { PANTONE_FLAVEDO } else { palette.border }))
                        .rounding(Rounding::same(6.0)),
                    );
                    if btn.clicked() {
                        *current_theme = *theme_opt;
                    }
                }
            });
        });

        ui.add_space(12.0);

        // 2. Activation Triggers Card
        let trigger_frame = egui::Frame::none()
            .fill(palette.card_bg)
            .stroke(Stroke::new(1.0_f32, palette.border))
            .rounding(Rounding::same(10.0))
            .inner_margin(egui::Margin::same(16.0));

        trigger_frame.show(ui, |ui| {
            ui.label(egui::RichText::new("Global Activation Triggers").strong().color(palette.text_primary));
            ui.label("• 2-Time Circular Motion Gesture: Drag a file and trace 2 circular loops (>=720 deg) to summon the widget with the active file immediately on cursor.");
            ui.label("• Global Hotkey: WIN + CTRL summons the sweet lime radial widget centered under cursor.");
            ui.label("• System Tray: Right-click the citrus tray icon for themes, exam presets, and controls.");
            ui.label("• Long Press: Hold click on any petal (>=350ms) to inspect quick settings.");
        });

        ui.add_space(12.0);

        // 3. Certified Pantone Swatches
        let pantone_frame = egui::Frame::none()
            .fill(palette.card_bg)
            .stroke(Stroke::new(1.0_f32, palette.border))
            .rounding(Rounding::same(10.0))
            .inner_margin(egui::Margin::same(16.0));

        pantone_frame.show(ui, |ui| {
            ui.label(egui::RichText::new("Certified Pantone Citrus Palette").strong().color(palette.text_primary));
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("■").color(PANTONE_FLAVEDO).size(16.0));
                ui.label("Pantone 376 C (#78BE20) - Raw Peel Green (Active Accents)");
            });
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("■").color(PANTONE_GOLD).size(16.0));
                ui.label("Pantone 1235 C (#FFB81C) - Citrus Gold (ALL Sector)");
            });
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("■").color(PANTONE_CARPEL).size(16.0));
                ui.label("Pantone 123 C (#FFC72C) - Sweet Lime Carpel Pulp");
            });
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("■").color(PANTONE_CORE).size(16.0));
                ui.label("Pantone 102 C (#FCE300) - Central Core Columella");
            });
        });
    }
}
