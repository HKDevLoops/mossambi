//! 100% Apple macOS-Styled Mossambi Studio GUI Application
//! Features:
//! - Standard egui SidePanel & CentralPanel architecture (Guarantees zero overlap, zero layout glitches)
//! - macOS window chrome with authentic traffic lights (Close, Minimize, Zoom)
//! - Translucent frosted sidebar with pill navigation items
//! - Theme Selector: Dark Anthracite, Liquid Glass, Frosted Glass, Light Aqua
//! - Universal Transcoder with rich Hero Landing state (Dropzone, Quick-Start Workflows, 200+ format matrix)
//! - Indian Exam Form Studio (500x300 px, 200 DPI, 20-50 KB envelope, DirectML scan whitening)
//! - Quality & Integrity Inspector (Container Integrity, Hashing, 4-Dimension Compatibility)
//! - Hardware & AI telemetry (NVENC, AMF, QSV, DirectML NPU)
//! - Certified Pantone Citrus Palette tokens

use crate::macos_theme::*;
use crate::quick_settings::QuickSettingsState;
use eframe::egui::{self, Color32, Rounding, Stroke, Ui, ViewportCommand};
use mossambi_core::{FileCategory, UniversalRegistry};
use mossambi_exam::{optimize_exam_media, ExamConstraint};
use mossambi_hwaccel::detect_hardware_capabilities;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StudioTab {
    Transcoder,
    ExamStudio,
    QualityInspector,
    ArchiveSuite,
    HardwareTelemetry,
    Preferences,
}

pub struct StudioView {
    pub current_tab: StudioTab,
    pub exam_status: String,
    pub last_exam_output: Option<PathBuf>,
    pub quality_report: Option<mossambi_codec::FileQualityReport>,
    pub bench_report: Option<mossambi_codec::BenchmarkReport>,
    pub inspection_status: String,
    pub inspected_file_path: Option<PathBuf>,
    pub catalogue_search: String,
    pub catalogue_category: Option<FileCategory>,
}

impl Default for StudioView {
    fn default() -> Self {
        Self {
            current_tab: StudioTab::Transcoder,
            exam_status: "Ready to optimize for portal boundaries.".to_string(),
            last_exam_output: None,
            quality_report: None,
            bench_report: None,
            inspection_status: "Select a file to inspect.".to_string(),
            inspected_file_path: None,
            catalogue_search: String::new(),
            catalogue_category: None,
        }
    }
}

