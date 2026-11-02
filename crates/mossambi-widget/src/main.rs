//! Mossambi Standalone Desktop Radial Fruit Widget
//! 100% Topbarless, Menu-less, Ribbon-less, Frameless, 32-Bit Transparent Desktop Widget
//! Faithful reproduction of all 6 visual states from `Image reference.png`.

use eframe::egui::{self, Color32, Key, Pos2, Rect, Stroke, Vec2, ViewportBuilder, ViewportCommand};
use mossambi_codec::{transcode_file, LosslessMode};
use mossambi_core::{FileCategory, UniversalRegistry};
use mossambi_widget::{RadialWidgetGeometry, WidgetState, WidgetTheme};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
enum WidgetResult {
    Success(Vec<PathBuf>),
    Failure(String),
}

struct MossambiWidgetApp {
    geometry: RadialWidgetGeometry,
    sectors: Vec<&'static str>,
    hovered_carpel: Option<usize>,
    current_file: Option<PathBuf>,
    current_category: FileCategory,

    // State Tracking
    is_converting: bool,
    progress: f32,
    converted_count: usize,
    success_time: Option<Instant>,
    last_output: Option<PathBuf>,

    // Context Menu & Dragging
    show_context_menu: bool,
    context_menu_pos: Pos2,
    frame_count: u32,

    // Worker Channels
    result_rx: Receiver<WidgetResult>,
    result_tx: Sender<WidgetResult>,
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

            show_context_menu: false,
            context_menu_pos: Pos2::ZERO,
            frame_count: 0,

            result_rx: rx,
            result_tx: tx,
        }
    }
}

impl MossambiWidgetApp {
    fn select_file(&mut self, path: PathBuf) {
        let (cat, _ext) = UniversalRegistry::classify_file(&path);
        self.current_category = cat;
        self.sectors = UniversalRegistry::get_widget_sectors_for_category(cat);
        self.current_file = Some(path);
    }

    fn start_conversion(&mut self, target_format: &str) {
        let input = match self.current_file.clone() {
            Some(f) => f,
            None => return,
        };

        self.is_converting = true;
        self.progress = 0.15;
        let sender = self.result_tx.clone();
        let target = target_format.to_string();

        thread::spawn(move || {
            let res = match transcode_file(&input, &target, LosslessMode::StrictBitExact, None) {
                Ok(out) => WidgetResult::Success(vec![out]),
                Err(e) => WidgetResult::Failure(e.to_string()),
            };
            let _ = sender.send(res);
        });
    }
}

