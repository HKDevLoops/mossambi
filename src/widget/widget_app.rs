//! Mossambi Standalone Desktop Radial Fruit Widget Application
//! 100% Topbarless, Menu-less, Ribbon-less, Frameless, 32-Bit Transparent Desktop Widget
//! Faithful reproduction of all 6 visual states from `Image reference.png`.

use crate::codec::{transcode_file, LosslessMode};
use crate::core::{AsyncWorkerPool, FileCategory, TaskUpdate, UniversalRegistry};
use crate::widget::{RadialWidgetGeometry, WidgetState, WidgetTheme};
use eframe::egui::{
    self, Color32, Key, Pos2, Rect, Stroke, Vec2, ViewportBuilder, ViewportCommand,
};
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub enum WidgetResult {
    Success(Vec<PathBuf>),
    Failure(String),
}

pub struct MossambiWidgetApp {
    pub geometry: RadialWidgetGeometry,
    pub sectors: Vec<&'static str>,
    pub hovered_carpel: Option<usize>,
    pub current_file: Option<PathBuf>,
    pub current_category: FileCategory,

    // State Tracking
    pub is_converting: bool,
    pub progress: f32,
    pub converted_count: usize,
    pub success_time: Option<Instant>,
    pub last_output: Option<PathBuf>,
    /// Last conversion failure, surfaced in the hub status area (previously
    /// the widget discarded errors entirely, so the user saw nothing).
    pub last_error: Option<String>,
    pub error_time: Option<Instant>,

    // Context Menu & Dragging
    pub show_context_menu: bool,
    pub context_menu_pos: Pos2,
    pub frame_count: u32,
    /// Latched while a native drag hand-off is in flight for the current
    /// press; reset when the primary button is released.
    pub drag_out_in_flight: bool,

    // Worker Channels
    pub result_rx: Receiver<WidgetResult>,
    pub result_tx: Sender<WidgetResult>,
    pub worker_pool: AsyncWorkerPool,
}

impl Default for MossambiWidgetApp {
    fn default() -> Self {
        let (tx, rx) = channel();
        // Default petals matching Image reference.png: JPG, WEBP, HEIC, TIFF, AVIF, BMP, PDF, DOCX
        let default_sectors = vec!["JPG", "WEBP", "HEIC", "TIFF", "AVIF", "BMP", "PDF", "DOCX"];

        Self {
            geometry: RadialWidgetGeometry::new(Pos2::new(200.0, 200.0), 400.0),
            sectors: default_sectors,
            hovered_carpel: None,
            current_file: None,
            current_category: FileCategory::Image,

            is_converting: false,
            progress: 0.0,
            converted_count: 0,
            success_time: None,
            last_output: None,
            last_error: None,
            error_time: None,

            show_context_menu: false,
            context_menu_pos: Pos2::ZERO,
            frame_count: 0,
            drag_out_in_flight: false,

            result_rx: rx,
            result_tx: tx,
            worker_pool: AsyncWorkerPool::new(),
        }
    }
}

impl MossambiWidgetApp {
    pub fn select_file(&mut self, path: PathBuf) {
        let (cat, _ext) = UniversalRegistry::classify_file(&path);
        self.current_category = cat;
        self.sectors = UniversalRegistry::get_widget_sectors_for_category(cat);
        self.current_file = Some(path);
    }

    pub fn start_conversion(&mut self, target_format: &str) {
        if self.is_converting {
            log::warn!(
                "Widget conversion already in progress; ignoring request for {}",
                target_format
            );
            return;
        }

        let input = match self.current_file.clone() {
            Some(f) => f,
            None => return,
        };

        self.is_converting = true;
        self.progress = 0.15;
        self.last_error = None;
        let sender = self.result_tx.clone();
        let target = target_format.to_string();

        self.worker_pool
            .spawn_task("WidgetConvert", move |cancel, tx, task_id| {
                let res = if cancel.load(Ordering::Relaxed) {
                    WidgetResult::Failure("cancelled before it could start".to_string())
                } else {
                    match transcode_file(&input, &target, LosslessMode::StrictBitExact, None) {
                        Ok(out) => WidgetResult::Success(vec![out]),
                        Err(e) => WidgetResult::Failure(e.to_string()),
                    }
                };

                match &res {
                    WidgetResult::Success(files) => {
                        let _ = tx.send(TaskUpdate::Completed {
                            id: task_id,
                            output: files.first().cloned(),
                            message: format!("Widget converted file to {}", target),
                        });
                    }
                    WidgetResult::Failure(err) => {
                        let _ = tx.send(TaskUpdate::Failed {
                            id: task_id,
                            error: err.clone(),
                        });
                    }
                }
                let _ = sender.send(res);
            });
    }
}

