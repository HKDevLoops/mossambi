//! Mossambi Native Desktop Suite (GUI & Radial Citrus Widget)
//! 100% Pure Native Rust Application
//! - Frameless translucent Mossambi citrus fruit radial widget matching `brain/Image reference.png`
//! - Botanical anatomy: Flavedo peel (Pantone 376 C), Albedo pith, 8 carpel pulp wedges, Columella core hub, Golden ALL sector
//! - Contextual quick settings popup for every converter (Vector PDF, Image, Audio, Video, Exam)
//! - Activation via global `WIN + CTRL` hotkey and circular mouse drag gesture (>=300 deg)
//! - Hardware acceleration (GPU + NPU) and multi-threaded Rayon execution
//! - Zero external Python or script dependencies

use eframe::egui::{self, Color32, Margin, Pos2, Rounding, Vec2, ViewportBuilder, ViewportCommand};
use mossambi_archive::{pack_archive, unpack_archive, ArchiveType};
use mossambi_codec::{
    convert_text_to_pdf_vector, transcode_file, transcode_to_all_formats, LosslessMode,
    TextPdfSettings,
};
use mossambi_core::{FileCategory, UniversalRegistry};
use mossambi_exam::{optimize_exam_media, ExamConstraint};
use mossambi_gui::citrus_geometry::{CitrusGeometry, FLAVEDO_GREEN};
use mossambi_gui::quick_settings::QuickSettingsState;
use mossambi_gui::tray::{ActivationManager, WakeupReason};
use mossambi_hwaccel::detect_hardware_capabilities;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum ConversionResult {
    Success(Vec<PathBuf>),
    Failure(String),
}

pub struct MossambiApp {
    geometry: CitrusGeometry,
    quick_settings: QuickSettingsState,
    activation_manager: ActivationManager,

    // Dynamic state
    current_file: Option<PathBuf>,
    current_category: FileCategory,
    current_sectors: Vec<&'static str>,
    hovered_carpel: Option<usize>,
    hub_status: String,
    is_converting: bool,
    progress_angle: f32,
    last_completed_output: Option<PathBuf>,
    status_message: String,

    // Channels for background worker
    result_receiver: Receiver<ConversionResult>,
    result_sender: Sender<ConversionResult>,

    // UI View toggle
    show_studio_view: bool,
}

impl Default for MossambiApp {
    fn default() -> Self {
        let (tx, rx) = channel();
        let default_cat = FileCategory::Document;
        let default_sectors = UniversalRegistry::get_widget_sectors_for_category(default_cat);

        Self {
            geometry: CitrusGeometry::new(Pos2::new(160.0, 160.0), 320.0),
            quick_settings: QuickSettingsState::default(),
            activation_manager: ActivationManager::new(),

            current_file: None,
            current_category: default_cat,
            current_sectors: default_sectors,
            hovered_carpel: None,
            hub_status: "DROP FILE".to_string(),
            is_converting: false,
            progress_angle: 0.0,
            last_completed_output: None,
            status_message: "Ready. Press WIN+CTRL or drag file in circle to wake up.".to_string(),

            result_receiver: rx,
            result_sender: tx,

            show_studio_view: false,
        }
    }
}

impl MossambiApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self::default()
    }

    /// Handles file selection or drop
    pub fn select_file(&mut self, path: PathBuf) {
        let (cat, ext) = UniversalRegistry::classify_file(&path);
        self.current_file = Some(path);
        self.current_category = cat;
        self.current_sectors = UniversalRegistry::get_widget_sectors_for_category(cat);
        self.hub_status = if ext.is_empty() { "FILE".to_string() } else { ext };
        if let Some(first_fmt) = self.current_sectors.first() {
            self.quick_settings.set_active_format(first_fmt);
        }
    }

    /// Triggers asynchronous conversion pipeline
    pub fn start_conversion(&mut self, target_format: &str) {
        let input_file = match self.current_file.clone() {
            Some(f) => f,
            None => {
                self.status_message = "Please drop or select a file first.".to_string();
                return;
            }
        };

        self.is_converting = true;
        self.hub_status = "BUSY".to_string();
        self.status_message = format!("Converting '{}' -> {}...", input_file.file_name().unwrap_or_default().to_string_lossy(), target_format);

        let sender = self.result_sender.clone();
        let target_str = target_format.to_string();
        let settings = self.quick_settings.clone();

        thread::spawn(move || {
            let res = execute_conversion(&input_file, &target_str, &settings);
            let _ = sender.send(res);
        });
    }
}

