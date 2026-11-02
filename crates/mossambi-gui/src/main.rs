//! Mossambi Universal File Conversion Studio GUI Application
//! Pure Native Rust Application
//! - 100% Modern Desktop Application Studio with traffic lights, sidebar, and Indian Exam Studio
//! - Universal Transcoder with rich Hero Landing state (Dropzone, Quick-Start Workflows, 200+ format matrix)
//! - Quality & Integrity Inspector (Container Integrity, SHA-256/CRC-32 Streaming, 4-Dimension Compatibility)
//! - Dedicated Indian Competitive Exam Portal Conformer (UPSC, SSC, IBPS, NTA, State PSC)
//! - Fast UNIX mv-like CLI, TUI, and global WIN+SHIFT+C (Fruit Widget) & WIN+CTRL+C (Studio) triggers
//! - 0.00% Zero-Loss Standard, Hardware Acceleration (GPU + NPU), 16 Rayon worker threads

use eframe::egui::{self, Color32, Rounding, Stroke, Vec2, ViewportBuilder, ViewportCommand};
use mossambi_archive::{pack_archive, unpack_archive, ArchiveType};
use mossambi_codec::{
    convert_text_to_pdf_vector, transcode_file, transcode_to_all_formats, LosslessMode,
    TextPdfSettings,
};
use mossambi_core::{FileCategory, UniversalRegistry};
use mossambi_exam::{optimize_exam_media, ExamConstraint};
use mossambi_gui::macos_theme::*;
use mossambi_gui::quick_settings::QuickSettingsState;
use mossambi_gui::studio_view::StudioView;
use mossambi_gui::tray::{ActivationManager, AppControlEvent};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub enum ConversionResult {
    Success(Vec<PathBuf>),
    Failure(String),
}

pub struct MossambiApp {
    theme: ThemeStyle,
    quick_settings: QuickSettingsState,
    studio_view: StudioView,
    activation_manager: ActivationManager,

    // File Staging & Conversion
    current_file: Option<PathBuf>,
    current_category: FileCategory,
    current_sectors: Vec<&'static str>,
    is_converting: bool,
    last_completed_output: Option<PathBuf>,
    completion_time: Option<Instant>,
    status_message: String,

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
            theme: ThemeStyle::DarkAnthracite,
            quick_settings: QuickSettingsState::default(),
            studio_view: StudioView::default(),
            activation_manager: ActivationManager::new(),

            current_file: None,
            current_category: default_cat,
            current_sectors: default_sectors,
            is_converting: false,
            last_completed_output: None,
            completion_time: None,
            status_message: "Ready. Drop any file or select a preset to begin.".to_string(),

            result_receiver: rx,
            result_sender: tx,
        }
    }
}

