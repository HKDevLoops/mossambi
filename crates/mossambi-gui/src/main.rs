//! Mossambi 100% Apple macOS-Styled Desktop Suite (GUI & Radial Citrus Widget)
//! Pure Native Rust Application
//! - Hero standalone circular Sweet Lime Citrus Widget (no ugly attached dashboard)
//! - Long-Press (>=350ms) on any converter petal triggers an anchored floating macOS popover
//! - 100% macOS designed Mossambi Studio window with traffic lights, sidebar, and Indian Exam Studio
//! - Fast UNIX mv-like CLI, TUI, and global WIN+CTRL & circle-drag activation hooks
//! - 0.00% Zero-Loss Standard, Hardware Acceleration (GPU + NPU), 16 Rayon worker threads

use eframe::egui::{self, Color32, Key, Pos2, Vec2, ViewportBuilder, ViewportCommand};
use mossambi_archive::{pack_archive, unpack_archive, ArchiveType};
use mossambi_codec::{
    convert_text_to_pdf_vector, transcode_file, transcode_to_all_formats, LosslessMode,
    TextPdfSettings,
};
use mossambi_core::{FileCategory, UniversalRegistry};
use mossambi_exam::{optimize_exam_media, ExamConstraint};
use mossambi_gui::citrus_geometry::CitrusGeometry;
use mossambi_gui::macos_theme::*;
use mossambi_gui::popover::PetalPopover;
use mossambi_gui::quick_settings::QuickSettingsState;
use mossambi_gui::studio_view::StudioView;
use mossambi_gui::tray::ActivationManager;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppViewMode {
    RadialWidget, // Hero circular sweet lime overlay (360x360, transparent)
    Studio,       // 100% macOS styled window with sidebar (980x640)
}

#[derive(Debug, Clone)]
pub enum ConversionResult {
    Success(Vec<PathBuf>),
    Failure(String),
}

pub struct MossambiApp {
    mode: AppViewMode,
    geometry: CitrusGeometry,
    quick_settings: QuickSettingsState,
    studio_view: StudioView,
    activation_manager: ActivationManager,

    // File Staging & Conversion
    current_file: Option<PathBuf>,
    current_category: FileCategory,
    current_sectors: Vec<&'static str>,
    hovered_carpel: Option<usize>,
    hub_status: String,
    is_converting: bool,
    progress_angle: f32,
    last_completed_output: Option<PathBuf>,
    status_message: String,

    // Long-Press Tracking (>=350ms triggers Quick Settings popover)
    press_start: Option<(Instant, usize)>,
    popover_active_sector: Option<usize>,

    // Background worker channels
    result_receiver: Receiver<ConversionResult>,
    result_sender: Sender<ConversionResult>,
}

impl Default for MossambiApp {
    fn default() -> Self {
        let (tx, rx) = channel();
        let default_cat = FileCategory::Document;
        let default_sectors = UniversalRegistry::get_widget_sectors_for_category(default_cat);

        Self {
            mode: AppViewMode::RadialWidget,
            geometry: CitrusGeometry::new(Pos2::new(180.0, 180.0), 320.0),
            quick_settings: QuickSettingsState::default(),
            studio_view: StudioView::default(),
            activation_manager: ActivationManager::new(),

            current_file: None,
            current_category: default_cat,
            current_sectors: default_sectors,
            hovered_carpel: None,
            hub_status: "DROP FILE".to_string(),
            is_converting: false,
            progress_angle: 0.0,
            last_completed_output: None,
            status_message: "Ready. WIN+CTRL or drag in circle to summon.".to_string(),

            press_start: None,
            popover_active_sector: None,

            result_receiver: rx,
            result_sender: tx,
        }
    }
}

