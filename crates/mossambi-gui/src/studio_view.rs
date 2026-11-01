//! 100% Apple macOS-Styled Mossambi Studio GUI Application
//! Features:
//! - macOS window chrome with authentic traffic lights (Close, Minimize, Zoom)
//! - Frosted sidebar with pill navigation items
//! - Universal Transcoder & 200+ format matrix
//! - Indian Exam Form Studio (USP: 500x300 px, 200 DPI, 100-500 KB envelope, DirectML scan whitening)
//! - Archive suite & Zip-Slip protection
//! - Hardware & AI telemetry (NVENC, AMF, QSV, DirectML NPU)
//! - Pantone palette customizer

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
        current_file: &Option<PathBuf>,
        quick_settings: &mut QuickSettingsState,
        on_pick_file: impl FnOnce(),
        on_convert: impl FnMut(String),
        on_open_folder: impl FnMut(&Path),
        on_switch_to_widget: impl FnOnce(),
    ) {
        let full_rect = ui.available_rect_before_wrap();
        let mut req_switch_to_widget = false;

        // 1. macOS Window Backdrop
        ui.painter().rect_filled(full_rect, Rounding::same(12.0), MAC_WINDOW_BG);
        ui.painter().rect_stroke(full_rect, Rounding::same(12.0), Stroke::new(1.0_f32, MAC_BORDER));

        ui.horizontal(|ui| {
            // 2. Left Sidebar (Width 220px)
            let sidebar_rect = Rect::from_min_size(full_rect.min, Vec2::new(220.0, full_rect.height()));

            let mut sidebar_ui = ui.new_child(egui::UiBuilder::new().max_rect(sidebar_rect.shrink(14.0)));
            sidebar_ui.painter().rect_filled(sidebar_rect, Rounding::ZERO, MAC_SIDEBAR_BG);
            sidebar_ui.painter().line_segment(
                [Pos2::new(sidebar_rect.max.x, sidebar_rect.min.y), Pos2::new(sidebar_rect.max.x, sidebar_rect.max.y)],
                Stroke::new(1.0_f32, MAC_DIVIDER),
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
                        .color(MAC_TEXT_MUTED),
                );
            });

            sidebar_ui.add_space(16.0);
            sidebar_ui.separator();
            sidebar_ui.add_space(12.0);

            // Sidebar Navigation Pills
            if render_sidebar_item(&mut sidebar_ui, "Transcoder", "⚡", self.current_tab == StudioTab::Transcoder) {
                self.current_tab = StudioTab::Transcoder;
            }
            sidebar_ui.add_space(2.0);

            if render_sidebar_item(&mut sidebar_ui, "Exam Studio", "🇮🇳", self.current_tab == StudioTab::ExamStudio) {
                self.current_tab = StudioTab::ExamStudio;
                quick_settings.exam_mode_enabled = true;
            }
            sidebar_ui.add_space(2.0);

            if render_sidebar_item(&mut sidebar_ui, "Archive Suite", "📦", self.current_tab == StudioTab::ArchiveSuite) {
                self.current_tab = StudioTab::ArchiveSuite;
            }
            sidebar_ui.add_space(2.0);

            if render_sidebar_item(&mut sidebar_ui, "Hardware & AI", "🏎️", self.current_tab == StudioTab::HardwareTelemetry) {
                self.current_tab = StudioTab::HardwareTelemetry;
            }
            sidebar_ui.add_space(2.0);

            if render_sidebar_item(&mut sidebar_ui, "Preferences", "⚙️", self.current_tab == StudioTab::Preferences) {
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
                    self.render_transcoder_tab(&mut content_ui, current_file, quick_settings, on_pick_file, on_convert);
                }
                StudioTab::ExamStudio => {
                    self.render_exam_studio_tab(&mut content_ui, current_file, quick_settings, on_pick_file, on_open_folder);
                }
                StudioTab::ArchiveSuite => {
                    self.render_archive_tab(&mut content_ui, current_file, quick_settings, on_pick_file, on_convert);
                }
                StudioTab::HardwareTelemetry => {
                    self.render_telemetry_tab(&mut content_ui);
                }
                StudioTab::Preferences => {
                    self.render_preferences_tab(&mut content_ui, quick_settings);
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
        current_file: &Option<PathBuf>,
        settings: &mut QuickSettingsState,
        on_pick_file: impl FnOnce(),
        mut on_convert: impl FnMut(String),
    ) {
        ui.label(egui::RichText::new("Universal File Transcoder").size(18.0).strong().color(MAC_TEXT_PRIMARY));
        ui.label(egui::RichText::new("0.00% Zero-Loss Standard • 200+ Formats • Multi-Threaded Rayon Concurrency").size(11.5).color(MAC_TEXT_SECONDARY));
        ui.add_space(12.0);

        // Staged File Card
        let card_frame = egui::Frame::none()
            .fill(MAC_CARD_BG)
            .stroke(Stroke::new(1.0_f32, MAC_BORDER))
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
                        ui.label(egui::RichText::new(path.file_name().unwrap_or_default().to_string_lossy()).size(13.5).strong().color(MAC_TEXT_PRIMARY));
                        ui.label(egui::RichText::new(format!("Category: {:?} | Extension: .{} | Size: {:.1} KB", cat, ext, size_kb)).size(11.0).color(MAC_TEXT_MUTED));
                    });
                } else {
                    ui.label(egui::RichText::new("📥").size(24.0));
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("No File Staged").size(13.5).strong().color(MAC_TEXT_PRIMARY));
                        ui.label(egui::RichText::new("Drag and drop any file from Explorer or click Browse").size(11.0).color(MAC_TEXT_MUTED));
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
                ui.label(egui::RichText::new("Select Target Output Format:").size(13.0).strong().color(MAC_TEXT_PRIMARY));
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
                        egui::Button::new(egui::RichText::new(fmt).size(11.5).color(if is_sel { Color32::WHITE } else { MAC_TEXT_SECONDARY }))
                            .fill(if is_sel { PANTONE_FLAVEDO } else { MAC_CARD_BG })
                            .stroke(Stroke::new(1.0_f32, if is_sel { PANTONE_FLAVEDO } else { MAC_BORDER }))
                            .rounding(Rounding::same(6.0)),
                    );
                    if chip.clicked() {
                        settings.set_active_format(fmt);
                    }
                }
            });

            ui.add_space(16.0);

            // Action Card
            let action_fmt = settings.active_format.clone();
            let action_card = egui::Frame::none()
                .fill(Color32::from_rgb(26, 32, 24))
                .stroke(Stroke::new(1.0_f32, PANTONE_FLAVEDO))
                .rounding(Rounding::same(10.0))
                .inner_margin(egui::Margin::same(14.0));

            action_card.show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("Target: {}", action_fmt))
                            .size(14.0)
                            .strong()
                            .color(PANTONE_FLAVEDO),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let btn = ui.add(
                            egui::Button::new(
                                egui::RichText::new(format!("⚡ Start Conversion to {}", action_fmt))
                                    .size(13.0)
                                    .strong()
                                    .color(Color32::from_rgb(20, 30, 15)),
                            )
                            .fill(PANTONE_FLAVEDO)
                            .rounding(Rounding::same(8.0)),
                        );
                        if btn.clicked() {
                            on_convert(action_fmt);
                        }
                    });
                });
            });
        } else {
            ui.label(egui::RichText::new("Drop a file above to view domain converter matrix.").color(MAC_TEXT_MUTED));
        }
    }

    /// Tab 2: Indian Competitive Exam & Job Application Form Studio (USP)
    fn render_exam_studio_tab(
        &mut self,
        ui: &mut Ui,
        current_file: &Option<PathBuf>,
        settings: &mut QuickSettingsState,
        on_pick_file: impl FnOnce(),
        mut on_open_folder: impl FnMut(&Path),
    ) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("🇮🇳 Indian Exam Form Studio").size(18.0).strong().color(PANTONE_FLAVEDO));
            ui.label(egui::RichText::new("UPSC • SSC • IBPS • State PSC Portals").size(11.0).color(MAC_TEXT_MUTED));
        });
        ui.label(
            egui::RichText::new("Ensures 100% boundary compliance for DPI, exact pixel dimensions, and strict byte-envelope constraints.")
                .size(11.5)
                .color(MAC_TEXT_SECONDARY),
        );
        ui.add_space(10.0);

        // Preset Selector Card
        let frame = egui::Frame::none()
            .fill(MAC_CARD_BG)
            .stroke(Stroke::new(1.0_f32, MAC_BORDER))
            .rounding(Rounding::same(10.0))
            .inner_margin(egui::Margin::same(14.0));

        frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Exam Preset:").size(12.5).strong().color(MAC_TEXT_PRIMARY));
                egui::ComboBox::from_label("")
                    .selected_text(&settings.exam_preset_name)
                    .show_ui(ui, |ui| {
                        if ui.selectable_value(&mut settings.exam_preset_name, "UPSC Photo".to_string(), "UPSC Photo (350x350, 200 DPI, 20-300 KB)").clicked() {
                            settings.exam_width = 350; settings.exam_height = 350; settings.exam_dpi = 200; settings.exam_min_kb = 20; settings.exam_max_kb = 300;
                        }
                        if ui.selectable_value(&mut settings.exam_preset_name, "UPSC Signature".to_string(), "UPSC Signature (350x350, 200 DPI, 10-40 KB)").clicked() {
                            settings.exam_width = 350; settings.exam_height = 350; settings.exam_dpi = 200; settings.exam_min_kb = 10; settings.exam_max_kb = 40;
                        }
                        if ui.selectable_value(&mut settings.exam_preset_name, "SSC Photo".to_string(), "SSC Photo (140x190, 200 DPI, 20-50 KB)").clicked() {
                            settings.exam_width = 140; settings.exam_height = 190; settings.exam_dpi = 200; settings.exam_min_kb = 20; settings.exam_max_kb = 50;
                        }
                        if ui.selectable_value(&mut settings.exam_preset_name, "SSC Signature".to_string(), "SSC Signature (140x60, 200 DPI, 10-20 KB)").clicked() {
                            settings.exam_width = 140; settings.exam_height = 60; settings.exam_dpi = 200; settings.exam_min_kb = 10; settings.exam_max_kb = 20;
                        }
                        if ui.selectable_value(&mut settings.exam_preset_name, "IBPS Photo".to_string(), "IBPS Photo (200x230, 200 DPI, 20-50 KB)").clicked() {
                            settings.exam_width = 200; settings.exam_height = 230; settings.exam_dpi = 200; settings.exam_min_kb = 20; settings.exam_max_kb = 50;
                        }
                        if ui.selectable_value(&mut settings.exam_preset_name, "IBPS Signature".to_string(), "IBPS Signature (140x60, 200 DPI, 10-20 KB)").clicked() {
                            settings.exam_width = 140; settings.exam_height = 60; settings.exam_dpi = 200; settings.exam_min_kb = 10; settings.exam_max_kb = 20;
                        }
                        ui.selectable_value(&mut settings.exam_preset_name, "Custom Envelope".to_string(), "Custom Specification");
                    });
            });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            // Parameter Grid
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Dimensions:").size(11.5));
                ui.add(egui::DragValue::new(&mut settings.exam_width).prefix("W: ").suffix(" px"));
                ui.add(egui::DragValue::new(&mut settings.exam_height).prefix("H: ").suffix(" px"));

                ui.add_space(16.0);
                ui.label(egui::RichText::new("Target DPI:").size(11.5));
                ui.add(egui::DragValue::new(&mut settings.exam_dpi).suffix(" DPI"));
            });

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Byte Envelope:").size(11.5));
                ui.add(egui::DragValue::new(&mut settings.exam_min_kb).prefix("Min: ").suffix(" KB"));
                ui.add(egui::DragValue::new(&mut settings.exam_max_kb).prefix("Max: ").suffix(" KB"));
            });

            ui.add_space(8.0);
            ui.checkbox(&mut settings.exam_whiten, "Auto-Whiten Background (NPU/DirectML scan normalization)");
        });

        ui.add_space(12.0);

        // Action & Status
        if let Some(ref path) = current_file {
            let p_clone = path.clone();
            let parent = path.parent().unwrap_or_else(|| Path::new("."));
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("exam_output");
            let out_file = parent.join(format!("{}_portal_ready.jpg", stem));

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

            ui.horizontal(|ui| {
                let btn = ui.add(
                    egui::Button::new(
                        egui::RichText::new("⚡ Generate Portal-Ready Document")
                            .size(13.0)
                            .strong()
                            .color(Color32::from_rgb(20, 30, 15)),
                    )
                    .fill(PANTONE_FLAVEDO)
                    .rounding(Rounding::same(8.0)),
                );

                if btn.clicked() {
                    match optimize_exam_media(&p_clone, &out_file, &constraint) {
                        Ok(res) => {
                            let meta = std::fs::metadata(&res).ok();
                            let kb = meta.map(|m| m.len() as f64 / 1024.0).unwrap_or(0.0);
                            self.exam_status = format!("✓ Generated: {} ({:.1} KB) within [{}-{} KB] boundary!", res.file_name().unwrap_or_default().to_string_lossy(), kb, settings.exam_min_kb, settings.exam_max_kb);
                            self.last_exam_output = Some(res);
                        }
                        Err(e) => {
                            self.exam_status = format!("❌ Optimization error: {}", e);
                        }
                    }
                }

                if let Some(ref out) = self.last_exam_output {
                    if ui.button(egui::RichText::new("📂 Open in Explorer").size(12.0)).clicked() {
                        on_open_folder(out.parent().unwrap_or_else(|| Path::new(".")));
                    }
                }
            });

            ui.add_space(8.0);
            ui.label(egui::RichText::new(&self.exam_status).size(11.5).color(PANTONE_GOLD));
        } else {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Please choose a scanned document or photograph to tune:").color(MAC_TEXT_MUTED));
                if ui.button("📁 Browse Document").clicked() {
                    on_pick_file();
                }
            });
        }
    }

    /// Tab 3: Archive Suite
    fn render_archive_tab(
        &mut self,
        ui: &mut Ui,
        current_file: &Option<PathBuf>,
        _settings: &mut QuickSettingsState,
        on_pick_file: impl FnOnce(),
        mut on_convert: impl FnMut(String),
    ) {
        ui.label(egui::RichText::new("Archive & Compression Suite").size(18.0).strong().color(MAC_TEXT_PRIMARY));
        ui.label(egui::RichText::new("ZIP • 7Z • TAR • TAR.GZ • TAR.XZ • TAR.ZST with Safe Zip-Slip Protection").size(11.5).color(MAC_TEXT_SECONDARY));
        ui.add_space(14.0);

        if current_file.is_some() {
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
            ui.label(egui::RichText::new("Select a file or folder to compress or extract.").color(MAC_TEXT_MUTED));
            if ui.button("📁 Select Input").clicked() {
                on_pick_file();
            }
        }
    }

    /// Tab 4: Hardware & AI Telemetry
    fn render_telemetry_tab(&mut self, ui: &mut Ui) {
        let hw = detect_hardware_capabilities();
        ui.label(egui::RichText::new("Hardware Acceleration & Neural Telemetry").size(18.0).strong().color(MAC_TEXT_PRIMARY));
        ui.label(egui::RichText::new("GPU Encoder Pipelines • Windows DirectML • Multi-Core Concurrency").size(11.5).color(MAC_TEXT_SECONDARY));
        ui.add_space(14.0);

        let card_frame = egui::Frame::none()
            .fill(MAC_CARD_BG)
            .stroke(Stroke::new(1.0_f32, MAC_BORDER))
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

    /// Tab 5: Preferences & Certified Pantone Swatches
    fn render_preferences_tab(&mut self, ui: &mut Ui, _settings: &mut QuickSettingsState) {
        ui.label(egui::RichText::new("Preferences & Pantone Palette").size(18.0).strong().color(MAC_TEXT_PRIMARY));
        ui.label(egui::RichText::new("Certified Pantone Color Codes & System Shortcuts").size(11.5).color(MAC_TEXT_SECONDARY));
        ui.add_space(14.0);

        let card_frame = egui::Frame::none()
            .fill(MAC_CARD_BG)
            .stroke(Stroke::new(1.0_f32, MAC_BORDER))
            .rounding(Rounding::same(10.0))
            .inner_margin(egui::Margin::same(16.0));

        card_frame.show(ui, |ui| {
            ui.label(egui::RichText::new("Global Activation Triggers").strong().color(MAC_TEXT_PRIMARY));
            ui.label("• Global Hotkey: WIN + CTRL wakes the radial widget centered under cursor.");
            ui.label("• Circle-Drag Gesture: Drag a file and trace a circle (>=300 deg) to summon.");
            ui.label("• Long Press: Hold click on any petal (>=350ms) to inspect quick settings.");

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(12.0);

            ui.label(egui::RichText::new("Certified Pantone Citrus Palette").strong().color(MAC_TEXT_PRIMARY));
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
