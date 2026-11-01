//! Mossambi 100% Apple macOS-Styled Desktop Suite (GUI & Radial Citrus Widget)
//! Pure Native Rust Application
//! - Hero standalone circular Sweet Lime Citrus Widget (no ugly attached dashboard)
//! - Long-Press (>=350ms) on any converter petal triggers an anchored floating macOS popover
//! - 100% macOS designed Mossambi Studio window with traffic lights, sidebar, and Indian Exam Studio
//! - Fast UNIX mv-like CLI, TUI, and global WIN+CTRL & circle-drag activation hooks
//! - 0.00% Zero-Loss Standard, Hardware Acceleration (GPU + NPU), 16 Rayon worker threads

use eframe::egui::{self, Color32, Key, Pos2, Rect, Vec2, ViewportBuilder, ViewportCommand};
use mossambi_archive::{pack_archive, unpack_archive, ArchiveType};
use mossambi_codec::{
    convert_text_to_pdf_vector, transcode_file, transcode_to_all_formats, LosslessMode,
    TextPdfSettings,
};
use mossambi_core::{FileCategory, MossambiConfig, UniversalRegistry};
use mossambi_exam::{optimize_exam_media, ExamConstraint};
use mossambi_gui::citrus_geometry::{CitrusGeometry, CitrusVariety};
use mossambi_gui::macos_theme::*;
use mossambi_gui::popover::PetalPopover;
use mossambi_gui::quick_settings::QuickSettingsState;
use mossambi_gui::studio_view::StudioView;
use mossambi_gui::tray::{ActivationManager, AppControlEvent};
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
    theme: ThemeStyle,
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
    completion_time: Option<Instant>,
    status_message: String,

    // Long-Press Tracking (>=350ms triggers Quick Settings popover)
    press_start: Option<(Instant, usize)>,
    popover_active_sector: Option<usize>,

    // Format Catalogue Modal
    pub show_catalogue_modal: bool,
    pub catalogue_search_query: String,

    // Background worker channels
    result_receiver: Receiver<ConversionResult>,
    result_sender: Sender<ConversionResult>,
}