impl MossambiApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self::default()
    }

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
        self.popover_active_sector = None;
        self.status_message = format!("Converting '{}' -> {}...", input_file.file_name().unwrap_or_default().to_string_lossy(), target_format);

        let sender = self.result_sender.clone();
        let target_str = target_format.to_string();
        let settings = self.quick_settings.clone();

        thread::spawn(move || {
            let res = execute_conversion(&input_file, &target_str, &settings);
            let _ = sender.send(res);
        });
    }

    pub fn switch_to_studio(&mut self, ctx: &egui::Context) {
        self.mode = AppViewMode::Studio;
        ctx.send_viewport_cmd(ViewportCommand::InnerSize(Vec2::new(980.0, 640.0)));
        ctx.send_viewport_cmd(ViewportCommand::Decorations(false));
    }

    pub fn switch_to_widget(&mut self, ctx: &egui::Context) {
        self.mode = AppViewMode::RadialWidget;
        self.popover_active_sector = None;
        ctx.send_viewport_cmd(ViewportCommand::InnerSize(Vec2::new(360.0, 360.0)));
    }
}

/// Standalone conversion routine
fn execute_conversion(
    input: &Path,
    target: &str,
    settings: &QuickSettingsState,
) -> ConversionResult {
    let target_upper = target.to_uppercase();

    // 1. Indian Exam Form Mode
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

    // 2. ALL Sector Batch Transcode
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
        ctx.request_repaint_after(Duration::from_millis(25));

        // 1. Check for activation events from global hotkey or circle gesture
        if let Some(event) = self.activation_manager.try_recv() {
            ctx.send_viewport_cmd(ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(ViewportCommand::Focus);

            if self.mode == AppViewMode::RadialWidget {
                let win_x = (event.cursor_x - 180.0).max(0.0);
                let win_y = (event.cursor_y - 180.0).max(0.0);
                ctx.send_viewport_cmd(ViewportCommand::OuterPosition(Pos2::new(win_x, win_y)));
            }

            self.status_message = format!("Summoned at ({:.0}, {:.0})", event.cursor_x, event.cursor_y);
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

        // 3. Progress angle animation
        if self.is_converting {
            self.progress_angle += 0.16;
            if self.progress_angle > 2.0 * std::f32::consts::PI {
                self.progress_angle -= 2.0 * std::f32::consts::PI;
            }
        }

        // 4. Handle Drag-and-Drop Files from Explorer
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                if let Some(drop) = i.raw.dropped_files.first() {
                    if let Some(ref path) = drop.path {
                        self.select_file(path.clone());

                        // If dropped onto a specific carpel sector, convert immediately!
                        if let Some(carpel_idx) = self.hovered_carpel {
                            if let Some(target) = self.current_sectors.get(carpel_idx) {
                                self.start_conversion(target);
                            }
                        }
                    }
                }
            }

            // Esc key dismisses the overlay widget
            if i.key_pressed(Key::Escape) {
                if self.popover_active_sector.is_some() {
                    self.popover_active_sector = None;
                } else if self.mode == AppViewMode::RadialWidget {
                    ctx.send_viewport_cmd(ViewportCommand::Visible(false));
                }
            }
        });

        // 5. Render active view mode
        match self.mode {
            AppViewMode::RadialWidget => {
                egui::CentralPanel::default()
                    .frame(egui::Frame::none().fill(Color32::TRANSPARENT))
                    .show(ctx, |ui| {
                        self.render_pure_radial_widget(ui, ctx);
                    });
            }
            AppViewMode::Studio => {
                egui::CentralPanel::default()
                    .frame(egui::Frame::none().fill(MAC_WINDOW_BG))
                    .show(ctx, |ui| {
                        let cur_file = self.current_file.clone();
                        let mut qs = self.quick_settings.clone();

                        let mut file_to_pick = false;
                        let mut convert_to_target: Option<String> = None;
                        let mut open_target_folder: Option<PathBuf> = None;
                        let mut req_switch_widget = false;

                        self.studio_view.render(
                            ui,
                            ctx,
                            &cur_file,
                            &mut qs,
                            || file_to_pick = true,
                            |t| convert_to_target = Some(t),
                            |p| open_target_folder = Some(p.to_path_buf()),
                            || req_switch_widget = true,
                        );

                        self.quick_settings = qs;

                        if file_to_pick {
                            if let Some(path) = rfd::FileDialog::new().pick_file() {
                                self.select_file(path);
                            }
                        }
                        if let Some(target) = convert_to_target {
                            self.start_conversion(&target);
                        }
                        if let Some(p) = open_target_folder {
                            open_in_file_manager(&p);
                        }
                        if req_switch_widget {
                            self.switch_to_widget(ctx);
                        }
                    });
            }
        }
    }
}

