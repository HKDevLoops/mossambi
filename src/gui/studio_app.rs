//! Mossambi Universal Desktop File Conversion Studio Application

use crate::archive::{pack_archive, unpack_archive, ArchiveType};
use crate::codec::{
    convert_text_to_pdf_vector, transcode_file, transcode_to_all_formats, LosslessMode,
    TextPdfSettings,
};
use crate::core::mouse_nav::{MouseNavEvent, MouseNavListener};
use crate::core::{AsyncWorkerPool, FileCategory, TaskUpdate, UniversalRegistry};
use crate::exam::{optimize_exam_media, ExamConstraint};
use crate::gui::macos_theme::*;
use crate::gui::quick_settings::QuickSettingsState;
use crate::gui::studio_view::StudioView;
use crate::gui::tray::{ActivationManager, AppControlEvent};
use eframe::egui::{self, Vec2, ViewportBuilder, ViewportCommand};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub enum ConversionResult {
    Success(Vec<PathBuf>),
    Failure(String),
}

pub struct MossambiApp {
    pub theme: ThemeStyle,
    pub quick_settings: QuickSettingsState,
    pub studio_view: StudioView,
    pub activation_manager: ActivationManager,

    // File Staging & Conversion
    pub current_file: Option<PathBuf>,
    pub current_category: FileCategory,
    pub current_sectors: Vec<&'static str>,
    pub is_converting: bool,
    pub last_completed_output: Option<PathBuf>,
    pub completion_time: Option<Instant>,
    pub status_message: String,

    // Background worker channels
    pub result_receiver: Receiver<ConversionResult>,
    pub result_sender: Sender<ConversionResult>,

    // Pooled background execution: conversions run through the shared
    // watchdog-supervised worker pool instead of ad-hoc thread::spawn.
    pub worker_pool: AsyncWorkerPool,

    // System-wide mouse thumb-button / swipe history navigation (Windows
    // low-level hook; a harmless no-op on other platforms).
    pub mouse_nav: MouseNavListener,

    // Windows DWM & Backdrop Sync
    pub frame_count: u64,
    pub last_applied_theme: Option<ThemeStyle>,
    pub last_applied_glass: Option<crate::core::GlassEffect>,

    // Ctrl+Mouse-Wheel UI zoom (Finder/Explorer parity), 1.0 = native scale
    pub ui_zoom: f32,
}

impl Default for MossambiApp {
    fn default() -> Self {
        let (tx, rx) = channel();
        let default_cat = FileCategory::Document;
        let default_sectors = UniversalRegistry::get_widget_sectors_for_category(default_cat);

        let cfg = crate::core::MossambiConfig::load();
        let current_theme = ThemeStyle::from(cfg.theme_mode);

        Self {
            theme: current_theme,
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

            worker_pool: AsyncWorkerPool::new(),
            mouse_nav: MouseNavListener::new(),

            frame_count: 0,
            last_applied_theme: None,
            last_applied_glass: None,
            ui_zoom: 1.0,
        }
    }
}

