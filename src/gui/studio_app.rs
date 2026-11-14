//! Mossambi Universal Desktop File Conversion Studio Application

use crate::archive::{pack_archive, unpack_archive, ArchiveType};
use crate::codec::{
    convert_text_to_pdf_vector, probe_media_duration, transcode_file_ctx, transcode_to_all_formats,
    EncodeHandle, LosslessMode, TextPdfSettings,
};
use crate::core::conversion_queue::QueueAction;
use crate::core::mouse_nav::{MouseNavEvent, MouseNavListener};
use crate::core::{AsyncWorkerPool, FileCategory, ItemOutcome, TaskUpdate, UniversalRegistry};
use crate::exam::{optimize_exam_media_ctx, ExamConstraint};
use crate::gui::macos_theme::*;
use crate::gui::quick_settings::QuickSettingsState;
use crate::gui::studio_view::{StudioTab, StudioView};
use crate::gui::tray::{ActivationManager, AppControlEvent};
use eframe::egui::{self, Vec2, ViewportBuilder, ViewportCommand};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, OnceLock};
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

    // Cancel token for the ACTIVE conversion run (W-7): set by the queue
    // panel's Cancel button; the encode stops gracefully, then is killed.
    pub active_cancel: Option<Arc<AtomicBool>>,

    // File-picker dialog runs on a plain background thread (W-7 Batch B):
    // rfd blocks its own thread, never the UI frame.
    pub picker_sender: Sender<Option<PathBuf>>,
    pub picker_receiver: Receiver<Option<PathBuf>>,
    pub picker_in_flight: bool,

    // System-wide mouse thumb-button / swipe history navigation (Windows
    // low-level hook; a harmless no-op on other platforms).
    pub mouse_nav: MouseNavListener,

    // Windows DWM & Backdrop Sync
    pub frame_count: u64,
    pub last_applied_theme: Option<ThemeStyle>,
    pub last_applied_glass: Option<crate::core::GlassEffect>,

    // Cached persistent configuration (avoids reading disk every frame)
    pub config: crate::core::MossambiConfig,

    // Ctrl+Mouse-Wheel UI zoom (Finder/Explorer parity), 1.0 = native scale
    pub ui_zoom: f32,

    // Mirror of cfg.grid_icon_size so the config-to-view sync only fires
    // when the Preferences knob actually changes (does not fight the
    // toolbar/status-bar sliders, which write explorer_view.icon_size).
    pub last_grid_icon_size: u32,
    /// Last explorer-pref values both sides agreed on; tells which side
    /// (Preferences/config or the explorer UI) changed since.
    pub explorer_pref_mirror: Option<ExplorerPrefMirror>,
}

impl Default for MossambiApp {
    fn default() -> Self {
        let (tx, rx) = channel();
        let (pick_tx, pick_rx) = channel();
        let default_cat = FileCategory::Document;
        let default_sectors = UniversalRegistry::get_widget_sectors_for_category(default_cat);

        let cfg = crate::core::MossambiConfig::load();
        let current_theme = ThemeStyle::from(cfg.theme_mode);

        Self {
            theme: current_theme,
            quick_settings: QuickSettingsState::from_config(&cfg),
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
            active_cancel: None,
            picker_sender: pick_tx,
            picker_receiver: pick_rx,
            picker_in_flight: false,
            mouse_nav: MouseNavListener::new(),

            frame_count: 0,
            last_applied_theme: None,
            last_applied_glass: None,
            config: cfg,
            ui_zoom: 1.0,
            last_grid_icon_size: 0,
            explorer_pref_mirror: None,
        }
    }
}