impl MossambiApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Modern, high-end Dark Slate / Zinc Visuals (Eliminates retro XP look!)
        let mut visuals = egui::Visuals::dark();
        visuals.window_fill = Color32::from_rgb(15, 17, 23); // Modern deep slate
        visuals.panel_fill = Color32::from_rgb(15, 17, 23);
        visuals.faint_bg_color = Color32::from_rgb(22, 26, 36);
        visuals.extreme_bg_color = Color32::from_rgb(11, 13, 18);

        visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(22, 26, 36);
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(180, 186, 200));
        visuals.widgets.noninteractive.rounding = Rounding::same(8.0);

        visuals.widgets.inactive.bg_fill = Color32::from_rgb(26, 30, 42);
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(215, 220, 232));
        visuals.widgets.inactive.rounding = Rounding::same(8.0);

        visuals.widgets.hovered.bg_fill = Color32::from_rgb(38, 44, 60);
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.2_f32, Color32::WHITE);
        visuals.widgets.hovered.rounding = Rounding::same(8.0);

        visuals.widgets.active.bg_fill = Color32::from_rgb(120, 190, 32); // Pantone 376 C
        visuals.widgets.active.fg_stroke = Stroke::new(1.2_f32, Color32::WHITE);
        visuals.widgets.active.rounding = Rounding::same(8.0);

        visuals.selection.bg_fill = Color32::from_rgba_premultiplied(120, 190, 32, 100);
        visuals.selection.stroke = Stroke::new(1.0_f32, Color32::from_rgb(120, 190, 32));

        cc.egui_ctx.set_visuals(visuals);

        let mut style = (*cc.egui_ctx.style()).clone();
        style.spacing.item_spacing = Vec2::new(10.0, 8.0);
        style.spacing.button_padding = Vec2::new(14.0, 7.0);
        style.spacing.indent = 16.0;
        cc.egui_ctx.set_style(style);

        Self::default()
    }

    pub fn select_file(&mut self, path: PathBuf) {
        let (cat, _ext) = UniversalRegistry::classify_file(&path);
        self.current_file = Some(path.clone());
        self.current_category = cat;
        self.current_sectors = UniversalRegistry::get_sectors_for_file(&path);
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
        self.status_message = format!(
            "Converting '{}' -> {}...",
            input_file.file_name().unwrap_or_default().to_string_lossy(),
            target_format
        );

        let sender = self.result_sender.clone();
        let target_str = target_format.to_string();
        let settings = self.quick_settings.clone();

        thread::spawn(move || {
            let res = execute_conversion(&input_file, &target_str, &settings);
            let _ = sender.send(res);
        });
    }

    pub fn launch_radial_widget(&self) {
        self.launch_radial_widget_at(None, None);
    }

    pub fn launch_radial_widget_at(&self, pos: Option<(f32, f32)>, file: Option<&Path>) {
        #[cfg(target_os = "windows")]
        let bin = "mossambi-widget.exe";
        #[cfg(not(target_os = "windows"))]
        let bin = "mossambi-widget";

        let mut cmd = std::process::Command::new(bin);
        if let Ok(exe) = std::env::current_exe() {
            if let Some(parent) = exe.parent() {
                let sibling = parent.join(bin);
                if sibling.exists() {
                    cmd = std::process::Command::new(sibling);
                }
            }
        }

        if let Some((x, y)) = pos {
            cmd.arg("--pos").arg(format!("{:.0},{:.0}", x, y));
        }
        if let Some(f) = file {
            cmd.arg("--active-drag");
            cmd.arg(f);
        }
        let _ = cmd.spawn();
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
            positioning: mossambi_exam::PositioningMode::CenterCrop,
            background_color: [255, 255, 255],
        };

        let out = mossambi_core::resolve_clean_sibling_output_path(input, "jpg", None);

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
        let out_path = mossambi_core::resolve_clean_sibling_output_path(input, "pdf", None);
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
        let out_archive = mossambi_core::resolve_clean_sibling_output_path(input, &target_upper, None);
        let arch_type = ArchiveType::from_extension(&target_upper).unwrap_or(ArchiveType::Zip);
        match pack_archive(&[input.to_path_buf()], &out_archive, arch_type) {
            Ok(()) => return ConversionResult::Success(vec![out_archive]),
            Err(e) => return ConversionResult::Failure(format!("Archive packing failed: {}", e)),
        }
    }

    // 5. Standard Media Transcode (Bit-exact lossless standard)
    let mode = if settings.image_lossless || settings.audio_strict_lossless {
        LosslessMode::StrictBitExact
    } else {
        LosslessMode::PerceptualHighFidelity
    };

    match transcode_file(input, &target_upper, mode, None) {
        Ok(out) => ConversionResult::Success(vec![out]),
        Err(e) => ConversionResult::Failure(e.to_string()),
    }
}