/// Standalone background execution routine
fn execute_conversion(
    input: &Path,
    target: &str,
    settings: &QuickSettingsState,
) -> ConversionResult {
    let target_upper = target.to_uppercase();

    // 1. Check Indian Exam Form Mode
    if settings.exam_mode_enabled || target_upper.contains("EXAM") {
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

        let parent = input.parent().unwrap_or_else(|| Path::new("."));
        let stem = input.file_stem().and_then(|s| s.to_str()).unwrap_or("exam_output");
        let mut out = parent.join(format!("{}_exam.jpg", stem));
        let mut c = 1;
        while out.exists() {
            out = parent.join(format!("{}_exam_{}.jpg", stem, c));
            c += 1;
        }

        match optimize_exam_media(input, &out, &constraint) {
            Ok(p) => return ConversionResult::Success(vec![p]),
            Err(e) => return ConversionResult::Failure(format!("Exam tuning failed: {}", e)),
        }
    }

    // 2. Check ALL Sector Batch Transcode
    if target_upper == "ALL" {
        let mode = if settings.image_lossless || settings.audio_strict_lossless {
            LosslessMode::StrictBitExact
        } else {
            LosslessMode::PerceptualHighFidelity
        };

        match transcode_to_all_formats(input, mode, None) {
            Ok(outputs) => return ConversionResult::Success(outputs),
            Err(e) => return ConversionResult::Failure(format!("Batch ALL transcode failed: {}", e)),
        }
    }

    // 3. Vector Text-to-PDF Conversion
    let (cat, in_ext) = UniversalRegistry::classify_file(input);
    let is_text = matches!(
        in_ext.as_str(),
        "TXT" | "MD" | "RST" | "LOG" | "JSON" | "XML" | "YAML" | "YML" | "RS" | "PY" | "JS"
            | "TS" | "C" | "CPP" | "H" | "HTML" | "HTM" | "ORG" | "TEX"
    );

    if target_upper == "PDF" && (cat == FileCategory::Document || is_text) {
        let out_path = mossambi_core::resolve_sibling_output_path(input, None, "PDF", None);
        let pdf_cfg = TextPdfSettings {
            page_size: settings.pdf_page_size.clone(),
            font_family: settings.pdf_font_family.clone(),
            font_size: settings.pdf_font_size,
            line_spacing: 1.35,
            margin_pt: settings.pdf_margin,
            show_header_footer: settings.pdf_header_footer,
            title: Some(input.file_stem().and_then(|s| s.to_str()).unwrap_or("Document").to_string()),
        };

        match convert_text_to_pdf_vector(input, &out_path, Some(pdf_cfg)) {
            Ok(()) => return ConversionResult::Success(vec![out_path]),
            Err(e) => return ConversionResult::Failure(format!("Vector PDF generation failed: {}", e)),
        }
    }

    // 4. Archive Extraction or Packing
    if target_upper == "EXTRACT" {
        let parent = input.parent().unwrap_or_else(|| Path::new("."));
        let stem = input.file_stem().and_then(|s| s.to_str()).unwrap_or("extracted");
        let dest = parent.join(stem);
        match unpack_archive(input, &dest) {
            Ok(()) => return ConversionResult::Success(vec![dest]),
            Err(e) => return ConversionResult::Failure(format!("Archive extraction failed: {}", e)),
        }
    }

    if ["ZIP", "7Z", "TAR", "TAR.GZ", "TAR.XZ", "TAR.ZST"].contains(&target_upper.as_str()) {
        let parent = input.parent().unwrap_or_else(|| Path::new("."));
        let stem = input.file_stem().and_then(|s| s.to_str()).unwrap_or("archive");
        let out_archive = parent.join(format!("{}.{}", stem, target_upper.to_lowercase()));
        let arch_type = ArchiveType::from_extension(&target_upper).unwrap_or(ArchiveType::Zip);
        match pack_archive(&[input.to_path_buf()], &out_archive, arch_type) {
            Ok(()) => return ConversionResult::Success(vec![out_archive]),
            Err(e) => return ConversionResult::Failure(format!("Archive packing failed: {}", e)),
        }
    }

    // 5. Standard Media Transcode
    let mode = if settings.image_lossless || settings.audio_strict_lossless {
        LosslessMode::StrictBitExact
    } else {
        LosslessMode::PerceptualHighFidelity
    };

    match transcode_file(input, &target_upper, mode, None) {
        Ok(out) => ConversionResult::Success(vec![out]),
        Err(e) => ConversionResult::Failure(format!("Conversion failed: {}", e)),
    }
}