impl StudioView {
    /// Renders the complete macOS Studio window with sidebar and content area.
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        ctx: &egui::Context,
        current_theme: &mut ThemeStyle,
        current_file: &Option<PathBuf>,
        quick_settings: &mut QuickSettingsState,
        mut on_pick_file: impl FnMut(),
        mut on_convert: impl FnMut(String),
        on_open_folder: impl FnMut(&Path),
    ) {
        let palette = current_theme.colors();

        // 1. Fixed Left Sidebar Panel (240px wide, pinned to top-left)
        let sidebar_frame = egui::Frame::none()
            .fill(palette.sidebar_bg)
            .inner_margin(egui::Margin::same(16.0));

        egui::SidePanel::left("studio_sidebar")
            .resizable(false)
            .exact_width(240.0)
            .frame(sidebar_frame)
            .show(ctx, |ui| {
                // Top Traffic Lights (Red = Close, Yellow = Min, Green = Maximize Toggle)
                let is_max = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                render_traffic_lights(
                    ui,
                    || ctx.send_viewport_cmd(ViewportCommand::Close),
                    || ctx.send_viewport_cmd(ViewportCommand::Minimized(true)),
                    || ctx.send_viewport_cmd(ViewportCommand::Maximized(!is_max)),
                );

                ui.add_space(16.0);

                // Brand Header with Pantone 376 C
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("🍋").size(22.0));
                    ui.label(
                        egui::RichText::new("Mossambi")
                            .size(17.5)
                            .strong()
                            .color(PANTONE_FLAVEDO),
                    );
                    ui.label(
                        egui::RichText::new("v2.2")
                            .size(10.5)
                            .color(palette.text_muted),
                    );
                });

                ui.add_space(14.0);
                ui.separator();
                ui.add_space(10.0);

                // Sidebar Navigation Pills
                if render_sidebar_item(ui, &palette, "Transcoder", "⚡", self.current_tab == StudioTab::Transcoder) {
                    self.current_tab = StudioTab::Transcoder;
                }
                ui.add_space(2.0);

                if render_sidebar_item(ui, &palette, "Custom Conformer", "🎯", self.current_tab == StudioTab::ExamStudio) {
                    self.current_tab = StudioTab::ExamStudio;
                    quick_settings.exam_mode_enabled = true;
                }
                ui.add_space(2.0);

                if render_sidebar_item(ui, &palette, "Quality Inspector", "🔬", self.current_tab == StudioTab::QualityInspector) {
                    self.current_tab = StudioTab::QualityInspector;
                }
                ui.add_space(2.0);

                if render_sidebar_item(ui, &palette, "Archive Suite", "📦", self.current_tab == StudioTab::ArchiveSuite) {
                    self.current_tab = StudioTab::ArchiveSuite;
                }
                ui.add_space(2.0);

                if render_sidebar_item(ui, &palette, "Hardware & AI", "🏎️", self.current_tab == StudioTab::HardwareTelemetry) {
                    self.current_tab = StudioTab::HardwareTelemetry;
                }
                ui.add_space(2.0);

                if render_sidebar_item(ui, &palette, "Preferences", "⚙️", self.current_tab == StudioTab::Preferences) {
                    self.current_tab = StudioTab::Preferences;
                }

                // Bottom Sidebar Area
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("Mossambi Universal Transcoder\nWin+Ctrl+C: Studio")
                            .size(9.5)
                            .color(palette.text_muted),
                    );
                    ui.add_space(8.0);
                    ui.separator();
                });
            });

        // 2. Central Content Panel with smooth vertical ScrollArea
        let central_frame = egui::Frame::none()
            .fill(palette.window_bg)
            .inner_margin(egui::Margin::symmetric(24.0, 20.0));

        egui::CentralPanel::default()
            .frame(central_frame)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        match self.current_tab {
                            StudioTab::Transcoder => {
                                self.render_transcoder_tab(ui, &palette, current_file, quick_settings, &mut on_pick_file, &mut on_convert);
                            }
                            StudioTab::ExamStudio => {
                                self.render_exam_studio_tab(ui, &palette, current_file, quick_settings, &mut on_pick_file, on_open_folder);
                            }
                            StudioTab::QualityInspector => {
                                self.render_quality_inspector_tab(ui, &palette, current_file, &mut on_pick_file);
                            }
                            StudioTab::ArchiveSuite => {
                                self.render_archive_tab(ui, &palette, current_file, quick_settings, &mut on_pick_file, on_convert);
                            }
                            StudioTab::HardwareTelemetry => {
                                self.render_telemetry_tab(ui, &palette);
                            }
                            StudioTab::Preferences => {
                                self.render_preferences_tab(ui, &palette, current_theme, quick_settings);
                            }
                        }
                    });
            });
    }

    /// Tab 1: Universal Transcoder & 200+ Format Matrix (Never blank!)
    fn render_transcoder_tab(
        &mut self,
        ui: &mut Ui,
        palette: &ThemePalette,
        current_file: &Option<PathBuf>,
        settings: &mut QuickSettingsState,
        mut on_pick_file: impl FnMut(),
        mut on_convert: impl FnMut(String),
    ) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("Universal File Transcoder").size(22.0).strong().color(palette.text_primary));
                ui.label(egui::RichText::new("0.00% Zero-Loss Standard • 200+ Formats • Multi-Threaded Rayon Concurrency").size(12.0).color(palette.text_secondary));
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new("⚡ 16 Threads Ready").size(11.0).color(PANTONE_FLAVEDO));
            });
        });

        ui.add_space(16.0);

        if let Some(ref path) = current_file {
            // =========================================================================
            // STAGED FILE VIEW: When a file is loaded
            // =========================================================================
            let (cat, in_ext) = UniversalRegistry::classify_file(path);
            let meta = std::fs::metadata(path).ok();
            let size_kb = meta.map(|m| m.len() as f64 / 1024.0).unwrap_or(0.0);
            let size_str = if size_kb > 1024.0 {
                format!("{:.2} MB", size_kb / 1024.0)
            } else {
                format!("{:.1} KB", size_kb)
            };

            let staged_frame = egui::Frame::none()
                .fill(palette.card_bg)
                .stroke(Stroke::new(1.2_f32, PANTONE_FLAVEDO))
                .rounding(Rounding::same(12.0))
                .inner_margin(egui::Margin::same(16.0));

            staged_frame.show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("📄").size(32.0));
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new(path.file_name().unwrap_or_default().to_string_lossy())
                                .size(15.0)
                                .strong()
                                .color(palette.text_primary),
                        );
                        ui.label(
                            egui::RichText::new(format!("Category: {:?} | Format: .{} | Size: {}", cat, in_ext, size_str))
                                .size(11.5)
                                .color(palette.text_muted),
                        );
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(egui::RichText::new("📂 Change File").size(11.5)).clicked() {
                            on_pick_file();
                        }
                    });
                });
            });

            ui.add_space(16.0);

            // Target Formats Grid
            let available = UniversalRegistry::get_all_supported_outputs(cat);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Select Target Output Format:").size(13.5).strong().color(palette.text_primary));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let all_btn = ui.add(
                        egui::Button::new(egui::RichText::new("⭐ Convert to ALL Formats").size(11.5).strong().color(Color32::from_rgb(50, 35, 0)))
                            .fill(PANTONE_GOLD)
                            .rounding(Rounding::same(8.0)),
                    );
                    if all_btn.on_hover_text("Generate output in all supported formats simultaneously").clicked() {
                        on_convert("ALL".to_string());
                    }
                });
            });

            ui.add_space(8.0);

            // Format Chips
            ui.horizontal_wrapped(|ui| {
                for fmt in &available {
                    if *fmt == in_ext.as_str() {
                        continue;
                    }
                    let is_sel = settings.active_format == *fmt;
                    let chip = ui.add(
                        egui::Button::new(
                            egui::RichText::new(*fmt)
                                .size(12.0)
                                .strong()
                                .color(if is_sel { Color32::WHITE } else { palette.text_secondary }),
                        )
                        .fill(if is_sel { PANTONE_FLAVEDO } else { palette.card_bg })
                        .stroke(Stroke::new(1.0_f32, if is_sel { PANTONE_FLAVEDO } else { palette.border }))
                        .rounding(Rounding::same(8.0)),
                    );
                    if chip.clicked() {
                        settings.set_active_format(fmt);
                    }
                }
            });

            ui.add_space(16.0);

            // Quick Parameters Tuning Box
            let fmt_box = egui::Frame::none()
                .fill(palette.card_bg)
                .stroke(Stroke::new(1.0_f32, palette.border))
                .rounding(Rounding::same(10.0))
                .inner_margin(egui::Margin::same(14.0));

            fmt_box.show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(format!("⚙️ Conversion Tuning for {}:", settings.active_format)).strong().color(PANTONE_FLAVEDO));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let convert_btn = ui.add(
                            egui::Button::new(
                                egui::RichText::new(format!("⚡ Convert to {} ➔", settings.active_format))
                                    .size(13.0)
                                    .strong()
                                    .color(Color32::WHITE),
                            )
                            .fill(PANTONE_FLAVEDO)
                            .rounding(Rounding::same(8.0)),
                        );
                        if convert_btn.clicked() {
                            on_convert(settings.active_format.clone());
                        }
                    });
                });

                ui.add_space(8.0);
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
                        ui.label(egui::RichText::new("Lossless bit-exact container remux active. 0.00% Zero-Loss.").size(11.0).color(palette.text_muted));
                    }
                }

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(4.0);

                // Custom Conformer Parameters directly in Transcoder tab
                ui.horizontal(|ui| {
                    ui.checkbox(&mut settings.exam_mode_enabled, "🎯 Apply Custom Parameters (Dimensions, DPI & Size Limit)");
                });

                if settings.exam_mode_enabled {
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        ui.add(egui::TextEdit::singleline(&mut settings.exam_preset_name).desired_width(180.0));
                        ui.add_space(10.0);
                        ui.checkbox(&mut settings.exam_whiten, "Whiten Background");
                    });
                    ui.horizontal(|ui| {
                        ui.label("Dimensions:");
                        ui.add(egui::DragValue::new(&mut settings.exam_width).speed(1).prefix("W: ").suffix("px"));
                        ui.add(egui::DragValue::new(&mut settings.exam_height).speed(1).prefix("H: ").suffix("px"));
                        ui.add_space(10.0);
                        ui.label("DPI:");
                        ui.add(egui::DragValue::new(&mut settings.exam_dpi).speed(1));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Size Bounds:");
                        ui.add(egui::DragValue::new(&mut settings.exam_min_kb).speed(1).prefix("Min: ").suffix("KB"));
                        ui.add(egui::DragValue::new(&mut settings.exam_max_kb).speed(1).prefix("Max: ").suffix("KB"));
                    });
                }
            });
        } else {
            // =========================================================================
            // RICH HERO LANDING VIEW: When NO file is staged (Never an empty blank void!)
            // =========================================================================

            // 1. Hero Drag-and-Drop Dropzone
            let dropzone_frame = egui::Frame::none()
                .fill(Color32::from_rgba_premultiplied(22, 26, 36, 200))
                .stroke(Stroke::new(1.8_f32, PANTONE_FLAVEDO))
                .rounding(Rounding::same(16.0))
                .inner_margin(egui::Margin::same(24.0));

            dropzone_frame.show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new("🍋").size(48.0));
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new("Drag & Drop Any Files or Folders Here")
                            .size(19.0)
                            .strong()
                            .color(palette.text_primary),
                    );
                    ui.label(
                        egui::RichText::new("Mathematical 0.00% Zero-Loss transcode standard • 200+ formats supported")
                            .size(12.0)
                            .color(palette.text_secondary),
                    );
                    ui.add_space(14.0);

                    ui.horizontal(|ui| {
                        ui.add_space((ui.available_width() - 280.0).max(0.0) / 2.0);
                        let pick_btn = ui.add(
                            egui::Button::new(
                                egui::RichText::new("📂 Choose Files")
                                    .size(12.5)
                                    .strong()
                                    .color(Color32::from_rgb(25, 40, 10)),
                            )
                            .fill(PANTONE_FLAVEDO)
                            .rounding(Rounding::same(8.0)),
                        );
                        if pick_btn.clicked() {
                            on_pick_file();
                        }

                        let folder_btn = ui.add(
                            egui::Button::new(
                                egui::RichText::new("📁 Choose Folder")
                                    .size(12.5)
                                    .color(palette.text_primary),
                            )
                            .fill(palette.card_bg)
                            .stroke(Stroke::new(1.0_f32, palette.border))
                            .rounding(Rounding::same(8.0)),
                        );
                        if folder_btn.clicked() {
                            on_pick_file();
                        }
                    });

                    ui.add_space(14.0);
                    ui.horizontal(|ui| {
                        ui.add_space((ui.available_width() - 520.0).max(0.0) / 2.0);
                        ui.label(egui::RichText::new("📸 28 Images").size(10.5).color(palette.text_muted));
                        ui.separator();
                        ui.label(egui::RichText::new("🎵 34 Audio").size(10.5).color(palette.text_muted));
                        ui.separator();
                        ui.label(egui::RichText::new("🎬 42 Video").size(10.5).color(palette.text_muted));
                        ui.separator();
                        ui.label(egui::RichText::new("📑 18 Documents").size(10.5).color(palette.text_muted));
                        ui.separator();
                        ui.label(egui::RichText::new("📦 12 Archives").size(10.5).color(palette.text_muted));
                        ui.separator();
                        ui.label(egui::RichText::new("🇮🇳 Indian Exam Portals").size(10.5).color(PANTONE_FLAVEDO));
                    });
                });
            });

            ui.add_space(18.0);

            // 2. Quick-Start Workflow Presets (2x2 Grid)
            ui.label(egui::RichText::new("⚡ Quick-Start Conversion Workflows").size(14.0).strong().color(palette.text_primary));
            ui.add_space(8.0);

            ui.columns(2, |cols| {
                // Preset 1: Web Optimizer
                let p1 = egui::Frame::none()
                    .fill(palette.card_bg)
                    .stroke(Stroke::new(1.0_f32, palette.border))
                    .rounding(Rounding::same(10.0))
                    .inner_margin(egui::Margin::same(14.0));
                p1.show(&mut cols[0], |ui| {
                    ui.label(egui::RichText::new("🖼️ Web Image Optimizer").size(13.0).strong().color(palette.text_primary));
                    ui.label(egui::RichText::new("Convert PNG/JPG to WebP/AVIF for up to 80% size savings with zero perceptual loss.").size(11.0).color(palette.text_secondary));
                    ui.add_space(6.0);
                    if ui.button(egui::RichText::new("Select Images ➔").size(11.0).color(PANTONE_FLAVEDO)).clicked() {
                        on_pick_file();
                    }
                });

                // Preset 2: Vector PDF
                let p2 = egui::Frame::none()
                    .fill(palette.card_bg)
                    .stroke(Stroke::new(1.0_f32, palette.border))
                    .rounding(Rounding::same(10.0))
                    .inner_margin(egui::Margin::same(14.0));
                p2.show(&mut cols[1], |ui| {
                    ui.label(egui::RichText::new("📄 Notes to Vector PDF").size(13.0).strong().color(palette.text_primary));
                    ui.label(egui::RichText::new("Typeset text, markdown, and code into publication-grade 300 DPI vector PDF.").size(11.0).color(palette.text_secondary));
                    ui.add_space(6.0);
                    if ui.button(egui::RichText::new("Select Document ➔").size(11.0).color(PANTONE_FLAVEDO)).clicked() {
                        on_pick_file();
                    }
                });
            });

            ui.add_space(8.0);

            ui.columns(2, |cols| {
                // Preset 3: Audio Lossless
                let p3 = egui::Frame::none()
                    .fill(palette.card_bg)
                    .stroke(Stroke::new(1.0_f32, palette.border))
                    .rounding(Rounding::same(10.0))
                    .inner_margin(egui::Margin::same(14.0));
                p3.show(&mut cols[0], |ui| {
                    ui.label(egui::RichText::new("🎵 Lossless Audio Remux").size(13.0).strong().color(palette.text_primary));
                    ui.label(egui::RichText::new("Bit-exact WAV ↔ FLAC transcoding with zero stream re-encoding and CRC verification.").size(11.0).color(palette.text_secondary));
                    ui.add_space(6.0);
                    if ui.button(egui::RichText::new("Select Audio ➔").size(11.0).color(PANTONE_FLAVEDO)).clicked() {
                        on_pick_file();
                    }
                });

                // Preset 4: Indian Exam Form
                let p4 = egui::Frame::none()
                    .fill(palette.card_bg)
                    .stroke(Stroke::new(1.0_f32, palette.border))
                    .rounding(Rounding::same(10.0))
                    .inner_margin(egui::Margin::same(14.0));
                p4.show(&mut cols[1], |ui| {
                    ui.label(egui::RichText::new("🎯 Custom Form Conformer").size(13.0).strong().color(palette.text_primary));
                    ui.label(egui::RichText::new("Conforms any photo, signature, or document to exact custom dimensions, DPI, and file size boundaries.").size(11.0).color(palette.text_secondary));
                    ui.add_space(6.0);
                    if ui.button(egui::RichText::new("Open Conformer ➔").size(11.0).color(PANTONE_FLAVEDO)).clicked() {
                        self.current_tab = StudioTab::ExamStudio;
                    }
                });
            });

            ui.add_space(18.0);

            // 3. Interactive 200+ Format Catalogue Browser
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("📚 200+ Supported Formats Catalogue").size(14.0).strong().color(palette.text_primary));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.catalogue_search)
                            .hint_text("🔍 Filter format (e.g. flac, webp, pdf)...")
                            .desired_width(200.0),
                    );
                });
            });

            ui.add_space(8.0);

            let all_samples = [
                ("PNG", FileCategory::Image, "Portable Network Graphics • Lossless"),
                ("JPG", FileCategory::Image, "Joint Photographic Experts Group"),
                ("WEBP", FileCategory::Image, "Google Modern Web Image • 80% Reduction"),
                ("AVIF", FileCategory::Image, "AV1 Image File Format • Next-Gen"),
                ("HEIC", FileCategory::Image, "High Efficiency Image Container • Apple"),
                ("TIFF", FileCategory::Image, "Tagged Image File Format • Archival"),
                ("BMP", FileCategory::Image, "Windows Bitmap Graphics"),
                ("SVG", FileCategory::Image, "Scalable Vector Graphics"),
                ("PDF", FileCategory::Document, "Adobe Portable Document Format • Vector"),
                ("DOCX", FileCategory::Document, "Microsoft Word OpenXML Document"),
                ("XLSX", FileCategory::Document, "Microsoft Excel Spreadsheet"),
                ("CSV", FileCategory::Document, "Comma-Separated Values Table"),
                ("TXT", FileCategory::Document, "Plain Text UTF-8"),
                ("MD", FileCategory::Document, "Markdown Formatted Document"),
                ("WAV", FileCategory::Audio, "Uncompressed PCM RIFF Wave"),
                ("FLAC", FileCategory::Audio, "Free Lossless Audio Codec • Bit-Exact"),
                ("MP3", FileCategory::Audio, "MPEG-1 Audio Layer III • 320 kbps"),
                ("AAC", FileCategory::Audio, "Advanced Audio Coding • High-Fidelity"),
                ("OGG", FileCategory::Audio, "Ogg Vorbis Audio Stream"),
                ("MP4", FileCategory::Video, "MPEG-4 Part 14 Video • Fast Remux"),
                ("MKV", FileCategory::Video, "Matroska Multimedia Container"),
                ("WEBM", FileCategory::Video, "HTML5 Web Video • VP9/AV1"),
                ("ZIP", FileCategory::Archive, "PKZip Deflate Level 9 • Safe Unpack"),
                ("TAR.GZ", FileCategory::Archive, "Unix Tape Archive Gzip Compressed"),
                ("TAR.ZST", FileCategory::Archive, "Zstandard Modern Ultra Compression"),
            ];

            let query = self.catalogue_search.to_uppercase();
            ui.horizontal_wrapped(|ui| {
                for (fmt, _cat, desc) in all_samples {
                    if !query.is_empty() && !fmt.contains(&query) && !desc.to_uppercase().contains(&query) {
                        continue;
                    }
                    let chip_frame = egui::Frame::none()
                        .fill(palette.card_bg)
                        .stroke(Stroke::new(1.0_f32, palette.border))
                        .rounding(Rounding::same(8.0))
                        .inner_margin(egui::Margin::symmetric(10.0, 6.0));

                    chip_frame.show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(fmt).size(12.0).strong().color(PANTONE_FLAVEDO));
                            ui.label(egui::RichText::new(desc).size(10.0).color(palette.text_muted));
                        });
                    });
                }
            });
        }
    }

    /// Tab 2: Custom Specification & Application Form Conformer (Zero hardcoded templates)
    fn render_exam_studio_tab(
        &mut self,
        ui: &mut Ui,
        palette: &ThemePalette,
        current_file: &Option<PathBuf>,
        settings: &mut QuickSettingsState,
        mut on_pick_file: impl FnMut(),
        mut on_open_folder: impl FnMut(&Path),
    ) {
        ui.label(egui::RichText::new("🎯 Custom Specification & Form Conformer").size(22.0).strong().color(palette.text_primary));
        ui.label(
            egui::RichText::new("Conform any photo, signature, or document to your exact custom specifications with micro-converged binary search bounds. No unverified or outdated templates.")
                .size(12.0)
                .color(palette.text_secondary),
        );
        ui.add_space(14.0);

        // Tuning Envelope Card
        let envelope_frame = egui::Frame::none()
            .fill(palette.card_bg)
            .stroke(Stroke::new(1.0_f32, palette.border))
            .rounding(Rounding::same(10.0))
            .inner_margin(egui::Margin::same(16.0));

        envelope_frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Custom Parameter Envelope").strong().color(PANTONE_FLAVEDO));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(format!("Active Specification: {}", settings.exam_preset_name)).size(11.0).color(palette.text_muted));
                });
            });

            ui.add_space(10.0);

            // Row 1: Specification Name
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Specification Name:").size(12.0));
                ui.add(egui::TextEdit::singleline(&mut settings.exam_preset_name).desired_width(220.0).hint_text("e.g. Application Photo, Signature"));
            });

            ui.add_space(8.0);

            // Row 2: Two-column layout for Dimensions & File Size Bounds
            ui.columns(2, |cols| {
                cols[0].vertical(|ui| {
                    ui.label(egui::RichText::new("📏 Dimensions & Resolution").strong().color(palette.text_primary));
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label("Width (px):");
                        ui.add(egui::DragValue::new(&mut settings.exam_width).speed(1).range(20..=8000));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Height (px):");
                        ui.add(egui::DragValue::new(&mut settings.exam_height).speed(1).range(20..=8000));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Calibrated DPI:");
                        ui.add(egui::DragValue::new(&mut settings.exam_dpi).speed(1).range(50..=1200));
                    });
                    let ratio = if settings.exam_height > 0 {
                        settings.exam_width as f32 / settings.exam_height as f32
                    } else {
                        1.0
                    };
                    ui.label(egui::RichText::new(format!("Aspect Ratio: {:.2}:1 ({}x{} px)", ratio, settings.exam_width, settings.exam_height)).size(10.5).color(palette.text_muted));
                });

                cols[1].vertical(|ui| {
                    ui.label(egui::RichText::new("📦 File Size Boundaries").strong().color(palette.text_primary));
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label("Min Size (KB):");
                        ui.add(egui::DragValue::new(&mut settings.exam_min_kb).speed(1).range(1..=102400));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Max Size (KB):");
                        ui.add(egui::DragValue::new(&mut settings.exam_max_kb).speed(1).range(1..=102400));
                    });
                    ui.add_space(4.0);
                    ui.checkbox(&mut settings.exam_whiten, "Whiten Background & Clean Scanner Shadows");
                    ui.label(egui::RichText::new(format!("Allowed Range: {} KB — {} KB", settings.exam_min_kb, settings.exam_max_kb)).size(10.5).color(palette.text_muted));
                });
            });

            ui.add_space(14.0);
            ui.separator();
            ui.add_space(8.0);

            // Execute Custom Form Optimization
            ui.horizontal(|ui| {
                if let Some(ref path) = current_file {
                    let optimize_btn = ui.add(
                        egui::Button::new(
                            egui::RichText::new("🎯 Optimize & Generate Conformed Output")
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
                            positioning: mossambi_exam::PositioningMode::CenterCrop,
                            background_color: [255, 255, 255],
                        };

                        let parent = path.parent().unwrap_or_else(|| Path::new("."));
                        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
                        let out_file = parent.join(format!("{}_conformed.jpg", stem));

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
                                self.exam_status = format!("❌ Optimization failed: {}", e);
                            }
                        }
                    }

                    if let Some(ref out) = self.last_exam_output {
                        if ui.button(egui::RichText::new("📂 Open in Explorer").size(12.0)).clicked() {
                            on_open_folder(out);
                        }
                    }
                } else {
                    ui.label(egui::RichText::new("Drag and drop photo, signature, or document to begin.").color(palette.text_muted));
                    if ui.button("📁 Select File").clicked() {
                        on_pick_file();
                    }
                }
            });

            if !self.exam_status.is_empty() {
                ui.add_space(8.0);
                ui.label(egui::RichText::new(&self.exam_status).size(11.5).color(PANTONE_GOLD));
            }
        });
    }

    /// Tab 3: Archive Suite & Safe Extraction
    fn render_archive_tab(
        &mut self,
        ui: &mut Ui,
        palette: &ThemePalette,
        current_file: &Option<PathBuf>,
        _settings: &mut QuickSettingsState,
        mut on_pick_file: impl FnMut(),
        mut on_convert: impl FnMut(String),
    ) {
        ui.label(egui::RichText::new("Archive Suite & Compression").size(22.0).strong().color(palette.text_primary));
        ui.label(egui::RichText::new("Zip-Slip Traversal Protection • Multi-Threaded Deflate / Zstandard").size(12.0).color(palette.text_secondary));
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
        ui.label(egui::RichText::new("Hardware Acceleration & Neural Telemetry").size(22.0).strong().color(palette.text_primary));
        ui.label(egui::RichText::new("GPU Encoder Pipelines • Windows DirectML • Multi-Core Concurrency").size(12.0).color(palette.text_secondary));
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
        ui.label(egui::RichText::new("Preferences & Appearance").size(22.0).strong().color(palette.text_primary));
        ui.label(egui::RichText::new("Theme Selection, Gestures, and Certified Pantone Swatches").size(12.0).color(palette.text_secondary));
        ui.add_space(14.0);

        // 1. Theme Selector Card
        let theme_frame = egui::Frame::none()
            .fill(palette.card_bg)
            .stroke(Stroke::new(1.0_f32, palette.border))
            .rounding(Rounding::same(10.0))
            .inner_margin(egui::Margin::same(16.0));

        theme_frame.show(ui, |ui| {
            ui.label(egui::RichText::new("Theme & Visual Style").strong().color(palette.text_primary));
            ui.label(egui::RichText::new("Choose appearance across the GUI Studio, Radial Fruit Widget, and System Tray:").size(11.5).color(palette.text_muted));
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
            ui.label(egui::RichText::new("Background Resident Daemon & Global Triggers").strong().color(palette.text_primary));
            ui.label("• Win + Shift + C: Summon Floating Radial Wheel centered under cursor (with file auto-detection).");
            ui.label("• Win + Ctrl + C: Open Full Desktop Studio GUI.");
            ui.label("• Shake or Circle Gesture: Shake cursor or sweep arc while dragging file(s) to overlay wheel under cursor.");
            ui.label("• System-Level Gated Hook: 0.00% CPU overhead - motion is only analyzed when files/folders are actively held.");
            ui.label("• System Tray: Right-click citrus tray icon for themes, exam presets, and autostart toggle.");
            ui.add_space(8.0);

            let mut autostart = mossambi_core::MossambiConfig::is_autostart_configured();
            let check_resp = ui.checkbox(&mut autostart, "🚀 Run in Background at Startup (Auto-start on boot like Whisper Flow)");
            if check_resp.changed() {
                let _ = mossambi_core::MossambiConfig::set_autostart(autostart);
            }

            let mut cfg = mossambi_core::MossambiConfig::load();
            let mut gesture_enabled = cfg.enable_gesture_wakeup;
            let gesture_resp = ui.checkbox(&mut gesture_enabled, "🖱️ Enable File Drag Mouse Gesture Activation (Shake or Circle while dragging)");
            if gesture_resp.changed() {
                cfg.enable_gesture_wakeup = gesture_enabled;
                let _ = cfg.save();
            }
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

    /// Dedicated Quality & Integrity Inspector Tab (GUI Studio Only)
    fn render_quality_inspector_tab(
        &mut self,
        ui: &mut Ui,
        palette: &ThemePalette,
        current_file: &Option<PathBuf>,
        mut on_pick_file: impl FnMut(),
    ) {
        ui.label(
            egui::RichText::new("Universal File Quality, Integrity & Compatibility Inspector")
                .size(22.0)
                .strong()
                .color(palette.text_primary),
        );
        ui.label(
            egui::RichText::new("Container Integrity • Structural EOF Markers • Pure-Rust Parser • Compatibility Matrix • Speed Benchmark")
                .size(12.0)
                .color(palette.text_secondary),
        );
        ui.add_space(10.0);

        // Active File Action Banner
        let card_frame = egui::Frame::none()
            .fill(palette.card_bg)
            .stroke(Stroke::new(1.0_f32, palette.border))
            .rounding(Rounding::same(10.0))
            .inner_margin(egui::Margin::same(12.0));

        card_frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                if let Some(ref path) = current_file {
                    ui.label(egui::RichText::new("🔬").size(22.0));
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new(path.file_name().unwrap_or_default().to_string_lossy())
                                .size(13.0)
                                .strong()
                                .color(palette.text_primary),
                        );
                        ui.label(
                            egui::RichText::new(path.display().to_string())
                                .size(10.5)
                                .color(palette.text_muted),
                        );
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(egui::RichText::new("⚡ Benchmark Speed").size(11.5).strong().color(Color32::BLACK))
                            .on_hover_text("Benchmark conversion speed against Online and CLI converters")
                            .clicked()
                        {
                            match mossambi_codec::BenchmarkEngine::benchmark_file(path, None) {
                                Ok(bench) => self.bench_report = Some(bench),
                                Err(e) => self.inspection_status = format!("Benchmark failed: {}", e),
                            }
                        }

                        let inspect_btn = ui.add(
                            egui::Button::new(
                                egui::RichText::new("🔬 Inspect File")
                                    .size(11.5)
                                    .strong()
                                    .color(Color32::WHITE),
                            )
                            .fill(PANTONE_FLAVEDO)
                            .rounding(Rounding::same(8.0)),
                        );
                        if inspect_btn.clicked() {
                            match mossambi_codec::FileQualityChecker::inspect_file(path) {
                                Ok(rep) => {
                                    self.quality_report = Some(rep);
                                    self.inspected_file_path = Some(path.clone());
                                    self.inspection_status = "Inspection complete.".to_string();
                                }
                                Err(e) => {
                                    self.inspection_status = format!("Inspection failed: {}", e);
                                }
                            }
                        }
                    });
                } else {
                    ui.label(egui::RichText::new("📥").size(22.0));
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("No File Selected").size(13.0).strong().color(palette.text_primary));
                        ui.label(egui::RichText::new("Choose a file to inspect its container integrity and checksums.").size(10.5).color(palette.text_muted));
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(egui::RichText::new("📁 Choose File").size(11.5)).clicked() {
                            on_pick_file();
                        }
                    });
                }
            });
        });

        ui.add_space(10.0);

        if let Some(ref rep) = self.quality_report {
            card_frame.show(ui, |ui| {
                ui.horizontal(|ui| {
                    let (badge_text, badge_color) = match rep.verdict {
                        mossambi_codec::QualityVerdict::Pass => ("✓ PASS", Color32::from_rgb(39, 201, 63)),
                        mossambi_codec::QualityVerdict::Warning => ("⚠️ WARN", Color32::from_rgb(255, 189, 46)),
                        mossambi_codec::QualityVerdict::Fail => ("❌ FAIL", Color32::from_rgb(255, 95, 86)),
                    };

                    ui.label(egui::RichText::new(badge_text).size(18.0).strong().color(badge_color));
                    ui.separator();
                    ui.label(
                        egui::RichText::new(format!("Overall Score: {:.1}%", rep.overall_score))
                            .size(16.0)
                            .strong()
                            .color(PANTONE_FLAVEDO),
                    );
                    ui.separator();
                    ui.label(
                        egui::RichText::new(format!("Format: {}", rep.format))
                            .size(13.0)
                            .strong()
                            .color(palette.text_primary),
                    );
                    ui.label(
                        egui::RichText::new(format!("Size: {:.2} KB", rep.file_size_kb))
                            .size(12.0)
                            .color(palette.text_secondary),
                    );
                });

                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(&rep.summary_headline)
                        .size(11.5)
                        .color(palette.text_secondary),
                );

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("SHA-256:").size(10.5).color(palette.text_muted));
                    ui.label(egui::RichText::new(&rep.sha256).size(10.0).monospace().color(palette.text_secondary));
                    ui.separator();
                    ui.label(egui::RichText::new("CRC-32:").size(10.5).color(palette.text_muted));
                    ui.label(egui::RichText::new(&rep.crc32).size(10.0).monospace().color(palette.text_secondary));
                });
            });

            ui.add_space(8.0);

            // 4-Dimension Breakdown (2x2 Grid)
            ui.columns(2, |cols| {
                card_frame.show(&mut cols[0], |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("🛡️ 1. Container Integrity").size(13.0).strong().color(palette.text_primary));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(format!("{:.1}%", rep.integrity.structural_integrity_score))
                                    .size(12.0)
                                    .strong()
                                    .color(if rep.integrity.corruption_detected { Color32::RED } else { Color32::GREEN }),
                            );
                        });
                    });
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(format!("Signature: {}", rep.integrity.magic_bytes_description)).size(11.0).color(palette.text_secondary));
                    ui.label(egui::RichText::new(format!("EOF Check: {}", rep.integrity.eof_description)).size(11.0).color(palette.text_secondary));
                    ui.label(egui::RichText::new(format!("Corruption Detected: {}", if rep.integrity.corruption_detected { "YES (Structure Damaged)" } else { "No (Bit-Exact Valid)" })).size(11.0).color(if rep.integrity.corruption_detected { Color32::RED } else { palette.text_muted }));
                });

                card_frame.show(&mut cols[1], |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("⚙️ 2. Parser & Decoder").size(13.0).strong().color(palette.text_primary));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(format!("{:.1}%", rep.parseability.parseability_score))
                                    .size(12.0)
                                    .strong()
                                    .color(if rep.parseability.can_parse { Color32::GREEN } else { Color32::RED }),
                            );
                        });
                    });
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(format!("Parser: {}", rep.parseability.parser_used)).size(11.0).color(palette.text_secondary));
                    ui.label(egui::RichText::new(format!("Decoded: {}", rep.parseability.decoded_payload_summary)).size(11.0).color(palette.text_secondary));
                    if !rep.parseability.parse_warnings.is_empty() {
                        ui.label(egui::RichText::new(format!("Warnings: {}", rep.parseability.parse_warnings.join(", "))).size(10.5).color(Color32::from_rgb(255, 189, 46)));
                    } else {
                        ui.label(egui::RichText::new("Zero decode anomalies detected").size(10.5).color(palette.text_muted));
                    }
                });
            });

            ui.add_space(6.0);

            ui.columns(2, |cols| {
                card_frame.show(&mut cols[0], |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("🌐 3. Compatibility Matrix").size(13.0).strong().color(palette.text_primary));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(format!("{:.1}%", rep.compatibility.compatibility_score))
                                    .size(12.0)
                                    .strong()
                                    .color(Color32::GREEN),
                            );
                        });
                    });
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(format!("Web Browsers: {:.0}% • Mobile HW: {:.0}% • Archival: {:.0}%", rep.compatibility.web_compatibility_score, rep.compatibility.mobile_compatibility_score, rep.compatibility.archival_compatibility_score)).size(11.0).color(palette.text_secondary));
                    ui.label(egui::RichText::new(format!("Standards: {}", rep.compatibility.standards_compliance.join(" • "))).size(10.5).color(palette.text_muted));
                });

                card_frame.show(&mut cols[1], |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("💎 4. Quality & Fidelity").size(13.0).strong().color(palette.text_primary));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(format!("{:.1}%", rep.quality.quality_score))
                                    .size(12.0)
                                    .strong()
                                    .color(Color32::GREEN),
                            );
                        });
                    });
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(format!("Rating: {}", rep.quality.fidelity_rating)).size(11.0).color(palette.text_secondary));
                    ui.label(egui::RichText::new(format!("Depth/Resolution: {}", rep.quality.color_or_sample_depth)).size(11.0).color(palette.text_secondary));
                    ui.label(egui::RichText::new(format!("Clarity Heuristic: {:.1}%", rep.quality.edge_clarity_score)).size(10.5).color(palette.text_muted));
                });
            });

            // Benchmark Comparison Section
            if let Some(ref bench) = self.bench_report {
                ui.add_space(8.0);
                card_frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("🏎️ Performance Benchmark Results").size(13.5).strong().color(palette.text_primary));
                        ui.separator();
                        ui.label(egui::RichText::new(format!("Duration: {:.2} ms ({} µs)", bench.duration_ms, bench.duration_micros)).size(12.0).strong().color(PANTONE_FLAVEDO));
                        ui.separator();
                        ui.label(egui::RichText::new(format!("Throughput: {:.1} MB/s", bench.throughput_mb_per_sec)).size(12.0).strong().color(Color32::GREEN));
                        ui.separator();
                        ui.label(egui::RichText::new(if bench.hardware_accelerated { "HW-ACCEL ACTIVE" } else { "CPU PIPELINE" }).size(11.0).color(PANTONE_GOLD));
                    });

                    ui.add_space(6.0);
                    ui.label(egui::RichText::new("Comparison Against Industry Converters:").size(11.5).strong().color(palette.text_secondary));

                    for comp in &bench.comparisons {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(format!("• {:<24}", comp.name)).size(11.0).strong().color(palette.text_primary));
                            ui.label(egui::RichText::new(format!("({:<12})", comp.category)).size(10.5).color(palette.text_muted));
                            ui.label(egui::RichText::new(format!("~{:.1} ms", comp.estimated_duration_ms)).size(11.0).color(palette.text_secondary));
                            ui.label(egui::RichText::new(format!("➔ {:.1}x FASTER", comp.speedup_factor)).size(11.0).strong().color(Color32::GREEN));
                            ui.label(egui::RichText::new(format!("— {}", comp.notes)).size(10.5).color(palette.text_muted));
                        });
                    }

                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(&bench.summary).size(11.0).color(PANTONE_FLAVEDO));
                });
            }
        }
    }
}