#[cfg(target_os = "windows")]
fn enforce_borderless_win32() {
    use std::ffi::c_void;
    use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetClassNameW, GetWindowLongPtrW, GetWindowThreadProcessId,
        SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE, GWL_STYLE, SWP_FRAMECHANGED, SWP_NOMOVE,
        SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW, WS_CLIPCHILDREN, WS_CLIPSIBLINGS,
        WS_EX_LAYERED, WS_EX_TOPMOST, WS_POPUP, WS_VISIBLE,
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
                // Strip WS_CAPTION (0x00C00000), WS_SYSMENU (0x00080000), WS_THICKFRAME (0x00040000),
                // WS_MINIMIZEBOX (0x00020000), WS_MAXIMIZEBOX (0x00010000), WS_BORDER (0x00800000), WS_DLGFRAME (0x00400000)
                // Mask 0x00CF0000 removes all caption and frame chrome.
                let target_style = (style & !0x00CF0000)
                    | WS_POPUP
                    | WS_VISIBLE
                    | WS_CLIPSIBLINGS
                    | WS_CLIPCHILDREN;

                let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
                // Strip WS_EX_WINDOWEDGE (0x00000100), WS_EX_CLIENTEDGE (0x00000200), WS_EX_STATICEDGE (0x00020000), WS_EX_DLGMODALFRAME (0x00000001)
                // Add WS_EX_LAYERED (0x00080000) and WS_EX_TOPMOST (0x00000008)
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

                // Physically clip and shape the OS window hierarchy into an organic circular fruit region.
                // Guarantees ZERO rectangular window corners exist in the Windows window manager.
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
                        SWP_FRAMECHANGED
                            | SWP_NOMOVE
                            | SWP_NOSIZE
                            | SWP_NOZORDER
                            | SWP_SHOWWINDOW,
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
            let mut visuals = egui::Visuals::light();
            visuals.window_fill = Color32::TRANSPARENT;
            visuals.panel_fill = Color32::TRANSPARENT;
            visuals.extreme_bg_color = Color32::TRANSPARENT;
            visuals.faint_bg_color = Color32::TRANSPARENT;
            ctx.set_visuals(visuals);

            // Automatically check clipboard for copied files if none was passed via CLI
            if self.current_file.is_none() {
                let clip_files = mossambi_core::extract_clipboard_files();
                if let Some(first) = clip_files.first() {
                    self.select_file(first.clone());
                }
            }
        }

        // Continuously hammer Win32 styles for the first 60 frames (first ~1s of lifecycle)
        // and periodically every 30 frames to squash any winit reset during visibility or sizing
        if self.frame_count <= 60 || self.frame_count % 30 == 0 {
            #[cfg(target_os = "windows")]
            enforce_borderless_win32();
        }

        // Adaptive repaint: 16ms (60 FPS) when converting or animating, 120ms when idle
        let is_active = self.is_converting || self.success_time.is_some();
        let repaint_delay = if is_active {
            Duration::from_millis(16)
        } else {
            Duration::from_millis(120)
        };
        ctx.request_repaint_after(repaint_delay);

        // 1. Process Conversion Results
        if let Ok(res) = self.result_rx.try_recv() {
            self.is_converting = false;
            match res {
                WidgetResult::Success(files) => {
                    self.converted_count = files.len();
                    self.success_time = Some(Instant::now());
                    self.last_output = files.first().cloned();
                }
                WidgetResult::Failure(_err) => {
                    self.converted_count = 0;
                    self.success_time = None;
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

        // 3. Clear Success State after 4 seconds
        if let Some(t) = self.success_time {
            if t.elapsed() >= Duration::from_secs(4) {
                self.success_time = None;
            }
        }

        // 4. Handle Drag-and-Drop Files directly onto petals
        ctx.input(|i| {
            // Hovered files
            if let Some(hover) = i.raw.hovered_files.first() {
                if let Some(ref path) = hover.path {
                    if self.current_file.as_ref() != Some(path) {
                        self.select_file(path.clone());
                    }
                }
            }

            // Dropped files
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

            // Handle button release over petal when file is pre-loaded via gesture activation
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

            // Clipboard paste shortcut (Ctrl+V / Cmd+V)
            if i.modifiers.command && i.key_pressed(Key::V) {
                let clip_files = mossambi_core::extract_clipboard_files();
                if let Some(first) = clip_files.first() {
                    self.select_file(first.clone());
                }
            }

            // Keyboard shortcut ESC to dismiss
            if i.key_pressed(Key::Escape) {
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
        });

        // 5. Render Frameless Transparent Widget (NO titlebar, NO borders, NO ribbon)
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(Color32::TRANSPARENT))
            .show(ctx, |ui| {
                let (rect, response) = ui.allocate_exact_size(Vec2::new(400.0, 400.0), egui::Sense::click_and_drag());
                let painter = ui.painter_at(rect);
                self.geometry.center = rect.center();

                let hover_pos = response.hover_pos();
                self.hovered_carpel = hover_pos.and_then(|p| self.geometry.hit_test_petal(p));

                // Dragging the entire window across desktop by dragging the hub or canvas
                if response.dragged() {
                    if let Some(pos) = hover_pos {
                        if self.geometry.hit_test_hub(pos) && !self.is_converting {
                            ctx.send_viewport_cmd(ViewportCommand::StartDrag);
                        }
                    }
                }

                // Clicking center hub to pick file
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

                // Right click shows sleek frosted context popup
                if response.secondary_clicked() {
                    if let Some(pos) = hover_pos {
                        self.show_context_menu = true;
                        self.context_menu_pos = pos;
                    }
                }

                // Determine Current State (1..6 from Image reference.png)
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

                // Render the complete 8-Petal Radial Fruit Geometry
                self.geometry.render(&painter, &self.sectors, current_state, self.hovered_carpel);

                // 6. Right-Click Context Menu Pill
                if self.show_context_menu {
                    let menu_rect = Rect::from_min_size(self.context_menu_pos, Vec2::new(175.0, 110.0));
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
                                        "☀️ Light Mode"
                                    } else {
                                        "🌙 Dark Mode"
                                    };
                                    if ui.button(theme_text).clicked() {
                                        self.geometry.theme = if self.geometry.theme == WidgetTheme::Dark {
                                            WidgetTheme::Light
                                        } else {
                                            WidgetTheme::Dark
                                        };
                                        let mut visuals = if self.geometry.theme == WidgetTheme::Dark {
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

                                    let is_autostart = mossambi_core::MossambiConfig::is_autostart_configured();
                                    let autostart_label = if is_autostart {
                                        "✓ Run at Startup"
                                    } else {
                                        "🚀 Run at Startup"
                                    };
                                    if ui.button(autostart_label).clicked() {
                                        let _ = mossambi_core::MossambiConfig::set_autostart(!is_autostart);
                                        self.show_context_menu = false;
                                    }

                                    if ui.button("🖥️ Open Studio (Win+Ctrl+C)").clicked() {
                                        self.show_context_menu = false;
                                        #[cfg(target_os = "windows")]
                                        let _ = std::process::Command::new("mossambi-gui.exe").spawn();
                                        #[cfg(not(target_os = "windows"))]
                                        let _ = std::process::Command::new("mossambi-gui").spawn();
                                    }

                                    ui.separator();
                                    ui.label(
                                        egui::RichText::new("Win+Shift+C: Wheel\nWin+Ctrl+C: Studio")
                                            .size(9.5)
                                            .color(Color32::from_rgb(130, 136, 148)),
                                    );

                                    if ui.button("❌ Close Widget").clicked() {
                                        ctx.send_viewport_cmd(ViewportCommand::Close);
                                    }
                                });
                            });
                    });

                    // Dismiss context menu if clicking outside
                    if response.clicked() && !menu_rect.contains(hover_pos.unwrap_or(Pos2::ZERO)) {
                        self.show_context_menu = false;
                    }
                }
            });
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

    let args: Vec<String> = std::env::args().collect();
    let mut initial_pos: Option<Pos2> = None;
    let mut initial_file: Option<PathBuf> = None;

    let mut i = 1;
    while i < args.len() {
        if args[i] == "--pos" && i + 1 < args.len() {
            let parts: Vec<&str> = args[i + 1].split(',').collect();
            if parts.len() == 2 {
                if let (Ok(x), Ok(y)) = (parts[0].trim().parse::<f32>(), parts[1].trim().parse::<f32>()) {
                    initial_pos = Some(Pos2::new(x - 200.0, y - 200.0));
                }
            }
            i += 2;
        } else if args[i] == "--active-drag" {
            i += 1;
        } else if !args[i].starts_with('-') {
            let p = PathBuf::from(&args[i]);
            if p.exists() {
                initial_file = Some(p);
            }
            i += 1;
        } else {
            i += 1;
        }
    }

    let mut builder = ViewportBuilder::default()
        .with_title("")
        .with_inner_size([400.0, 400.0])
        .with_min_inner_size([400.0, 400.0])
        .with_max_inner_size([400.0, 400.0])
        .with_resizable(false)
        .with_decorations(false) // ZERO titlebar, ZERO system borders
        .with_transparent(true)  // 100% borderless transparency
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

    let options = eframe::NativeOptions {
        viewport: builder,
        ..Default::default()
    };

    eframe::run_native(
        "",
        options,
        Box::new(move |_cc| {
            let mut app = MossambiWidgetApp::default();
            if let Some(file) = initial_file {
                app.select_file(file);
            }
            Ok(Box::new(app))
        }),
    )
}