impl eframe::App for MossambiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Continuous repaint for smooth animations and gesture responsiveness
        ctx.request_repaint_after(Duration::from_millis(30));

        // 1. Check for activation events from global hotkey or circle gesture
        if let Some(event) = self.activation_manager.try_recv() {
            ctx.send_viewport_cmd(ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(ViewportCommand::Focus);
            // Center the widget directly under the cursor location
            let win_x = (event.cursor_x - 160.0).max(0.0);
            let win_y = (event.cursor_y - 160.0).max(0.0);
            ctx.send_viewport_cmd(ViewportCommand::OuterPosition(Pos2::new(win_x, win_y)));

            let reason_str = match event.reason {
                WakeupReason::HotkeyWinCtrl => "Global Hotkey (WIN+CTRL)",
                WakeupReason::CircleDragGesture => "Circle-Drag Gesture",
                WakeupReason::TrayClick => "System Tray",
            };
            self.status_message = format!("Summoned via {} at ({:.0}, {:.0})", reason_str, event.cursor_x, event.cursor_y);
        }

        // 2. Poll for background conversion results
        if let Ok(res) = self.result_receiver.try_recv() {
            self.is_converting = false;
            match res {
                ConversionResult::Success(files) => {
                    self.hub_status = "DONE".to_string();
                    if let Some(first) = files.first() {
                        self.last_completed_output = Some(first.clone());
                    }
                    self.status_message = format!("✨ Successfully converted {} file(s)!", files.len());
                }
                ConversionResult::Failure(err) => {
                    self.hub_status = "ERR".to_string();
                    self.status_message = format!("❌ {}", err);
                }
            }
        }

        // 3. Update rotating progress angle
        if self.is_converting {
            self.progress_angle += 0.15;
            if self.progress_angle > 2.0 * std::f32::consts::PI {
                self.progress_angle -= 2.0 * std::f32::consts::PI;
            }
        }

        // 4. Handle Drag-and-Drop Files from OS Explorer
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                if let Some(drop) = i.raw.dropped_files.first() {
                    if let Some(ref path) = drop.path {
                        self.select_file(path.clone());

                        // If dropped onto a specific carpel wedge, convert immediately
                        if let Some(carpel_idx) = self.hovered_carpel {
                            if let Some(target) = self.current_sectors.get(carpel_idx) {
                                self.start_conversion(target);
                            }
                        }
                    }
                }
            }
        });

        // 5. Render Main Interface
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(Color32::from_black_alpha(0))) // Fully transparent window background
            .show(ctx, |ui| {
                if self.show_studio_view {
                    self.render_studio_view(ui);
                } else {
                    self.render_radial_widget_view(ui);
                }
            });
    }
}