#[cfg(target_os = "windows")]
fn enforce_borderless_win32() {
    use std::ffi::c_void;
    use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetClassNameW, GetWindowLongPtrW, GetWindowThreadProcessId, SetWindowLongPtrW,
        SetWindowPos, GWL_EXSTYLE, GWL_STYLE, SWP_FRAMECHANGED, SWP_NOMOVE, SWP_NOSIZE,
        SWP_NOZORDER, SWP_SHOWWINDOW, WS_CLIPCHILDREN, WS_CLIPSIBLINGS, WS_EX_LAYERED,
        WS_EX_TOPMOST, WS_POPUP, WS_VISIBLE,
    };

    extern "system" {
        fn GetCurrentProcessId() -> u32;
    }

    #[repr(C)]
    struct Margins {
        cx_left: i32,
        cx_right: i32,
        cy_top: i32,
        cy_bottom: i32,
    }

    #[link(name = "dwmapi")]
    extern "system" {
        fn DwmExtendFrameIntoClientArea(hwnd: HWND, p_mar_inset: *const Margins) -> i32;
        fn DwmSetWindowAttribute(
            hwnd: HWND,
            dw_attribute: u32,
            pv_attribute: *const c_void,
            cb_attribute: u32,
        ) -> i32;
    }

    unsafe extern "system" fn enum_proc(hwnd: HWND, _: LPARAM) -> BOOL {
        let current_pid = GetCurrentProcessId();
        let mut wnd_pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut wnd_pid);

        if wnd_pid == current_pid {
            let mut class_name = [0u16; 256];
            let len = GetClassNameW(hwnd, class_name.as_mut_ptr(), 256);
            let class_str = String::from_utf16_lossy(&class_name[..len as usize]);

            if class_str == "Window Class" {
                let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
                let target_style = (style & !0x00CF0000)
                    | WS_POPUP
                    | WS_VISIBLE
                    | WS_CLIPSIBLINGS
                    | WS_CLIPCHILDREN;

                let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
                let target_ex = (ex_style & !0x00020301) | WS_EX_LAYERED | WS_EX_TOPMOST;

                let mut changed = false;
                if style != target_style {
                    SetWindowLongPtrW(hwnd, GWL_STYLE, target_style as isize);
                    changed = true;
                }
                if ex_style != target_ex {
                    SetWindowLongPtrW(hwnd, GWL_EXSTYLE, target_ex as isize);
                    changed = true;
                }

                let margins = Margins {
                    cx_left: -1,
                    cx_right: -1,
                    cy_top: -1,
                    cy_bottom: -1,
                };
                let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);

                let corner_pref: u32 = 1; // DWMWCP_DONOTROUND
                let _ = DwmSetWindowAttribute(hwnd, 33, &corner_pref as *const _ as *const _, 4);

                let color_none: u32 = 0xFFFFFFFE; // DWMWA_COLOR_NONE
                let _ = DwmSetWindowAttribute(hwnd, 34, &color_none as *const _ as *const _, 4);

                use windows_sys::Win32::Graphics::Gdi::{CreateEllipticRgn, SetWindowRgn};
                let rgn = CreateEllipticRgn(6, 6, 394, 394);
                if !rgn.is_null() {
                    let _ = SetWindowRgn(hwnd, rgn, 1);
                }

                if changed {
                    SetWindowPos(
                        hwnd,
                        std::ptr::null_mut(),
                        0,
                        0,
                        0,
                        0,
                        SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_SHOWWINDOW,
                    );
                }
            }
        }
        1
    }

    unsafe {
        EnumWindows(Some(enum_proc), 0);
    }
}