impl MossambiApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self::new_with_path(cc, None)
    }

    pub fn new_with_path(cc: &eframe::CreationContext<'_>, initial_path: Option<PathBuf>) -> Self {
        log::info!("MossambiApp::new_with_path entry");
        let cfg = crate::core::MossambiConfig::load();
        let current_theme = ThemeStyle::from(cfg.theme_mode);
        let palette = current_theme.colors();
        crate::gui::macos_theme::apply_theme_visuals(&cc.egui_ctx, &palette);

        let mut style = (*cc.egui_ctx.style()).clone();
        // Finder-grade compact metrics: tight button pills & calm 8px rhythm
        style.spacing.item_spacing = Vec2::new(8.0, 6.0);
        style.spacing.button_padding = Vec2::new(10.0, 5.0);
        style.spacing.indent = 16.0;
        // Cursor discipline (strict rule): the I-beam text cursor is allowed
        // ONLY in text-editing widgets. egui 0.29 defaults both flags to true,
        // which paints CursorIcon::Text over every static label app-wide
        // (context menus included).
        style.interaction.selectable_labels = false;
        style.interaction.multi_widget_text_select = false;
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
        if let Ok(tab_str) = std::env::var("MOSSAMBI_INITIAL_TAB") {
            match tab_str.to_lowercase().as_str() {
                "preferences" | "prefs" | "settings" => {
                    app.studio_view.current_tab = crate::gui::studio_view::StudioTab::Preferences;
                    if let Ok(cat_str) = std::env::var("MOSSAMBI_PREFS_CAT") {
                        if let Ok(idx) = cat_str.parse::<usize>() {
                            app.studio_view.prefs_view.selected = idx;
                        }
                    }
                }
                "transcoder" | "transcode" => {
                    app.studio_view.current_tab = crate::gui::studio_view::StudioTab::Transcoder;
                }
                "customconformer" | "conformer" => {
                    app.studio_view.current_tab = crate::gui::studio_view::StudioTab::ExamStudio;
                }
                "inspector" | "quality" => {
                    app.studio_view.current_tab =
                        crate::gui::studio_view::StudioTab::QualityInspector;
                }
                "archive" | "archives" => {
                    app.studio_view.current_tab = crate::gui::studio_view::StudioTab::ArchiveSuite;
                }
                "hardware" | "hw" | "ai" => {
                    app.studio_view.current_tab =
                        crate::gui::studio_view::StudioTab::HardwareTelemetry;
                }
                _ => {}
            }
        }
        if let Ok(mode_str) = std::env::var("MOSSAMBI_VIEW_MODE") {
            match mode_str.to_lowercase().as_str() {
                "grid" | "icons" => {
                    app.studio_view.explorer_view.view_mode =
                        crate::gui::explorer_view::ViewMode::Grid;
                }
                "list" | "details" => {
                    app.studio_view.explorer_view.view_mode =
                        crate::gui::explorer_view::ViewMode::Details;
                }
                "columns" | "miller" => {
                    app.studio_view.explorer_view.view_mode =
                        crate::gui::explorer_view::ViewMode::Columns;
                }
                _ => {}
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
    /// True when the file name carries a known archive extension (W-7
    /// Batch C): drives the archive-tab drop zone's pack-or-extract choice.
    fn looks_like_archive(path: &Path) -> bool {
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_ascii_lowercase(),
            None => return false,
        };
        [
            ".zip", ".7z", ".tar", ".gz", ".tgz", ".tar.gz", ".bz2", ".tbz2", ".tar.bz2", ".xz",
            ".txz", ".tar.xz", ".zst", ".tzst", ".tar.zst", ".rar", ".cab", ".iso", ".wim",
        ]
        .iter()
        .any(|ext| name.ends_with(ext))
    }

    /// Delegates to [`start_conversion_batch`] - `studio_view.rs` calls this.
    pub fn start_conversion(&mut self, target_format: &str) {
        let files: Vec<PathBuf> = self.current_file.clone().into_iter().collect();
        self.start_conversion_batch(files, target_format);
    }

    /// Converts a batch of files through the worker pool with per-item queue
    /// tracking (W-7). Explicit user action - always allowed, even when
    /// `background_work` is disabled (that knob only gates AUTO-triggered
    /// conversions).
    pub fn start_conversion_batch(&mut self, files: Vec<PathBuf>, target: &str) {
        if files.is_empty() {
            self.status_message = "Please drop or select a file first.".to_string();
            return;
        }

        // Double-start guard: a second click used to spawn two threads
        // clobbering the same output names.
        if self.is_converting {
            log::warn!(
                "Conversion already in progress - ignoring start request for {} file(s) -> {}",
                files.len(),
                target
            );
            self.status_message = "A conversion is already running - please wait.".to_string();
            return;
        }

        let staged: Vec<(usize, PathBuf, String)> = self
            .studio_view
            .queue
            .enqueue_batch(files, target)
            .into_iter()
            .map(|(id, path)| (id, path, target.to_string()))
            .collect();

        // Plan multi-hop routes once per process (W-7 Batch C): the
        // standard matrix is large, so it is built lazily into a shared
        // static and each enqueue stamps its items' planned chips.
        static ROUTE_PLANNER: OnceLock<crate::core::route_planner::RoutePlanner> = OnceLock::new();
        let planner = ROUTE_PLANNER
            .get_or_init(crate::core::route_planner::RoutePlanner::new_with_standard_matrix);
        for (id, path, tgt) in &staged {
            let src_ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_uppercase();
            if let Some(route) = planner.find_route(&src_ext, tgt, 3) {
                self.studio_view
                    .queue
                    .set_planned_route(*id, route.display_path());
            }
        }

        self.status_message = format!("Queued {} file(s) -> {}...", staged.len(), target);
        log::info!("conversion batch: {} file(s) -> {}", staged.len(), target);
        self.begin_conversion_run(staged);
    }

    /// Runs one conversion pass over already-enqueued queue items
    /// (item_id, source, target) triples. Each item reports its own
    /// `ItemResult`; batch-level `Progress` drives the overall bar.
    fn begin_conversion_run(&mut self, run: Vec<(usize, PathBuf, String)>) {
        if run.is_empty() {
            return;
        }
        self.is_converting = true;
        self.studio_view.batch_progress = Some(0.0);

        let sender = self.result_sender.clone();
        let settings = self.quick_settings.clone();

        let (_spawned_id, cancel_token) =
            self.worker_pool
                .spawn_task("StudioConversion", move |cancel, tx, task_id| {
                    let run_len = run.len();
                    let mut done = 0usize;
                    let mut failed = 0usize;
                    let mut cancelled = 0usize;

                    for (idx, (item_id, file, target_str)) in run.into_iter().enumerate() {
                        // Cooperative stop between files: remaining items report
                        // Cancelled so the queue panel reflects reality.
                        if cancel.load(Ordering::Relaxed) {
                            let _ = tx.send(TaskUpdate::ItemResult {
                                id: task_id,
                                item: Some(item_id),
                                outcome: ItemOutcome::Cancelled {
                                    message: "Skipped after cancel request.".to_string(),
                                },
                            });
                            cancelled += 1;
                            continue;
                        }

                        let _ = tx.send(TaskUpdate::Progress {
                            id: task_id,
                            fraction: idx as f32 / run_len.max(1) as f32,
                            status: format!("Starting {}", file.display()),
                            item: Some(item_id),
                        });

                        // Probe media duration (Audio/Video only) for determinate
                        // intra-item progress; indeterminate otherwise.
                        let (category, _) = UniversalRegistry::classify_file(&file);
                        let duration_secs =
                            if matches!(category, FileCategory::Audio | FileCategory::Video) {
                                probe_media_duration(&file)
                            } else {
                                None
                            };

                        let tx_progress = tx.clone();
                        let item_id_for_cb = item_id;
                        let handle = EncodeHandle::new(
                            Arc::clone(&cancel),
                            duration_secs,
                            Box::new(move |fraction: f32, status: &str| {
                                let _ = tx_progress.send(TaskUpdate::Progress {
                                    id: task_id,
                                    fraction,
                                    status: status.to_string(),
                                    item: Some(item_id_for_cb),
                                });
                            }),
                        );

                        let outcome = match execute_conversion_ctx(
                            &file,
                            &target_str,
                            &settings,
                            Some(&handle),
                        ) {
                            ConversionResult::Success(outputs) => {
                                done += 1;
                                ItemOutcome::Done {
                                    outputs,
                                    message: "Converted".to_string(),
                                }
                            }
                            ConversionResult::Failure(err) => {
                                if cancel.load(Ordering::Relaxed) {
                                    cancelled += 1;
                                    ItemOutcome::Cancelled {
                                        message: "Stopped by user.".to_string(),
                                    }
                                } else {
                                    failed += 1;
                                    ItemOutcome::Failed { error: err }
                                }
                            }
                        };

                        let _ = tx.send(TaskUpdate::ItemResult {
                            id: task_id,
                            item: Some(item_id),
                            outcome,
                        });

                        let _ = tx.send(TaskUpdate::Progress {
                            id: task_id,
                            fraction: (idx + 1) as f32 / run_len.max(1) as f32,
                            status: format!("Processed {}/{} file(s)", idx + 1, run_len),
                            item: None,
                        });
                    }

                    let summary = format!(
                        "Batch finished: {} done, {} failed, {} cancelled - details in the queue",
                        done, failed, cancelled
                    );
                    let result = if failed > 0 || cancelled > 0 {
                        ConversionResult::Failure(summary.clone())
                    } else {
                        ConversionResult::Success(Vec::new())
                    };
                    let _ = tx.send(TaskUpdate::Completed {
                        id: task_id,
                        output: None,
                        message: summary,
                    });
                    let _ = sender.send(result);
                });

        self.active_cancel = Some(cancel_token);
    }

    /// Requests graceful cancellation of the active conversion run (W-7):
    /// signals the cancel token, flips the queue into Stopping, and lets the
    /// encoder stop on its own before the hard-kill fallback.
    pub fn cancel_active_batch(&mut self) {
        if !self.is_converting {
            return;
        }
        if let Some(token) = &self.active_cancel {
            token.store(true, Ordering::Relaxed);
        }
        self.studio_view.queue.request_cancel();
        self.status_message =
            "Cancellation requested - waiting for the encoder to stop...".to_string();
        log::info!("conversion cancel requested");
    }

    /// AUTO-triggered conversion hook (reserved for programmatic converters).
    /// Honors `config.background_work == false` by refusing automatic work,
    /// while explicit user conversions always remain available.
    pub fn start_conversion_auto(&mut self, file: PathBuf, target: &str) {
        let cfg = crate::core::MossambiConfig::load();
        if !cfg.background_work {
            log::info!(
                "background work disabled - skipping AUTO conversion of {} ({})",
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
        let mut cmd = std::process::Command::new(
            exe.as_deref()
                .unwrap_or_else(|| std::path::Path::new("mossambi.exe")),
        );
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
    execute_conversion_ctx(input, target, settings, None)
}

pub fn execute_conversion_ctx(
    input: &Path,
    target: &str,
    settings: &QuickSettingsState,
    handle: Option<&EncodeHandle>,
) -> ConversionResult {
    let target_upper = target.to_uppercase();
    // Preferences "Output": Custom sends results to the chosen folder
    // (it used to be ignored; everything went next to the source).
    let out_cfg = crate::core::MossambiConfig::load();
    let out_dir: Option<PathBuf> = match out_cfg.output_mode {
        crate::core::OutputMode::Custom => out_cfg.custom_output_dir.clone().filter(|d| d.is_dir()),
        crate::core::OutputMode::Sibling => None,
    };
    let out_dir = out_dir.as_deref();

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

        let parent = out_dir.unwrap_or_else(|| input.parent().unwrap_or_else(|| Path::new(".")));
        let stem = input
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");
        let out = parent.join(format!("{}_conformed.jpg", stem));

        match optimize_exam_media_ctx(input, &out, &constraint, handle) {
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

        match transcode_to_all_formats(input, mode, out_dir) {
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
        let out_path = crate::core::resolve_clean_sibling_output_path(input, "pdf", out_dir);
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
            crate::core::resolve_clean_sibling_output_path(input, &target_upper, out_dir);
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

    match transcode_file_ctx(input, &target_upper, mode, out_dir, handle) {
        Ok(out) => ConversionResult::Success(vec![out]),
        Err(e) => ConversionResult::Failure(e.to_string()),
    }
}

impl eframe::App for MossambiApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        let pal = self.theme.colors();
        if pal.is_translucent {
            // Panels carry the glass alpha; clearing to transparent lets
            // the OS backdrop show where they are see-through.
            return [0.0, 0.0, 0.0, 0.0];
        }
        [
            pal.window_bg.r() as f32 / 255.0,
            pal.window_bg.g() as f32 / 255.0,
            pal.window_bg.b() as f32 / 255.0,
            1.0,
        ]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // -- Ctrl + Mouse Wheel ----------------------------------------
        // Plain Ctrl+wheel    -> Explorer zoom ladder: five Grid icon-size
        //                       levels (0..=4, persisted); one notch below
        //                       the smallest crosses into Details and back
        //                       (session-only, debounced against wiggle)
        // Ctrl+0              -> reset explorer zoom to the default level 2
        // Ctrl+Shift+wheel    -> whole-UI pixel scale (Finder/Explorer parity)
        // Ctrl+Shift+0        -> reset UI scale to native
        // B-1 gate: while the command palette modal is open it owns
        // pointer wheel and digit keys; zoom must not mutate the
        // view underneath the backdrop.
        let palette_open = self.studio_view.command_palette.is_open;
        let (ui_zoom_scroll, ui_zoom_reset, explorer_scroll, explorer_reset) = ctx.input(|i| {
            let ctrl = i.modifiers.ctrl || i.modifiers.command;
            let shift = i.modifiers.shift;
            let scroll = if ctrl { i.raw_scroll_delta.y } else { 0.0 };
            let num0 = i.key_pressed(egui::Key::Num0);
            (
                if ctrl && shift && !palette_open {
                    scroll
                } else {
                    0.0
                },
                ctrl && shift && num0 && !palette_open,
                if ctrl && !shift && !palette_open {
                    scroll
                } else {
                    0.0
                },
                ctrl && !shift && num0 && !palette_open,
            )
        });
        if explorer_reset {
            self.studio_view.explorer_view.explorer_zoom = 2;
            log::debug!("explorer zoom reset to level 2 (Ctrl+0)");
        } else if explorer_scroll != 0.0 {
            let step = if explorer_scroll > 0.0 { 1 } else { -1 };
            self.studio_view.explorer_view.zoom_ladder_step(step);
        } else if ui_zoom_reset {
            self.ui_zoom = 1.0;
            self.last_applied_theme = None;
        } else if ui_zoom_scroll != 0.0 {
            let step = if ui_zoom_scroll > 0.0 {
                1.08_f32
            } else {
                1.0_f32 / 1.08
            };
            self.ui_zoom = (self.ui_zoom * step).clamp(0.6, 2.5);
            self.last_applied_theme = None;
        }
        // Zoom = Preferences "Interface size" x the Ctrl+Shift+wheel session
        // zoom. zoom_factor (not a fixed pixels_per_point) keeps following
        // the monitor's own DPI when the window moves between screens.
        let zoom = (self.config.ui_scale.clamp(0.75, 2.0) * self.ui_zoom).clamp(0.5, 3.0);
        if (ctx.zoom_factor() - zoom).abs() > 0.001 {
            ctx.set_zoom_factor(zoom);
        }

        // -- Mouse history navigation (thumb buttons / middle-drag swipe) ---
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
        if self.frame_count == 0 {
            log::info!("MossambiApp::update first frame rendering!");
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        }
        self.frame_count = self.frame_count.saturating_add(1);

        // Automated QA snapshot capture via MOSSAMBI_SNAPSHOT_PATH
        if let Ok(snap_path) = std::env::var("MOSSAMBI_SNAPSHOT_PATH") {
            if self.frame_count == 8 {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot);
            }
            for event in &ctx.input(|i| i.events.clone()) {
                if let egui::Event::Screenshot { image, .. } = event {
                    let (w, h) = (image.width() as u32, image.height() as u32);
                    let mut raw = Vec::with_capacity((w * h * 4) as usize);
                    for p in &image.pixels {
                        raw.push(p.r());
                        raw.push(p.g());
                        raw.push(p.b());
                        raw.push(p.a());
                    }
                    if let Some(rgba) = image::RgbaImage::from_raw(w, h, raw) {
                        let _ = rgba.save(&snap_path);
                        log::info!("Saved QA snapshot to {}", snap_path);
                    }
                    if std::env::var("MOSSAMBI_SNAPSHOT_EXIT").is_ok() {
                        std::process::exit(0);
                    }
                }
            }
        }
        // Pick up config edits made outside this window (other windows,
        // hand edits) about once a second. A frame count is not a clock:
        // at the 5 Hz idle rate "every 120 frames" meant ~24 s.
        let reload_id = egui::Id::new("mossambi_config_reload_at");
        let now_t = ctx.input(|i| i.time);
        let last_t = ctx
            .data(|d| d.get_temp::<f64>(reload_id))
            .unwrap_or(f64::MIN);
        if now_t - last_t >= 1.0 {
            ctx.data_mut(|d| d.insert_temp(reload_id, now_t));
            self.config = crate::core::MossambiConfig::load();
        }
        // Config -> explorer for the prefs both sides can change. Before,
        // only the reverse direction existed and ran every frame, so the
        // explorer's defaults overwrote whatever Preferences saved (icon
        // pack, zoom, checkbox mode, menu style, all animation settings).
        self.pull_explorer_prefs();
        // OS integrations follow their Preferences toggles (both were only
        // stored before). Applied on change, never at startup, and off the
        // UI thread because they shell out to reg.exe.
        for (key, on, apply) in [
            (
                "mossambi_applied_autostart",
                self.config.launch_at_login,
                crate::core::MossambiConfig::set_autostart as fn(bool) -> anyhow::Result<()>,
            ),
            (
                "mossambi_applied_shell_menu",
                self.config.shell_context_menu,
                crate::core::MossambiConfig::set_shell_context_menu,
            ),
        ] {
            let id = egui::Id::new(key);
            match ctx.data(|d| d.get_temp::<bool>(id)) {
                None => ctx.data_mut(|d| d.insert_temp(id, on)),
                Some(prev) if prev != on => {
                    ctx.data_mut(|d| d.insert_temp(id, on));
                    std::thread::spawn(move || {
                        if let Err(e) = apply(on) {
                            log::warn!("{key}: {e}");
                        }
                    });
                }
                _ => {}
            }
        }
        // Always-on-top was only honoured by the CLI.
        let aot_id = egui::Id::new("mossambi_applied_always_on_top");
        let aot = self.config.always_on_top;
        if ctx.data(|d| d.get_temp::<bool>(aot_id)) != Some(aot) {
            ctx.data_mut(|d| d.insert_temp(aot_id, aot));
            ctx.send_viewport_cmd(ViewportCommand::WindowLevel(if aot {
                egui::WindowLevel::AlwaysOnTop
            } else {
                egui::WindowLevel::Normal
            }));
        }
        let cfg = &self.config;
        // Sync grid icon size from config (Preferences -> live view).
        // The mirror prevents this from fighting the toolbar/status-bar
        // sliders, which write explorer_view.icon_size directly.
        if cfg.grid_icon_size != self.last_grid_icon_size {
            self.last_grid_icon_size = cfg.grid_icon_size;
            self.studio_view.explorer_view.icon_size = cfg.grid_icon_size as f32;
        }
        let current_glass = cfg.glass_effect;
        // Appearance (light/dark/system) and opacity change the palette
        // without changing the theme id, so compare the resolved palette too.
        let sig_id = egui::Id::new("mossambi_applied_palette_sig");
        let pal_now = self.theme.colors();
        let sig = (
            pal_now.window_bg.to_array(),
            pal_now.sidebar_bg.to_array(),
            pal_now.is_dark,
        );
        let sig_changed = ctx.data(|d| d.get_temp::<([u8; 4], [u8; 4], bool)>(sig_id)) != Some(sig);
        if self.frame_count == 1
            || sig_changed
            || self.last_applied_theme != Some(self.theme)
            || self.last_applied_glass != Some(current_glass)
        {
            ctx.data_mut(|d| d.insert_temp(sig_id, sig));
            let palette = self.theme.colors();
            crate::gui::macos_theme::apply_theme_visuals(ctx, &palette);
            crate::gui::dwm_helper::apply_system_backdrop(pal_now.is_dark, current_glass);
            self.last_applied_theme = Some(self.theme);
            self.last_applied_glass = Some(current_glass);
        }

        // Continuous driver honoring the Max-FPS knob: when enabled, drop
        // vsync-driven pacing and repaint at ~500Hz for maximum responsiveness.
        //
        // The spring animations have no repaint hook of their own, so the
        // driver runs fast while the user is interacting (and ~1.5 s after,
        // for springs to settle) but drops to 5 Hz when the window is idle.
        // Before, Max-FPS (on by default) repainted at 500 Hz forever and
        // pinned a CPU core with nothing on screen changing.
        let is_animating = self.is_converting || self.completion_time.is_some();
        let activity_id = egui::Id::new("mossambi_last_activity_time");
        let (now, had_input, pointer_still, pointer_down) = ctx.input(|i| {
            (
                i.time,
                !i.events.is_empty(),
                i.pointer.time_since_last_movement(),
                i.pointer.any_down(),
            )
        });
        if had_input || pointer_down {
            ctx.data_mut(|d| d.insert_temp(activity_id, now));
        }
        let last_activity = ctx.data(|d| d.get_temp::<f64>(activity_id)).unwrap_or(now);
        let recently_active = pointer_still < 1.5 || (now - last_activity) < 1.5;
        let fast = if cfg.max_fps {
            Duration::from_millis(2)
        } else {
            Duration::from_millis(16)
        };
        let repaint_interval = if is_animating || recently_active {
            fast
        } else {
            Duration::from_millis(200)
        };
        ctx.request_repaint_after(repaint_interval);

        // Drain pooled worker updates: per-item progress/results feed the
        // queue reducer on StudioView; batch-level progress feeds the
        // overall bar. Watchdog failures stall the queue honestly.
        while let Some(update) = self.worker_pool.try_recv() {
            match update {
                TaskUpdate::Started { name, .. } => log::debug!("worker started: {}", name),
                TaskUpdate::Progress {
                    fraction,
                    status,
                    item,
                    ..
                } => match item {
                    Some(item_id) => {
                        self.studio_view
                            .queue
                            .on_progress(item_id, fraction, &status);
                        log::debug!("worker progress {:.0}%: {}", fraction * 100.0, status);
                    }
                    None => {
                        self.studio_view.batch_progress = Some(fraction);
                        log::debug!("batch progress {:.0}%: {}", fraction * 100.0, status);
                    }
                },
                TaskUpdate::ItemResult { item, outcome, .. } => {
                    if let Some(item_id) = item {
                        match outcome {
                            ItemOutcome::Done { outputs, message } => {
                                // Mirror exam jobs back to the exam tab
                                // status line (W-7 Batch B).
                                let is_exam = self.studio_view.queue.items().iter().any(|i| {
                                    i.id == item_id && i.target.eq_ignore_ascii_case("EXAM")
                                });
                                if is_exam {
                                    self.studio_view.exam_status =
                                        format!("Exam complete: {}", message);
                                    self.studio_view.last_exam_output = outputs.first().cloned();
                                }
                                self.studio_view
                                    .queue
                                    .on_item_done(item_id, outputs, &message);
                            }
                            ItemOutcome::Failed { error } => {
                                let is_exam = self.studio_view.queue.items().iter().any(|i| {
                                    i.id == item_id && i.target.eq_ignore_ascii_case("EXAM")
                                });
                                if is_exam {
                                    self.studio_view.exam_status =
                                        format!("Exam failed: {}", error);
                                }
                                self.studio_view.queue.on_item_failed(item_id, &error);
                            }
                            ItemOutcome::Cancelled { message } => {
                                self.studio_view.queue.on_item_cancelled(item_id);
                                log::info!("item cancelled: {}", message);
                            }
                        }
                    }
                }
                TaskUpdate::Completed { message, .. } => log::info!("worker finished: {}", message),
                TaskUpdate::Failed { error, .. } => {
                    // Watchdog stall: per-item outcomes will never arrive.
                    self.studio_view.queue.on_task_stalled(&error);
                    self.is_converting = false;
                    self.active_cancel = None;
                    self.studio_view.batch_progress = None;
                    self.status_message = format!("Conversion stalled: {}", error);
                    log::error!("worker reported failure: {}", error);
                }
            }
        }

        // Drain the background file-picker thread (W-7 Batch B): the rfd
        // dialog blocks its own thread, never the UI frame.
        while let Ok(picked) = self.picker_receiver.try_recv() {
            self.picker_in_flight = false;
            if let Some(path) = picked {
                self.select_file(path);
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
                    if let Err(e) = cfg.save() {
                        log::warn!("Could not persist theme change: {e}");
                    }
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
                    if let Err(e) = cfg.save() {
                        log::warn!("Could not persist autostart change: {e}");
                    }
                }
                AppControlEvent::QuitApp => {
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                }
            }
        }

        // 2. Poll for background conversion results (drain the WHOLE queue -
        // batch conversions may stack several results between frames)
        while let Ok(res) = self.result_receiver.try_recv() {
            self.is_converting = false;
            self.active_cancel = None;
            self.studio_view.batch_progress = None;
            match res {
                ConversionResult::Success(files) => {
                    self.completion_time = Some(Instant::now());
                    if let Some(first) = files.first() {
                        self.last_completed_output = Some(first.clone());
                    }
                    self.status_message =
                        "Batch finished - details in the conversion queue.".to_string();
                }
                ConversionResult::Failure(err) => {
                    log::error!("Studio conversion finished: {}", err);
                    self.completion_time = Some(Instant::now());
                    self.status_message = err;
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
                // First resolvable path wins, so one malformed entry no
                // longer swallows the whole drop; the Windows rescue (active
                // shell window under the cursor) only runs when the drop
                // carried no resolvable path at all.
                let cursor_rescue: Option<PathBuf>;
                #[cfg(target_os = "windows")]
                {
                    use windows_sys::Win32::Foundation::POINT;
                    use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;
                    let mut pt = POINT { x: 0, y: 0 };
                    unsafe { GetCursorPos(&mut pt) };
                    cursor_rescue = i
                        .raw
                        .dropped_files
                        .iter()
                        .filter_map(|drop| drop.path.clone())
                        .next()
                        .or_else(|| {
                            crate::gui::active_file::get_active_file_under_cursor(pt.x, pt.y)
                        });
                }
                #[cfg(not(target_os = "windows"))]
                {
                    // The platform-provided dropped path is the source of
                    // truth; there is no shell-rescue equivalent here.
                    cursor_rescue = i
                        .raw
                        .dropped_files
                        .iter()
                        .filter_map(|drop| drop.path.clone())
                        .next();
                }

                if let Some(path) = cursor_rescue {
                    self.select_file(path.clone());
                    // Per-tab drop zones (W-7 Batch C): the tab under the
                    // drop decides the follow-up action; plain staging
                    // remains the baseline everywhere else.
                    match self.studio_view.current_tab {
                        StudioTab::ArchiveSuite => {
                            let target = if Self::looks_like_archive(&path) {
                                "EXTRACT"
                            } else {
                                "ZIP"
                            };
                            self.start_conversion_batch(vec![path], target);
                        }
                        StudioTab::QualityInspector => {
                            if !self.studio_view.inspection_busy {
                                self.studio_view.inspection_busy = true;
                                self.studio_view.inspection_status =
                                    "Inspecting in background...".to_string();
                                crate::core::tool_console::inspect_file_async(
                                    self.studio_view.tool_console.sender(),
                                    path,
                                );
                            }
                        }
                        StudioTab::ExamStudio => {
                            self.studio_view
                                .queue_actions
                                .push(QueueAction::RunExam(path));
                        }
                        _ => {}
                    }
                }
            }
        });

        // 4. Render Desktop Studio Application Layout
        let cur_file = self.current_file.clone();
        let mut qs = self.quick_settings.clone();

        let mut file_to_pick = false;
        let mut file_to_stage: Option<PathBuf> = None;
        let mut convert_batch: Option<(Vec<PathBuf>, String)> = None;
        let mut open_target_folder: Option<PathBuf> = None;

        self.studio_view.render(
            ctx,
            &mut self.theme,
            &mut self.config,
            &cur_file,
            &mut qs,
            || file_to_pick = true,
            |p| file_to_stage = Some(p),
            |paths, t| convert_batch = Some((paths, t)),
            |p| open_target_folder = Some(p.to_path_buf()),
        );

        self.quick_settings = qs;

        if let Some(path) = file_to_stage {
            self.select_file(path);
        }

        if file_to_pick && !self.picker_in_flight {
            self.picker_in_flight = true;
            let pick_tx = self.picker_sender.clone();
            std::thread::spawn(move || {
                let picked = rfd::FileDialog::new().pick_file();
                let _ = pick_tx.send(picked);
            });
        }
        if let Some((paths, target)) = convert_batch {
            // Single batched pipeline for N files -> one format (the old path
            // staged+converted per file, so only the last file survived).
            self.start_conversion_batch(paths, &target);
        }
        if let Some(p) = open_target_folder {
            open_in_file_manager(&p);
        }

        // 5. Apply deferred conversion-queue actions (W-7)
        for action in std::mem::take(&mut self.studio_view.queue_actions) {
            match action {
                QueueAction::CancelActive => self.cancel_active_batch(),
                QueueAction::RetryFailed => {
                    if !self.is_converting {
                        let retry = self.studio_view.queue.take_failed_for_retry();
                        if !retry.is_empty() {
                            log::info!("retrying {} failed item(s)", retry.len());
                            self.begin_conversion_run(retry);
                        }
                    }
                }
                QueueAction::ClearFinished => self.studio_view.queue.clear_finished(),
                QueueAction::Reveal(path) => {
                    if let Some(parent) = path.parent() {
                        open_in_file_manager(parent);
                    }
                }
                QueueAction::RunExam(path) => {
                    if self.is_converting {
                        self.status_message = "Conversion busy - exam job skipped.".to_string();
                    } else {
                        self.start_conversion_batch(vec![path], "EXAM");
                    }
                }
            }
        }

        // 6. Persist any explorer preference mutations from this frame
        // (icon pack, zoom level, checkboxes, menu style, animations) into the
        // config - same save pattern as the theme-change handlers above.
        self.sync_explorer_prefs();

        // 7. Edge resizing. The window has no OS title bar or frame (Finder
        // chrome is drawn by the app), so Windows offers no resize border.
        // Run last so the resize cursor wins over widgets near the edge.
        edge_resize(ctx);
    }
}

/// Which window edge/corner (if any) a point is on, `margin` px deep.
/// Corners win over edges and take a larger grab zone, like native windows.
fn resize_direction_at(
    rect: egui::Rect,
    pos: egui::Pos2,
    margin: f32,
) -> Option<egui::viewport::ResizeDirection> {
    use eframe::egui::viewport::ResizeDirection as D;
    if !rect.expand(1.0).contains(pos) {
        return None;
    }
    let corner = margin * 2.5;
    let near = |d: f32, m: f32| d <= m;
    let (dl, dr, dt, db) = (
        pos.x - rect.left(),
        rect.right() - pos.x,
        pos.y - rect.top(),
        rect.bottom() - pos.y,
    );
    let dir = if (near(dl, corner) && near(dt, margin)) || (near(dl, margin) && near(dt, corner)) {
        D::NorthWest
    } else if (near(dr, corner) && near(dt, margin)) || (near(dr, margin) && near(dt, corner)) {
        D::NorthEast
    } else if (near(dl, corner) && near(db, margin)) || (near(dl, margin) && near(db, corner)) {
        D::SouthWest
    } else if (near(dr, corner) && near(db, margin)) || (near(dr, margin) && near(db, corner)) {
        D::SouthEast
    } else if near(dl, margin) {
        D::West
    } else if near(dr, margin) {
        D::East
    } else if near(dt, margin) {
        D::North
    } else if near(db, margin) {
        D::South
    } else {
        return None;
    };
    Some(dir)
}

/// Resize cursor + OS-driven drag resize from the borderless window edges.
fn edge_resize(ctx: &egui::Context) {
    use eframe::egui::viewport::ResizeDirection as D;
    let (maximized, fullscreen) = ctx.input(|i| {
        (
            i.viewport().maximized.unwrap_or(false),
            i.viewport().fullscreen.unwrap_or(false),
        )
    });
    if maximized || fullscreen {
        return;
    }
    let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) else {
        return;
    };
    let Some(dir) = resize_direction_at(ctx.screen_rect(), pos, 5.0) else {
        return;
    };
    ctx.set_cursor_icon(match dir {
        D::North => egui::CursorIcon::ResizeNorth,
        D::South => egui::CursorIcon::ResizeSouth,
        D::East => egui::CursorIcon::ResizeEast,
        D::West => egui::CursorIcon::ResizeWest,
        D::NorthEast => egui::CursorIcon::ResizeNorthEast,
        D::NorthWest => egui::CursorIcon::ResizeNorthWest,
        D::SouthEast => egui::CursorIcon::ResizeSouthEast,
        D::SouthWest => egui::CursorIcon::ResizeSouthWest,
    });
    if ctx.input(|i| i.pointer.primary_pressed()) {
        ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(dir));
    }
}