impl eframe::App for MossambiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let is_animating = self.is_converting || self.completion_time.is_some();
        let repaint_interval = if is_animating {
            Duration::from_millis(16)
        } else {
            Duration::from_millis(200)
        };
        ctx.request_repaint_after(repaint_interval);

        // 1. Check for activation events from global hotkey or system tray
        while let Some(control_event) = self.activation_manager.try_recv() {
            match control_event {
                AppControlEvent::Wakeup(event) => {
                    ctx.send_viewport_cmd(ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd(ViewportCommand::Focus);

                    if let Some(path) = event.active_file {
                        self.select_file(path);
                    }
                    self.status_message = format!("Summoned at ({:.0}, {:.0})", event.cursor_x, event.cursor_y);
                }
                AppControlEvent::ChangeTheme(new_theme) => {
                    self.theme = new_theme;
                }
                AppControlEvent::ChangeExamPreset(preset_name) => {
                    self.quick_settings.exam_mode_enabled = true;
                    self.quick_settings.exam_preset_name = preset_name;
                }
                AppControlEvent::ToggleZeroLoss(_enabled) => {}
                AppControlEvent::OpenStudio => {
                    ctx.send_viewport_cmd(ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd(ViewportCommand::Focus);
                }
                AppControlEvent::SummonWidget {
                    cursor_x,
                    cursor_y,
                    active_file,
                } => {
                    self.launch_radial_widget_at(Some((cursor_x, cursor_y)), active_file.as_deref());
                }
                AppControlEvent::ToggleAutostart(enabled) => {
                    let mut cfg = mossambi_core::MossambiConfig::load();
                    cfg.launch_at_login = enabled;
                    let _ = cfg.save();
                }
                AppControlEvent::QuitApp => {
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                }
            }
        }

        // 2. Poll for background conversion results
        if let Ok(res) = self.result_receiver.try_recv() {
            self.is_converting = false;
            match res {
                ConversionResult::Success(files) => {
                    self.completion_time = Some(Instant::now());
                    if let Some(first) = files.first() {
                        self.last_completed_output = Some(first.clone());
                    }
                    self.status_message = format!("✨ Successfully converted {} file(s)!", files.len());
                }
                ConversionResult::Failure(err) => {
                    self.status_message = format!("❌ {}", err);
                }
            }
        }

        // Auto-fade status after 6 seconds of completion
        if let Some(completed_at) = self.completion_time {
            if completed_at.elapsed() >= Duration::from_secs(6) {
                self.completion_time = None;
            }
        }

        // 3. Handle Drag-and-Drop Files directly into Studio window
        ctx.input(|i| {
            if let Some(hover) = i.raw.hovered_files.first() {
                if let Some(ref path) = hover.path {
                    if self.current_file.as_ref() != Some(path) {
                        self.select_file(path.clone());
                    }
                }
            }

            if !i.raw.dropped_files.is_empty() {
                if let Some(drop) = i.raw.dropped_files.first() {
                    let dropped_path = drop.path.clone().or_else(|| {
                        #[cfg(target_os = "windows")]
                        {
                            use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;
                            use windows_sys::Win32::Foundation::POINT;
                            let mut pt = POINT { x: 0, y: 0 };
                            unsafe { GetCursorPos(&mut pt) };
                            mossambi_gui::active_file::get_active_file_under_cursor(pt.x, pt.y)
                        }
                        #[cfg(not(target_os = "windows"))]
                        None
                    });

                    if let Some(path) = dropped_path {
                        self.select_file(path);
                    }
                }
            }
        });

        // 4. Render Desktop Studio Application Layout (SidePanel + CentralPanel)
        let cur_file = self.current_file.clone();
        let mut qs = self.quick_settings.clone();

        let mut file_to_pick = false;
        let mut file_to_stage: Option<PathBuf> = None;
        let mut convert_to_target: Option<String> = None;
        let mut open_target_folder: Option<PathBuf> = None;

        self.studio_view.render(
            ctx,
            &mut self.theme,
            &cur_file,
            &mut qs,
            || file_to_pick = true,
            |p| file_to_stage = Some(p),
            |t| convert_to_target = Some(t),
            |p| open_target_folder = Some(p.to_path_buf()),
        );

        self.quick_settings = qs;

        if let Some(path) = file_to_stage {
            self.select_file(path);
        }

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

fn load_app_icon() -> Option<egui::IconData> {
    let icon_bytes = include_bytes!("../../../assets/icons/mossambi-256x256.png");
    if let Ok(img) = image::load_from_memory(icon_bytes) {
        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();
        Some(egui::IconData {
            rgba: rgba.into_raw(),
            width: w,
            height: h,
        })
    } else {
        None
    }
}

fn main() -> Result<(), eframe::Error> {
    env_logger::init();

    let mut builder = ViewportBuilder::default()
        .with_title("Mossambi — Universal File Conversion Studio")
        .with_inner_size([1120.0, 720.0])
        .with_min_inner_size([920.0, 580.0])
        .with_decorations(true) // Full Desktop Studio Window
        .with_resizable(true);

    if let Some(icon) = load_app_icon() {
        builder = builder.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport: builder,
        ..Default::default()
    };

    eframe::run_native(
        "Mossambi — Universal File Conversion Studio",
        options,
        Box::new(|cc| Ok(Box::new(MossambiApp::new(cc)))),
    )
}