impl eframe::App for MossambiWidgetApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0] // 100% Fully transparent OpenGL backbuffer
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.frame_count = self.frame_count.saturating_add(1);

        if self.frame_count == 1 {
            let cfg = crate::core::MossambiConfig::load();
            let is_dark = cfg.theme_mode.is_dark();
            self.geometry.theme = if is_dark {
                WidgetTheme::Dark
            } else {
                WidgetTheme::Light
            };

            let mut visuals = if is_dark {
                egui::Visuals::dark()
            } else {
                egui::Visuals::light()
            };
            visuals.window_fill = Color32::TRANSPARENT;
            visuals.panel_fill = Color32::TRANSPARENT;
            visuals.extreme_bg_color = Color32::TRANSPARENT;
            visuals.faint_bg_color = Color32::TRANSPARENT;
            ctx.set_visuals(visuals);

            if self.current_file.is_none() {
                let clip_files = crate::core::extract_clipboard_files();
                if let Some(first) = clip_files.first() {
                    self.select_file(first.clone());
                }
            }
        }

        if self.frame_count <= 60 || self.frame_count % 30 == 0 {
            #[cfg(target_os = "windows")]
            enforce_borderless_win32();
        }

        // Continuous driver honoring the Max-FPS knob (disables idle pacing).
        let max_fps = crate::core::MossambiConfig::load().max_fps;
        let is_active = self.is_converting || self.success_time.is_some();
        let repaint_delay = if max_fps {
            Duration::from_millis(2)
        } else if is_active {
            Duration::from_millis(16)
        } else {
            Duration::from_millis(120)
        };
        ctx.request_repaint_after(repaint_delay);

        // Drain pooled worker updates (keeps the pool channel bounded).
        while let Some(update) = self.worker_pool.try_recv() {
            match update {
                TaskUpdate::Started { name, .. } => log::debug!("widget worker started: {}", name),
                TaskUpdate::Progress {
                    fraction, status, ..
                } => log::debug!(
                    "widget worker progress {:.0}%: {}",
                    fraction * 100.0,
                    status
                ),
                TaskUpdate::Completed { message, .. } => {
                    log::info!("widget worker finished: {}", message)
                }
                TaskUpdate::Failed { error, .. } => log::error!("widget worker failed: {}", error),
            }
        }

        // 1. Process Conversion Results
        while let Ok(res) = self.result_rx.try_recv() {
            self.is_converting = false;
            match res {
                WidgetResult::Success(files) => {
                    self.converted_count = files.len();
                    self.success_time = Some(Instant::now());
                    self.last_output = files.first().cloned();
                    self.last_error = None;
                }
                WidgetResult::Failure(err) => {
                    self.converted_count = 0;
                    self.success_time = None;
                    self.error_time = Some(Instant::now());
                    log::error!("Widget conversion failed: {}", err);
                    self.last_error = Some(err);
                }
            }
        }

        // 2. Animate Conversion Progress
        if self.is_converting {
            self.progress += 0.02;
            if self.progress > 0.95 {
                self.progress = 0.95;
            }
        }

        // 3. Clear Success / Error State after a few seconds
        if let Some(t) = self.success_time {
            if t.elapsed() >= Duration::from_secs(4) {
                self.success_time = None;
            }
        }
        if let Some(t) = self.error_time {
            if t.elapsed() >= Duration::from_secs(6) {
                self.error_time = None;
                self.last_error = None;
            }
        }

        // 4. Handle Drag-and-Drop Files directly onto petals
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
                    if let Some(ref path) = drop.path {
                        self.select_file(path.clone());
                        let drop_pos = i.pointer.hover_pos().unwrap_or(self.geometry.center);
                        if let Some(carpel_idx) = self.geometry.hit_test_petal(drop_pos) {
                            if let Some(target) = self.sectors.get(carpel_idx).copied() {
                                self.start_conversion(target);
                            }
                        } else if self.geometry.hit_test_hub(drop_pos) {
                            if let Some(first_fmt) = self.sectors.first().copied() {
                                self.start_conversion(first_fmt);
                            }
                        }
                    }
                }
            }

            if i.pointer.button_released(egui::PointerButton::Primary) && !self.is_converting {
                if let Some(pos) = i.pointer.hover_pos() {
                    if let Some(carpel_idx) = self.geometry.hit_test_petal(pos) {
                        if let Some(target) = self.sectors.get(carpel_idx).copied() {
                            if self.current_file.is_some() {
                                self.start_conversion(target);
                            }
                        }
                    }
                }
            }

            if i.modifiers.command && i.key_pressed(Key::V) {
                let clip_files = crate::core::extract_clipboard_files();
                if let Some(first) = clip_files.first() {
                    self.select_file(first.clone());
                }
            }

            if i.key_pressed(Key::Escape) {
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
        });

        // 5. Render Frameless Transparent Widget (NO titlebar, NO borders, NO ribbon)
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(Color32::TRANSPARENT))
            .show(ctx, |ui| {
                let (rect, response) =
                    ui.allocate_exact_size(Vec2::new(400.0, 400.0), egui::Sense::click_and_drag());
                let painter = ui.painter_at(rect);
                self.geometry.center = rect.center();

                let hover_pos = response.hover_pos();
                self.hovered_carpel = hover_pos.and_then(|p| self.geometry.hit_test_petal(p));

                if response.dragged() {
                    if let Some(pos) = hover_pos {
                        if self.geometry.hit_test_hub(pos) && !self.is_converting {
                            // Hub drag semantics: with a file loaded and a
                            // real drag motion, hand the file to the OS drag
                            // mechanism (Windows OLE; Linux file clipboard).
                            // Without a file, move the widget window.
                            let decided = ui.ctx().input(|i| i.pointer.is_decidedly_dragging());
                            if self.current_file.is_some() {
                                if decided && !self.drag_out_in_flight {
                                    self.drag_out_in_flight = true;
                                    if let Some(path) = self.current_file.clone() {
                                        if let Err(err) = crate::core::begin_file_drag_out(
                                            std::slice::from_ref(&path),
                                        ) {
                                            self.last_error = Some(err);
                                        }
                                    }
                                }
                            } else {
                                ctx.send_viewport_cmd(ViewportCommand::StartDrag);
                            }
                        }
                    }
                }
                // Reset the per-press drag-out latch once the button is up.
                if !ui.ctx().input(|i| i.pointer.any_down()) {
                    self.drag_out_in_flight = false;
                }

                if response.clicked() {
                    if let Some(pos) = hover_pos {
                        if self.geometry.hit_test_hub(pos) && !self.is_converting {
                            if let Some(path) = rfd::FileDialog::new().pick_file() {
                                self.select_file(path);
                            }
                        } else if let Some(petal_idx) = self.hovered_carpel {
                            if let Some(fmt) = self.sectors.get(petal_idx).copied() {
                                if self.current_file.is_some() {
                                    self.start_conversion(fmt);
                                } else if let Some(path) = rfd::FileDialog::new().pick_file() {
                                    self.select_file(path);
                                    self.start_conversion(fmt);
                                }
                            }
                        }
                    }
                }

                if response.secondary_clicked() {
                    if let Some(pos) = hover_pos {
                        self.show_context_menu = true;
                        self.context_menu_pos = pos;
                    }
                }

                let current_state = if self.is_converting {
                    WidgetState::Converting {
                        progress: self.progress,
                    }
                } else if self.success_time.is_some() {
                    WidgetState::Success {
                        file_count: self.converted_count,
                    }
                } else if let Some(idx) = self.hovered_carpel {
                    WidgetState::Hovered(idx)
                } else {
                    WidgetState::Idle
                };

                self.geometry
                    .render(&painter, &self.sectors, current_state, self.hovered_carpel);

                // Hub status area: surface the last conversion failure where
                // success feedback appears (previously swallowed entirely).
                if !self.is_converting && self.success_time.is_none() && self.last_error.is_some() {
                    let err = self.last_error.clone().unwrap_or_default();
                    // Tail of the message: ffmpeg errors carry their verdict last.
                    let tail: String = err
                        .chars()
                        .rev()
                        .take(80)
                        .collect::<Vec<_>>()
                        .into_iter()
                        .rev()
                        .collect();
                    painter.text(
                        rect.center() + egui::vec2(0.0, 46.0),
                        egui::Align2::CENTER_CENTER,
                        tail,
                        egui::FontId::proportional(15.5),
                        Color32::from_rgb(214, 69, 57),
                    );
                }

                if self.show_context_menu {
                    let menu_rect =
                        Rect::from_min_size(self.context_menu_pos, Vec2::new(175.0, 110.0));
                    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(menu_rect), |ui| {
                        let fill_bg = if self.geometry.theme == WidgetTheme::Dark {
                            Color32::from_rgba_premultiplied(22, 26, 34, 245)
                        } else {
                            Color32::from_rgba_premultiplied(250, 250, 252, 245)
                        };
                        let border_col = if self.geometry.theme == WidgetTheme::Dark {
                            Color32::from_rgb(55, 62, 78)
                        } else {
                            Color32::from_rgb(215, 220, 228)
                        };

                        egui::Frame::none()
                            .fill(fill_bg)
                            .stroke(Stroke::new(1.0_f32, border_col))
                            .rounding(10.0)
                            .inner_margin(8.0)
                            .show(ui, |ui| {
                                ui.vertical(|ui| {
                                    let theme_text = if self.geometry.theme == WidgetTheme::Dark {
                                        " Light Mode"
                                    } else {
                                        " Dark Mode"
                                    };
                                    if ui.button(theme_text).clicked() {
                                        self.geometry.theme =
                                            if self.geometry.theme == WidgetTheme::Dark {
                                                WidgetTheme::Light
                                            } else {
                                                WidgetTheme::Dark
                                            };
                                        let mut cfg = crate::core::MossambiConfig::load();
                                        cfg.theme_mode = if self.geometry.theme == WidgetTheme::Dark
                                        {
                                            crate::core::ThemeMode::DarkAnthracite
                                        } else {
                                            crate::core::ThemeMode::LightBrilliant
                                        };
                                        let _ = cfg.save();
                                        let mut visuals =
                                            if self.geometry.theme == WidgetTheme::Dark {
                                                egui::Visuals::dark()
                                            } else {
                                                egui::Visuals::light()
                                            };
                                        visuals.window_fill = Color32::TRANSPARENT;
                                        visuals.panel_fill = Color32::TRANSPARENT;
                                        visuals.extreme_bg_color = Color32::TRANSPARENT;
                                        visuals.faint_bg_color = Color32::TRANSPARENT;
                                        ctx.set_visuals(visuals);
                                        self.show_context_menu = false;
                                    }

                                    let is_autostart =
                                        crate::core::MossambiConfig::is_autostart_configured();
                                    let autostart_label = if is_autostart {
                                        "Disable Run at Startup"
                                    } else {
                                        "Enable Run at Startup"
                                    };
                                    if ui.button(autostart_label).clicked() {
                                        let _ = crate::core::MossambiConfig::set_autostart(
                                            !is_autostart,
                                        );
                                        self.show_context_menu = false;
                                    }

                                    if ui.button(" Open Studio").clicked() {
                                        self.show_context_menu = false;
                                        #[cfg(target_os = "windows")]
                                        let _ = std::process::Command::new("mossambi.exe")
                                            .arg("gui")
                                            .spawn();
                                        #[cfg(not(target_os = "windows"))]
                                        let _ = std::process::Command::new("mossambi")
                                            .arg("gui")
                                            .spawn();
                                    }

                                    ui.separator();
                                    ui.label(
                                        egui::RichText::new(
                                            "Win+Shift+C: Wheel\nWin+Ctrl+C: Studio",
                                        )
                                        .size(17.5)
                                        .color(Color32::from_rgb(130, 136, 148)),
                                    );

                                    if ui.button(" Close Widget").clicked() {
                                        ctx.send_viewport_cmd(ViewportCommand::Close);
                                    }
                                });
                            });
                    });

                    if response.clicked() && !menu_rect.contains(hover_pos.unwrap_or(Pos2::ZERO)) {
                        self.show_context_menu = false;
                    }
                }
            });
    }
}