impl MossambiApp {
    /// Dirty-checks the live [`StudioView`]'s explorer preference fields
    /// against the persisted config and saves only when something changed.
    /// Pushes config values that changed (Preferences, a hand-edited
    /// config file, first frame) into the explorer view.
    fn pull_explorer_prefs(&mut self) {
        let from_cfg = ExplorerPrefMirror::from_config(&self.config);
        if self.explorer_pref_mirror.as_ref() == Some(&from_cfg) {
            return;
        }
        let ev = &mut self.studio_view.explorer_view;
        ev.icon_pack = crate::core::icon_packs::IconPack::from_config_id(&from_cfg.icon_pack);
        ev.explorer_zoom = from_cfg.explorer_zoom;
        ev.checkbox_mode = from_cfg.checkbox_mode;
        ev.menu_style = from_cfg.menu_style;
        ev.animations = from_cfg.animations;
        ev.show_hidden_files = from_cfg.show_hidden_files;
        self.explorer_pref_mirror = Some(from_cfg);
    }

    /// Explorer -> config: only for prefs the explorer UI itself changed
    /// this frame (zoom ladder, toolbar toggles). Comparing against the
    /// mirror instead of the config is what keeps a Preferences edit made
    /// in the same frame from being overwritten.
    fn sync_explorer_prefs(&mut self) {
        let now = ExplorerPrefMirror::from_explorer(&self.studio_view.explorer_view);
        let Some(prev) = self.explorer_pref_mirror.clone() else {
            self.explorer_pref_mirror = Some(now);
            return;
        };
        let mut dirty = false;
        if now.icon_pack != prev.icon_pack {
            self.config.icon_pack = now.icon_pack.clone();
            dirty = true;
        }
        if now.explorer_zoom != prev.explorer_zoom {
            self.config.explorer_zoom = now.explorer_zoom;
            dirty = true;
        }
        if now.checkbox_mode != prev.checkbox_mode {
            self.config.checkbox_mode = now.checkbox_mode;
            dirty = true;
        }
        if now.menu_style != prev.menu_style {
            self.config.menu_style = now.menu_style;
            dirty = true;
        }
        if now.animations != prev.animations {
            self.config.animations = now.animations;
            dirty = true;
        }
        if now.show_hidden_files != prev.show_hidden_files {
            self.config.show_hidden_files = now.show_hidden_files;
            dirty = true;
        }
        self.explorer_pref_mirror = Some(now);
        if dirty {
            log::debug!(
                "explorer prefs changed - saving config (zoom={}, pack={})",
                self.config.explorer_zoom,
                self.config.icon_pack
            );
            if let Err(e) = self.config.save() {
                log::warn!("Could not persist explorer prefs: {e}");
            }
        }
    }
}