impl Default for MossambiApp {
    fn default() -> Self {
        let (tx, rx) = channel();
        let default_cat = FileCategory::Document;
        let default_sectors = UniversalRegistry::get_widget_sectors_for_category(default_cat);

        let config = MossambiConfig::load();
        let variety = match config.citrus_shade.to_lowercase().as_str() {
            "tangerine" | "mandarin" => CitrusVariety::MandarinTangerine,
            "grapefruit" | "ruby" => CitrusVariety::RubyGrapefruit,
            "persian" | "persianlime" | "kaffirlime" => CitrusVariety::PersianLime,
            _ => CitrusVariety::SweetLime,
        };

        let mut geometry = CitrusGeometry::new(Pos2::new(180.0, 180.0), 320.0);
        geometry.variety = variety;

        Self {
            mode: AppViewMode::Studio,
            theme: ThemeStyle::LiquidGlass,
            geometry,
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
            completion_time: None,
            status_message: "Ready. WIN+CTRL or drag in circle to summon.".to_string(),

            press_start: None,
            popover_active_sector: None,

            show_catalogue_modal: false,
            catalogue_search_query: String::new(),

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
        self.current_file = Some(path.clone());
        self.current_category = cat;
        self.current_sectors = UniversalRegistry::get_sectors_for_file(&path);
        self.hub_status = if ext.is_empty() { "FILE".to_string() } else { ext };
        if let Some(first_fmt) = self.current_sectors.first() {
            self.quick_settings.set_active_format(first_fmt);
        }
    }

    pub fn start_conversion(&mut self, target_format: &str) {
        if target_format == "MORE" {
            self.show_catalogue_modal = true;
            return;
        }

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
        ctx.send_viewport_cmd(ViewportCommand::InnerSize(Vec2::new(1040.0, 680.0)));
        ctx.send_viewport_cmd(ViewportCommand::Decorations(true));
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

    pub fn switch_to_widget(&mut self, ctx: &egui::Context) {
        self.launch_radial_widget();
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
        // Adaptive repaint rate: 16ms (60 FPS) during conversion, active celebration, or long-press,
        // and 200ms when idle to eliminate battery/CPU churn on idle desktops.
        let is_animating = self.is_converting
            || self.completion_time.is_some()
            || self.press_start.is_some();
        let repaint_interval = if is_animating {
            Duration::from_millis(16)
        } else {
            Duration::from_millis(200)
        };
        ctx.request_repaint_after(repaint_interval);

        // 1. Check for activation events from global hotkey, 2-circle gesture, or system tray
        while let Some(control_event) = self.activation_manager.try_recv() {
            match control_event {
                AppControlEvent::Wakeup(event) => {
                    ctx.send_viewport_cmd(ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd(ViewportCommand::Focus);

                    if self.mode == AppViewMode::RadialWidget {
                        let win_x = (event.cursor_x - 180.0).max(0.0);
                        let win_y = (event.cursor_y - 180.0).max(0.0);
                        ctx.send_viewport_cmd(ViewportCommand::OuterPosition(Pos2::new(win_x, win_y)));
                    }

                    // Directly stage active file identified under cursor!
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
                AppControlEvent::ToggleZeroLoss(_enabled) => {
                    // Quick settings zero loss toggle
                }
                AppControlEvent::OpenStudio => {
                    self.switch_to_studio(ctx);
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
                    self.hub_status = "DONE".to_string();
                    self.completion_time = Some(Instant::now());
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

        // Auto-fade celebration morph after 5 seconds of inactivity
        if let Some(completed_at) = self.completion_time {
            if completed_at.elapsed() >= Duration::from_secs(5) {
                self.completion_time = None;
                if self.hub_status == "DONE" {
                    self.hub_status = self
                        .current_file
                        .as_ref()
                        .and_then(|f| f.extension())
                        .and_then(|e| e.to_str())
                        .unwrap_or("READY")
                        .to_string();
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

        // 4. Handle Drag-and-Drop Files and Radial Keyboard Shortcuts
        ctx.input(|i| {
            // A. Capture active hovered file while user drags cursor over widget!
            if let Some(hover) = i.raw.hovered_files.first() {
                if let Some(ref path) = hover.path {
                    if self.current_file.as_ref() != Some(path) {
                        self.select_file(path.clone());
                    }
                } else {
                    // On Windows, OLE masks path during DragOver; query file under cursor from active Explorer!
                    #[cfg(target_os = "windows")]
                    {
                        use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;
                        use windows_sys::Win32::Foundation::POINT;
                        let mut pt = POINT { x: 0, y: 0 };
                        unsafe { GetCursorPos(&mut pt) };
                        if let Some(active_path) = mossambi_gui::active_file::get_active_file_under_cursor(pt.x, pt.y) {
                            if self.current_file.as_ref() != Some(&active_path) {
                                self.select_file(active_path);
                            }
                        }
                    }
                }
            }

            // B. Capture dropped file & instantly convert on petal drop!
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
                        self.select_file(path.clone());

                        let drop_pos = i.pointer.hover_pos()
                            .or_else(|| i.pointer.latest_pos())
                            .unwrap_or(self.geometry.center);

                        // If dropped onto a specific carpel sector, convert immediately!
                        let carpel_idx = self.geometry.hit_test_carpel(drop_pos).or(self.hovered_carpel);
                        if let Some(idx) = carpel_idx {
                            if let Some(target) = self.current_sectors.get(idx).copied() {
                                self.start_conversion(target);
                            }
                        } else if self.geometry.hit_test_hub(drop_pos) {
                            // Center hub drop: convert to top format for this file
                            if let Some(first_fmt) = self.current_sectors.first().copied() {
                                self.start_conversion(first_fmt);
                            }
                        }
                    }
                }
            }

            // C. Radial Keyboard Shortcuts (Keys 1-8 for sectors, Tab to cycle, Space/Enter to convert)
            let num_keys = [
                Key::Num1, Key::Num2, Key::Num3, Key::Num4,
                Key::Num5, Key::Num6, Key::Num7, Key::Num8,
            ];
            for (idx, key) in num_keys.iter().enumerate() {
                if i.key_pressed(*key) {
                    self.hovered_carpel = Some(idx);
                    if let Some(target) = self.current_sectors.get(idx) {
                        self.quick_settings.set_active_format(target);
                        self.start_conversion(target);
                    }
                }
            }

            if i.key_pressed(Key::Tab) {
                let next_idx = match self.hovered_carpel {
                    Some(cur) => (cur + 1) % 8,
                    None => 0,
                };
                self.hovered_carpel = Some(next_idx);
                if let Some(fmt) = self.current_sectors.get(next_idx) {
                    self.quick_settings.set_active_format(fmt);
                }
            }

            if i.key_pressed(Key::Space) || i.key_pressed(Key::Enter) {
                if let Some(carpel_idx) = self.hovered_carpel {
                    if let Some(target) = self.current_sectors.get(carpel_idx) {
                        self.start_conversion(target);
                    }
                } else if let Some(first_fmt) = self.current_sectors.first() {
                    self.start_conversion(first_fmt);
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
                let theme_palette = self.theme.colors();
                egui::CentralPanel::default()
                    .frame(egui::Frame::none().fill(theme_palette.window_bg))
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
                            &mut self.theme,
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

        // 4. Render Format Catalogue Modal if requested via MORE sector
        if self.show_catalogue_modal {
            let mut close_modal = false;
            let mut selected_fmt: Option<String> = None;

            egui::Window::new("🔍 Mossambi Format Catalogue (200+ Formats)")
                .collapsible(false)
                .resizable(false)
                .fixed_size(Vec2::new(330.0, 310.0))
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Search:");
                        ui.text_edit_singleline(&mut self.catalogue_search_query);
                        if ui.button("✖ Close").clicked() {
                            close_modal = true;
                        }
                    });
                    ui.separator();
                    egui::ScrollArea::vertical().max_height(230.0).show(ui, |ui| {
                        let results = UniversalRegistry::search_catalogue(&self.catalogue_search_query);
                        for spec in results {
                            ui.horizontal(|ui| {
                                let badge_color = match spec.nature {
                                    mossambi_core::FormatNature::Lossless => Color32::from_rgb(40, 180, 40),
                                    mossambi_core::FormatNature::Uncompressed => Color32::from_rgb(0, 160, 220),
                                    mossambi_core::FormatNature::Lossy => Color32::from_rgb(230, 130, 20),
                                    mossambi_core::FormatNature::Specialized => Color32::from_rgb(170, 90, 210),
                                };

                                if ui.button(egui::RichText::new(spec.extension).strong()).clicked() {
                                    selected_fmt = Some(spec.extension.to_string());
                                }
                                ui.colored_label(badge_color, format!("[{}]", spec.nature.badge_label()));
                                ui.label(egui::RichText::new(spec.display_name).size(11.0));
                            });
                        }
                    });
                });

            if close_modal {
                self.show_catalogue_modal = false;
            }
            if let Some(fmt) = selected_fmt {
                self.show_catalogue_modal = false;
                self.start_conversion(&fmt);
            }
        }

        // 5. Celebration Morph: Action Pills ("Reveal in Explorer", "Copy Path")
        if self.hub_status == "DONE" {
            if let Some(ref completed_path) = self.last_completed_output {
                let pill_rect = Rect::from_center_size(
                    Pos2::new(self.geometry.center.x, self.geometry.center.y + 118.0),
                    Vec2::new(280.0, 34.0),
                );
                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(pill_rect), |ui| {
                    ui.horizontal_centered(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(8.0, 0.0);
                        let reveal_btn = ui.add(
                            egui::Button::new(
                                egui::RichText::new("🔍 Reveal").size(11.0).color(Color32::WHITE),
                            )
                            .fill(Color32::from_rgba_premultiplied(20, 20, 24, 220))
                            .rounding(16.0),
                        );
                        if reveal_btn.clicked() {
                            open_in_file_manager(completed_path);
                        }

                        let copy_btn = ui.add(
                            egui::Button::new(
                                egui::RichText::new("📋 Copy Path").size(11.0).color(Color32::WHITE),
                            )
                            .fill(Color32::from_rgba_premultiplied(20, 20, 24, 220))
                            .rounding(16.0),
                        );
                        if copy_btn.clicked() {
                            ctx.copy_text(completed_path.display().to_string());
                            self.status_message = "Path copied to clipboard!".to_string();
                        }
                    });
                });
            }
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

    // Launch as the full Desktop Application Workspace window
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title("Mossambi — Universal File Conversion Studio")
            .with_inner_size([1040.0, 680.0])
            .with_min_inner_size([880.0, 560.0])
            .with_decorations(true) // Standard desktop application window
            .with_resizable(true),
        ..Default::default()
    };

    eframe::run_native(
        "Mossambi — Universal File Conversion Studio",
        options,
        Box::new(|cc| Ok(Box::new(MossambiApp::new(cc)))),
    )
}