fn load_app_icon() -> Option<egui::IconData> {
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

/// Runs the Standalone Desktop Radial Fruit Widget
pub fn run_widget() -> Result<(), eframe::Error> {
    run_widget_with_args(None, None)
}

pub fn run_widget_with_args(
    initial_pos: Option<Pos2>,
    initial_file: Option<PathBuf>,
) -> Result<(), eframe::Error> {
    // Logging must come first so widget-scope diagnostics are captured.
    let _ = env_logger::try_init();

    let mut builder = ViewportBuilder::default()
        .with_title("Mossambi Radial Widget")
        .with_inner_size([400.0, 400.0])
        .with_min_inner_size([400.0, 400.0])
        .with_max_inner_size([400.0, 400.0])
        .with_resizable(false)
        .with_decorations(false) // ZERO titlebar, ZERO system borders
        .with_transparent(true) // 100% borderless transparency
        .with_minimize_button(false)
        .with_maximize_button(false)
        .with_close_button(false)
        .with_always_on_top();

    if let Some(icon) = load_app_icon() {
        builder = builder.with_icon(icon);
    }

    if let Some(pos) = initial_pos {
        builder = builder.with_position([pos.x.max(0.0), pos.y.max(0.0)]);
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

    eframe::run_native(
        "Mossambi Radial Widget",
        options,
        Box::new(move |cc| {
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

            let mut app = MossambiWidgetApp::default();
            if let Some(file) = initial_file {
                app.select_file(file);
            }
            Ok(Box::new(app))
        }),
    )
}