impl MossambiApp {
    /// Renders the Hero Standalone Circular Sweet Lime Radial Wheel with Long-Press Popover support
    fn render_pure_radial_widget(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let (rect, response) = ui.allocate_exact_size(Vec2::new(360.0, 360.0), egui::Sense::click_and_drag());
        let painter = ui.painter_at(rect);

        self.geometry.center = rect.center();

        let hover_pos = response.hover_pos();
        self.hovered_carpel = hover_pos.and_then(|p| self.geometry.hit_test_carpel(p));

        // 1. Long-Press Detection (>=350ms on a carpel sector)
        let is_primary_down = ui.input(|i| i.pointer.primary_down());

        if is_primary_down {
            if let Some(idx) = self.hovered_carpel {
                if self.press_start.is_none() {
                    self.press_start = Some((Instant::now(), idx));
                } else if let Some((start_time, sector_idx)) = self.press_start {
                    if sector_idx == idx && start_time.elapsed() >= Duration::from_millis(350) {
                        self.popover_active_sector = Some(idx);
                        if let Some(fmt) = self.current_sectors.get(idx) {
                            self.quick_settings.set_active_format(fmt);
                        }
                    }
                }
            } else {
                self.press_start = None;
            }
        } else {
            // Mouse released
            if let Some((start_time, sector_idx)) = self.press_start {
                if start_time.elapsed() < Duration::from_millis(350) && self.hovered_carpel == Some(sector_idx) {
                    // Short tap: immediate conversion!
                    if let Some(fmt) = self.current_sectors.get(sector_idx) {
                        self.start_conversion(fmt);
                    }
                }
            }
            self.press_start = None;
        }

        // Center hub interaction
        if response.clicked() {
            if let Some(pos) = hover_pos {
                if self.geometry.hit_test_hub(pos) {
                    if let Some(path) = rfd::FileDialog::new().pick_file() {
                        self.select_file(path);
                    }
                }
            }
        }

        if response.secondary_clicked() || response.double_clicked() {
            // Right-click or double-click switches to full macOS Studio!
            self.switch_to_studio(ctx);
            return;
        }

        // 2. Render Hero Botanical Citrus Fruit Slice
        self.geometry.render(
            &painter,
            &self.current_sectors,
            self.hovered_carpel,
            &self.hub_status,
            self.is_converting,
            self.progress_angle,
        );

        // 3. Render Long-Press Floating macOS Popover if active
        if let Some(pop_idx) = self.popover_active_sector {
            let sector_span = (2.0 * std::f32::consts::PI) / 8.0;
            let angle = -std::f32::consts::PI / 2.0 + (pop_idx as f32 + 0.5) * sector_span;
            let r_mid = (self.geometry.carpel_outer + self.geometry.carpel_inner) / 2.0;
            let anchor_pos = Pos2::new(
                self.geometry.center.x + r_mid * angle.cos(),
                self.geometry.center.y + r_mid * angle.sin(),
            );

            let fmt_name = self.current_sectors.get(pop_idx).copied().unwrap_or("FMT");

            PetalPopover::render(
                ui,
                anchor_pos,
                fmt_name,
                &mut self.quick_settings,
                || self.popover_active_sector = None,
            );
        }
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

    // Start with frameless, transparent overlay window for the Sweet Lime Radial Widget
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title("Mossambi")
            .with_inner_size([360.0, 360.0])
            .with_min_inner_size([340.0, 340.0])
            .with_decorations(false) // Frameless window
            .with_transparent(true)  // Native borderless transparency
            .with_always_on_top(),
        ..Default::default()
    };

    eframe::run_native(
        "Mossambi",
        options,
        Box::new(|cc| Ok(Box::new(MossambiApp::new(cc)))),
    )
}