impl MossambiApp {
    /// Renders the radial botanical sweet lime widget and contextual quick settings
    fn render_radial_widget_view(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // Left Column: The 8-Petal Mossambi Fruit Radial Widget
            let (rect, response) = ui.allocate_exact_size(Vec2::new(320.0, 320.0), egui::Sense::click_and_drag());
            let painter = ui.painter_at(rect);

            self.geometry.center = rect.center();

            // Hit test hovered carpel
            if let Some(hover_pos) = response.hover_pos() {
                self.hovered_carpel = self.geometry.hit_test_carpel(hover_pos);
                if let Some(idx) = self.hovered_carpel {
                    if let Some(fmt) = self.current_sectors.get(idx) {
                        self.quick_settings.set_active_format(fmt);
                    }
                }

                // If user clicks on a carpel wedge, start conversion to that format
                if response.clicked() {
                    if let Some(idx) = self.hovered_carpel {
                        if let Some(fmt) = self.current_sectors.get(idx) {
                            self.start_conversion(fmt);
                        }
                    } else if self.geometry.hit_test_hub(hover_pos) {
                        // Clicking hub opens file picker
                        if let Some(path) = rfd::FileDialog::new().pick_file() {
                            self.select_file(path);
                        }
                    }
                }
            } else {
                self.hovered_carpel = None;
            }

            // Draw botanical citrus fruit geometry
            self.geometry.render(
                &painter,
                &self.current_sectors,
                self.hovered_carpel,
                &self.hub_status,
                self.is_converting,
                self.progress_angle,
            );

            // Right Column: Contextual Quick Settings Menu Panel (Requested by User)
            ui.add_space(8.0);
            ui.vertical(|ui| {
                // Top control bar
                ui.horizontal(|ui| {
                    if ui.button(egui::RichText::new("📁 Browse").size(11.0)).clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_file() {
                            self.select_file(path);
                        }
                    }

                    if ui.button(egui::RichText::new("⚡ Studio").size(11.0)).clicked() {
                        self.show_studio_view = true;
                    }

                    let btn_text = if self.quick_settings.is_open { "⚙ Hide Settings" } else { "⚙ Quick Settings" };
                    if ui.button(egui::RichText::new(btn_text).size(11.0)).clicked() {
                        self.quick_settings.is_open = !self.quick_settings.is_open;
                    }
                });

                ui.add_space(6.0);

                // Quick Settings Card
                if self.quick_settings.is_open {
                    self.quick_settings.render_ui(ui);
                }

                ui.add_space(6.0);

                // Action Bar: Instant Convert Button
                let active_fmt = self.quick_settings.active_format.clone();
                let can_convert = self.current_file.is_some() && !self.is_converting;

                ui.horizontal(|ui| {
                    let conv_btn = ui.add_enabled(
                        can_convert,
                        egui::Button::new(
                            egui::RichText::new(format!("⚡ Convert to {}", active_fmt))
                                .size(12.5)
                                .strong()
                                .color(Color32::from_rgb(20, 30, 15)),
                        )
                        .fill(FLAVEDO_GREEN),
                    );

                    if conv_btn.clicked() {
                        let fmt = active_fmt.clone();
                        self.start_conversion(&fmt);
                    }

                    // Open Output Folder Button
                    if let Some(ref out) = self.last_completed_output {
                        if ui.button(egui::RichText::new("📂 Open Output").size(11.0)).clicked() {
                            if let Some(parent) = out.parent() {
                                open_in_file_manager(parent);
                            }
                        }
                    }
                });

                // Status ticker
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(&self.status_message)
                        .size(10.5)
                        .color(Color32::from_rgb(200, 220, 190)),
                );
            });
        });
    }

    /// Renders the full Mossambi Studio dashboard
    fn render_studio_view(&mut self, ui: &mut egui::Ui) {
        egui::Frame::none()
            .fill(Color32::from_rgb(24, 28, 22))
            .rounding(Rounding::same(12.0))
            .inner_margin(Margin::same(16.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("🍋 Mossambi Studio (Full Suite)")
                            .size(16.0)
                            .strong()
                            .color(FLAVEDO_GREEN),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("⮌ Radial Widget").clicked() {
                            self.show_studio_view = false;
                        }
                    });
                });

                ui.separator();
                ui.add_space(10.0);

                let hw = detect_hardware_capabilities();
                ui.label(egui::RichText::new("Hardware Acceleration Engines Active:").strong());
                ui.label(format!("• GPU: {:?} | Encoder: {} | HEVC: {}", hw.vendor, hw.h264_encoder, hw.hevc_encoder));
                ui.label(format!("• NPU AI Accelerator: {} | Concurrency: {} Rayon Threads", hw.npu_name, hw.optimal_thread_count));

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(10.0);

                ui.label(egui::RichText::new("Selected File:").strong());
                if let Some(ref f) = self.current_file {
                    ui.label(format!("{}", f.display()));
                } else {
                    ui.label("No file loaded yet. Use Browse or drop a file.");
                }

                ui.add_space(10.0);
                if ui.button(egui::RichText::new("📁 Select File to Convert").size(13.0)).clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_file() {
                        self.select_file(path);
                    }
                }
            });
    }
}

pub fn open_in_file_manager(path: &Path) {
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("explorer").arg(path).spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(path).spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(path).spawn();
    }
}

fn main() -> Result<(), eframe::Error> {
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title("Mossambi Native Suite")
            .with_inner_size([650.0, 360.0])
            .with_min_inner_size([340.0, 340.0])
            .with_decorations(false) // Frameless window
            .with_transparent(true)  // Native borderless transparency
            .with_always_on_top(),
        ..Default::default()
    };

    eframe::run_native(
        "Mossambi Native Suite",
        options,
        Box::new(|cc| Ok(Box::new(MossambiApp::new(cc)))),
    )
}
