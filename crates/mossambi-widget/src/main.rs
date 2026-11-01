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

    // Worker Channels
    result_rx: Receiver<WidgetResult>,
    result_tx: Sender<WidgetResult>,
}

impl Default for MossambiWidgetApp {
    fn default() -> Self {
        let (tx, rx) = channel();
        // Default petals matching Image reference.png: DOCX, JPG, WEBP, HEIC, TIFF, AVIF, BMP, PDF
        let default_sectors = vec!["JPG", "WEBP", "HEIC", "TIFF", "AVIF", "BMP", "PDF", "DOCX"];

        Self {
            geometry: RadialWidgetGeometry::default(),
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

impl eframe::App for MossambiWidgetApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Adaptive repaint: 16ms (60 FPS) when converting or animating, 150ms when idle
        let is_active = self.is_converting || self.success_time.is_some();
        let repaint_delay = if is_active {
            Duration::from_millis(16)
        } else {
            Duration::from_millis(150)
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

            // Keyboard shortcut ESC to dismiss
            if i.key_pressed(Key::Escape) {
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
        });

        // 5. Render Frameless Transparent Widget (NO titlebar, NO borders, NO ribbon)
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(Color32::TRANSPARENT))
            .show(ctx, |ui| {
                let (rect, response) = ui.allocate_exact_size(Vec2::new(360.0, 360.0), egui::Sense::click_and_drag());
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
                    let menu_rect = Rect::from_min_size(self.context_menu_pos, Vec2::new(170.0, 100.0));
                    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(menu_rect), |ui| {
                        let fill_bg = if self.geometry.theme == WidgetTheme::Dark {
                            Color32::from_rgba_premultiplied(30, 34, 42, 240)
                        } else {
                            Color32::from_rgba_premultiplied(250, 250, 252, 240)
                        };
                        let border_col = if self.geometry.theme == WidgetTheme::Dark {
                            Color32::from_rgb(60, 68, 85)
                        } else {
                            Color32::from_rgb(220, 224, 230)
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
                    initial_pos = Some(Pos2::new(x - 180.0, y - 180.0));
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
        .with_title("Mossambi Radial Widget")
        .with_inner_size([360.0, 360.0])
        .with_min_inner_size([360.0, 360.0])
        .with_max_inner_size([360.0, 360.0])
        .with_resizable(false)
        .with_decorations(false) // ZERO titlebar, ZERO system borders
        .with_transparent(true)  // 100% borderless transparency
        .with_always_on_top();

    if let Some(pos) = initial_pos {
        builder = builder.with_position([pos.x.max(0.0), pos.y.max(0.0)]);
    }

    let options = eframe::NativeOptions {
        viewport: builder,
        ..Default::default()
    };

    eframe::run_native(
        "Mossambi Radial Widget",
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