/// Snapshot of the explorer prefs that live in both the config and the
/// explorer view.
#[derive(Debug, Clone, PartialEq)]
pub struct ExplorerPrefMirror {
    icon_pack: String,
    explorer_zoom: u8,
    checkbox_mode: crate::core::config::CheckboxMode,
    menu_style: crate::core::config::MenuStyle,
    animations: crate::core::config::AnimationSettings,
    show_hidden_files: bool,
}

impl ExplorerPrefMirror {
    fn from_explorer(e: &crate::gui::explorer_view::ExplorerView) -> Self {
        Self {
            icon_pack: e.icon_pack.id().to_string(),
            explorer_zoom: e.explorer_zoom,
            checkbox_mode: e.checkbox_mode,
            menu_style: e.menu_style,
            animations: e.animations,
            show_hidden_files: e.show_hidden_files,
        }
    }

    fn from_config(c: &crate::core::MossambiConfig) -> Self {
        Self {
            icon_pack: c.icon_pack.clone(),
            explorer_zoom: c.explorer_zoom,
            checkbox_mode: c.checkbox_mode,
            menu_style: c.menu_style,
            animations: c.animations,
            show_hidden_files: c.show_hidden_files,
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
        .with_title("Mossambi - Universal File Conversion Studio")
        .with_inner_size([1120.0, 720.0])
        .with_min_inner_size([920.0, 580.0])
        .with_decorations(false)
        // Transparent so the DWM Acrylic backdrop can show through in the
        // Frosted / Liquid glass modes (Opaque paints a solid canvas).
        .with_transparent(true)
        .with_resizable(true)
        .with_visible(true)
        .with_active(true);

    if let Some(icon) = load_app_icon() {
        builder = builder.with_icon(icon);
    }

    // Honor the Max-FPS knob: when enabled, uncap the frame driver by
    // disabling vsync (repaint pacing is handled per-frame in update()).
    let max_fps = crate::core::MossambiConfig::load().max_fps;
    let mut options = eframe::NativeOptions {
        viewport: builder,
        vsync: !max_fps,
        ..Default::default()
    };
    // Under Wayland-with-XWayland, force the X11 windowing backend so
    // external file drag & drop can reach us (XDND receive only exists in
    // winit's X11 backend). See core/dnd_backend.rs for the policy matrix.
    crate::core::dnd_backend::apply_backend_policy(&mut options);

    // Set Windows Per-Monitor V2 DPI awareness before creating the native window
    #[cfg(target_os = "windows")]
    unsafe {
        #[link(name = "user32")]
        extern "system" {
            fn SetProcessDpiAwarenessContext(value: isize) -> i32;
        }
        const DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2: isize = -4;
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }

    eframe::run_native(
        "Mossambi - Universal File Conversion Studio",
        options,
        Box::new(move |cc| Ok(Box::new(MossambiApp::new_with_path(cc, initial_path)))),
    )
}

#[cfg(test)]
mod edge_resize_tests {
    use super::resize_direction_at;
    use eframe::egui::viewport::ResizeDirection as D;
    use eframe::egui::{pos2, Rect};

    #[test]
    fn edges_and_corners_resolve() {
        let r = Rect::from_min_max(pos2(0.0, 0.0), pos2(800.0, 600.0));
        assert_eq!(resize_direction_at(r, pos2(2.0, 300.0), 5.0), Some(D::West));
        assert_eq!(
            resize_direction_at(r, pos2(798.0, 300.0), 5.0),
            Some(D::East)
        );
        assert_eq!(
            resize_direction_at(r, pos2(400.0, 1.0), 5.0),
            Some(D::North)
        );
        assert_eq!(
            resize_direction_at(r, pos2(400.0, 599.0), 5.0),
            Some(D::South)
        );
        assert_eq!(
            resize_direction_at(r, pos2(797.0, 590.0), 5.0),
            Some(D::SouthEast)
        );
        assert_eq!(
            resize_direction_at(r, pos2(3.0, 3.0), 5.0),
            Some(D::NorthWest)
        );
        assert_eq!(resize_direction_at(r, pos2(400.0, 300.0), 5.0), None);
    }
}