impl MossambiApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self::new_with_path(cc, None)
    }

    pub fn new_with_path(cc: &eframe::CreationContext<'_>, initial_path: Option<PathBuf>) -> Self {
        let cfg = crate::core::MossambiConfig::load();
        let current_theme = ThemeStyle::from(cfg.theme_mode);
        let palette = current_theme.colors();
        crate::gui::macos_theme::apply_theme_visuals(&cc.egui_ctx, &palette);

        let mut style = (*cc.egui_ctx.style()).clone();
        // Finder-grade compact metrics: tight button pills & calm 8px rhythm
        style.spacing.item_spacing = Vec2::new(8.0, 6.0);
        style.spacing.button_padding = Vec2::new(10.0, 5.0);
        style.spacing.indent = 16.0;
        cc.egui_ctx.set_style(style);

        // Configure Embedded Agave Nerd Font
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "AgaveNerdFont-Regular".to_owned(),
            egui::FontData::from_static(include_bytes!(
                "../../assets/fonts/AgaveNerdFont-Regular.ttf"
            )),
        );
        fonts.font_data.insert(
            "AgaveNerdFont-Bold".to_owned(),
            egui::FontData::from_static(include_bytes!(
                "../../assets/fonts/AgaveNerdFont-Bold.ttf"
            )),
        );
        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .insert(0, "AgaveNerdFont-Regular".to_owned());
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(1, "AgaveNerdFont-Regular".to_owned());
        cc.egui_ctx.set_fonts(fonts);

        let mut app = Self::default();
        if let Some(path) = initial_path {
            if path.is_dir() {
                app.studio_view.explorer_view.navigate_to(path);
            } else if path.is_file() {
                if let Some(parent) = path.parent() {
                    app.studio_view
                        .explorer_view
                        .navigate_to(parent.to_path_buf());
                }
                app.select_file(path);
            }
        }
        app
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

    /// Starts a conversion for the currently staged file (explicit user action).
    /// Delegates to [`start_conversion_batch`] — `studio_view.rs` calls this.
    pub fn start_conversion(&mut self, target_format: &str) {
        let files: Vec<PathBuf> = self.current_file.clone().into_iter().collect();
        self.start_conversion_batch(files, target_format);
    }

    /// Converts a batch of files through the worker pool, reporting one final
    /// `ConversionResult` per batch on the result channel. Explicit user
    /// action — always allowed, even when `background_work` is disabled
    /// (that knob only gates AUTO-triggered conversions).
    pub fn start_conversion_batch(&mut self, files: Vec<PathBuf>, target: &str) {
        if files.is_empty() {
            self.status_message = "Please drop or select a file first.".to_string();
            return;
        }

        // Double-start guard: a second click used to spawn two threads
        // clobbering the same output names.
        if self.is_converting {
            log::warn!(
                "Conversion already in progress — ignoring start request for {} file(s) -> {}",
                files.len(),
                target
            );
            self.status_message = "⏳ A conversion is already running — please wait."
                .to_string();
            return;
        }

        self.is_converting = true;
        self.status_message = format!(
            "Converting {} file(s) -> {}...",
            files.len(),
            target
        );
        log::info!("conversion batch: {} file(s) -> {}", files.len(), target);

        let sender = self.result_sender.clone();
        let target_str = target.to_string();
        let settings = self.quick_settings.clone();

        self.worker_pool.spawn_task("StudioConversion", move |cancel, tx, task_id| {
            let total = files.len();
            let mut outputs: Vec<PathBuf> = Vec::new();
            let mut first_error: Option<String> = None;

            for (idx, file) in files.iter().enumerate() {
                // Cooperative stop between files (the codec engine itself has
                // no cancellation hook yet).
                if cancel.load(Ordering::Relaxed) {
                    break;
                }
                match execute_conversion(file, &target_str, &settings) {
                    ConversionResult::Success(mut outs) => outputs.append(&mut outs),
                    ConversionResult::Failure(err) => {
                        if first_error.is_none() {
                            first_error = Some(format!("{}: {}", file.display(), err));
                        }
                    }
                }
                let _ = tx.send(TaskUpdate::Progress {
                    id: task_id,
                    fraction: (idx + 1) as f32 / total.max(1) as f32,
                    status: format!("Converted {}/{} file(s)", idx + 1, total),
                });
            }

            let result = match first_error {
                Some(err) => ConversionResult::Failure(err),
                None => ConversionResult::Success(outputs),
            };
            match &result {
                ConversionResult::Success(outs) => {
                    let _ = tx.send(TaskUpdate::Completed {
                        id: task_id,
                        output: outs.first().cloned(),
                        message: format!("Converted {} file(s) to {}", outs.len(), target_str),
                    });
                }
                ConversionResult::Failure(err) => {
                    let _ = tx.send(TaskUpdate::Failed {
                        id: task_id,
                        error: err.clone(),
                    });
                }
            }
            let _ = sender.send(result);
        });
    }

    /// AUTO-triggered conversion hook (reserved for programmatic converters).
    /// Honors `config.background_work == false` by refusing automatic work,
    /// while explicit user conversions always remain available.
    pub fn start_conversion_auto(&mut self, file: PathBuf, target: &str) {
        let cfg = crate::core::MossambiConfig::load();
        if !cfg.background_work {
            log::info!(
                "background work disabled — skipping AUTO conversion of {} ({})",
                file.display(),
                target
            );
            return;
        }
        self.start_conversion_batch(vec![file], target);
    }

    pub fn launch_radial_widget(&self) {
        self.launch_radial_widget_at(None, None);
    }

    pub fn launch_radial_widget_at(&self, pos: Option<(f32, f32)>, file: Option<&Path>) {
        let exe = std::env::current_exe().ok();
        let mut cmd = std::process::Command::new(exe.as_deref().unwrap_or_else(|| std::path::Path::new("mossambi.exe")));
        cmd.arg("widget");

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

pub fn execute_conversion(
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
            positioning: crate::exam::PositioningMode::CenterCrop,
            background_color: [255, 255, 255],
        };

        let out = crate::core::resolve_clean_sibling_output_path(input, "jpg", None);

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
            Err(e) => {
                return ConversionResult::Failure(format!("Batch ALL transcode failed: {}", e))
            }
        }
    }

    // 3. Vector Text-to-PDF Conversion
    let (cat, in_ext) = UniversalRegistry::classify_file(input);
    let is_text = matches!(
        in_ext.as_str(),
        "TXT"
            | "MD"
            | "RST"
            | "LOG"
            | "JSON"
            | "XML"
            | "YAML"
            | "YML"
            | "RS"
            | "PY"
            | "JS"
            | "TS"
            | "C"
            | "CPP"
            | "H"
            | "HTML"
            | "HTM"
            | "ORG"
            | "TEX"
    );

    if target_upper == "PDF" && (cat == FileCategory::Document || is_text) {
        let out_path = crate::core::resolve_clean_sibling_output_path(input, "pdf", None);
        let pdf_cfg = TextPdfSettings {
            page_size: settings.pdf_page_size.clone(),
            font_family: settings.pdf_font_family.clone(),
            font_size: settings.pdf_font_size,
            line_spacing: 1.35,
            margin_pt: settings.pdf_margin,
            show_header_footer: settings.pdf_header_footer,
            title: Some(
                input
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Document")
                    .to_string(),
            ),
        };

        match convert_text_to_pdf_vector(input, &out_path, Some(pdf_cfg)) {
            Ok(()) => return ConversionResult::Success(vec![out_path]),
            Err(e) => {
                return ConversionResult::Failure(format!("Vector PDF generation failed: {}", e))
            }
        }
    }

    // 4. Archive Extraction or Packing
    if target_upper == "EXTRACT" {
        let parent = input.parent().unwrap_or_else(|| Path::new("."));
        let stem = input
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("extracted");
        let dest = parent.join(stem);
        match unpack_archive(input, &dest) {
            Ok(()) => return ConversionResult::Success(vec![dest]),
            Err(e) => {
                return ConversionResult::Failure(format!("Archive extraction failed: {}", e))
            }
        }
    }

    if ["ZIP", "7Z", "TAR", "TAR.GZ", "TAR.XZ", "TAR.ZST"].contains(&target_upper.as_str()) {
        let out_archive =
            crate::core::resolve_clean_sibling_output_path(input, &target_upper, None);
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
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ── Ctrl + Mouse Wheel ───────────────────────────────────────────────
        // Plain Ctrl+wheel    → Explorer content zoom levels (0..=4, persisted)
        // Ctrl+0              → reset explorer zoom to the default level 2
        // Ctrl+Shift+wheel    → whole-UI pixel scale (Finder/Explorer parity)
        // Ctrl+Shift+0        → reset UI scale to native
        let (ui_zoom_scroll, ui_zoom_reset, explorer_scroll, explorer_reset) =
            ctx.input(|i| {
                let ctrl = i.modifiers.ctrl || i.modifiers.command;
                let shift = i.modifiers.shift;
                let scroll = if ctrl { i.raw_scroll_delta.y } else { 0.0 };
                let num0 = i.key_pressed(egui::Key::Num0);
                (
                    if ctrl && shift { scroll } else { 0.0 },
                    ctrl && shift && num0,
                    if ctrl && !shift { scroll } else { 0.0 },
                    ctrl && !shift && num0,
                )
            });
        if explorer_reset {
            self.studio_view.explorer_view.explorer_zoom = 2;
            log::debug!("explorer zoom reset to level 2 (Ctrl+0)");
        } else if explorer_scroll != 0.0 {
            let step = if explorer_scroll > 0.0 { 1 } else { -1 };
            let next = (self.studio_view.explorer_view.explorer_zoom as i32 + step).clamp(0, 4) as u8;
            if next != self.studio_view.explorer_view.explorer_zoom {
                log::debug!(
                    "explorer zoom level: {} -> {}",
                    self.studio_view.explorer_view.explorer_zoom,
                    next
                );
                self.studio_view.explorer_view.explorer_zoom = next;
            }
        } else if ui_zoom_reset {
            self.ui_zoom = 1.0;
            let native = ctx.native_pixels_per_point().unwrap_or(1.0).max(0.5);
            ctx.set_pixels_per_point(native);
            self.last_applied_theme = None;
        } else if ui_zoom_scroll != 0.0 {
            let step = if ui_zoom_scroll > 0.0 { 1.08_f32 } else { 1.0_f32 / 1.08 };
            self.ui_zoom = (self.ui_zoom * step).clamp(0.6, 2.5);
            let native = ctx.native_pixels_per_point().unwrap_or(1.0).max(0.5);
            ctx.set_pixels_per_point(native * self.ui_zoom);
            self.last_applied_theme = None;
        }

        // ── Mouse history navigation (thumb buttons / middle-drag swipe) ────
        while let Some(evt) = self.mouse_nav.try_recv() {
            let explorer = &mut self.studio_view.explorer_view;
            match evt {
                MouseNavEvent::Back => {
                    if explorer.can_go_back() {
                        log::debug!("mouse nav: back");
                        explorer.go_back();
                    }
                }
                MouseNavEvent::Forward => {
                    if explorer.can_go_forward() {
                        log::debug!("mouse nav: forward");
                        explorer.go_forward();
                    }
                }
            }
        }

        // Synchronize Windows DWM Backdrop (Acrylic / Mica) with active theme & glass effect
        self.frame_count = self.frame_count.saturating_add(1);
        let cfg = crate::core::MossambiConfig::load();
        // Sync icon pack from config (live)
        self.studio_view.explorer_view.icon_pack = crate::core::icon_packs::IconPack::from_config_id(&cfg.icon_pack);
        let current_glass = cfg.glass_effect;
        if self.frame_count <= 10
            || self.last_applied_theme != Some(self.theme)
            || self.last_applied_glass != Some(current_glass)
        {
            let palette = self.theme.colors();
            crate::gui::macos_theme::apply_theme_visuals(ctx, &palette);
            crate::gui::dwm_helper::apply_system_backdrop(self.theme.is_dark(), current_glass);
            self.last_applied_theme = Some(self.theme);
            self.last_applied_glass = Some(current_glass);
        }

        // Continuous driver honoring the Max-FPS knob: when enabled, drop
        // vsync-driven pacing and repaint at ~500Hz for maximum responsiveness.
        let is_animating = self.is_converting || self.completion_time.is_some();
        let repaint_interval = if cfg.max_fps {
            Duration::from_millis(2)
        } else if is_animating {
            Duration::from_millis(16)
        } else {
            Duration::from_millis(200)
        };
        ctx.request_repaint_after(repaint_interval);

        // Drain pooled worker updates (keeps the pool channel bounded and the
        // log informative; authoritative status comes from the result channel).
        while let Some(update) = self.worker_pool.try_recv() {
            match update {
                TaskUpdate::Started { name, .. } => log::debug!("worker started: {}", name),
                TaskUpdate::Progress {
                    fraction, status, ..
                } => log::debug!("worker progress {:.0}%: {}", fraction * 100.0, status),
                TaskUpdate::Completed { message, .. } => log::info!("worker finished: {}", message),
                TaskUpdate::Failed { error, .. } => log::error!("worker reported failure: {}", error),
            }
        }

        // 1. Check for activation events from global hotkey or system tray
        while let Some(control_event) = self.activation_manager.try_recv() {
            match control_event {
                AppControlEvent::Wakeup(event) => {
                    ctx.send_viewport_cmd(ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd(ViewportCommand::Focus);

                    if let Some(path) = event.active_file {
                        self.select_file(path);
                    }
                    self.status_message =
                        format!("Summoned at ({:.0}, {:.0})", event.cursor_x, event.cursor_y);
                }
                AppControlEvent::ChangeTheme(new_theme) => {
                    self.theme = new_theme;
                    let mut cfg = crate::core::MossambiConfig::load();
                    cfg.theme_mode = new_theme.into();
                    let _ = cfg.save();
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
                    self.launch_radial_widget_at(
                        Some((cursor_x, cursor_y)),
                        active_file.as_deref(),
                    );
                }
                AppControlEvent::ToggleAutostart(enabled) => {
                    let mut cfg = crate::core::MossambiConfig::load();
                    cfg.launch_at_login = enabled;
                    let _ = cfg.save();
                }
                AppControlEvent::QuitApp => {
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                }
            }
        }

        // 2. Poll for background conversion results (drain the WHOLE queue —
        // batch conversions may stack several results between frames)
        while let Ok(res) = self.result_receiver.try_recv() {
            self.is_converting = false;
            match res {
                ConversionResult::Success(files) => {
                    self.completion_time = Some(Instant::now());
                    if let Some(first) = files.first() {
                        self.last_completed_output = Some(first.clone());
                    }
                    self.status_message =
                        format!("✨ Successfully converted {} file(s)!", files.len());
                }
                ConversionResult::Failure(err) => {
                    log::error!("Studio conversion failed: {}", err);
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
                            use windows_sys::Win32::Foundation::POINT;
                            use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;
                            let mut pt = POINT { x: 0, y: 0 };
                            unsafe { GetCursorPos(&mut pt) };
                            crate::gui::active_file::get_active_file_under_cursor(pt.x, pt.y)
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

        // 4. Render Desktop Studio Application Layout
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

        // 6. Persist any explorer preference mutations from this frame
        // (icon pack, zoom level, checkboxes, menu style, animations) into the
        // config — same save pattern as the theme-change handlers above.
        self.sync_explorer_prefs();
    }
}

impl MossambiApp {
    /// Dirty-checks the live [`StudioView`]'s explorer preference fields
    /// against the persisted config and saves only when something changed.
    fn sync_explorer_prefs(&self) {
        let explorer = &self.studio_view.explorer_view;
        let mut cfg = crate::core::MossambiConfig::load();
        let mut dirty = false;

        let pack_id = explorer.icon_pack.id().to_string();
        if cfg.icon_pack != pack_id {
            cfg.icon_pack = pack_id;
            dirty = true;
        }
        if cfg.explorer_zoom != explorer.explorer_zoom {
            cfg.explorer_zoom = explorer.explorer_zoom;
            dirty = true;
        }
        if cfg.checkbox_mode != explorer.checkbox_mode {
            cfg.checkbox_mode = explorer.checkbox_mode;
            dirty = true;
        }
        if cfg.menu_style != explorer.menu_style {
            cfg.menu_style = explorer.menu_style;
            dirty = true;
        }
        if cfg.animations != explorer.animations {
            cfg.animations = explorer.animations;
            dirty = true;
        }

        if dirty {
            log::debug!(
                "explorer prefs changed — saving config (zoom={}, pack={})",
                cfg.explorer_zoom,
                cfg.icon_pack
            );
            let _ = cfg.save();
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

pub fn load_app_icon() -> Option<egui::IconData> {
    let icon_bytes = include_bytes!("../../assets/icons/mossambi-256x256.png");
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

/// Runs the complete native macOS-Styled Mossambi Studio GUI Application
pub fn run_studio() -> Result<(), eframe::Error> {
    run_studio_with_path(None)
}

/// Runs the complete native macOS-Styled Mossambi Studio GUI Application with an initial path
pub fn run_studio_with_path(initial_path: Option<PathBuf>) -> Result<(), eframe::Error> {
    let _ = env_logger::try_init();

    let mut builder = ViewportBuilder::default()
        .with_title("Mossambi — Universal File Conversion Studio")
        .with_inner_size([1120.0, 720.0])
        .with_min_inner_size([920.0, 580.0])
        .with_decorations(false) // Undecorated: eliminate duplicate OS titlebar!
        .with_transparent(true)
        .with_resizable(true);

    if let Some(icon) = load_app_icon() {
        builder = builder.with_icon(icon);
    }

    // Honor the Max-FPS knob: when enabled, uncap the frame driver by
    // disabling vsync (repaint pacing is handled per-frame in update()).
    let max_fps = crate::core::MossambiConfig::load().max_fps;
    let options = eframe::NativeOptions {
        viewport: builder,
        vsync: !max_fps,
        ..Default::default()
    };

    eframe::run_native(
        "Mossambi — Universal File Conversion Studio",
        options,
        Box::new(move |cc| Ok(Box::new(MossambiApp::new_with_path(cc, initial_path)))),
    )
}
