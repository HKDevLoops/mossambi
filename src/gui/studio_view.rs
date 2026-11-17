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

use crate::core::config::MossambiConfig;
use crate::core::conversion_queue::{ConversionQueue, QueueAction, QueueItemStatus};
use crate::core::icon_packs::IconGlyph;
use crate::core::look_spec::{
    AddressBarStyle, SearchPosition, SidebarHeaderStyle, TabPosition, TitleBarStyle, ToolbarItem,
    WindowControlsPosition,
};
use crate::core::{FileCategory, UniversalRegistry};
use crate::gui::command_palette::{CommandPalette, PaletteAction};
use crate::gui::design::{self, radius, type_scale};
use crate::gui::explorer_view::ExplorerView;
use crate::gui::finder_chrome::{
    render_sidebar_row, render_sidebar_zone_header, render_sidebar_zone_header_with_drop,
    SidebarRowSpec,
};
use crate::gui::macos_theme::*;
use crate::gui::quick_settings::QuickSettingsState;
use eframe::egui::{
    self, Align2, Color32, FontId, Pos2, Rect, Rounding, Sense, Stroke, Ui, Vec2, ViewportCommand,
};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct PinnedRowDragPayload {
    pub from_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StudioTab {
    FileExplorer,
    Transcoder,
    ExamStudio,
    QualityInspector,
    ArchiveSuite,
    HardwareTelemetry,
    Preferences,
}

/// Display label + glyph for the tab-strip pill of a non-explorer
/// view (B-3). Names and glyphs mirror the sidebar Tools list.
fn studio_tab_label(tab: StudioTab) -> (&'static str, IconGlyph) {
    match tab {
        StudioTab::Transcoder => ("Transcoder", IconGlyph::ActionRefresh),
        StudioTab::ExamStudio => ("Custom Conformer", IconGlyph::ActionEdit),
        StudioTab::QualityInspector => ("Quality Inspector", IconGlyph::Search),
        StudioTab::ArchiveSuite => ("Archive Suite", IconGlyph::PageArchive),
        StudioTab::HardwareTelemetry => ("Hardware & AI", IconGlyph::ActionShield),
        StudioTab::Preferences => ("Preferences", IconGlyph::Gear),
        StudioTab::FileExplorer => ("File Explorer", IconGlyph::Folder),
    }
}

fn dispatch_sidebar_drop(
    explorer: &mut ExplorerView,
    ui: &egui::Ui,
    dest: &Path,
    payload: crate::gui::explorer_view::FileDragPayload,
) {
    let (ctrl, shift, alt) = ui.input(|i| {
        (
            i.modifiers.ctrl || i.modifiers.command,
            i.modifiers.shift,
            i.modifiers.alt,
        )
    });
    let released_right = ui.input(|i| i.pointer.button_released(egui::PointerButton::Secondary));
    let button =
        if released_right || payload.button == crate::core::drop_action::DropMouseButton::Right {
            crate::core::drop_action::DropMouseButton::Right
        } else {
            crate::core::drop_action::DropMouseButton::Left
        };
    let drop_pos = ui
        .input(|i| i.pointer.hover_pos())
        .unwrap_or(egui::Pos2::new(100.0, 100.0));
    explorer.handle_drop(crate::gui::explorer_view::PendingDrop {
        dest_dir: dest.to_path_buf(),
        sources: payload.paths,
        button,
        modifiers: crate::core::drop_action::DropModifiers { ctrl, shift, alt },
        position: drop_pos,
    });
}

/// True when `events` contains a key press matching a stored display binding
/// ("Ctrl+P"). Modifier tokens: ctrl (cmd/command/win/super alias to ctrl),
/// alt, shift; any other token is the key name, matched case-insensitively
/// against the egui Key debug name. Modifiers must match exactly.
fn shortcut_pressed(events: &[egui::Event], binding: &str) -> bool {
    let binding = binding.trim();
    if binding.is_empty() {
        return false;
    }
    let mut want_ctrl = false;
    let mut want_alt = false;
    let mut want_shift = false;
    let mut key_name = String::new();
    for token in binding.split('+') {
        let token = token.trim().to_ascii_lowercase();
        match token.as_str() {
            "ctrl" | "cmd" | "command" | "win" | "super" => want_ctrl = true,
            "alt" | "option" => want_alt = true,
            "shift" => want_shift = true,
            _ => key_name = token,
        }
    }
    if key_name.is_empty() {
        return false;
    }
    events.iter().any(|event| match event {
        egui::Event::Key {
            key,
            pressed: true,
            modifiers,
            ..
        } => {
            modifiers.ctrl == want_ctrl
                && modifiers.alt == want_alt
                && modifiers.shift == want_shift
                && format!("{:?}", key).eq_ignore_ascii_case(key_name.as_str())
        }
        _ => false,
    })
}

pub struct StudioView {
    pub current_tab: StudioTab,
    pub previous_tab: StudioTab,
    pub explorer_view: crate::gui::explorer_view::ExplorerView,
    pub exam_status: String,
    pub last_exam_output: Option<PathBuf>,
    pub quality_report: Option<crate::codec::FileQualityReport>,
    pub bench_report: Option<crate::codec::BenchmarkReport>,
    pub inspection_status: String,
    pub inspected_file_path: Option<PathBuf>,
    pub catalogue_search: String,
    pub catalogue_category: Option<FileCategory>,

    // Sidebar zone collapse states live in MossambiConfig (sidebar_*_expanded
    // keys) so they persist across restarts.

    // macOS System Settings-style Preferences surface
    pub prefs_view: crate::gui::preferences_view::PreferencesView,

    // Finder-style Command Palette (Ctrl+P / Cmd+K)
    pub command_palette: CommandPalette,

    /// Flag requesting window minimize animation
    pub request_minimize: bool,

    // Live script execution console: worker threads send outcomes here
    // through a channel; the Scripts preferences page drains it per frame.
    pub script_console: crate::core::scripting::ScriptConsole,

    // Universal Media Conformer Studio State
    pub conformer_unit: usize,
    pub conformer_preset_selector: String,

    // Conversion queue (W-7): per-item jobs, deferred actions, overall progress
    pub queue: ConversionQueue,
    pub queue_actions: Vec<QueueAction>,
    pub batch_progress: Option<f32>,

    // Background tool operations (W-7 Batch B): inspection, benchmarking,
    // and hardware telemetry run on plain threads and drain once per frame.
    pub tool_console: crate::core::tool_console::ToolConsole,
    pub telemetry_hw: Option<crate::hwaccel::HwCapabilities>,
    pub telemetry_specs: Option<crate::hwaccel::SystemSpecs>,
    pub telemetry_detecting: bool,
    pub inspection_busy: bool,
    pub bench_busy: bool,

    // Once-per-open listings for the sibling-folder context menu
    // (F-2 fix): cleared when a navigation target is applied.
    pub sibling_menu_cache: std::collections::HashMap<PathBuf, Vec<crate::core::FileItem>>,

    /// Smooth animated sidebar width spring for collapse/expand (Ctrl+B).
    pub sidebar_width_spring: crate::gui::anim::Spring,

    // Navigation button micro-animation click timestamps
    pub nav_anim_back: Option<std::time::Instant>,
    pub nav_anim_fwd: Option<std::time::Instant>,
    pub nav_anim_up: Option<std::time::Instant>,
    pub nav_anim_refresh: Option<std::time::Instant>,
    pub nav_anim_newfolder: Option<std::time::Instant>,

    /// Cached Mossambi brand-logo texture for the top-left header. Built once
    /// from the bundled PNG via the `image` crate (egui's `include_image!`
    /// bytes loader fails to decode here and renders a red error texture).
    pub brand_logo_tex: Option<egui::TextureHandle>,
}

impl Default for StudioView {
    fn default() -> Self {
        Self {
            current_tab: StudioTab::FileExplorer,
            previous_tab: StudioTab::FileExplorer,
            explorer_view: crate::gui::explorer_view::ExplorerView::default(),
            exam_status: "Ready to optimize for portal boundaries.".to_string(),
            last_exam_output: None,
            quality_report: None,
            bench_report: None,
            inspection_status: "Select a file to inspect.".to_string(),
            inspected_file_path: None,
            catalogue_search: String::new(),
            catalogue_category: None,

            sidebar_width_spring: crate::gui::anim::Spring::new(240.0),
            nav_anim_back: None,
            nav_anim_fwd: None,
            nav_anim_up: None,
            nav_anim_refresh: None,
            nav_anim_newfolder: None,

            prefs_view: crate::gui::preferences_view::PreferencesView::default(),
            command_palette: CommandPalette::default(),
            request_minimize: false,
            script_console: crate::core::scripting::ScriptConsole::default(),

            conformer_unit: 0,
            conformer_preset_selector: "Passport Photo (35x45mm / 300 DPI)".to_string(),

            queue: ConversionQueue::new(),
            queue_actions: Vec::new(),
            batch_progress: None,
            tool_console: crate::core::tool_console::ToolConsole::default(),
            telemetry_hw: None,
            telemetry_specs: None,
            telemetry_detecting: false,
            inspection_busy: false,
            bench_busy: false,
            sibling_menu_cache: std::collections::HashMap::new(),
            brand_logo_tex: None,
        }
    }
}

impl StudioView {
    /// Switches the current studio view tab, recording the previous tab
    /// so Ctrl+W or close can restore it.
    pub fn switch_to_tab(&mut self, tab: StudioTab) {
        if self.current_tab != tab {
            self.previous_tab = self.current_tab;
            self.current_tab = tab;
        }
    }

    /// Visible exit affordance for every non-explorer tool view.
    ///
    /// Users reported that settings/tools could only be left with Ctrl+W;
    /// this header puts a persistent Done button (plus the view title) at
    /// the top of Preferences and every tool tab. It never touches the
    /// explorer view, so the main file manager keeps its full canvas.
    fn render_tool_view_header(&mut self, ui: &mut Ui, palette: &ThemePalette, title: &str) {
        ui.horizontal(|ui| {
            // Apple-style back affordance: a crisp vector chevron (not the
            // ASCII "<") next to the label, on a squircle with a hairline
            // border instead of a heavy boxed button.
            let pack = crate::core::icon_packs::IconPack::from_config_id(
                &MossambiConfig::load().icon_pack,
            );
            let accent = [palette.accent.r(), palette.accent.g(), palette.accent.b()];
            let back_tex = crate::core::icon_packs::icon_texture(
                ui.ctx(),
                pack,
                IconGlyph::ActionBack,
                15.0,
                accent,
            );
            let label = egui::RichText::new("Back")
                .color(palette.accent)
                .size(type_scale::BODY);
            let back_btn = match back_tex {
                Some(t) => egui::Button::image_and_text(
                    egui::load::SizedTexture::new(t.id(), Vec2::splat(15.0)),
                    label,
                ),
                None => egui::Button::new(label),
            };
            let done = ui.add(
                back_btn
                    .fill(palette.card_bg)
                    .stroke(Stroke::new(0.5_f32, palette.border))
                    .rounding(Rounding::same(radius::MD)),
            );
            if done.clicked() {
                self.current_tab = if self.previous_tab != self.current_tab
                    && self.previous_tab != StudioTab::Preferences
                {
                    self.previous_tab
                } else {
                    StudioTab::FileExplorer
                };
            }
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new(title)
                    .size(16.0)
                    .strong()
                    .color(palette.text_primary),
            );
        });
        ui.separator();
    }

    /// Sibling-folder menu listing, loaded once per directory and
    /// cached (F-2 fix): the context-menu closure re-runs every frame
    /// while open, so a synchronous re-list each frame would freeze
    /// the UI on slow directories.
    fn sibling_dirs(&mut self, parent_dir: &Path) -> Vec<crate::core::FileItem> {
        if !self.sibling_menu_cache.contains_key(parent_dir) {
            let dirs: Vec<crate::core::FileItem> = crate::core::list_directory_sync(parent_dir)
                .into_iter()
                .filter(|e| e.is_dir)
                .collect();
            self.sibling_menu_cache
                .insert(parent_dir.to_path_buf(), dirs);
        }
        self.sibling_menu_cache
            .get(parent_dir)
            .cloned()
            .unwrap_or_default()
    }

    /// Per-tab OS drag-hover banner (W-7 Batch C): a slim accent frame
    /// shown while external files hover the window, mirroring the
    /// explorer banner. Each studio tab states what a release will do.
    fn render_tab_drop_banner(ui: &mut Ui, palette: &ThemePalette, message: &str) {
        let hovered = ui.input(|i| !i.raw.hovered_files.is_empty());
        if !hovered {
            return;
        }
        ui.add_space(4.0);
        let drop_frame = egui::Frame::none()
            .fill(Color32::from_rgba_unmultiplied(120, 190, 32, 50))
            .stroke(Stroke::new(1.5_f32, palette.accent))
            .rounding(Rounding::same(8.0))
            .inner_margin(egui::Margin::symmetric(14.0, 7.0));
        drop_frame.show(ui, |ui| {
            ui.label(egui::RichText::new(message).strong().color(palette.accent));
        });
        ui.add_space(4.0);
    }

    fn render_zone_header(
        ui: &mut Ui,
        palette: &ThemePalette,
        title: &str,
        expanded: bool,
        style: SidebarHeaderStyle,
    ) -> bool {
        match style {
            SidebarHeaderStyle::SectionTitle => {
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new(title.to_uppercase())
                        .size(11.0)
                        .color(palette.text_muted)
                        .strong(),
                );
                ui.add_space(4.0);
                false
            }
            SidebarHeaderStyle::CollapsibleZone => {
                render_sidebar_zone_header(ui, palette, title, expanded)
            }
            SidebarHeaderStyle::None => false,
        }
    }

    /// Renders the complete macOS Studio window with sidebar and content area.
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        ctx: &egui::Context,
        current_theme: &mut ThemeStyle,
        config: &mut MossambiConfig,
        current_file: &Option<PathBuf>,
        quick_settings: &mut QuickSettingsState,
        mut on_pick_file: impl FnMut(),
        mut on_stage_file: impl FnMut(PathBuf),
        mut on_convert_batch: impl FnMut(Vec<PathBuf>, String),
        on_open_folder: impl FnMut(&Path),
    ) {
        let palette = current_theme.colors();

        // Drain tool-console events from background threads (W-7 Batch B):
        // inspection, benchmarking, and telemetry finish off the UI thread
        // and land here.
        for event in self.tool_console.drain() {
            match event {
                crate::core::tool_console::ToolEvent::Inspected { path, result } => {
                    self.inspection_busy = false;
                    match result {
                        Ok(rep) => {
                            self.quality_report = Some(*rep);
                            self.inspected_file_path = Some(path);
                            self.inspection_status = "Inspection complete.".to_string();
                        }
                        Err(e) => {
                            self.inspection_status = format!("Inspection failed: {}", e);
                        }
                    }
                }
                crate::core::tool_console::ToolEvent::Benchmarked { result, .. } => {
                    self.bench_busy = false;
                    match result {
                        Ok(bench) => self.bench_report = Some(*bench),
                        Err(e) => {
                            self.inspection_status = format!("Benchmark failed: {}", e);
                        }
                    }
                }
                crate::core::tool_console::ToolEvent::Telemetry { hw, specs } => {
                    self.telemetry_hw = Some(*hw);
                    self.telemetry_specs = Some(*specs);
                    self.telemetry_detecting = false;
                }
            }
        }

        // Hardware telemetry is probed once, on a plain thread, then cached
        // on the view for the transcoder header and the telemetry tab.
        if self.telemetry_hw.is_none() && !self.telemetry_detecting {
            self.telemetry_detecting = true;
            crate::core::tool_console::detect_telemetry_async(self.tool_console.sender());
        }

        // Rebindable studio shortcuts: match this frame's raw key events
        // against the stored display bindings. While the Shortcuts page is
        // capturing a rebind, the studio shortcuts stay quiet so the pressed
        // key lands in the binding instead.
        let capturing = self.prefs_view.capturing_action.is_some();
        let events: Vec<egui::Event> = ctx.input(|i| i.events.clone());
        let toggle_palette =
            !capturing && shortcut_pressed(&events, &config.shortcut_for("palette"));
        let palette_cmd_mode = !capturing
            && ctx.input(|i| {
                (i.modifiers.ctrl || i.modifiers.command)
                    && i.modifiers.shift
                    && !i.modifiers.alt
                    && i.key_pressed(egui::Key::P)
            });
        let palette_quick_open = !capturing
            && ctx.input(|i| {
                (i.modifiers.ctrl || i.modifiers.command)
                    && !i.modifiers.shift
                    && !i.modifiers.alt
                    && i.key_pressed(egui::Key::P)
            });
        // Fixed secondary alias (Ctrl+K / Cmd+K) so the palette stays
        // discoverable even after a rebind.
        let palette_alias = !capturing
            && ctx.input(|i| {
                (i.modifiers.ctrl || i.modifiers.command)
                    && !i.modifiers.shift
                    && !i.modifiers.alt
                    && i.key_pressed(egui::Key::K)
            });
        if palette_cmd_mode {
            self.command_palette.open_with_query(">");
        } else if toggle_palette || palette_alias || palette_quick_open {
            self.command_palette.toggle();
        }
        // B-1 gate flag: explorer raw-key handling is suppressed
        // while the palette modal is open (read in explorer_view).
        self.explorer_view.palette_open = self.command_palette.is_open;

        if self.explorer_view.requested_open_preferences {
            self.explorer_view.requested_open_preferences = false;
            self.switch_to_tab(StudioTab::Preferences);
        }

        let open_preferences = !capturing
            && ctx.input(|i| {
                (i.modifiers.ctrl || i.modifiers.command)
                    && !i.modifiers.shift
                    && !i.modifiers.alt
                    && i.key_pressed(egui::Key::Comma)
            });
        if open_preferences && !self.command_palette.is_open {
            if self.current_tab == StudioTab::Preferences {
                self.current_tab = if self.previous_tab != StudioTab::Preferences {
                    self.previous_tab
                } else {
                    StudioTab::FileExplorer
                };
            } else {
                self.switch_to_tab(StudioTab::Preferences);
            }
        }

        let open_new_tab = !capturing && shortcut_pressed(&events, &config.shortcut_for("new_tab"));
        if open_new_tab && !self.command_palette.is_open {
            self.switch_to_tab(StudioTab::FileExplorer);
            self.explorer_view
                .add_tab(self.explorer_view.current_dir.clone());
        }

        let close_tab = !capturing && shortcut_pressed(&events, &config.shortcut_for("close_tab"));
        if close_tab && !self.command_palette.is_open {
            let action = crate::core::close_target(
                self.current_tab == StudioTab::FileExplorer,
                self.explorer_view.tabs.len(),
                self.explorer_view.active_tab_index,
            );
            match action {
                crate::core::CloseAction::LeaveToolView => {
                    self.current_tab = if self.previous_tab != self.current_tab
                        && self.previous_tab != StudioTab::Preferences
                    {
                        self.previous_tab
                    } else {
                        StudioTab::FileExplorer
                    };
                }
                crate::core::CloseAction::CloseExplorerTab(idx) => {
                    self.explorer_view.close_tab(idx);
                }
                crate::core::CloseAction::Nothing => {}
            }
        }

        // Esc closes Preferences when no text field has focus
        if self.current_tab == StudioTab::Preferences
            && !ctx.wants_keyboard_input()
            && ctx.input(|i| i.key_pressed(egui::Key::Escape))
        {
            self.current_tab = if self.previous_tab != self.current_tab
                && self.previous_tab != StudioTab::Preferences
            {
                self.previous_tab
            } else {
                StudioTab::FileExplorer
            };
        }

        // 1. Unified Top Panel (Windows 11 Fluent Tabs + Navigation Toolbar)
        // Use the chrome surface so the DWM acrylic backdrop shows through.
        let top_frame = egui::Frame::none()
            .fill(palette.window_bg)
            .inner_margin(egui::Margin::symmetric(10.0, 4.0));

        let is_max = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
        let chrome_spec = config.chrome_spec();

        egui::TopBottomPanel::top("studio_unified_header")
            .frame(top_frame)
            .show(ctx, |ui| {
                // ==============================================================
                // ROW 1: Tabs + Drag Window Area + Window Controls
                // ==============================================================
                let mut do_minimize = false;
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing =
                        Vec2::new(chrome_spec.spacing.toolbar_item_spacing, 0.0);

                    // Left traffic lights (Finder style)
                    if chrome_spec.window_controls == WindowControlsPosition::LeftTraffic {
                        crate::gui::finder_chrome::render_traffic_lights(
                            ui,
                            &palette,
                            || ctx.send_viewport_cmd(ViewportCommand::Close),
                            || do_minimize = true,
                            || ctx.send_viewport_cmd(ViewportCommand::Maximized(!is_max)),
                        );
                        ui.add_space(6.0);
                    }

                    // Brand: the real Mossambi logo (green fruit), replacing the
                    // placeholder shield. Decoded once from the bundled PNG via
                    // the `image` crate: egui's `include_image!` bytes loader
                    // rendered a red error texture on this build.
                    if self.brand_logo_tex.is_none() {
                        if let Ok(img) =
                            image::load_from_memory(&include_bytes!("../../assets/icons/mossambi-32x32.png")[..])
                        {
                            let rgba = img.to_rgba8();
                            let (w, h) = rgba.dimensions();
                            let color_image = egui::ColorImage::from_rgba_unmultiplied(
                                [w as usize, h as usize],
                                rgba.as_raw(),
                            );
                            self.brand_logo_tex = Some(ctx.load_texture(
                                "mossambi_brand_logo",
                                color_image,
                                egui::TextureOptions::LINEAR,
                            ));
                        }
                    }
                    if let Some(tex) = &self.brand_logo_tex {
                        ui.add(egui::Image::new(tex).fit_to_exact_size(Vec2::splat(20.0)));
                    }

                    if chrome_spec.tab_position != TabPosition::None {
                        // Tabs
                        let mut tab_to_select: Option<usize> = None;
                        let mut tab_to_close: Option<usize> = None;
                        let tabs_count = self.explorer_view.tabs.len();
                        let pack = self.explorer_view.icon_pack;

                        for (idx, tab) in self.explorer_view.tabs.iter().enumerate() {
                            let is_active = self.current_tab == StudioTab::FileExplorer
                                && idx == self.explorer_view.active_tab_index;
                            let tab_bg = if is_active {
                                palette.card_bg
                            } else {
                                Color32::TRANSPARENT
                            };
                            let tab_stroke = if is_active {
                                Stroke::new(1.0_f32, palette.border)
                            } else {
                                Stroke::NONE
                            };

                            let (rect, resp) =
                                ui.allocate_exact_size(Vec2::new(130.0, 26.0), Sense::click());
                            let is_hovered = resp.hovered();

                            let bg = if is_active {
                                tab_bg
                            } else if is_hovered {
                                palette.card_hover
                            } else {
                                tab_bg
                            };

                            crate::gui::squircle::squircle_filled(ui.painter(),  rect, Rounding::same(6.0), bg);
                            if is_active {
                                crate::gui::squircle::squircle_stroke(ui.painter(),  rect, Rounding::same(6.0), tab_stroke);
                            }

                            // Real vector tab icon
                            let tab_glyph = if tab.is_home {
                                IconGlyph::Home
                            } else if let Some(sf) =
                                crate::core::special_folders::lookup_path(&tab.path)
                            {
                                IconGlyph::FolderSpecial(sf)
                            } else {
                                IconGlyph::Folder
                            };
                            let tab_icon_rect = Rect::from_center_size(
                                rect.left_center() + Vec2::new(14.0, 0.0),
                                Vec2::splat(16.0),
                            );
                            let tab_icon_rgb = if is_active {
                                [79, 168, 255]
                            } else {
                                [
                                    palette.text_secondary.r(),
                                    palette.text_secondary.g(),
                                    palette.text_secondary.b(),
                                ]
                            };
                            crate::core::icon_packs::paint_icon(
                                ui,
                                pack,
                                tab_glyph,
                                tab_icon_rect,
                                tab_icon_rgb,
                            );

                            // Tab title (truncated)
                            let max_title_chars = 10;
                            let title_display = if tab.title.chars().count() > max_title_chars {
                                format!(
                                    "{}...",
                                    tab.title.chars().take(max_title_chars).collect::<String>()
                                )
                            } else {
                                tab.title.clone()
                            };

                            ui.painter().text(
                                rect.left_center() + Vec2::new(30.0, 0.0),
                                egui::Align2::LEFT_CENTER,
                                title_display,
                                egui::FontId::proportional(13.5),
                                if is_active {
                                    palette.text_primary
                                } else {
                                    palette.text_secondary
                                },
                            );

                            // Close button on tab
                            if tabs_count > 1 {
                                let close_rect = Rect::from_center_size(
                                    rect.right_center() + Vec2::new(-12.0, 0.0),
                                    Vec2::splat(14.0),
                                );
                                let close_resp = ui.interact(
                                    close_rect,
                                    ui.make_persistent_id(("tab_close", tab.id)),
                                    Sense::click(),
                                );
                                if close_resp.hovered() {
                                    ui.painter().circle_filled(
                                        close_rect.center(),
                                        7.0,
                                        palette.card_hover,
                                    );
                                }
                                ui.painter().text(
                                    close_rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    "x",
                                    egui::FontId::proportional(11.0),
                                    if close_resp.hovered() {
                                        palette.text_primary
                                    } else {
                                        palette.text_muted
                                    },
                                );
                                if close_resp.clicked() {
                                    tab_to_close = Some(idx);
                                }
                            }

                            if resp.clicked() {
                                tab_to_select = Some(idx);
                            }
                        }

                        // B-3: when a non-explorer view is on screen the
                        // strip must show what is actually displayed
                        // instead of only stale explorer pills.
                        let mut close_tool_tab = false;
                        if self.current_tab != StudioTab::FileExplorer {
                            let (label, glyph) = studio_tab_label(self.current_tab);
                            let (p_rect, _) =
                                ui.allocate_exact_size(Vec2::new(130.0, 26.0), Sense::hover());
                            crate::gui::squircle::squircle_filled(ui.painter(),  p_rect, Rounding::same(6.0), palette.card_bg);
                            crate::gui::squircle::squircle_stroke(ui.painter(),
                                p_rect,
                                Rounding::same(6.0),
                                Stroke::new(1.0_f32, palette.border),
                            );
                            let pill_icon_rect = Rect::from_center_size(
                                p_rect.left_center() + Vec2::new(14.0, 0.0),
                                Vec2::splat(16.0),
                            );
                            crate::core::icon_packs::paint_icon(
                                ui,
                                pack,
                                glyph,
                                pill_icon_rect,
                                [79, 168, 255],
                            );
                            let max_title_chars = 10;
                            let title_display = if label.chars().count() > max_title_chars {
                                format!(
                                    "{}...",
                                    label.chars().take(max_title_chars).collect::<String>()
                                )
                            } else {
                                label.to_string()
                            };
                            ui.painter().text(
                                p_rect.left_center() + Vec2::new(30.0, 0.0),
                                egui::Align2::LEFT_CENTER,
                                title_display,
                                egui::FontId::proportional(13.5),
                                palette.text_primary,
                            );

                            // Close button "x" on tool/preferences pill
                            let close_rect = Rect::from_center_size(
                                p_rect.right_center() + Vec2::new(-12.0, 0.0),
                                Vec2::splat(14.0),
                            );
                            let close_resp = ui.interact(
                                close_rect,
                                ui.make_persistent_id("tool_tab_close_btn"),
                                Sense::click(),
                            );
                            if close_resp.hovered() {
                                ui.painter().circle_filled(
                                    close_rect.center(),
                                    7.0,
                                    palette.card_hover,
                                );
                            }
                            ui.painter().text(
                                close_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                "x",
                                egui::FontId::proportional(11.0),
                                if close_resp.hovered() {
                                    palette.text_primary
                                } else {
                                    palette.text_muted
                                },
                            );
                            if close_resp.clicked() {
                                close_tool_tab = true;
                            }
                        }

                        if close_tool_tab {
                            self.current_tab = if self.previous_tab != self.current_tab
                                && self.previous_tab != StudioTab::Preferences
                            {
                                self.previous_tab
                            } else {
                                StudioTab::FileExplorer
                            };
                        } else if let Some(idx) = tab_to_close {
                            self.explorer_view.close_tab(idx);
                        } else if let Some(idx) = tab_to_select {
                            self.current_tab = StudioTab::FileExplorer;
                            self.explorer_view.switch_to_tab(idx);
                        }

                        // Plus button for new tab
                        let new_tab_btn = egui::Button::new(
                            egui::RichText::new("+")
                                .size(13.0)
                                .color(palette.text_secondary),
                        )
                        .min_size(Vec2::splat(24.0))
                        .fill(Color32::TRANSPARENT)
                        .rounding(Rounding::same(4.0));
                        if ui
                            .add(new_tab_btn)
                            .on_hover_text("New tab (Ctrl+T)")
                            .clicked()
                        {
                            let target = self.explorer_view.current_dir.clone();
                            self.explorer_view.add_tab(target);
                            self.current_tab = StudioTab::FileExplorer;
                        }
                    } else {
                        // Title label when tabs are disabled (Thunar / Nautilus)
                        let title = if self.current_tab == StudioTab::FileExplorer {
                            if self.explorer_view.is_home_view {
                                "Home".to_string()
                            } else {
                                self.explorer_view
                                    .current_dir
                                    .file_name()
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_else(|| {
                                        self.explorer_view.current_dir.to_string_lossy().to_string()
                                    })
                            }
                        } else {
                            studio_tab_label(self.current_tab).0.to_string()
                        };
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new(title)
                                .size(13.5)
                                .strong()
                                .color(palette.text_primary),
                        );
                    }

                    // Draggable Window Title Region
                    let right_reserve =
                        if chrome_spec.window_controls == WindowControlsPosition::RightCaption {
                            110.0_f32
                        } else {
                            20.0_f32
                        };
                    let drag_w = (ui.available_width() - right_reserve).max(20.0);
                    let (_drag_rect, drag_resp) =
                        ui.allocate_exact_size(Vec2::new(drag_w, 26.0), Sense::click_and_drag());
                    if drag_resp.drag_started() {
                        ctx.send_viewport_cmd(ViewportCommand::StartDrag);
                    }
                    if drag_resp.double_clicked() {
                        ctx.send_viewport_cmd(ViewportCommand::Maximized(!is_max));
                    }

                    // Right caption buttons (Windows / GNOME style)
                    if chrome_spec.window_controls == WindowControlsPosition::RightCaption {
                        crate::gui::finder_chrome::render_caption_buttons(
                            ui,
                            &palette,
                            || ctx.send_viewport_cmd(ViewportCommand::Close),
                            || do_minimize = true,
                            || ctx.send_viewport_cmd(ViewportCommand::Maximized(!is_max)),
                        );
                    }
                });
                if do_minimize {
                    self.request_minimize = true;
                }

                // Menubar (Thunar style)
                if chrome_spec.menubar {
                    ui.add_space(2.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(8.0, 0.0);
                        egui::menu::bar(ui, |ui| {
                            ui.menu_button("File", |ui| {
                                if ui.button("New Window (Ctrl+N)").clicked() {
                                    crate::gui::explorer_view::spawn_new_explorer_window(Some(
                                        &self.explorer_view.current_dir,
                                    ));
                                    ui.close_menu();
                                }
                                if ui.button("New Folder (Ctrl+Shift+N)").clicked() {
                                    self.explorer_view.show_new_folder_dialog = true;
                                    ui.close_menu();
                                }
                                ui.separator();
                                if ui.button("Close Window (Alt+F4)").clicked() {
                                    ctx.send_viewport_cmd(ViewportCommand::Close);
                                    ui.close_menu();
                                }
                            });
                            ui.menu_button("Edit", |ui| {
                                if ui.button("Cut (Ctrl+X)").clicked() {
                                    self.explorer_view.cut_selected_to_clipboard();
                                    ui.close_menu();
                                }
                                if ui.button("Copy (Ctrl+C)").clicked() {
                                    self.explorer_view.copy_selected_to_clipboard();
                                    ui.close_menu();
                                }
                                if ui.button("Paste (Ctrl+V)").clicked() {
                                    self.explorer_view.paste_from_clipboard();
                                    ui.close_menu();
                                }
                                ui.separator();
                                if ui.button("Select All (Ctrl+A)").clicked() {
                                    self.explorer_view.select_all();
                                    ui.close_menu();
                                }
                                ui.separator();
                                if ui.button("Preferences (Ctrl+,)").clicked() {
                                    self.switch_to_tab(StudioTab::Preferences);
                                    ui.close_menu();
                                }
                            });
                            ui.menu_button("View", |ui| {
                                if ui.button("Icons View (Ctrl+1)").clicked() {
                                    self.explorer_view.view_mode =
                                        crate::gui::explorer_view::ViewMode::Grid;
                                    ui.close_menu();
                                }
                                if ui.button("Details View (Ctrl+2)").clicked() {
                                    self.explorer_view.view_mode =
                                        crate::gui::explorer_view::ViewMode::Details;
                                    ui.close_menu();
                                }
                                if ui.button("Columns View (Ctrl+3)").clicked() {
                                    self.explorer_view.view_mode =
                                        crate::gui::explorer_view::ViewMode::Columns;
                                    ui.close_menu();
                                }
                                ui.separator();
                                if ui.button("Refresh (F5)").clicked() {
                                    self.explorer_view
                                        .navigate_to(self.explorer_view.current_dir.clone());
                                    ui.close_menu();
                                }
                            });
                            ui.menu_button("Go", |ui| {
                                let toggle_label = if config.sidebar_collapsed {
                                    "Show Sidebar (Ctrl+B)"
                                } else {
                                    "Hide Sidebar (Ctrl+B)"
                                };
                                if ui.button(toggle_label).clicked() {
                                    config.sidebar_collapsed = !config.sidebar_collapsed;
                                    let _ = config.save();
                                    ui.close_menu();
                                }

                                let can_back = self.explorer_view.can_go_back();
                                if ui
                                    .add_enabled(can_back, egui::Button::new("Back (Alt+Left)"))
                                    .clicked()
                                {
                                    self.explorer_view.go_back();
                                    ui.close_menu();
                                }
                                let can_fwd = self.explorer_view.can_go_forward();
                                if ui
                                    .add_enabled(can_fwd, egui::Button::new("Forward (Alt+Right)"))
                                    .clicked()
                                {
                                    self.explorer_view.go_forward();
                                    ui.close_menu();
                                }
                                if ui.button("Parent Folder (Alt+Up)").clicked() {
                                    if let Some(p) = self.explorer_view.current_dir.parent() {
                                        self.explorer_view.navigate_to(p.to_path_buf());
                                    }
                                    ui.close_menu();
                                }
                                if ui.button("Home").clicked() {
                                    self.explorer_view.navigate_to_home();
                                    ui.close_menu();
                                }
                            });
                            ui.menu_button("Tools", |ui| {
                                if ui.button("Transcoder").clicked() {
                                    self.switch_to_tab(StudioTab::Transcoder);
                                    ui.close_menu();
                                }
                                if ui.button("Custom Conformer").clicked() {
                                    self.switch_to_tab(StudioTab::ExamStudio);
                                    ui.close_menu();
                                }
                                if ui.button("Quality Inspector").clicked() {
                                    self.switch_to_tab(StudioTab::QualityInspector);
                                    ui.close_menu();
                                }
                                if ui.button("Archive Suite").clicked() {
                                    self.switch_to_tab(StudioTab::ArchiveSuite);
                                    ui.close_menu();
                                }
                                if ui.button("Hardware & AI").clicked() {
                                    self.switch_to_tab(StudioTab::HardwareTelemetry);
                                    ui.close_menu();
                                }
                            });
                            ui.menu_button("Help", |ui| {
                                if ui.button("Preferences (Ctrl+,)").clicked() {
                                    self.switch_to_tab(StudioTab::Preferences);
                                    ui.close_menu();
                                }
                            });
                        });
                    });
                }

                // ROW 2 (Navigation Buttons + Breadcrumbs + Search + Utilities)
                // is ONLY shown when the user is in the File Explorer tab (macOS Finder parity).
                if self.current_tab == StudioTab::FileExplorer {
                    ui.add_space(6.0);

                    let row2 = ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);

                        // Navigation Cluster (Finder-style vector icon buttons)
                        let pack = self.explorer_view.icon_pack;
                        let text_rgb = [
                            palette.text_primary.r(),
                            palette.text_primary.g(),
                            palette.text_primary.b(),
                        ];
                        let muted_rgb = [
                            palette.text_muted.r(),
                            palette.text_muted.g(),
                            palette.text_muted.b(),
                        ];

                        // Sidebar collapse/expand toggle button (Ctrl+B)
                        let is_collapsed = config.sidebar_collapsed;
                        if crate::gui::finder_chrome::toolbar_icon_button(
                            ui,
                            &palette,
                            pack,
                            IconGlyph::ActionSidebarToggle,
                            if is_collapsed { muted_rgb } else { text_rgb },
                            "Toggle Sidebar (Ctrl+B)",
                            !is_collapsed,
                        ) {
                            config.sidebar_collapsed = !config.sidebar_collapsed;
                            let _ = config.save();
                        }

                        let can_back = self.explorer_view.can_go_back();
                        if crate::gui::finder_chrome::toolbar_animated_icon_button(
                            ui,
                            &palette,
                            pack,
                            IconGlyph::ActionBack,
                            if can_back { text_rgb } else { muted_rgb },
                            "Back (Alt+Left)",
                            false,
                            crate::gui::finder_chrome::ToolbarButtonAnim::SlideLeft,
                            &mut self.nav_anim_back,
                        ) && can_back
                        {
                            self.current_tab = StudioTab::FileExplorer;
                            self.explorer_view.go_back();
                        }

                        let can_fwd = self.explorer_view.can_go_forward();
                        if crate::gui::finder_chrome::toolbar_animated_icon_button(
                            ui,
                            &palette,
                            pack,
                            IconGlyph::ActionForward,
                            if can_fwd { text_rgb } else { muted_rgb },
                            "Forward (Alt+Right)",
                            false,
                            crate::gui::finder_chrome::ToolbarButtonAnim::SlideRight,
                            &mut self.nav_anim_fwd,
                        ) && can_fwd
                        {
                            self.current_tab = StudioTab::FileExplorer;
                            self.explorer_view.go_forward();
                        }

                        if crate::gui::finder_chrome::toolbar_animated_icon_button(
                            ui,
                            &palette,
                            pack,
                            IconGlyph::ActionUp,
                            text_rgb,
                            "Up to parent folder (Alt+Up / Backspace)",
                            false,
                            crate::gui::finder_chrome::ToolbarButtonAnim::BounceUp,
                            &mut self.nav_anim_up,
                        ) {
                            self.current_tab = StudioTab::FileExplorer;
                            if let Some(parent) = self.explorer_view.current_dir.parent() {
                                let p = parent.to_path_buf();
                                if !p.as_os_str().is_empty() {
                                    self.explorer_view.navigate_to(p);
                                }
                            }
                        }

                        if crate::gui::finder_chrome::toolbar_animated_icon_button(
                            ui,
                            &palette,
                            pack,
                            IconGlyph::ActionRefresh,
                            text_rgb,
                            "Refresh directory (F5 / Ctrl+R)",
                            false,
                            crate::gui::finder_chrome::ToolbarButtonAnim::SpinRotate,
                            &mut self.nav_anim_refresh,
                        ) {
                            self.explorer_view
                                .navigate_to(self.explorer_view.current_dir.clone());
                        }

                        if crate::gui::finder_chrome::toolbar_animated_icon_button(
                            ui,
                            &palette,
                            pack,
                            IconGlyph::ActionNewFolder,
                            text_rgb,
                            "New Folder (Ctrl+Shift+N)",
                            false,
                            crate::gui::finder_chrome::ToolbarButtonAnim::PulsePop,
                            &mut self.nav_anim_newfolder,
                        ) {
                            self.explorer_view.show_new_folder_dialog = true;
                        }

                        ui.add_space(4.0);

                        let mut nav_target: Option<PathBuf> = None;
                        let mut toggle_edit = false;

                        match chrome_spec.address {
                            AddressBarStyle::PillPathBar => {
                                // Authentic macOS Finder Header: Folder Icon + Title
                                let is_home = self.explorer_view.is_home_view;
                                let title_str = if is_home {
                                    "Home".to_string()
                                } else {
                                    self.explorer_view
                                        .current_dir
                                        .file_name()
                                        .map(|n| n.to_string_lossy().to_string())
                                        .unwrap_or_else(|| {
                                            self.explorer_view
                                                .current_dir
                                                .to_string_lossy()
                                                .to_string()
                                        })
                                };
                                let glyph = if is_home {
                                    IconGlyph::Home
                                } else if let Some(sf) = crate::core::special_folders::lookup_path(
                                    &self.explorer_view.current_dir,
                                ) {
                                    IconGlyph::FolderSpecial(sf)
                                } else if self.explorer_view.current_dir.parent().is_none() {
                                    IconGlyph::Drive
                                } else {
                                    IconGlyph::Folder
                                };

                                ui.add_space(6.0);
                                let (h_rect, _) =
                                    ui.allocate_exact_size(Vec2::splat(18.0), Sense::hover());
                                crate::core::icon_packs::paint_icon(
                                    ui,
                                    pack,
                                    glyph,
                                    h_rect,
                                    [palette.accent.r(), palette.accent.g(), palette.accent.b()],
                                );
                                ui.add_space(4.0);
                                if ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new(&title_str)
                                                .size(15.0)
                                                .strong()
                                                .color(palette.text_primary),
                                        )
                                        .fill(Color32::TRANSPARENT)
                                        .stroke(Stroke::NONE),
                                    )
                                    .on_hover_text("Click to edit folder path (Ctrl+L)")
                                    .clicked()
                                {
                                    toggle_edit = true;
                                }
                            }
                            AddressBarStyle::PathButtons => {
                                // Thunar path buttons
                                let segments = crate::core::parse_breadcrumbs(
                                    &self.explorer_view.current_dir,
                                );
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing = Vec2::new(2.0, 0.0);
                                    for (idx, seg) in segments.iter().enumerate() {
                                        if idx > 0 {
                                            ui.label(
                                                egui::RichText::new("/")
                                                    .size(12.0)
                                                    .color(palette.text_muted),
                                            );
                                        }
                                        let is_current = idx + 1 == segments.len();
                                        let btn = ui.add(
                                            egui::Button::new(
                                                egui::RichText::new(&seg.label).size(12.5).color(
                                                    if is_current {
                                                        palette.accent
                                                    } else {
                                                        palette.text_primary
                                                    },
                                                ),
                                            )
                                            .fill(if is_current {
                                                palette.card_bg
                                            } else {
                                                Color32::TRANSPARENT
                                            })
                                            .rounding(Rounding::same(4.0)),
                                        );
                                        if btn.clicked() {
                                            nav_target = Some(seg.target_path.clone());
                                        }
                                    }
                                });
                            }
                            AddressBarStyle::Breadcrumb | AddressBarStyle::TextEntry => {
                                // Windows Explorer Path Ribbon
                                let search_area_w = 320.0_f32;
                                let ribbon_w =
                                    (ui.available_width() - search_area_w - 8.0).max(180.0);

                                let (bc_rect, _) = ui.allocate_exact_size(
                                    Vec2::new(ribbon_w, 28.0),
                                    Sense::hover(),
                                );
                                crate::gui::squircle::squircle_filled(ui.painter(),  bc_rect, Rounding::same(6.0), palette.card_bg);
                                crate::gui::squircle::squircle_stroke(ui.painter(),
                                    bc_rect,
                                    Rounding::same(6.0),
                                    Stroke::new(1.0_f32, palette.border),
                                );

                                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(bc_rect), |ui| {
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing = Vec2::new(2.0, 0.0);
                                        ui.add_space(4.0);

                                        if self.explorer_view.is_home_view {
                                            let (h_rect, _) = ui.allocate_exact_size(
                                                Vec2::splat(18.0),
                                                Sense::hover(),
                                            );
                                            crate::core::icon_packs::paint_icon(
                                                ui,
                                                pack,
                                                IconGlyph::Home,
                                                h_rect,
                                                [
                                                    palette.accent.r(),
                                                    palette.accent.g(),
                                                    palette.accent.b(),
                                                ],
                                            );
                                            ui.label(
                                                egui::RichText::new("Home")
                                                    .size(13.0)
                                                    .strong()
                                                    .color(palette.text_primary),
                                            );
                                        } else if self.explorer_view.is_editing_path {
                                            let edit_box_w = (ribbon_w - 74.0).max(100.0);
                                            let text_resp = ui.add(
                                                egui::TextEdit::singleline(
                                                    &mut self.explorer_view.path_edit_buffer,
                                                )
                                                .desired_width(edit_box_w)
                                                .font(egui::TextStyle::Monospace),
                                            );
                                            if text_resp.lost_focus()
                                                && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                            {
                                                let expanded = crate::core::platform_paths::expand_path_str(
                                                    &self.explorer_view.path_edit_buffer,
                                                );
                                                if crate::core::platform_paths::is_dashboard_location(&expanded) {
                                                    nav_target = Some(crate::core::platform_paths::dashboard_location());
                                                } else if expanded.exists() {
                                                    nav_target = Some(expanded);
                                                }
                                                self.explorer_view.is_editing_path = false;
                                            }
                                            if ui.small_button("Go").clicked() {
                                                let expanded = crate::core::platform_paths::expand_path_str(
                                                    &self.explorer_view.path_edit_buffer,
                                                );
                                                if crate::core::platform_paths::is_dashboard_location(&expanded) {
                                                    nav_target = Some(crate::core::platform_paths::dashboard_location());
                                                } else if expanded.exists() {
                                                    nav_target = Some(expanded);
                                                }
                                                self.explorer_view.is_editing_path = false;
                                            }
                                            if ui
                                                .small_button("x")
                                                .on_hover_text("Cancel")
                                                .clicked()
                                            {
                                                self.explorer_view.is_editing_path = false;
                                            }
                                        } else {
                                            let segments = crate::core::parse_breadcrumbs(
                                                &self.explorer_view.current_dir,
                                            );
                                            let num_segs = segments.len();

                                            // Deep trail horizontal scroll
                                            let edit_reserve = 52.0_f32;
                                            let trail_w =
                                                (ribbon_w - 8.0 - edit_reserve).max(120.0);
                                            ui.allocate_ui_with_layout(
                                                Vec2::new(trail_w, 28.0),
                                                egui::Layout::left_to_right(egui::Align::Center),
                                                |ui| {
                                                    egui::ScrollArea::horizontal()
                                                        .id_salt("studio_breadcrumb_hscroll")
                                                        .auto_shrink([false, true])
                                                        .scroll_bar_visibility(
                                                            egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded,
                                                        )
                                                        .show(ui, |ui| {
                                                            for (idx, seg) in
                                                                segments.iter().enumerate()
                                                            {
                                                                if idx > 0 {
                                                                    ui.add_space(2.0);
                                                                    crate::gui::finder_chrome::paint_path_chevron(
                                                                        ui,
                                                                        palette.text_muted,
                                                                    );
                                                                    ui.add_space(2.0);
                                                                }
                                                                let is_current =
                                                                    idx + 1 == num_segs;
                                                                let is_root = idx == 0
                                                                    && (seg.target_path.parent().is_none()
                                                                        || seg.label.contains(':'));
                                                                let glyph = if is_root {
                                                                    IconGlyph::Drive
                                                                } else if let Some(sf) =
                                                                    crate::core::special_folders::lookup(
                                                                        &seg.label,
                                                                        Some(&seg.target_path),
                                                                    )
                                                                {
                                                                    IconGlyph::FolderSpecial(sf)
                                                                } else if seg.label.eq_ignore_ascii_case("home")
                                                                    || seg.label.eq_ignore_ascii_case("users")
                                                                {
                                                                    IconGlyph::Home
                                                                } else {
                                                                    IconGlyph::Folder
                                                                };

                                                                let text_color = if is_current {
                                                                    palette.text_primary
                                                                } else {
                                                                    palette.text_secondary
                                                                };
                                                                let label_font = FontId::proportional(12.5);
                                                                let galley = ui.painter().layout_no_wrap(
                                                                    seg.label.clone(),
                                                                    label_font,
                                                                    text_color,
                                                                );
                                                                let icon_side = 16.0;
                                                                let gap = 4.0;
                                                                let pad_x = 4.0;
                                                                let seg_w = pad_x * 2.0 + icon_side + gap + galley.size().x;
                                                                let (seg_rect, seg_resp) = ui.allocate_exact_size(
                                                                    Vec2::new(seg_w, 24.0),
                                                                    Sense::click(),
                                                                );
                                                                if seg_resp.hovered() {
                                                                    crate::gui::squircle::squircle_filled(ui.painter(),
                                                                        seg_rect,
                                                                        Rounding::same(4.0),
                                                                        palette.card_hover,
                                                                    );
                                                                }
                                                                let s_icon_rect = Rect::from_center_size(
                                                                    Pos2::new(seg_rect.left() + pad_x + icon_side * 0.5, seg_rect.center().y),
                                                                    Vec2::splat(icon_side),
                                                                );
                                                                crate::core::icon_packs::paint_icon(
                                                                    ui,
                                                                    pack,
                                                                    glyph,
                                                                    s_icon_rect,
                                                                    if is_current {
                                                                        [
                                                                            palette.accent.r(),
                                                                            palette.accent.g(),
                                                                            palette.accent.b(),
                                                                        ]
                                                                    } else {
                                                                        muted_rgb
                                                                    },
                                                                );
                                                                let text_pos = Pos2::new(
                                                                    seg_rect.left() + pad_x + icon_side + gap,
                                                                    seg_rect.center().y - galley.size().y * 0.5,
                                                                );
                                                                ui.painter().galley(text_pos, galley, text_color);

                                                                if seg_resp.clicked() {
                                                                    nav_target =
                                                                        Some(seg.target_path.clone());
                                                                }

                                                                seg_resp.context_menu(
                                                                    |menu_ui| {
                                                                        menu_ui.set_max_width(220.0);
                                                                        let parent_dir = seg
                                                                            .target_path
                                                                            .parent()
                                                                            .unwrap_or(
                                                                                &seg.target_path,
                                                                            );
                                                                        let dirs = self
                                                                            .sibling_dirs(parent_dir);
                                                                        if dirs.is_empty() {
                                                                            menu_ui.label(
                                                                                egui::RichText::new("No sibling folders")
                                                                                    .size(12.0)
                                                                                    .color(palette.text_muted),
                                                                            );
                                                                        } else {
                                                                            for d in dirs {
                                                                                let is_this = d.path
                                                                                    == seg.target_path;
                                                                                let item_text = egui::RichText::new(
                                                                                    format!("> {}", d.name),
                                                                                )
                                                                                .size(12.5)
                                                                                .color(if is_this {
                                                                                    palette.accent
                                                                                } else {
                                                                                    palette.text_primary
                                                                                });
                                                                                if menu_ui.button(item_text).clicked() {
                                                                                    nav_target = Some(d.path);
                                                                                    menu_ui.close_menu();
                                                                                }
                                                                            }
                                                                        }
                                                                    },
                                                                );
                                                            }
                                                        });
                                                },
                                            );

                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    if ui
                                                        .small_button("Edit")
                                                        .on_hover_text("Edit path (Ctrl+L / Alt+D)")
                                                        .clicked()
                                                    {
                                                        toggle_edit = true;
                                                    }
                                                },
                                            );
                                        }
                                    });
                                });
                            }
                        }

                        if let Some(target) = nav_target {
                            self.sibling_menu_cache.clear();
                            self.explorer_view.navigate_to(target);
                        }
                        if toggle_edit {
                            self.explorer_view.path_edit_buffer =
                                self.explorer_view.current_dir.to_string_lossy().to_string();
                            self.explorer_view.is_editing_path = true;
                        }

                        // Search Box & Action Icons on Right
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            // Search Box / Toggle
                            match chrome_spec.search {
                                SearchPosition::Right | SearchPosition::InAddressRow => {
                                    let search_w = 170.0_f32;
                                    let search_resp = ui.add(
                                        egui::TextEdit::singleline(
                                            &mut self.explorer_view.search_query,
                                        )
                                        .id(egui::Id::new("mossambi_global_search_input"))
                                        .hint_text("Search (/)")
                                        .desired_width(search_w),
                                    );
                                    if self.explorer_view.focus_search_requested {
                                        search_resp.request_focus();
                                        self.explorer_view.focus_search_requested = false;
                                    }

                                    if !self.explorer_view.search_query.is_empty()
                                        && crate::gui::finder_chrome::toolbar_icon_button(
                                            ui,
                                            &palette,
                                            pack,
                                            IconGlyph::ActionClose,
                                            text_rgb,
                                            "Clear search (Esc)",
                                            false,
                                        )
                                    {
                                        self.explorer_view.search_query.clear();
                                        ui.memory_mut(|m| {
                                            m.surrender_focus(egui::Id::new(
                                                "mossambi_global_search_input",
                                            ))
                                        });
                                    }
                                }
                                SearchPosition::Toggle => {
                                    if crate::gui::finder_chrome::toolbar_icon_button(
                                        ui,
                                        &palette,
                                        pack,
                                        IconGlyph::Search,
                                        text_rgb,
                                        "Toggle search",
                                        !self.explorer_view.search_query.is_empty(),
                                    ) {
                                        self.explorer_view.focus_search_requested = true;
                                    }
                                    if !self.explorer_view.search_query.is_empty() {
                                        let search_w = 120.0_f32;
                                        ui.add(
                                            egui::TextEdit::singleline(
                                                &mut self.explorer_view.search_query,
                                            )
                                            .id(egui::Id::new("mossambi_global_search_input"))
                                            .desired_width(search_w),
                                        );
                                    }
                                }
                            }

                            ui.add_space(4.0);

                            // Settings Gear
                            if chrome_spec.toolbar.contains(&ToolbarItem::Settings)
                                && crate::gui::finder_chrome::toolbar_icon_button(
                                    ui,
                                    &palette,
                                    pack,
                                    IconGlyph::Gear,
                                    text_rgb,
                                    "Preferences & Settings (Cmd+,)",
                                    false,
                                )
                            {
                                self.switch_to_tab(StudioTab::Preferences);
                            }

                            // New Window Button (Ctrl+N)
                            if chrome_spec.toolbar.contains(&ToolbarItem::NewWindow)
                                && crate::gui::finder_chrome::toolbar_icon_button(
                                    ui,
                                    &palette,
                                    pack,
                                    IconGlyph::ActionNewFolder,
                                    text_rgb,
                                    "Open New Concurrent Explorer Window (Ctrl+N)",
                                    false,
                                )
                            {
                                crate::gui::explorer_view::spawn_new_explorer_window(Some(
                                    &self.explorer_view.current_dir,
                                ));
                            }

                            ui.add_space(6.0);

                            // Connected Segmented View Switcher: [ Icons | List | Columns ]
                            if chrome_spec.toolbar.contains(&ToolbarItem::ViewSwitcher)
                                && chrome_spec.title_bar != TitleBarStyle::Separate
                            {
                                crate::gui::finder_chrome::render_styled_segmented_view_switcher(
                                    ui,
                                    &palette,
                                    &self.explorer_view.design_tokens,
                                    pack,
                                    self.explorer_view.view_mode,
                                    |mode| {
                                        self.explorer_view.view_mode = mode;
                                    },
                                );
                            }

                            // Main menu button (Nautilus / GNOME header bar)
                            if chrome_spec.toolbar.contains(&ToolbarItem::Hamburger) {
                                ui.add_space(4.0);
                                let (h_click, h_resp) =
                                    crate::gui::finder_chrome::toolbar_icon_button_resp(
                                        ui,
                                        &palette,
                                        pack,
                                        IconGlyph::ActionMenu,
                                        text_rgb,
                                        "Main Menu",
                                        false,
                                    );
                                let popup_id = ui.make_persistent_id("gnome_hamburger_menu_popover");
                                if h_click {
                                    ui.memory_mut(|mem| mem.toggle_popup(popup_id));
                                }
                                egui::popup::popup_below_widget(
                                    ui,
                                    popup_id,
                                    &h_resp,
                                    egui::popup::PopupCloseBehavior::CloseOnClickOutside,
                                    |popup_ui| {
                                        popup_ui.set_min_width(190.0);
                                        if popup_ui.button("Preferences (Ctrl+,)").clicked() {
                                            self.switch_to_tab(StudioTab::Preferences);
                                            popup_ui.memory_mut(|mem| mem.close_popup());
                                        }
                                        if popup_ui.button("New Window (Ctrl+N)").clicked() {
                                            crate::gui::explorer_view::spawn_new_explorer_window(
                                                Some(&self.explorer_view.current_dir),
                                            );
                                            popup_ui.memory_mut(|mem| mem.close_popup());
                                        }
                                        if popup_ui.button("New Folder (Ctrl+Shift+N)").clicked() {
                                            self.explorer_view.show_new_folder_dialog = true;
                                            popup_ui.memory_mut(|mem| mem.close_popup());
                                        }
                                        popup_ui.separator();
                                        popup_ui.menu_button("Tools", |sub_ui| {
                                            sub_ui.set_min_width(160.0);
                                            if sub_ui.button("Transcoder").clicked() {
                                                self.switch_to_tab(StudioTab::Transcoder);
                                                sub_ui.memory_mut(|mem| mem.close_popup());
                                            }
                                            if sub_ui.button("Custom Conformer").clicked() {
                                                self.switch_to_tab(StudioTab::ExamStudio);
                                                sub_ui.memory_mut(|mem| mem.close_popup());
                                            }
                                            if sub_ui.button("Quality Inspector").clicked() {
                                                self.switch_to_tab(StudioTab::QualityInspector);
                                                sub_ui.memory_mut(|mem| mem.close_popup());
                                            }
                                            if sub_ui.button("Archive Suite").clicked() {
                                                self.switch_to_tab(StudioTab::ArchiveSuite);
                                                sub_ui.memory_mut(|mem| mem.close_popup());
                                            }
                                            if sub_ui.button("Hardware & AI").clicked() {
                                                self.switch_to_tab(StudioTab::HardwareTelemetry);
                                                sub_ui.memory_mut(|mem| mem.close_popup());
                                            }
                                        });
                                    },
                                );
                            }
                        });
                    });
                    let row2_h = row2.response.rect.height();
                    ui.data_mut(|d| d.insert_temp(egui::Id::new("mossambi_toolbar_row2_h"), row2_h));

                    // Command Bar for Windows 11 Explorer preset (Separate title bar style)
                    if chrome_spec.title_bar == TitleBarStyle::Separate {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);
                            let pack = self.explorer_view.icon_pack;
                            let text_rgb = [
                                palette.text_primary.r(),
                                palette.text_primary.g(),
                                palette.text_primary.b(),
                            ];
                            for item in &chrome_spec.toolbar {
                                match item {
                                    ToolbarItem::NewFolder => {
                                        if ui
                                            .button(
                                                egui::RichText::new("+ New")
                                                    .size(12.5)
                                                    .color(palette.text_primary),
                                            )
                                            .on_hover_text("New Folder (Ctrl+Shift+N)")
                                            .clicked()
                                        {
                                            self.explorer_view.show_new_folder_dialog = true;
                                        }
                                    }
                                    ToolbarItem::Cut => {
                                        if crate::gui::finder_chrome::toolbar_icon_button(
                                            ui,
                                            &palette,
                                            pack,
                                            IconGlyph::ActionCut,
                                            text_rgb,
                                            "Cut (Ctrl+X)",
                                            false,
                                        ) {
                                            self.explorer_view.cut_selected_to_clipboard();
                                        }
                                    }
                                    ToolbarItem::Copy => {
                                        if crate::gui::finder_chrome::toolbar_icon_button(
                                            ui,
                                            &palette,
                                            pack,
                                            IconGlyph::ActionCopy,
                                            text_rgb,
                                            "Copy (Ctrl+C)",
                                            false,
                                        ) {
                                            self.explorer_view.copy_selected_to_clipboard();
                                        }
                                    }
                                    ToolbarItem::Paste => {
                                        if crate::gui::finder_chrome::toolbar_icon_button(
                                            ui,
                                            &palette,
                                            pack,
                                            IconGlyph::ActionPaste,
                                            text_rgb,
                                            "Paste (Ctrl+V)",
                                            false,
                                        ) {
                                            self.explorer_view.paste_from_clipboard();
                                        }
                                    }
                                    ToolbarItem::Rename => {
                                        if crate::gui::finder_chrome::toolbar_icon_button(
                                            ui,
                                            &palette,
                                            pack,
                                            IconGlyph::ActionRename,
                                            text_rgb,
                                            "Rename (F2)",
                                            false,
                                        ) {
                                            self.explorer_view.rename_selected();
                                        }
                                    }
                                    ToolbarItem::Share => {
                                        if crate::gui::finder_chrome::toolbar_icon_button(
                                            ui,
                                            &palette,
                                            pack,
                                            IconGlyph::ActionShortcut,
                                            text_rgb,
                                            "Share / Copy Path",
                                            false,
                                        ) {
                                            self.explorer_view.copy_current_path_to_clipboard();
                                        }
                                    }
                                    ToolbarItem::Delete => {
                                        if crate::gui::finder_chrome::toolbar_icon_button(
                                            ui,
                                            &palette,
                                            pack,
                                            IconGlyph::ActionDelete,
                                            text_rgb,
                                            "Delete (Delete)",
                                            false,
                                        ) {
                                            self.explorer_view.delete_selected();
                                        }
                                    }
                                    ToolbarItem::Sort => {
                                        if crate::gui::finder_chrome::toolbar_icon_button(
                                            ui,
                                            &palette,
                                            pack,
                                            IconGlyph::ActionSort,
                                            text_rgb,
                                            "Sort items",
                                            false,
                                        ) {
                                            self.explorer_view.sort_direction =
                                                self.explorer_view.sort_direction.toggle();
                                        }
                                    }
                                    ToolbarItem::ViewSwitcher => {
                                        crate::gui::finder_chrome::render_styled_segmented_view_switcher(
                                            ui,
                                            &palette,
                                            &self.explorer_view.design_tokens,
                                            pack,
                                            self.explorer_view.view_mode,
                                            |mode| {
                                                self.explorer_view.view_mode = mode;
                                            },
                                        );
                                    }
                                    _ => {}
                                }
                            }

                            // Command Bar Overflow Menu: "..." ("See more")
                            let (more_click, more_resp) =
                                crate::gui::finder_chrome::toolbar_icon_button_resp(
                                    ui,
                                    &palette,
                                    pack,
                                    IconGlyph::ActionMore,
                                    text_rgb,
                                    "See more",
                                    false,
                                );
                            let popup_id = ui.make_persistent_id("win11_command_bar_overflow_popover");
                            if more_click {
                                ui.memory_mut(|mem| mem.toggle_popup(popup_id));
                            }
                            egui::popup::popup_below_widget(
                                ui,
                                popup_id,
                                &more_resp,
                                egui::popup::PopupCloseBehavior::CloseOnClickOutside,
                                |popup_ui| {
                                    popup_ui.set_min_width(190.0);
                                    if popup_ui.button("Options (Preferences)").clicked() {
                                        self.switch_to_tab(StudioTab::Preferences);
                                        popup_ui.memory_mut(|mem| mem.close_popup());
                                    }
                                    if popup_ui.button("New Window (Ctrl+N)").clicked() {
                                        crate::gui::explorer_view::spawn_new_explorer_window(
                                            Some(&self.explorer_view.current_dir),
                                        );
                                        popup_ui.memory_mut(|mem| mem.close_popup());
                                    }
                                    popup_ui.separator();
                                    popup_ui.menu_button("Tools", |sub_ui| {
                                        sub_ui.set_min_width(160.0);
                                        if sub_ui.button("Transcoder").clicked() {
                                            self.switch_to_tab(StudioTab::Transcoder);
                                            sub_ui.memory_mut(|mem| mem.close_popup());
                                        }
                                        if sub_ui.button("Custom Conformer").clicked() {
                                            self.switch_to_tab(StudioTab::ExamStudio);
                                            sub_ui.memory_mut(|mem| mem.close_popup());
                                        }
                                        if sub_ui.button("Quality Inspector").clicked() {
                                            self.switch_to_tab(StudioTab::QualityInspector);
                                            sub_ui.memory_mut(|mem| mem.close_popup());
                                        }
                                        if sub_ui.button("Archive Suite").clicked() {
                                            self.switch_to_tab(StudioTab::ArchiveSuite);
                                            sub_ui.memory_mut(|mem| mem.close_popup());
                                        }
                                        if sub_ui.button("Hardware & AI").clicked() {
                                            self.switch_to_tab(StudioTab::HardwareTelemetry);
                                            sub_ui.memory_mut(|mem| mem.close_popup());
                                        }
                                    });
                                    popup_ui.separator();
                                    if popup_ui.button("Properties (Alt+Enter)").clicked() {
                                        self.explorer_view.properties_item =
                                            Some(crate::core::FileItem::from_path(self.explorer_view.current_dir.clone()));
                                        popup_ui.memory_mut(|mem| mem.close_popup());
                                    }
                                },
                            );
                        });
                    }
                } else {
                    // Tool pages keep a toolbar row of the SAME height carrying
                    // the page title, so the sidebar and content never jump
                    // vertically when switching between Explorer and a tool.
                    ui.add_space(6.0);
                    let row2_h = ui
                        .data(|d| d.get_temp::<f32>(egui::Id::new("mossambi_toolbar_row2_h")))
                        .unwrap_or(28.0);
                    let (title, _) = studio_tab_label(self.current_tab);
                    ui.allocate_ui_with_layout(
                        Vec2::new(ui.available_width(), row2_h),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            ui.add_space(10.0);
                            ui.label(
                                egui::RichText::new(title)
                                    .size(15.0)
                                    .color(palette.text_primary),
                            );
                        },
                    );
                }
            });

        // 2. Left Sidebar Panel (Pinned, Drives, Cloud, Network, Tags, Tools)
        let target_sidebar_w =
            if config.sidebar_collapsed || self.current_tab == StudioTab::Preferences {
                0.0_f32
            } else {
                chrome_spec.sidebar.width
            };
        let dt = crate::gui::anim::frame_dt(ctx);
        let current_sidebar_w = if config.animations.reduced_motion {
            self.sidebar_width_spring.set(target_sidebar_w);
            target_sidebar_w
        } else {
            crate::gui::anim::advance(
                &mut self.sidebar_width_spring,
                target_sidebar_w,
                config.animations.toolbar,
                dt,
            )
        };
        if !self.sidebar_width_spring.at_rest() && !config.animations.reduced_motion {
            ctx.request_repaint();
        }

        if current_sidebar_w > 1.5_f32 {
            let sidebar_frame = egui::Frame::none()
                .fill(palette.sidebar_bg)
                .inner_margin(egui::Margin::symmetric(8.0, 8.0));

            egui::SidePanel::left("studio_sidebar")
                .resizable(false)
                .exact_width(current_sidebar_w)
                .frame(sidebar_frame)
                .show(ctx, |ui| {
                    ui.set_clip_rect(ui.max_rect());
                    let label_fade = ((current_sidebar_w - 60.0)
                        / (chrome_spec.sidebar.width - 60.0).max(1.0))
                    .clamp(0.0, 1.0);
                    if label_fade < 0.99 {
                        ui.set_opacity(label_fade);
                    }
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            let pack = self.explorer_view.icon_pack;
                            let cfg = &mut *config;

                        // Exactly ONE sidebar row is active: the deepest sidebar
                        // location that contains the current folder.
                        let active_sidebar_path: Option<PathBuf> = if self.current_tab
                            == StudioTab::FileExplorer
                            && !self.explorer_view.is_home_view
                        {
                            let cur = &self.explorer_view.current_dir;
                            let mut candidates: Vec<PathBuf> = vec![
                                crate::core::platform_paths::home_dir(),
                                crate::core::platform_paths::desktop_dir(),
                                crate::core::platform_paths::downloads_dir(),
                                crate::core::platform_paths::documents_dir(),
                                crate::core::platform_paths::pictures_dir(),
                                crate::core::platform_paths::music_dir(),
                                crate::core::platform_paths::videos_dir(),
                                crate::core::resolve_recycle_bin_target(),
                            ];
                            if cfg.show_custom_pinned {
                                candidates.extend(cfg.pinned_custom.iter().map(PathBuf::from));
                            }
                            candidates
                                .extend(self.explorer_view.drives.iter().map(|d| d.path.clone()));
                            if let Ok(up) = std::env::var("USERPROFILE") {
                                candidates.push(PathBuf::from(up).join("OneDrive"));
                            }
                            candidates
                                .into_iter()
                                .filter(|p| cur.starts_with(p))
                                .max_by_key(|p| p.components().count())
                        } else {
                            None
                        };
                        let is_sidebar_active =
                            |p: &PathBuf| active_sidebar_path.as_ref() == Some(p);

                        let mut sidebar_hover: Option<(crate::core::FileItem, Pos2)> = None;

                        if chrome_spec.sidebar.show_favorites {
                            // 1. Home Item (Apple HIG Home Blue)
                            let is_home_active = self.current_tab == StudioTab::FileExplorer
                                && self.explorer_view.is_home_view;
                            let home_spec = SidebarRowSpec {
                                pack,
                                glyph: IconGlyph::Home,
                                accent_rgb: [79, 168, 255],
                                label: "Home",
                                selected: is_home_active,
                                show_pin: false,
                                dimmed: false,
                                tooltip: Some("Home - Dashboard & Quick Access".to_string()),
                            };
                            let home_click = render_sidebar_row(ui, &palette, &home_spec);
                            if home_click.row {
                                self.current_tab = StudioTab::FileExplorer;
                                self.explorer_view.navigate_to_home();
                            }

                            // 2. FAVORITES Zone (static favorites + user pins,
                            // Finder semantics: pinned folders live inline)
                            if Self::render_zone_header(
                                ui,
                                &palette,
                                "Favorites",
                                cfg.sidebar_favorites_expanded,
                                chrome_spec.sidebar.header_style,
                            ) {
                                cfg.sidebar_favorites_expanded = !cfg.sidebar_favorites_expanded;
                                if let Err(e) = cfg.save() {
                                    log::warn!("Could not persist sidebar state: {e}");
                                }
                            }

                            let home_opt = crate::core::platform_paths::home_dir_opt();
                        if cfg.sidebar_favorites_expanded {
                            let user_home_info = home_opt.map(|p| {
                                let name = p
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("User")
                                    .to_string();
                                (name, p)
                            });
                            let recycle_target = crate::core::resolve_recycle_bin_target();
                            let mut pinned: Vec<(&str, IconGlyph, [u8; 3], PathBuf)> =
                                Vec::with_capacity(8);
                            if let Some((ref name, ref path)) = user_home_info {
                                pinned.push((
                                    name.as_str(),
                                    IconGlyph::FolderHome,
                                    [79, 168, 255],
                                    path.clone(),
                                ));
                            }
                            pinned.extend([
                                (
                                    "Desktop",
                                    IconGlyph::FolderDesktop,
                                    [70, 130, 240],
                                    crate::core::platform_paths::desktop_dir(),
                                ),
                                (
                                    "Downloads",
                                    IconGlyph::FolderDownloads,
                                    [40, 180, 220],
                                    crate::core::platform_paths::downloads_dir(),
                                ),
                                (
                                    "Documents",
                                    IconGlyph::FolderDocuments,
                                    [79, 168, 255],
                                    crate::core::platform_paths::documents_dir(),
                                ),
                                (
                                    "Pictures",
                                    IconGlyph::FolderPictures,
                                    [255, 100, 150],
                                    crate::core::platform_paths::pictures_dir(),
                                ),
                                (
                                    "Music",
                                    IconGlyph::FolderMusic,
                                    [255, 140, 30],
                                    crate::core::platform_paths::music_dir(),
                                ),
                                (
                                    "Videos",
                                    IconGlyph::FolderVideos,
                                    [170, 90, 240],
                                    crate::core::platform_paths::videos_dir(),
                                ),
                                (
                                    "Recycle Bin",
                                    IconGlyph::RecycleBin,
                                    [140, 145, 155],
                                    recycle_target,
                                ),
                            ]);

                            for (name, glyph, icon_col, path) in pinned {
                                let is_active = is_sidebar_active(&path);
                                let spec = SidebarRowSpec {
                                    pack,
                                    glyph,
                                    accent_rgb: icon_col,
                                    label: name,
                                    selected: is_active,
                                    show_pin: false,
                                    dimmed: false,
                                    tooltip: Some(format!("{name} ({})", path.display())),
                                };
                                let row_click = render_sidebar_row(ui, &palette, &spec);
                                if row_click.hovered && path.exists() {
                                    sidebar_hover = Some((
                                        crate::core::FileItem::from_path(path.clone()),
                                        row_click.rect.right_top() + Vec2::new(12.0, 0.0),
                                    ));
                                }
                                if let Some(payload) = row_click.dropped {
                                    dispatch_sidebar_drop(
                                        &mut self.explorer_view,
                                        ui,
                                        &path,
                                        payload,
                                    );
                                }
                                if row_click.row {
                                    if path.to_string_lossy().contains("$Recycle.Bin") {
                                        self.explorer_view.show_hidden_files = true;
                                    }
                                    self.current_tab = StudioTab::FileExplorer;
                                    self.explorer_view.navigate_to(path);
                                    self.explorer_view.is_home_view = false;
                                }
                                ui.add_space(1.0);
                            }
                        }

                        // 2.5 PINNED Zone (shows when pins exist or during folder drag)
                        let is_file_drag = egui::DragAndDrop::has_payload_of_type::<
                            crate::gui::explorer_view::FileDragPayload,
                        >(ui.ctx());
                        let show_pinned_section = !cfg.pinned_custom.is_empty() || is_file_drag;

                        if show_pinned_section {
                            ui.add_space(2.0);
                            // Drop gap between Favorites and Pinned
                            if is_file_drag {
                                let gap_rect =
                                    ui.allocate_space(Vec2::new(ui.available_width(), 6.0)).1;
                                let gap_resp = ui.interact(
                                    gap_rect,
                                    ui.id().with("gap_pinned_top"),
                                    Sense::hover(),
                                );
                                if let Some(payload) = gap_resp.dnd_release_payload::<
                                    crate::gui::explorer_view::FileDragPayload,
                                >() {
                                    let mut changed = false;
                                    for path in &payload.paths {
                                        if path.is_dir()
                                            && crate::core::pin_add(&mut cfg.pinned_custom, path)
                                        {
                                            changed = true;
                                        }
                                    }
                                    if changed {
                                        let _ = cfg.save();
                                    }
                                }
                            }

                            let (header_clicked, header_drop) =
                                render_sidebar_zone_header_with_drop(
                                    ui,
                                    &palette,
                                    "Pinned",
                                    cfg.sidebar_pinned_expanded,
                                );
                            if header_clicked {
                                cfg.sidebar_pinned_expanded = !cfg.sidebar_pinned_expanded;
                                let _ = cfg.save();
                            }
                            if let Some(payload) = header_drop {
                                let mut changed = false;
                                for path in &payload.paths {
                                    if path.is_dir()
                                        && crate::core::pin_add(&mut cfg.pinned_custom, path)
                                    {
                                        changed = true;
                                    }
                                }
                                if changed {
                                    let _ = cfg.save();
                                }
                            }

                            if cfg.sidebar_pinned_expanded {
                                let custom_pins = cfg.pinned_custom.clone();
                                let mut unpin_path: Option<PathBuf> = None;
                                let mut open_new_tab_path: Option<PathBuf> = None;
                                let mut reorder_move: Option<(usize, usize)> = None;

                                for (idx, raw) in custom_pins.iter().enumerate() {
                                    let path = PathBuf::from(raw);
                                    let is_dir = path.is_dir();
                                    let name = path
                                        .file_name()
                                        .map(|n| n.to_string_lossy().into_owned())
                                        .unwrap_or_else(|| raw.clone());
                                    let is_active = is_sidebar_active(&path);

                                    let row_h = cfg.sidebar_style.sidebar_row_height();
                                    let desired_size = Vec2::new(ui.available_width(), row_h);
                                    let (rect, hover_resp) =
                                        ui.allocate_exact_size(desired_size, Sense::hover());

                                    let pin_strip_w = 26.0_f32;
                                    let row_hit_rect = Rect::from_min_max(
                                        rect.min,
                                        Pos2::new(rect.max.x - pin_strip_w, rect.max.y),
                                    );
                                    let row_id = ui.make_persistent_id(("pinned_row", idx, raw));
                                    let row_resp =
                                        ui.interact(row_hit_rect, row_id, Sense::click_and_drag());
                                    let is_hovered = hover_resp.hovered() || row_resp.hovered();
                                    row_resp.clone().on_hover_ui(|ui| {
                                        crate::gui::finder_chrome::show_tooltip(
                                            ui,
                                            format!("{name} ({})", path.display()),
                                        )
                                    });
                                    if is_hovered && path.exists() {
                                        sidebar_hover = Some((
                                            crate::core::FileItem::from_path(path.clone()),
                                            rect.right_top() + Vec2::new(12.0, 0.0),
                                        ));
                                    }
                                    let painter = ui.painter_at(rect);

                                    // Drag to reorder
                                    if row_resp.drag_started() {
                                        egui::DragAndDrop::set_payload(
                                            ui.ctx(),
                                            PinnedRowDragPayload { from_index: idx },
                                        );
                                    }
                                    if let Some(payload) =
                                        row_resp.dnd_hover_payload::<PinnedRowDragPayload>()
                                    {
                                        if payload.from_index != idx {
                                            crate::gui::squircle::squircle_stroke(&painter,
                                                rect,
                                                Rounding::same(design::radius::SM),
                                                Stroke::new(1.5_f32, palette.accent),
                                            );
                                        }
                                    }
                                    if let Some(payload) =
                                        row_resp.dnd_release_payload::<PinnedRowDragPayload>()
                                    {
                                        if payload.from_index != idx {
                                            reorder_move = Some((payload.from_index, idx));
                                        }
                                    }

                                    // Drop files into folder
                                    let is_file_drop_hovered = row_resp
                                        .dnd_hover_payload::<crate::gui::explorer_view::FileDragPayload>()
                                        .is_some();
                                    if is_file_drop_hovered {
                                        crate::gui::squircle::squircle_stroke(&painter,
                                            rect,
                                            Rounding::same(design::radius::SM),
                                            Stroke::new(1.5_f32, palette.accent),
                                        );
                                    }
                                    if let Some(payload) = row_resp.dnd_release_payload::<
                                        crate::gui::explorer_view::FileDragPayload,
                                    >() {
                                        if is_dir {
                                            dispatch_sidebar_drop(
                                                &mut self.explorer_view,
                                                ui,
                                                &path,
                                                (*payload).clone(),
                                            );
                                        }
                                    }

                                    // Background
                                    let bg_color = if is_active {
                                        Color32::from_rgba_unmultiplied(
                                            palette.accent.r(),
                                            palette.accent.g(),
                                            palette.accent.b(),
                                            if palette.is_dark { 45 } else { 26 },
                                        )
                                    } else if is_hovered {
                                        palette.card_hover
                                    } else {
                                        Color32::TRANSPARENT
                                    };

                                    if bg_color != Color32::TRANSPARENT {
                                        crate::gui::squircle::squircle_filled(&painter,
                                            rect,
                                            Rounding::same(design::radius::SM),
                                            bg_color,
                                        );
                                    }

                                    if is_active {
                                        let bar_rect = Rect::from_min_size(
                                            rect.min + Vec2::new(2.0, 4.0),
                                            Vec2::new(3.0, (row_h - 8.0).max(4.0)),
                                        );
                                        crate::gui::squircle::squircle_filled(&painter,
                                            bar_rect,
                                            Rounding::same(1.5),
                                            palette.accent,
                                        );
                                    }

                                    // Icon (dimmed if !is_dir)
                                    let icon_side = 24.0;
                                    let icon_rect = Rect::from_center_size(
                                        Pos2::new(
                                            rect.min.x + 10.0 + icon_side / 2.0,
                                            rect.center().y,
                                        ),
                                        Vec2::splat(icon_side),
                                    );
                                    let icon_rgb = if !is_dir {
                                        [
                                            palette.text_muted.r(),
                                            palette.text_muted.g(),
                                            palette.text_muted.b(),
                                        ]
                                    } else {
                                        [79, 168, 255]
                                    };
                                    let pin_glyph = if is_dir {
                                        if let Some(sf) =
                                            crate::core::special_folders::lookup(&name, Some(&path))
                                        {
                                            IconGlyph::FolderSpecial(sf)
                                        } else {
                                            IconGlyph::Folder
                                        }
                                    } else {
                                        IconGlyph::PageDefault
                                    };
                                    crate::core::icon_packs::paint_icon(
                                        ui,
                                        pack,
                                        pin_glyph,
                                        icon_rect,
                                        icon_rgb,
                                    );

                                    // Label (dimmed if !is_dir)
                                    let label_x =
                                        rect.min.x + 10.0 + icon_side + design::space::MD;
                                    let label_max_w = (rect.max.x
                                        - label_x
                                        - pin_strip_w
                                        - design::space::SM)
                                        .max(design::space::LG);
                                    let font_id = FontId::proportional(14.0);
                                    let display_label =
                                        design::elide_end(ui, &name, &font_id, label_max_w);
                                    let text_color = if !is_dir {
                                        palette.text_muted
                                    } else if is_active || is_hovered {
                                        palette.text_primary
                                    } else {
                                        palette.text_secondary
                                    };
                                    painter.text(
                                        Pos2::new(label_x, rect.center().y),
                                        egui::Align2::LEFT_CENTER,
                                        display_label,
                                        font_id,
                                        text_color,
                                    );

                                    // Pin button on right edge
                                    let pin_rect = Rect::from_min_max(
                                        Pos2::new(rect.max.x - pin_strip_w, rect.min.y),
                                        rect.max,
                                    );
                                    let pin_id =
                                        ui.make_persistent_id(("pinned_pin_btn", idx, raw));
                                    let pin_resp = ui.interact(pin_rect, pin_id, Sense::click());
                                    let pin_rect_glyph = Rect::from_center_size(
                                        Pos2::new(rect.max.x - pin_strip_w / 2.0, rect.center().y),
                                        Vec2::splat(16.0),
                                    );
                                    let pin_color = if pin_resp.hovered() {
                                        [
                                            palette.text_primary.r(),
                                            palette.text_primary.g(),
                                            palette.text_primary.b(),
                                        ]
                                    } else if is_hovered {
                                        [
                                            palette.text_secondary.r(),
                                            palette.text_secondary.g(),
                                            palette.text_secondary.b(),
                                        ]
                                    } else {
                                        [
                                            palette.text_muted.r(),
                                            palette.text_muted.g(),
                                            palette.text_muted.b(),
                                        ]
                                    };
                                    crate::core::icon_packs::paint_icon(
                                        ui,
                                        pack,
                                        IconGlyph::ActionPin,
                                        pin_rect_glyph,
                                        pin_color,
                                    );
                                    if pin_resp.clicked() {
                                        unpin_path = Some(path.clone());
                                    }

                                    // Context menu
                                    row_resp.context_menu(|menu_ui| {
                                        menu_ui.set_min_width(140.0);
                                        if is_dir && menu_ui.button("Open in new tab").clicked() {
                                            open_new_tab_path = Some(path.clone());
                                            menu_ui.close_menu();
                                        }
                                        if menu_ui.button("Unpin from Sidebar").clicked() {
                                            unpin_path = Some(path.clone());
                                            menu_ui.close_menu();
                                        }
                                        if !is_dir
                                            && menu_ui.button("Remove (missing folder)").clicked()
                                        {
                                            unpin_path = Some(path.clone());
                                            menu_ui.close_menu();
                                        }
                                    });

                                    // Row click
                                    if row_resp.clicked() && is_dir {
                                        self.current_tab = StudioTab::FileExplorer;
                                        self.explorer_view.navigate_to(path);
                                        self.explorer_view.is_home_view = false;
                                    }

                                    ui.add_space(1.0);
                                }

                                if let Some((from, to)) = reorder_move {
                                    crate::core::pin_move_item(&mut cfg.pinned_custom, from, to);
                                    let _ = cfg.save();
                                }
                                if let Some(p) = unpin_path {
                                    crate::core::pin_remove(&mut cfg.pinned_custom, &p);
                                    let _ = cfg.save();
                                }
                                if let Some(p) = open_new_tab_path {
                                    self.current_tab = StudioTab::FileExplorer;
                                    self.explorer_view.add_tab(p);
                                }

                                // "Drop to pin" strip when dragging
                                if is_file_drag {
                                    let strip_size = Vec2::new(ui.available_width(), 28.0);
                                    let (strip_rect, strip_resp) =
                                        ui.allocate_exact_size(strip_size, Sense::hover());
                                    let is_strip_hovered = strip_resp
                                        .dnd_hover_payload::<crate::gui::explorer_view::FileDragPayload>()
                                        .is_some();
                                    let border_color = if is_strip_hovered {
                                        palette.accent
                                    } else {
                                        palette.border
                                    };
                                    let bg_color = if is_strip_hovered {
                                        palette.card_hover
                                    } else {
                                        Color32::TRANSPARENT
                                    };
                                    ui.painter().rect(
                                        strip_rect.shrink(2.0),
                                        Rounding::same(design::radius::SM),
                                        bg_color,
                                        Stroke::new(1.0_f32, border_color),
                                    );
                                    ui.painter().text(
                                        strip_rect.center(),
                                        Align2::CENTER_CENTER,
                                        "+ Drop folder to pin",
                                        FontId::proportional(12.0),
                                        if is_strip_hovered {
                                            palette.text_primary
                                        } else {
                                            palette.text_muted
                                        },
                                    );
                                    if let Some(payload) = strip_resp.dnd_release_payload::<
                                        crate::gui::explorer_view::FileDragPayload,
                                    >() {
                                        let mut changed = false;
                                        for path in &payload.paths {
                                            if path.is_dir()
                                                && crate::core::pin_add(
                                                    &mut cfg.pinned_custom,
                                                    path,
                                                )
                                            {
                                                changed = true;
                                            }
                                        }
                                        if changed {
                                            let _ = cfg.save();
                                        }
                                    }
                                }
                            }

                            // Drop gap between Pinned and Locations
                            if is_file_drag {
                                let gap_rect =
                                    ui.allocate_space(Vec2::new(ui.available_width(), 6.0)).1;
                                let gap_resp = ui.interact(
                                    gap_rect,
                                    ui.id().with("gap_pinned_bottom"),
                                    Sense::hover(),
                                );
                                if let Some(payload) = gap_resp.dnd_release_payload::<
                                    crate::gui::explorer_view::FileDragPayload,
                                >() {
                                    let mut changed = false;
                                    for path in &payload.paths {
                                        if path.is_dir()
                                            && crate::core::pin_add(&mut cfg.pinned_custom, path)
                                        {
                                            changed = true;
                                        }
                                    }
                                    if changed {
                                        let _ = cfg.save();
                                    }
                                }
                            }
                        }
                        }

                        // 3. LOCATIONS Zone (drives + cloud)
                        if chrome_spec.sidebar.show_locations {
                            if Self::render_zone_header(
                                ui,
                                &palette,
                                "Locations",
                                cfg.sidebar_locations_expanded,
                                chrome_spec.sidebar.header_style,
                            ) {
                                cfg.sidebar_locations_expanded = !cfg.sidebar_locations_expanded;
                                if let Err(e) = cfg.save() {
                                    log::warn!("Could not persist sidebar state: {e}");
                                }
                            }

                            if cfg.sidebar_locations_expanded {
                                let drives = self.explorer_view.drives.clone();
                                for drive in &drives {
                                    let is_active = is_sidebar_active(&drive.path);
                                    let glyph = if drive.is_system {
                                        IconGlyph::DriveSystem
                                    } else {
                                        IconGlyph::Drive
                                    };
                                    let drive_rgb: [u8; 3] = if drive.is_system {
                                        [79, 168, 255]
                                    } else {
                                        [140, 145, 155]
                                    };
                                    let spec = SidebarRowSpec {
                                        pack,
                                        glyph,
                                        accent_rgb: drive_rgb,
                                        label: &drive.display_name,
                                        selected: is_active,
                                        show_pin: false,
                                        dimmed: false,
                                        tooltip: Some(format!("{} ({})", drive.display_name, drive.path.display())),
                                    };
                                    let drive_click = render_sidebar_row(ui, &palette, &spec);
                                    if drive_click.hovered && drive.path.exists() {
                                        sidebar_hover = Some((
                                            crate::core::FileItem::from_path(drive.path.clone()),
                                            drive_click.rect.right_top() + Vec2::new(12.0, 0.0),
                                        ));
                                    }
                                    if let Some(payload) = drive_click.dropped {
                                        dispatch_sidebar_drop(
                                            &mut self.explorer_view,
                                            ui,
                                            &drive.path,
                                            payload,
                                        );
                                    }
                                    if drive_click.row {
                                        self.current_tab = StudioTab::FileExplorer;
                                        self.explorer_view.navigate_to(drive.path.clone());
                                        self.explorer_view.is_home_view = false;
                                    }
                                    ui.add_space(1.0);
                                }

                                if let Ok(userprofile) = std::env::var("USERPROFILE") {
                                    let od = PathBuf::from(&userprofile).join("OneDrive");
                                    if od.exists() {
                                        let is_active = is_sidebar_active(&od);
                                        let spec = SidebarRowSpec {
                                            pack,
                                            glyph: IconGlyph::Cloud,
                                            accent_rgb: [70, 130, 240],
                                            label: "OneDrive",
                                            selected: is_active,
                                            show_pin: false,
                                            dimmed: false,
                                            tooltip: Some(format!("OneDrive ({})", od.display())),
                                        };
                                        let cloud_click = render_sidebar_row(ui, &palette, &spec);
                                        if cloud_click.hovered && od.exists() {
                                            sidebar_hover = Some((
                                                crate::core::FileItem::from_path(od.clone()),
                                                cloud_click.rect.right_top() + Vec2::new(12.0, 0.0),
                                            ));
                                        }
                                        if let Some(payload) = cloud_click.dropped {
                                            dispatch_sidebar_drop(
                                                &mut self.explorer_view,
                                                ui,
                                                &od,
                                                payload,
                                            );
                                        }
                                        if cloud_click.row {
                                            self.current_tab = StudioTab::FileExplorer;
                                            self.explorer_view.navigate_to(od);
                                            self.explorer_view.is_home_view = false;
                                        }
                                    }
                                }
                            }
                        }

                        // 4. TAGS Zone (optional Finder-like tags)
                        if chrome_spec.sidebar.show_tags && cfg.show_tags_section {
                            if Self::render_zone_header(
                                ui,
                                &palette,
                                "Tags",
                                cfg.sidebar_tags_expanded,
                                chrome_spec.sidebar.header_style,
                            ) {
                                cfg.sidebar_tags_expanded = !cfg.sidebar_tags_expanded;
                                if let Err(e) = cfg.save() {
                                    log::warn!("Could not persist sidebar state: {e}");
                                }
                            }
                            if cfg.sidebar_tags_expanded {
                                let custom_tags = cfg.custom_tags.clone();
                                for tag_name in crate::core::tags::available_tags_in(&custom_tags) {
                                    let tag_col = crate::core::tags::tag_color(&tag_name);
                                    let is_active =
                                        self.explorer_view.active_tag.as_deref().map(|a| a.eq_ignore_ascii_case(&tag_name)).unwrap_or(false);
                                    let row_h = 24.0;
                                    let (rect, resp) = ui.allocate_exact_size(
                                        Vec2::new(ui.available_width(), row_h),
                                        Sense::click(),
                                    );
                                    let is_hovered = resp.hovered();
                                    let painter = ui.painter_at(rect);
                                    if is_active {
                                        crate::gui::squircle::squircle_filled(&painter,
                                            rect,
                                            Rounding::same(radius::SM),
                                            Color32::from_rgba_unmultiplied(
                                                palette.accent.r(),
                                                palette.accent.g(),
                                                palette.accent.b(),
                                                if palette.is_dark { 45 } else { 26 },
                                            ),
                                        );
                                    } else if is_hovered {
                                        crate::gui::squircle::squircle_filled(&painter,
                                            rect,
                                            Rounding::same(radius::SM),
                                            palette.card_hover,
                                        );
                                    }
                                    painter.circle_filled(
                                        Pos2::new(rect.min.x + 14.0, rect.center().y),
                                        4.5,
                                        tag_col,
                                    );
                                    let text_color = if is_active {
                                        if palette.is_dark {
                                            palette.accent
                                        } else {
                                            palette.text_primary
                                        }
                                    } else if is_hovered {
                                        palette.text_primary
                                    } else {
                                        palette.text_secondary
                                    };
                                    painter.text(
                                        Pos2::new(rect.min.x + 28.0, rect.center().y),
                                        egui::Align2::LEFT_CENTER,
                                        &tag_name,
                                        FontId::proportional(type_scale::BODY_SM),
                                        text_color,
                                    );
                                    if resp.clicked() {
                                        self.current_tab = StudioTab::FileExplorer;
                                        self.explorer_view.set_active_tag(if is_active {
                                            None
                                        } else {
                                            Some(tag_name.clone())
                                        });
                                    }
                                    ui.add_space(1.0);
                                }
                            }
                        }

                        // 5. TOOLS & CONVERTERS Zone
                        if chrome_spec.sidebar.show_tools {
                            if Self::render_zone_header(
                                ui,
                                &palette,
                                "Tools & Converters",
                                cfg.sidebar_tools_expanded,
                                chrome_spec.sidebar.header_style,
                            ) {
                                cfg.sidebar_tools_expanded = !cfg.sidebar_tools_expanded;
                                if let Err(e) = cfg.save() {
                                    log::warn!("Could not persist sidebar state: {e}");
                                }
                            }
                            if cfg.sidebar_tools_expanded {
                                let tools = [
                                    (
                                        "Transcoder",
                                        IconGlyph::ActionRefresh,
                                        StudioTab::Transcoder,
                                    ),
                                    (
                                        "Custom Conformer",
                                        IconGlyph::ActionEdit,
                                        StudioTab::ExamStudio,
                                    ),
                                    (
                                        "Quality Inspector",
                                        IconGlyph::Search,
                                        StudioTab::QualityInspector,
                                    ),
                                    (
                                        "Archive Suite",
                                        IconGlyph::PageArchive,
                                        StudioTab::ArchiveSuite,
                                    ),
                                    (
                                        "Hardware & AI",
                                        IconGlyph::ActionShield,
                                        StudioTab::HardwareTelemetry,
                                    ),
                                    ("Preferences", IconGlyph::Gear, StudioTab::Preferences),
                                ];

                                let sec_rgb = [
                                    palette.text_secondary.r(),
                                    palette.text_secondary.g(),
                                    palette.text_secondary.b(),
                                ];
                                for (label, glyph, tab) in tools {
                                    let is_active = self.current_tab == tab;
                                    let spec = SidebarRowSpec {
                                        pack,
                                        glyph,
                                        accent_rgb: if is_active {
                                            [
                                                palette.accent.r(),
                                                palette.accent.g(),
                                                palette.accent.b(),
                                            ]
                                        } else {
                                            sec_rgb
                                        },
                                        label,
                                        selected: is_active,
                                        show_pin: false,
                                        dimmed: false,
                                        tooltip: Some(format!("Open {label}")),
                                    };
                                    let tool_click = render_sidebar_row(ui, &palette, &spec);
                                    if tool_click.row {
                                        self.switch_to_tab(tab);
                                        if tab == StudioTab::ExamStudio {
                                            quick_settings.exam_mode_enabled = true;
                                        }
                                    }
                                    ui.add_space(1.0);
                                }
                            }
                        }

                        if let Some((item, anchor)) = sidebar_hover {
                            self.explorer_view.trigger_hover_preview(item, anchor);
                        } else {
                            self.explorer_view.clear_sidebar_hover_preview();
                        }

                        ui.add_space(12.0);
                    });
            });
        }

        // 2. Central Content Panel with smooth vertical ScrollArea.
        // The explorer tab renders flush to the window edges (Finder
        // bottom chrome docks seamlessly); other tabs keep the inset.
        let central_frame = egui::Frame::none().fill(palette.window_bg).inner_margin(
            if self.current_tab == StudioTab::FileExplorer {
                egui::Margin::same(0.0)
            } else {
                egui::Margin::symmetric(10.0, 6.0)
            },
        );

        egui::CentralPanel::default()
            .frame(central_frame)
            .show(ctx, |ui| {
                match self.current_tab {
                    StudioTab::FileExplorer => {
                        let mut staged: Option<PathBuf> = None;
                        let mut transcode_reqs: Vec<(PathBuf, String)> = Vec::new();
                        let mut exam_req: Option<PathBuf> = None;
                        let mut pack_req: Option<Vec<PathBuf>> = None;
                        let mut inspect_req: Option<PathBuf> = None;

                        self.explorer_view.design_tokens = config.tokens();
                        self.explorer_view.dashboard_slots = config.dashboard_slots.clone();
                        self.explorer_view.render(
                            ui,
                            &palette,
                            |p| staged = Some(p),
                            |p, t| transcode_reqs.push((p, t)),
                            |p| exam_req = Some(p),
                            |list| pack_req = Some(list),
                            |p| inspect_req = Some(p),
                        );

                        if let Some(p) = staged {
                            on_stage_file(p);
                        }
                        if !transcode_reqs.is_empty() {
                            // Group the (multi-select) requests by target format so
                            // each group becomes ONE batch conversion instead of
                            // N single-file conversions clobbering each other.
                            let mut groups: Vec<(String, Vec<PathBuf>)> = Vec::new();
                            for (p, t) in transcode_reqs {
                                match groups.iter_mut().find(|(ft, _)| *ft == t) {
                                    Some((_, list)) => list.push(p),
                                    None => groups.push((t, vec![p])),
                                }
                            }
                            for (t, mut paths) in groups {
                                paths.sort();
                                paths.dedup();
                                if let Some(first) = paths.first() {
                                    on_stage_file(first.clone());
                                }
                                log::info!("Explorer batch: {} file(s) -> {}", paths.len(), t);
                                on_convert_batch(paths, t);
                            }
                        }
                        if let Some(p) = exam_req {
                            on_stage_file(p);
                            self.current_tab = StudioTab::ExamStudio;
                            quick_settings.exam_mode_enabled = true;
                        }
                        if let Some(list) = pack_req {
                            if let Some(first) = list.first() {
                                on_stage_file(first.clone());
                            }
                            self.current_tab = StudioTab::ArchiveSuite;
                        }
                        if let Some(p) = inspect_req {
                            on_stage_file(p.clone());
                            self.current_tab = StudioTab::QualityInspector;
                            if !self.inspection_busy {
                                self.inspection_busy = true;
                                self.inspection_status = "Inspecting in background...".to_string();
                                crate::core::tool_console::inspect_file_async(
                                    self.tool_console.sender(),
                                    p,
                                );
                            }
                        }
                    }
                    StudioTab::Preferences => {
                        self.render_tool_view_header(ui, &palette, "Preferences");
                        self.render_preferences_tab(
                            ui,
                            &palette,
                            current_theme,
                            quick_settings,
                            config,
                        );
                    }
                    other => {
                        let (tool_title, _) = studio_tab_label(other);
                        self.render_tool_view_header(ui, &palette, tool_title);
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                let content_w = 680.0_f32.min(ui.available_width() - 48.0);
                                let pad_x = ((ui.available_width() - content_w) * 0.5).max(24.0);
                                ui.horizontal(|ui| {
                                    ui.add_space(pad_x);
                                    ui.vertical(|ui| {
                                        ui.set_width(content_w);
                                        ui.add_space(16.0);
                                        match other {
                                            StudioTab::FileExplorer | StudioTab::Preferences => {
                                                unreachable!()
                                            }
                                            StudioTab::Transcoder => {
                                                self.render_transcoder_tab(
                                                    ui,
                                                    &palette,
                                                    current_file,
                                                    quick_settings,
                                                    &mut on_pick_file,
                                                    &mut on_convert_batch,
                                                );
                                            }
                                            StudioTab::ExamStudio => {
                                                self.render_exam_studio_tab(
                                                    ui,
                                                    &palette,
                                                    current_file,
                                                    quick_settings,
                                                    &mut on_pick_file,
                                                    on_open_folder,
                                                );
                                            }
                                            StudioTab::QualityInspector => {
                                                self.render_quality_inspector_tab(
                                                    ui,
                                                    &palette,
                                                    current_file,
                                                    &mut on_pick_file,
                                                );
                                            }
                                            StudioTab::ArchiveSuite => {
                                                self.render_archive_tab(
                                                    ui,
                                                    &palette,
                                                    current_file,
                                                    quick_settings,
                                                    &mut on_pick_file,
                                                    &mut on_convert_batch,
                                                );
                                            }
                                            StudioTab::HardwareTelemetry => {
                                                self.render_telemetry_tab(ui, &palette);
                                            }
                                        }
                                    });
                                });
                            });
                    }
                }

                if self.current_tab != StudioTab::FileExplorer {
                    self.explorer_view.render_floating_preview(ui, &palette);
                }
            });

        // Finder-style Command Palette Modal (Ctrl+P / Cmd+K)
        let palette_action = self.command_palette.render(
            ctx,
            &palette,
            self.explorer_view.icon_pack,
            Some(&self.explorer_view.current_dir),
            &self.explorer_view.entries,
        );
        if let Some(action) = palette_action {
            self.handle_palette_action(action, &mut on_stage_file, &mut on_convert_batch);
        }
    }

    pub fn handle_palette_action(
        &mut self,
        action: PaletteAction,
        on_stage_file: &mut impl FnMut(PathBuf),
        on_convert_batch: &mut impl FnMut(Vec<PathBuf>, String),
    ) {
        match action {
            PaletteAction::NavigateHome => {
                self.current_tab = StudioTab::FileExplorer;
                self.explorer_view.navigate_to_home();
            }
            PaletteAction::NavigateDesktop => {
                self.current_tab = StudioTab::FileExplorer;
                let target = crate::core::platform_paths::desktop_dir();
                self.explorer_view.navigate_to(target);
                self.explorer_view.is_home_view = false;
            }
            PaletteAction::NavigateDownloads => {
                self.current_tab = StudioTab::FileExplorer;
                let target = crate::core::platform_paths::downloads_dir();
                self.explorer_view.navigate_to(target);
                self.explorer_view.is_home_view = false;
            }
            PaletteAction::NavigateDocuments => {
                self.current_tab = StudioTab::FileExplorer;
                let target = crate::core::platform_paths::documents_dir();
                self.explorer_view.navigate_to(target);
                self.explorer_view.is_home_view = false;
            }
            PaletteAction::NavigatePictures => {
                self.current_tab = StudioTab::FileExplorer;
                let target = crate::core::platform_paths::pictures_dir();
                self.explorer_view.navigate_to(target);
                self.explorer_view.is_home_view = false;
            }
            PaletteAction::NavigateVideos => {
                self.current_tab = StudioTab::FileExplorer;
                let target = crate::core::platform_paths::videos_dir();
                self.explorer_view.navigate_to(target);
                self.explorer_view.is_home_view = false;
            }
            PaletteAction::NavigateMusic => {
                self.current_tab = StudioTab::FileExplorer;
                let target = crate::core::platform_paths::music_dir();
                self.explorer_view.navigate_to(target);
                self.explorer_view.is_home_view = false;
            }
            PaletteAction::NavigateRecycleBin => {
                self.current_tab = StudioTab::FileExplorer;
                let recycle_target = crate::core::resolve_recycle_bin_target();
                self.explorer_view.show_hidden_files = true;
                self.explorer_view.navigate_to(recycle_target);
                self.explorer_view.is_home_view = false;
            }
            PaletteAction::ViewDetails => {
                self.explorer_view.view_mode = crate::gui::explorer_view::ViewMode::Details;
            }
            PaletteAction::ViewGrid => {
                self.explorer_view.view_mode = crate::gui::explorer_view::ViewMode::Grid;
            }
            PaletteAction::ViewColumns => {
                self.explorer_view.view_mode = crate::gui::explorer_view::ViewMode::Columns;
            }
            PaletteAction::ToggleSplitView => {
                self.current_tab = StudioTab::FileExplorer;
                self.explorer_view.toggle_split_view();
            }
            PaletteAction::ToggleInspectorPane => {
                self.current_tab = StudioTab::FileExplorer;
                self.explorer_view.toggle_inspector_pane();
            }
            PaletteAction::TabTranscoder => {
                self.switch_to_tab(StudioTab::Transcoder);
            }
            PaletteAction::TabExamStudio => {
                self.switch_to_tab(StudioTab::ExamStudio);
            }
            PaletteAction::TabQualityInspector => {
                self.switch_to_tab(StudioTab::QualityInspector);
            }
            PaletteAction::TabArchiveSuite => {
                self.switch_to_tab(StudioTab::ArchiveSuite);
            }
            PaletteAction::TabHardwareTelemetry => {
                self.switch_to_tab(StudioTab::HardwareTelemetry);
            }
            PaletteAction::TabPreferences => {
                self.switch_to_tab(StudioTab::Preferences);
            }
            PaletteAction::ToggleHiddenFiles => {
                self.explorer_view.show_hidden_files = !self.explorer_view.show_hidden_files;
                self.explorer_view.refresh();
            }
            PaletteAction::SelectAll => {
                self.explorer_view.select_all();
            }
            PaletteAction::ClearSelection => {
                self.explorer_view.clear_selection();
            }
            PaletteAction::Undo => {
                let _ = crate::core::action_journal::undo_last();
                self.explorer_view.refresh();
            }
            PaletteAction::Redo => {
                self.explorer_view.refresh();
            }
            PaletteAction::OpenFile(path) => {
                crate::gui::explorer_view::open_in_default_app(&path);
            }
            PaletteAction::NavigateTo(path) => {
                self.current_tab = StudioTab::FileExplorer;
                self.explorer_view.navigate_to(path);
                self.explorer_view.is_home_view = false;
            }
            PaletteAction::NewFolder => {
                self.current_tab = StudioTab::FileExplorer;
                self.explorer_view.show_new_folder_dialog = true;
            }
            PaletteAction::NewWindow => {
                crate::gui::explorer_view::spawn_new_explorer_window(Some(
                    &self.explorer_view.current_dir,
                ));
            }
            PaletteAction::NewTab => {
                self.current_tab = StudioTab::FileExplorer;
                self.explorer_view
                    .add_tab(self.explorer_view.current_dir.clone());
            }
            PaletteAction::CloseTab => {
                let idx = self.explorer_view.active_tab_index;
                self.explorer_view.close_tab(idx);
            }
            PaletteAction::Refresh => {
                self.explorer_view.refresh();
            }
            PaletteAction::FocusSearch => {
                self.current_tab = StudioTab::FileExplorer;
                self.explorer_view.focus_search_requested = true;
            }
            PaletteAction::ShowProperties => {
                if let Some(first_path) = self.explorer_view.selected_paths().first() {
                    if let Some(item) = self
                        .explorer_view
                        .entries
                        .iter()
                        .find(|e| &e.path == first_path)
                        .cloned()
                    {
                        self.explorer_view.properties_item = Some(item);
                    }
                }
            }
            PaletteAction::DeleteSelected => {
                self.explorer_view.delete_selected();
            }
            PaletteAction::EmptyRecycleBin => {
                let _ = crate::core::fs_utils::empty_native_recycle_bin();
                self.explorer_view.refresh();
            }
            PaletteAction::Copy => {
                self.explorer_view.copy_selected_to_clipboard();
            }
            PaletteAction::Cut => {
                self.explorer_view.cut_selected_to_clipboard();
            }
            PaletteAction::Paste => {
                self.explorer_view.paste_from_clipboard();
            }
            PaletteAction::ConvertTarget(fmt) => {
                let targets = self.explorer_view.selected_paths();
                if !targets.is_empty() {
                    if let Some(first) = targets.first() {
                        on_stage_file(first.clone());
                    }
                    on_convert_batch(targets, fmt.to_string());
                }
            }
            PaletteAction::ExamConform => {
                let targets = self.explorer_view.selected_paths();
                if let Some(first) = targets.first() {
                    on_stage_file(first.clone());
                }
                self.current_tab = StudioTab::ExamStudio;
            }
            PaletteAction::PackZip => {
                let targets = self.explorer_view.selected_paths();
                if let Some(first) = targets.first() {
                    on_stage_file(first.clone());
                }
                self.current_tab = StudioTab::ArchiveSuite;
            }
            PaletteAction::RunUserScript { title, body, lang } => {
                let current_file = self.explorer_view.selected_paths().first().cloned();
                let selected = self.explorer_view.selected_paths();
                let sender = self.script_console.sender();
                crate::core::scripting::execute_script_async(
                    body,
                    lang,
                    current_file,
                    selected,
                    move |outcome| {
                        // Terminal log stays for stdout runs; the live record
                        // goes to the Scripts page console.
                        if outcome.success {
                            println!("[Script] '{}' finished successfully", title);
                        } else {
                            eprintln!("[Script] '{}' failed: {:?}", title, outcome.error);
                        }
                        let _ = sender.send((title, outcome));
                    },
                );
            }
        }
    }

    /// Tab 1: Universal Transcoder & 200+ Format Matrix (Never blank!)
    fn render_transcoder_tab(
        &mut self,
        ui: &mut Ui,
        palette: &ThemePalette,
        current_file: &Option<PathBuf>,
        settings: &mut QuickSettingsState,
        mut on_pick_file: impl FnMut(),
        mut on_convert_batch: impl FnMut(Vec<PathBuf>, String),
    ) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("Universal File Transcoder").size(12.0).strong().color(palette.text_primary));
                ui.label(egui::RichText::new("0.00% Zero-Loss Standard | 200+ Formats | Multi-Threaded Rayon Concurrency").size(12.0).color(palette.text_secondary));
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let threads_label = if let Some(hw) = &self.telemetry_hw {
                    format!("{} Threads Ready", hw.optimal_thread_count)
                } else {
                    "Detecting threads...".to_string()
                };
                ui.label(
                    egui::RichText::new(threads_label)
                        .size(12.0)
                        .color(palette.accent),
                );
            });
        });

        ui.add_space(16.0);
        Self::render_tab_drop_banner(ui, palette, "Release to stage the file for conversion");

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
                .stroke(Stroke::new(1.0_f32, palette.border))
                .rounding(Rounding::same(12.0))
                .inner_margin(egui::Margin::same(16.0));

            staged_frame.show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Dc").size(12.0));
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new(
                                path.file_name().unwrap_or_default().to_string_lossy(),
                            )
                            .size(14.5)
                            .strong()
                            .color(palette.text_primary),
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "Category: {:?} | Format: .{} | Size: {}",
                                cat, in_ext, size_str
                            ))
                            .size(13.0)
                            .color(palette.text_muted),
                        );
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button(egui::RichText::new("Change File").size(12.5))
                            .clicked()
                        {
                            on_pick_file();
                        }
                    });
                });
            });

            ui.add_space(16.0);

            // Target Formats Grid
            let available = UniversalRegistry::get_all_supported_outputs(cat);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Select Target Output Format:")
                        .size(14.0)
                        .strong()
                        .color(palette.text_primary),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let all_btn = ui.add(
                        egui::Button::new(
                            egui::RichText::new("Convert to ALL Formats")
                                .size(13.0)
                                .strong()
                                .color(Color32::from_rgb(50, 35, 0)),
                        )
                        .fill(PANTONE_GOLD)
                        .rounding(Rounding::same(8.0)),
                    );
                    if all_btn
                        .on_hover_text("Generate output in all supported formats simultaneously")
                        .clicked()
                    {
                        on_convert_batch(current_file.iter().cloned().collect(), "ALL".to_string());
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
                        egui::Button::new(egui::RichText::new(*fmt).size(13.0).strong().color(
                            if is_sel {
                                Color32::WHITE
                            } else {
                                palette.text_secondary
                            },
                        ))
                        .fill(if is_sel {
                            palette.accent
                        } else {
                            palette.card_bg
                        })
                        .stroke(Stroke::new(
                            1.0_f32,
                            if is_sel {
                                palette.accent
                            } else {
                                palette.border
                            },
                        ))
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
                    ui.label(
                        egui::RichText::new(format!(
                            "Conversion Tuning for {}:",
                            settings.active_format
                        ))
                        .strong()
                        .color(palette.text_primary),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let convert_btn = ui.add(
                            egui::Button::new(
                                egui::RichText::new(format!(
                                    "Convert to {} ->",
                                    settings.active_format
                                ))
                                .size(13.5)
                                .strong()
                                .color(Color32::WHITE),
                            )
                            .fill(palette.accent)
                            .rounding(Rounding::same(8.0)),
                        );
                        if convert_btn.clicked() {
                            on_convert_batch(
                                current_file.iter().cloned().collect(),
                                settings.active_format.clone(),
                            );
                        }
                    });
                });

                ui.add_space(8.0);
                match settings.active_format.as_str() {
                    "PDF" => {
                        ui.horizontal(|ui| {
                            ui.label("Page Size:");
                            ui.selectable_value(
                                &mut settings.pdf_page_size,
                                "A4".to_string(),
                                "A4",
                            );
                            ui.selectable_value(
                                &mut settings.pdf_page_size,
                                "Letter".to_string(),
                                "Letter",
                            );
                            ui.add_space(16.0);
                            ui.label("Font Size:");
                            ui.add(
                                egui::Slider::new(&mut settings.pdf_font_size, 8.0..=18.0)
                                    .suffix(" pt"),
                            );
                        });
                    }
                    "MP3" | "AAC" | "OGG" => {
                        ui.horizontal(|ui| {
                            ui.label("Bitrate:");
                            ui.selectable_value(
                                &mut settings.audio_bitrate,
                                "128k".to_string(),
                                "128 kbps",
                            );
                            ui.selectable_value(
                                &mut settings.audio_bitrate,
                                "192k".to_string(),
                                "192 kbps",
                            );
                            ui.selectable_value(
                                &mut settings.audio_bitrate,
                                "320k".to_string(),
                                "320 kbps (Max)",
                            );
                        });
                    }
                    "JPG" | "WEBP" | "AVIF" => {
                        ui.horizontal(|ui| {
                            ui.label("Quality:");
                            ui.add(
                                egui::Slider::new(&mut settings.image_quality, 50..=100)
                                    .suffix("%"),
                            );
                            ui.checkbox(
                                &mut settings.image_lossless,
                                "Strict Lossless (Bit-Exact)",
                            );
                        });
                    }
                    _ => {
                        ui.label(
                            egui::RichText::new(
                                "Lossless bit-exact container remux active. 0.00% Zero-Loss.",
                            )
                            .size(12.5)
                            .color(palette.text_muted),
                        );
                    }
                }

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(4.0);

                // Custom Conformer Parameters directly in Transcoder tab
                ui.horizontal(|ui| {
                    ui.checkbox(
                        &mut settings.exam_mode_enabled,
                        "Apply Custom Parameters (Dimensions, DPI & Size Limit)",
                    );
                });

                if settings.exam_mode_enabled {
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        ui.add(
                            egui::TextEdit::singleline(&mut settings.exam_preset_name)
                                .desired_width(180.0),
                        );
                        ui.add_space(10.0);
                        ui.checkbox(&mut settings.exam_whiten, "Whiten Background");
                    });
                    ui.horizontal(|ui| {
                        ui.label("Dimensions:");
                        ui.add(
                            egui::DragValue::new(&mut settings.exam_width)
                                .speed(1)
                                .prefix("W: ")
                                .suffix("px"),
                        );
                        ui.add(
                            egui::DragValue::new(&mut settings.exam_height)
                                .speed(1)
                                .prefix("H: ")
                                .suffix("px"),
                        );
                        ui.add_space(10.0);
                        ui.label("DPI:");
                        ui.add(egui::DragValue::new(&mut settings.exam_dpi).speed(1));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Size Bounds:");
                        ui.add(
                            egui::DragValue::new(&mut settings.exam_min_kb)
                                .speed(1)
                                .prefix("Min: ")
                                .suffix("KB"),
                        );
                        ui.add(
                            egui::DragValue::new(&mut settings.exam_max_kb)
                                .speed(1)
                                .prefix("Max: ")
                                .suffix("KB"),
                        );
                    });
                }
            });
        } else {
            // =========================================================================
            // RICH HERO LANDING VIEW: When NO file is staged (Never an empty blank void!)
            // =========================================================================

            // 1. Hero Drag-and-Drop Dropzone
            let dropzone_frame = egui::Frame::none()
                .fill(if palette.is_dark {
                    Color32::from_rgba_unmultiplied(22, 24, 28, 230)
                } else {
                    Color32::from_rgba_unmultiplied(248, 250, 252, 240)
                })
                .stroke(Stroke::new(1.0_f32, palette.border))
                .rounding(Rounding::same(14.0))
                .inner_margin(egui::Margin::same(24.0));

            dropzone_frame.show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    let (hero_r, _) = ui.allocate_exact_size(Vec2::splat(28.0), Sense::hover());
                    crate::core::icon_packs::paint_icon(
                        ui,
                        self.explorer_view.icon_pack,
                        IconGlyph::ActionShield,
                        hero_r,
                        [palette.accent.r(), palette.accent.g(), palette.accent.b()],
                    );
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new("Drag & Drop Any Files or Folders Here")
                            .size(16.0)
                            .strong()
                            .color(palette.text_primary),
                    );
                    ui.label(
                        egui::RichText::new("Mathematical 0.00% Zero-Loss transcode standard | 200+ formats supported")
                            .size(13.0)
                            .color(palette.text_secondary),
                    );
                    ui.add_space(14.0);

                    ui.horizontal(|ui| {
                        ui.add_space((ui.available_width() - 280.0).max(0.0) / 2.0);
                        let pick_btn = ui.add(
                            egui::Button::new(
                                egui::RichText::new("Choose Files")
                                    .size(13.5)
                                    .strong()
                                    .color(Color32::WHITE),
                            )
                            .fill(palette.accent)
                            .rounding(Rounding::same(8.0)),
                        );
                        if pick_btn.clicked() {
                            on_pick_file();
                        }

                        let folder_btn = ui.add(
                            egui::Button::new(
                                egui::RichText::new("Choose Folder")
                                    .size(13.5)
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
                        ui.label(egui::RichText::new("28 Images").size(12.0).color(palette.text_muted));
                        ui.separator();
                        ui.label(egui::RichText::new("34 Audio").size(12.0).color(palette.text_muted));
                        ui.separator();
                        ui.label(egui::RichText::new("42 Video").size(12.0).color(palette.text_muted));
                        ui.separator();
                        ui.label(egui::RichText::new("18 Documents").size(12.0).color(palette.text_muted));
                        ui.separator();
                        ui.label(egui::RichText::new("12 Archives").size(12.0).color(palette.text_muted));
                        ui.separator();
                        ui.label(egui::RichText::new("Indian Exam Portals").size(12.0).color(palette.accent));
                    });
                });
            });

            ui.add_space(18.0);

            // 2. Quick-Start Workflow Presets (2x2 Grid)
            ui.label(
                egui::RichText::new("Quick-Start Conversion Workflows")
                    .size(14.0)
                    .strong()
                    .color(palette.text_primary),
            );
            ui.add_space(8.0);

            ui.columns(2, |cols| {
                // Preset 1: Web Optimizer
                let p1 = egui::Frame::none()
                    .fill(palette.card_bg)
                    .stroke(Stroke::new(1.0_f32, palette.border))
                    .rounding(Rounding::same(10.0))
                    .inner_margin(egui::Margin::same(14.0));
                p1.show(&mut cols[0], |ui| {
                    // Left-aligned (not justified) text inside the card.
                    ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.label(egui::RichText::new("Web Image Optimizer").size(14.0).strong().color(palette.text_primary));
                    ui.label(egui::RichText::new("Convert PNG/JPG to WebP/AVIF for up to 80% size savings with zero perceptual loss.").size(12.5).color(palette.text_secondary));
                    ui.add_space(6.0);
                    if ui.button(egui::RichText::new("Select Images ->").size(12.5).color(palette.accent)).clicked() {
                        on_pick_file();
                    }
                    });
                });

                // Preset 2: Vector PDF
                let p2 = egui::Frame::none()
                    .fill(palette.card_bg)
                    .stroke(Stroke::new(1.0_f32, palette.border))
                    .rounding(Rounding::same(10.0))
                    .inner_margin(egui::Margin::same(14.0));
                p2.show(&mut cols[1], |ui| {
                    // Left-aligned (not justified) text inside the card.
                    ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.label(egui::RichText::new("Notes to Vector PDF").size(14.0).strong().color(palette.text_primary));
                    ui.label(egui::RichText::new("Typeset text, markdown, and code into publication-grade 300 DPI vector PDF.").size(12.5).color(palette.text_secondary));
                    ui.add_space(6.0);
                    if ui.button(egui::RichText::new("Select Document ->").size(12.5).color(palette.accent)).clicked() {
                        on_pick_file();
                    }
                    });
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
                    // Left-aligned (not justified) text inside the card.
                    ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.label(egui::RichText::new("Lossless Audio Remux").size(14.0).strong().color(palette.text_primary));
                    ui.label(egui::RichText::new("Bit-exact WAV <-> FLAC transcoding with zero stream re-encoding and CRC verification.").size(12.5).color(palette.text_secondary));
                    ui.add_space(6.0);
                    if ui.button(egui::RichText::new("Select Audio ->").size(12.5).color(palette.accent)).clicked() {
                        on_pick_file();
                    }
                    });
                });

                // Preset 4: Indian Exam Form
                let p4 = egui::Frame::none()
                    .fill(palette.card_bg)
                    .stroke(Stroke::new(1.0_f32, palette.border))
                    .rounding(Rounding::same(10.0))
                    .inner_margin(egui::Margin::same(14.0));
                p4.show(&mut cols[1], |ui| {
                    // Left-aligned (not justified) text inside the card.
                    ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.label(egui::RichText::new("Custom Form Conformer").size(14.0).strong().color(palette.text_primary));
                    ui.label(egui::RichText::new("Conforms any photo, signature, or document to exact custom dimensions, DPI, and file size boundaries.").size(12.5).color(palette.text_secondary));
                    ui.add_space(6.0);
                    if ui.button(egui::RichText::new("Open Conformer ->").size(12.5).color(palette.accent)).clicked() {
                        self.current_tab = StudioTab::ExamStudio;
                    }
                    });
                });
            });

            ui.add_space(18.0);

            // 3. Interactive 200+ Format Catalogue Browser
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("200+ Supported Formats Catalogue")
                        .size(14.0)
                        .strong()
                        .color(palette.text_primary),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.catalogue_search)
                            .hint_text("Filter format (e.g. flac, webp, pdf)...")
                            .desired_width(200.0),
                    );
                });
            });

            ui.add_space(8.0);

            // Live catalogue (W-7 Batch C): chips come from the real
            // 200+ format registry instead of a hardcoded sample list;
            // the search box drives UniversalRegistry's extension/name/
            // description filter directly.
            const MAX_CATALOG_CHIPS: usize = 48;
            let filtered = UniversalRegistry::search_catalogue(&self.catalogue_search);
            // Chips are laid out with a KNOWN size (galley measured first):
            // a Frame inside horizontal_wrapped cannot wrap, so the row used
            // to run off the right edge of the window.
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
                for spec in filtered.iter().take(MAX_CATALOG_CHIPS) {
                    let mut job = egui::text::LayoutJob::default();
                    job.append(
                        spec.extension,
                        0.0,
                        egui::TextFormat::simple(
                            egui::FontId::proportional(13.0),
                            palette.text_primary,
                        ),
                    );
                    job.append(
                        spec.description,
                        8.0,
                        egui::TextFormat::simple(
                            egui::FontId::proportional(12.0),
                            palette.text_muted,
                        ),
                    );
                    let galley = ui.fonts(|fo| fo.layout_job(job));
                    let pad = Vec2::new(10.0, 6.0);
                    let (rect, _) =
                        ui.allocate_exact_size(galley.size() + pad * 2.0, egui::Sense::hover());
                    if ui.is_rect_visible(rect) {
                        let p = ui.painter();
                        crate::gui::squircle::squircle_filled(
                            p,
                            rect,
                            Rounding::same(6.0),
                            palette.card_bg,
                        );
                        crate::gui::squircle::squircle_stroke(
                            p,
                            rect,
                            Rounding::same(6.0),
                            Stroke::new(1.0_f32, palette.border),
                        );
                        p.galley(rect.min + pad, galley, palette.text_primary);
                    }
                }
            });

            if filtered.len() > MAX_CATALOG_CHIPS {
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(format!(
                        "+ {} more formats - refine the filter to narrow the catalogue",
                        filtered.len() - MAX_CATALOG_CHIPS
                    ))
                    .size(11.0)
                    .color(palette.text_muted),
                );
            }
        }

        // Live conversion queue (W-7): chips, per-item progress, actions.
        self.render_conversion_queue_panel(ui, palette);
    }

    /// Conversion queue panel (W-7 Batch A): per-item status chips, live
    /// progress bars, cancel/retry/clear actions, per-output Reveal buttons,
    /// and an overall batch bar. Renders nothing when the queue is empty.
    fn render_conversion_queue_panel(&mut self, ui: &mut Ui, palette: &ThemePalette) {
        let items = self.queue.items().to_vec();
        if items.is_empty() {
            return;
        }
        let summary = self.queue.summary();

        ui.add_space(14.0);
        ui.separator();
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Conversion Queue")
                    .size(12.0)
                    .strong()
                    .color(palette.text_primary),
            );
            ui.label(
                egui::RichText::new(format!(
                    "{} waiting | {} converting | {} done | {} failed | {} cancelled",
                    summary.queued,
                    summary.running,
                    summary.done,
                    summary.failed,
                    summary.cancelled
                ))
                .size(11.0)
                .monospace()
                .color(palette.text_secondary),
            );
        });

        let mut cancel_clicked = false;
        let mut retry_clicked = false;
        let mut clear_clicked = false;
        ui.horizontal(|ui| {
            if summary.active() > 0 && ui.small_button("Cancel").clicked() {
                cancel_clicked = true;
            }
            if summary.failed > 0 && ui.small_button("Retry Failed").clicked() {
                retry_clicked = true;
            }
            if summary.done + summary.failed + summary.cancelled > 0
                && ui.small_button("Clear Finished").clicked()
            {
                clear_clicked = true;
            }
        });

        if let Some(fraction) = self.batch_progress {
            ui.add(
                egui::ProgressBar::new(fraction)
                    .text(format!("Batch {:.0}%", fraction * 100.0))
                    .fill(palette.accent),
            );
        }

        let mut reveals: Vec<PathBuf> = Vec::new();
        egui::ScrollArea::vertical()
            .max_height(280.0)
            .show(ui, |ui| {
                for item in &items {
                    ui.add_space(6.0);
                    let (chip_label, chip_color) = match item.status {
                        QueueItemStatus::Queued => ("Waiting", palette.text_muted),
                        QueueItemStatus::Running => ("Converting", palette.accent),
                        QueueItemStatus::Stopping => ("Stopping", Color32::from_rgb(210, 153, 34)),
                        QueueItemStatus::Done => ("Done", Color32::from_rgb(46, 160, 67)),
                        QueueItemStatus::Failed => ("Failed", Color32::from_rgb(248, 81, 73)),
                        QueueItemStatus::Cancelled => {
                            ("Cancelled", Color32::from_rgb(110, 118, 129))
                        }
                    };
                    let fallback_name = item.source.display().to_string();
                    let file_name = item
                        .source
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or(fallback_name.as_str());

                    ui.horizontal(|ui| {
                        egui::Frame::none()
                            .fill(chip_color.gamma_multiply(0.22))
                            .rounding(Rounding::same(4.0))
                            .inner_margin(egui::Margin::symmetric(6.0, 2.0))
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(chip_label)
                                        .size(10.0)
                                        .strong()
                                        .color(chip_color),
                                );
                            });
                        ui.label(
                            egui::RichText::new(file_name)
                                .size(11.0)
                                .color(palette.text_primary),
                        );
                        ui.label(
                            egui::RichText::new(format!("-> {}", item.target))
                                .size(10.0)
                                .color(palette.text_secondary),
                        );
                    });

                    if let Some(route) = &item.planned_route {
                        ui.label(
                            egui::RichText::new(route)
                                .monospace()
                                .size(10.0)
                                .color(palette.text_muted),
                        );
                    }

                    if matches!(
                        item.status,
                        QueueItemStatus::Running | QueueItemStatus::Stopping
                    ) {
                        let mut bar = egui::ProgressBar::new(item.progress)
                            .fill(chip_color)
                            .show_percentage();
                        if item.status == QueueItemStatus::Running {
                            bar = bar.animate(true);
                        }
                        ui.add(bar);
                    }

                    if !item.status_text.is_empty() {
                        ui.label(
                            egui::RichText::new(item.status_text.as_str())
                                .monospace()
                                .size(10.0)
                                .color(palette.text_secondary),
                        );
                    }

                    if !item.outputs.is_empty() {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("Output:")
                                    .size(10.0)
                                    .color(palette.text_muted),
                            );
                            for out in &item.outputs {
                                let out_name = out
                                    .file_name()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("output")
                                    .to_string();
                                if ui.small_button(out_name.as_str()).clicked() {
                                    reveals.push(out.clone());
                                }
                            }
                        });
                    }

                    if let Some(err) = &item.error_detail {
                        egui::CollapsingHeader::new(
                            egui::RichText::new("Details")
                                .size(10.0)
                                .color(palette.text_secondary),
                        )
                        .id_salt(egui::Id::new(("queue_error", item.id)))
                        .default_open(false)
                        .show(ui, |ui| {
                            ui.monospace(err);
                        });
                    }

                    ui.separator();
                }
            });

        if cancel_clicked {
            self.queue_actions.push(QueueAction::CancelActive);
        }
        if retry_clicked {
            self.queue_actions.push(QueueAction::RetryFailed);
        }
        if clear_clicked {
            self.queue_actions.push(QueueAction::ClearFinished);
        }
        for out in reveals {
            self.queue_actions.push(QueueAction::Reveal(out));
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
        ui.label(
            egui::RichText::new("Custom Specification & Form Conformer")
                .size(12.0)
                .strong()
                .color(palette.text_primary),
        );
        ui.label(
            egui::RichText::new("Conform any photo, signature, or document to your exact custom specifications with micro-converged binary search bounds. No unverified or outdated templates.")
                .size(13.0)
                .color(palette.text_secondary),
        );
        ui.add_space(14.0);
        Self::render_tab_drop_banner(ui, palette, "Release to queue exam optimization");

        // Tuning Envelope Card
        let envelope_frame = egui::Frame::none()
            .fill(palette.card_bg)
            .stroke(Stroke::new(1.0_f32, palette.border))
            .rounding(Rounding::same(10.0))
            .inner_margin(egui::Margin::same(16.0));

        envelope_frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Custom Parameter Envelope")
                        .strong()
                        .color(palette.text_primary),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "Active Specification: {}",
                            settings.exam_preset_name
                        ))
                        .size(12.5)
                        .color(palette.text_muted),
                    );
                });
            });

            ui.add_space(10.0);

            // Row 0: Preset Selector with SQLite backing
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Preset:").strong().size(13.0));
                let presets = crate::core::state_db::load_presets();
                egui::ComboBox::from_id_salt("conformer_preset_combo")
                    .selected_text(&self.conformer_preset_selector)
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_label(
                                self.conformer_preset_selector
                                    == "Passport Photo (35x45mm / 300 DPI)",
                                "Passport Photo (35x45mm / 300 DPI)",
                            )
                            .clicked()
                        {
                            self.conformer_preset_selector =
                                "Passport Photo (35x45mm / 300 DPI)".into();
                            settings.exam_preset_name = "Passport Photo".into();
                            settings.exam_width = 413;
                            settings.exam_height = 531;
                            settings.exam_dpi = 300;
                            settings.exam_min_kb = 20;
                            settings.exam_max_kb = 50;
                            settings.exam_whiten = true;
                        }
                        if ui
                            .selectable_label(
                                self.conformer_preset_selector
                                    == "Application Signature (2:1 / 300 DPI)",
                                "Application Signature (2:1 / 300 DPI)",
                            )
                            .clicked()
                        {
                            self.conformer_preset_selector =
                                "Application Signature (2:1 / 300 DPI)".into();
                            settings.exam_preset_name = "Signature".into();
                            settings.exam_width = 400;
                            settings.exam_height = 200;
                            settings.exam_dpi = 300;
                            settings.exam_min_kb = 10;
                            settings.exam_max_kb = 20;
                            settings.exam_whiten = true;
                        }
                        if ui
                            .selectable_label(
                                self.conformer_preset_selector == "ID Document Scan (A4 / 300 DPI)",
                                "ID Document Scan (A4 / 300 DPI)",
                            )
                            .clicked()
                        {
                            self.conformer_preset_selector =
                                "ID Document Scan (A4 / 300 DPI)".into();
                            settings.exam_preset_name = "ID Document".into();
                            settings.exam_width = 2480;
                            settings.exam_height = 3508;
                            settings.exam_dpi = 300;
                            settings.exam_min_kb = 100;
                            settings.exam_max_kb = 500;
                            settings.exam_whiten = true;
                        }
                        if ui
                            .selectable_label(
                                self.conformer_preset_selector == "Square Avatar (600x600 px)",
                                "Square Avatar (600x600 px)",
                            )
                            .clicked()
                        {
                            self.conformer_preset_selector = "Square Avatar (600x600 px)".into();
                            settings.exam_preset_name = "Square Avatar".into();
                            settings.exam_width = 600;
                            settings.exam_height = 600;
                            settings.exam_dpi = 300;
                            settings.exam_min_kb = 30;
                            settings.exam_max_kb = 100;
                            settings.exam_whiten = false;
                        }
                        if ui
                            .selectable_label(
                                self.conformer_preset_selector
                                    == "US Visa Photo (2x2 in / 600x600 px)",
                                "US Visa Photo (2x2 in / 600x600 px)",
                            )
                            .clicked()
                        {
                            self.conformer_preset_selector =
                                "US Visa Photo (2x2 in / 600x600 px)".into();
                            settings.exam_preset_name = "US Visa".into();
                            settings.exam_width = 600;
                            settings.exam_height = 600;
                            settings.exam_dpi = 300;
                            settings.exam_min_kb = 20;
                            settings.exam_max_kb = 240;
                            settings.exam_whiten = true;
                        }
                        if ui
                            .selectable_label(
                                self.conformer_preset_selector == "Schengen Visa (35x45 mm)",
                                "Schengen Visa (35x45 mm)",
                            )
                            .clicked()
                        {
                            self.conformer_preset_selector = "Schengen Visa (35x45 mm)".into();
                            settings.exam_preset_name = "Schengen Visa".into();
                            settings.exam_width = 413;
                            settings.exam_height = 531;
                            settings.exam_dpi = 300;
                            settings.exam_min_kb = 30;
                            settings.exam_max_kb = 150;
                            settings.exam_whiten = true;
                        }
                        if ui
                            .selectable_label(
                                self.conformer_preset_selector == "Canada PR / Visa (50x70 mm)",
                                "Canada PR / Visa (50x70 mm)",
                            )
                            .clicked()
                        {
                            self.conformer_preset_selector = "Canada PR / Visa (50x70 mm)".into();
                            settings.exam_preset_name = "Canada PR".into();
                            settings.exam_width = 590;
                            settings.exam_height = 826;
                            settings.exam_dpi = 300;
                            settings.exam_min_kb = 100;
                            settings.exam_max_kb = 500;
                            settings.exam_whiten = true;
                        }
                        for p in &presets {
                            let label =
                                format!("Custom: {} ({}x{} px)", p.name, p.width_px, p.height_px);
                            if ui
                                .selectable_label(self.conformer_preset_selector == p.name, &label)
                                .clicked()
                            {
                                self.conformer_preset_selector = p.name.clone();
                                settings.exam_preset_name = p.name.clone();
                                settings.exam_width = p.width_px;
                                settings.exam_height = p.height_px;
                                settings.exam_dpi = p.dpi;
                                settings.exam_min_kb = p.min_bytes / 1024;
                                settings.exam_max_kb = p.max_bytes / 1024;
                                settings.exam_whiten = p.normalize_white_balance;
                            }
                        }
                    });

                if ui.button("Save Preset").clicked() {
                    let record = crate::core::state_db::CustomPresetRecord {
                        name: settings.exam_preset_name.clone(),
                        category: "Custom".into(),
                        target_format: "JPG".into(),
                        width_px: settings.exam_width,
                        height_px: settings.exam_height,
                        dpi: settings.exam_dpi,
                        min_bytes: settings.exam_min_kb * 1024,
                        max_bytes: settings.exam_max_kb * 1024,
                        normalize_white_balance: settings.exam_whiten,
                        background_rgb: [255, 255, 255],
                    };
                    let _ = crate::core::state_db::save_preset(&record);
                }

                if ui.button("Delete").clicked() {
                    let _ = crate::core::state_db::delete_preset(&settings.exam_preset_name);
                }
            });

            ui.add_space(8.0);

            // Row 1: Specification Name & Units
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Specification Name:").size(13.0));
                ui.add(
                    egui::TextEdit::singleline(&mut settings.exam_preset_name)
                        .desired_width(180.0)
                        .hint_text("e.g. Application Photo, Signature"),
                );
                ui.separator();
                ui.label("Units:");
                if ui
                    .selectable_label(self.conformer_unit == 0, "px")
                    .clicked()
                {
                    self.conformer_unit = 0;
                }
                if ui
                    .selectable_label(self.conformer_unit == 1, "cm")
                    .clicked()
                {
                    self.conformer_unit = 1;
                }
                if ui
                    .selectable_label(self.conformer_unit == 2, "mm")
                    .clicked()
                {
                    self.conformer_unit = 2;
                }
                if self.conformer_unit == 1 {
                    let w_cm = (settings.exam_width as f32 / settings.exam_dpi as f32) * 2.54;
                    let h_cm = (settings.exam_height as f32 / settings.exam_dpi as f32) * 2.54;
                    ui.label(
                        egui::RichText::new(format!("({:.2} x {:.2} cm)", w_cm, h_cm))
                            .color(palette.accent),
                    );
                } else if self.conformer_unit == 2 {
                    let w_mm = (settings.exam_width as f32 / settings.exam_dpi as f32) * 25.4;
                    let h_mm = (settings.exam_height as f32 / settings.exam_dpi as f32) * 25.4;
                    ui.label(
                        egui::RichText::new(format!("({:.1} x {:.1} mm)", w_mm, h_mm))
                            .color(palette.accent),
                    );
                }
            });

            ui.add_space(8.0);

            // Row 2: Two-column layout for Dimensions & File Size Bounds
            ui.columns(2, |cols| {
                cols[0].vertical(|ui| {
                    ui.label(
                        egui::RichText::new("Dimensions & Resolution")
                            .strong()
                            .color(palette.text_primary),
                    );
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label("Width (px):");
                        ui.add(
                            egui::DragValue::new(&mut settings.exam_width)
                                .speed(1)
                                .range(20..=8000),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label("Height (px):");
                        ui.add(
                            egui::DragValue::new(&mut settings.exam_height)
                                .speed(1)
                                .range(20..=8000),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label("Calibrated DPI:");
                        ui.add(
                            egui::DragValue::new(&mut settings.exam_dpi)
                                .speed(1)
                                .range(50..=1200),
                        );
                    });
                    let ratio = if settings.exam_height > 0 {
                        settings.exam_width as f32 / settings.exam_height as f32
                    } else {
                        1.0
                    };
                    ui.label(
                        egui::RichText::new(format!(
                            "Aspect Ratio: {:.2}:1 ({}x{} px)",
                            ratio, settings.exam_width, settings.exam_height
                        ))
                        .size(12.0)
                        .color(palette.text_muted),
                    );
                });

                cols[1].vertical(|ui| {
                    ui.label(
                        egui::RichText::new("File Size Boundaries")
                            .strong()
                            .color(palette.text_primary),
                    );
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label("Min Size (KB):");
                        ui.add(
                            egui::DragValue::new(&mut settings.exam_min_kb)
                                .speed(1)
                                .range(1..=102400),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label("Max Size (KB):");
                        ui.add(
                            egui::DragValue::new(&mut settings.exam_max_kb)
                                .speed(1)
                                .range(1..=102400),
                        );
                    });
                    ui.add_space(4.0);
                    ui.checkbox(
                        &mut settings.exam_whiten,
                        "Whiten Background & Clean Scanner Shadows",
                    );
                    ui.label(
                        egui::RichText::new(format!(
                            "Allowed Range: {} KB - {} KB",
                            settings.exam_min_kb, settings.exam_max_kb
                        ))
                        .size(12.0)
                        .color(palette.text_muted),
                    );
                });
            });

            ui.add_space(14.0);
            ui.separator();
            ui.add_space(8.0);

            // Pre-Scan Comparison Card (Input vs Target Spec)
            if let Some(ref path) = current_file {
                let file_len = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
                let in_kb = file_len as f64 / 1024.0;
                let file_name = path.file_name().unwrap_or_default().to_string_lossy();
                let ext = path
                    .extension()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_uppercase();

                egui::Frame::none()
                    .fill(palette.sidebar_bg)
                    .stroke(Stroke::new(1.0_f32, palette.border))
                    .rounding(Rounding::same(8.0))
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new("Pre-Scan Conformer Analysis")
                                .strong()
                                .size(13.0)
                                .color(palette.accent),
                        );
                        ui.add_space(6.0);
                        ui.columns(2, |cols| {
                            cols[0].vertical(|ui| {
                                ui.label(
                                    egui::RichText::new("[Input Source File]")
                                        .strong()
                                        .size(12.0),
                                );
                                ui.label(format!("File: {}", file_name));
                                ui.label(format!("Current Size: {:.1} KB", in_kb));
                                ui.label(format!("Format: {}", ext));
                            });
                            cols[1].vertical(|ui| {
                                ui.label(
                                    egui::RichText::new("[Conformed Target]")
                                        .strong()
                                        .size(12.0)
                                        .color(palette.accent),
                                );
                                ui.label(format!("Preset: {}", settings.exam_preset_name));
                                ui.label(format!(
                                    "Dimensions: {} x {} px @ {} DPI",
                                    settings.exam_width, settings.exam_height, settings.exam_dpi
                                ));
                                ui.label(format!(
                                    "Target Budget: {} - {} KB",
                                    settings.exam_min_kb, settings.exam_max_kb
                                ));
                                ui.label(format!(
                                    "Whitening: {}",
                                    if settings.exam_whiten {
                                        "Enabled (Clean White BG)"
                                    } else {
                                        "Disabled (Raw Image)"
                                    }
                                ));
                            });
                        });
                    });

                ui.add_space(10.0);
            }

            // Execute Custom Form Optimization
            ui.horizontal(|ui| {
                if let Some(ref path) = current_file {
                    let optimize_btn = ui.add(
                        egui::Button::new(
                            egui::RichText::new("Optimize & Generate Conformed Output")
                                .size(13.5)
                                .strong()
                                .color(Color32::WHITE),
                        )
                        .fill(palette.accent)
                        .rounding(Rounding::same(8.0)),
                    );

                    if optimize_btn.clicked() {
                        // Defer to the conversion queue (W-7 Batch B): the
                        // exam job runs on the worker pool with live progress
                        // and cooperative cancel instead of freezing the UI.
                        self.exam_status = "Queued for optimization...".to_string();
                        self.queue_actions.push(QueueAction::RunExam(path.clone()));
                    }

                    if let Some(ref out) = self.last_exam_output {
                        if ui
                            .button(egui::RichText::new("Open in Explorer").size(13.0))
                            .clicked()
                        {
                            on_open_folder(out);
                        }
                    }
                } else {
                    ui.label(
                        egui::RichText::new(
                            "Drag and drop photo, signature, or document to begin.",
                        )
                        .color(palette.text_muted),
                    );
                    if ui.button("Select File").clicked() {
                        on_pick_file();
                    }
                }
            });

            if !self.exam_status.is_empty() {
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new(&self.exam_status)
                        .size(13.0)
                        .color(PANTONE_GOLD),
                );
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
        mut on_convert_batch: impl FnMut(Vec<PathBuf>, String),
    ) {
        ui.label(
            egui::RichText::new("Archive Suite & Compression")
                .size(12.0)
                .strong()
                .color(palette.text_primary),
        );
        ui.label(
            egui::RichText::new(
                "Zip-Slip Traversal Protection | Multi-Threaded Deflate / Zstandard",
            )
            .size(13.0)
            .color(palette.text_secondary),
        );
        ui.add_space(14.0);
        Self::render_tab_drop_banner(ui, palette, "Release to pack as ZIP or safe-extract");

        if let Some(ref path) = current_file {
            ui.label(format!("Target Input: {}", path.display()));
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                if ui
                    .button(egui::RichText::new("Pack to ZIP").size(13.0))
                    .clicked()
                {
                    on_convert_batch(current_file.iter().cloned().collect(), "ZIP".to_string());
                }
                if ui
                    .button(egui::RichText::new("Pack to TAR.GZ").size(13.0))
                    .clicked()
                {
                    on_convert_batch(current_file.iter().cloned().collect(), "TAR.GZ".to_string());
                }
                if ui
                    .button(egui::RichText::new("Pack to TAR.ZST").size(13.0))
                    .clicked()
                {
                    on_convert_batch(
                        current_file.iter().cloned().collect(),
                        "TAR.ZST".to_string(),
                    );
                }
                if ui
                    .button(
                        egui::RichText::new("Safe Extract")
                            .size(13.0)
                            .color(palette.accent),
                    )
                    .clicked()
                {
                    on_convert_batch(
                        current_file.iter().cloned().collect(),
                        "EXTRACT".to_string(),
                    );
                }
            });
        } else {
            ui.label(
                egui::RichText::new("Select a file or folder to compress or extract.")
                    .color(palette.text_muted),
            );
            if ui.button("Select Input").clicked() {
                on_pick_file();
            }
        }
    }

    /// Tab 4: Hardware & AI Telemetry
    fn render_telemetry_tab(&mut self, ui: &mut Ui, palette: &ThemePalette) {
        ui.label(
            egui::RichText::new("Hardware Acceleration & Neural Telemetry")
                .size(12.0)
                .strong()
                .color(palette.text_primary),
        );
        ui.label(
            egui::RichText::new(
                "GPU Encoder Pipelines | Windows DirectML | Multi-Core Concurrency",
            )
            .size(13.0)
            .color(palette.text_secondary),
        );
        ui.add_space(14.0);

        // Hardware telemetry is probed once on a plain background thread
        // and cached on the view (W-7 Batch B); Refresh re-runs detection.
        let cache = match (self.telemetry_hw.clone(), self.telemetry_specs.clone()) {
            (Some(hw), Some(specs)) => (hw, specs),
            _ => {
                ui.label(
                    egui::RichText::new("Detecting hardware capabilities...")
                        .size(13.0)
                        .color(palette.text_secondary),
                );
                return;
            }
        };
        let (hw, specs) = cache;
        let suggestions = crate::hwaccel::get_system_auto_suggestions(&specs);

        let card_frame = egui::Frame::none()
            .fill(palette.card_bg)
            .stroke(Stroke::new(1.0_f32, palette.border))
            .rounding(Rounding::same(10.0))
            .inner_margin(egui::Margin::same(16.0));

        card_frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Primary GPU:")
                        .size(13.5)
                        .strong()
                        .color(palette.accent),
                );
                ui.label(format!(
                    "{} (H.264: {} | HEVC: {})",
                    specs.gpu_name, hw.h264_encoder, hw.hevc_encoder
                ));
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Physical Memory:")
                        .size(13.5)
                        .strong()
                        .color(palette.accent),
                );
                ui.label(format!(
                    "{:.1} GB Installed Physical RAM",
                    specs.total_ram_gb
                ));
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("NPU AI Accelerator:")
                        .size(13.5)
                        .strong()
                        .color(palette.accent),
                );
                ui.label(format!("{} (Active)", hw.npu_name));
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("CPU Concurrency:")
                        .size(13.5)
                        .strong()
                        .color(palette.accent),
                );
                ui.label(format!(
                    "{} Logical Cores (Optimal Concurrency: {} threads)",
                    specs.cpu_cores, specs.recommended_concurrency
                ));
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("SIMD Vector Units:")
                        .size(13.5)
                        .strong()
                        .color(palette.accent),
                );
                ui.label(format!(
                    "AVX2: {} | NEON: {}",
                    if hw.supports_avx2_simd {
                        "Active"
                    } else {
                        "Off"
                    },
                    if hw.supports_neon_simd {
                        "Active"
                    } else {
                        "Off"
                    }
                ));
            });
        });

        ui.add_space(16.0);
        ui.label(
            egui::RichText::new("Auto-Aware System Optimization & Recommendations")
                .size(14.0)
                .strong()
                .color(palette.text_primary),
        );
        ui.label(
            egui::RichText::new(
                "Derived automatically from physical CPU cores, GPU silicon, and RAM buffer",
            )
            .size(12.5)
            .color(palette.text_secondary),
        );
        ui.add_space(8.0);

        for item in suggestions {
            let item_frame = egui::Frame::none()
                .fill(palette.sidebar_bg)
                .stroke(Stroke::new(1.0_f32, palette.border))
                .rounding(Rounding::same(8.0))
                .inner_margin(egui::Margin::symmetric(14.0, 10.0));

            item_frame.show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("[{}]", item.badge))
                            .size(11.0)
                            .strong()
                            .color(palette.accent),
                    );
                    ui.label(
                        egui::RichText::new(item.title)
                            .size(13.0)
                            .strong()
                            .color(palette.text_primary),
                    );
                });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("-> Recommendation:")
                            .size(12.0)
                            .color(palette.accent),
                    );
                    ui.label(
                        egui::RichText::new(&item.recommendation)
                            .size(12.5)
                            .strong()
                            .color(palette.text_primary),
                    );
                });
                ui.add_space(2.0);
                ui.label(
                    egui::RichText::new(&item.rationale)
                        .size(11.5)
                        .color(palette.text_muted),
                );
            });
            ui.add_space(6.0);
        }

        ui.add_space(12.0);
        if ui
            .button("Refresh Detection")
            .on_hover_text("Re-probe GPU encoders, NPU engines, and CPU topology")
            .clicked()
        {
            self.telemetry_hw = None;
            self.telemetry_specs = None;
            self.telemetry_detecting = true;
            crate::core::tool_console::detect_telemetry_async(self.tool_console.sender());
        }
    }

    /// Tab 5: Preferences & Certified Pantone Swatches & Themes
    fn render_preferences_tab(
        &mut self,
        ui: &mut Ui,
        palette: &ThemePalette,
        current_theme: &mut ThemeStyle,
        _settings: &mut QuickSettingsState,
        config: &mut MossambiConfig,
    ) {
        // Single source of truth: PreferencesView must render against the LIVE
        // app config, not the construction-time snapshot stored in prefs_view.cfg.
        // Swap the live config in for the frame so every self.cfg mutation below
        // lands in studio_app.config, then swap the mutated config back out. The
        // stale snapshot is discarded in both directions. Safe because
        // PreferencesView::render has no early returns (verified 2026-11-10).
        std::mem::swap(&mut self.prefs_view.cfg, config);
        let run_ctx = {
            let selected = self.explorer_view.selected_paths();
            (selected.first().cloned(), selected)
        };
        self.prefs_view.render(
            ui,
            palette,
            current_theme,
            _settings,
            &mut self.script_console,
            run_ctx,
        );
        std::mem::swap(&mut self.prefs_view.cfg, config);
        self.explorer_view.view_mode = match config.default_explorer_layout {
            2 => crate::gui::explorer_view::ViewMode::Grid,
            3 => crate::gui::explorer_view::ViewMode::Columns,
            _ => crate::gui::explorer_view::ViewMode::Details,
        };
        self.explorer_view.row_density = config.row_density;
        self.explorer_view.calculate_checksums = config.calculate_checksums;
    }

    fn render_quality_inspector_tab(
        &mut self,
        ui: &mut Ui,
        palette: &ThemePalette,
        current_file: &Option<PathBuf>,
        mut on_pick_file: impl FnMut(),
    ) {
        ui.label(
            egui::RichText::new("Universal File Quality, Integrity & Compatibility Inspector")
                .size(12.0)
                .strong()
                .color(palette.text_primary),
        );
        ui.label(
            egui::RichText::new("Container Integrity | Structural EOF Markers | Pure-Rust Parser | Compatibility Matrix | Speed Benchmark")
                .size(13.0)
                .color(palette.text_secondary),
        );
        ui.add_space(10.0);
        Self::render_tab_drop_banner(ui, palette, "Release to inspect file quality and integrity");

        // Active File Action Banner
        let card_frame = egui::Frame::none()
            .fill(palette.card_bg)
            .stroke(Stroke::new(1.0_f32, palette.border))
            .rounding(Rounding::same(10.0))
            .inner_margin(egui::Margin::same(12.0));

        card_frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                if let Some(ref path) = current_file {
                    let (mic_r, _) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::hover());
                    crate::core::icon_packs::paint_icon(
                        ui,
                        self.explorer_view.icon_pack,
                        IconGlyph::Search,
                        mic_r,
                        [palette.accent.r(), palette.accent.g(), palette.accent.b()],
                    );
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new(
                                path.file_name().unwrap_or_default().to_string_lossy(),
                            )
                            .size(13.5)
                            .strong()
                            .color(palette.text_primary),
                        );
                        ui.label(
                            egui::RichText::new(path.display().to_string())
                                .size(12.0)
                                .color(palette.text_muted),
                        );
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button(
                                egui::RichText::new("Benchmark Speed")
                                    .size(13.0)
                                    .strong()
                                    .color(Color32::BLACK),
                            )
                            .on_hover_text(
                                "Benchmark conversion speed against Online and CLI converters",
                            )
                            .clicked()
                            && !self.bench_busy
                        {
                            self.bench_busy = true;
                            self.inspection_status = "Benchmarking in background...".to_string();
                            crate::core::tool_console::benchmark_file_async(
                                self.tool_console.sender(),
                                path.clone(),
                            );
                        }

                        let inspect_btn = ui.add(
                            egui::Button::new(
                                egui::RichText::new("Inspect File")
                                    .size(13.0)
                                    .strong()
                                    .color(Color32::WHITE),
                            )
                            .fill(palette.accent)
                            .rounding(Rounding::same(8.0)),
                        );
                        if inspect_btn.clicked() && !self.inspection_busy {
                            self.inspection_busy = true;
                            self.inspection_status = "Inspecting in background...".to_string();
                            crate::core::tool_console::inspect_file_async(
                                self.tool_console.sender(),
                                path.clone(),
                            );
                        }
                    });
                } else {
                    ui.label(egui::RichText::new("Dl").size(12.0));
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new("No File Selected")
                                .size(13.5)
                                .strong()
                                .color(palette.text_primary),
                        );
                        ui.label(
                            egui::RichText::new(
                                "Choose a file to inspect its container integrity and checksums.",
                            )
                            .size(12.0)
                            .color(palette.text_muted),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button(egui::RichText::new("Choose File").size(13.0))
                            .clicked()
                        {
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
                        crate::codec::QualityVerdict::Pass => {
                            ("PASS", Color32::from_rgb(39, 201, 63))
                        }
                        crate::codec::QualityVerdict::Warning => {
                            ("WARN", Color32::from_rgb(255, 189, 46))
                        }
                        crate::codec::QualityVerdict::Fail => {
                            ("FAIL", Color32::from_rgb(255, 95, 86))
                        }
                    };

                    ui.label(
                        egui::RichText::new(badge_text)
                            .size(16.0)
                            .strong()
                            .color(badge_color),
                    );
                    ui.separator();
                    ui.label(
                        egui::RichText::new(format!("Overall Score: {:.1}%", rep.overall_score))
                            .size(15.0)
                            .strong()
                            .color(palette.accent),
                    );
                    ui.separator();
                    ui.label(
                        egui::RichText::new(format!("Format: {}", rep.format))
                            .size(13.5)
                            .strong()
                            .color(palette.text_primary),
                    );
                    ui.label(
                        egui::RichText::new(format!("Size: {:.2} KB", rep.file_size_kb))
                            .size(13.0)
                            .color(palette.text_secondary),
                    );
                });

                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(&rep.summary_headline)
                        .size(13.0)
                        .color(palette.text_secondary),
                );

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("SHA-256:")
                            .size(12.0)
                            .color(palette.text_muted),
                    );
                    ui.label(
                        egui::RichText::new(&rep.sha256)
                            .size(12.0)
                            .monospace()
                            .color(palette.text_secondary),
                    );
                    ui.separator();
                    ui.label(
                        egui::RichText::new("CRC-32:")
                            .size(12.0)
                            .color(palette.text_muted),
                    );
                    ui.label(
                        egui::RichText::new(&rep.crc32)
                            .size(12.0)
                            .monospace()
                            .color(palette.text_secondary),
                    );
                });
            });

            ui.add_space(8.0);

            // 4-Dimension Breakdown (2x2 Grid)
            ui.columns(2, |cols| {
                card_frame.show(&mut cols[0], |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("1. Container Integrity")
                                .size(13.5)
                                .strong()
                                .color(palette.text_primary),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(format!(
                                    "{:.1}%",
                                    rep.integrity.structural_integrity_score
                                ))
                                .size(13.0)
                                .strong()
                                .color(
                                    if rep.integrity.corruption_detected {
                                        Color32::RED
                                    } else {
                                        Color32::GREEN
                                    },
                                ),
                            );
                        });
                    });
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(format!(
                            "Signature: {}",
                            rep.integrity.magic_bytes_description
                        ))
                        .size(12.5)
                        .color(palette.text_secondary),
                    );
                    ui.label(
                        egui::RichText::new(format!(
                            "EOF Check: {}",
                            rep.integrity.eof_description
                        ))
                        .size(12.5)
                        .color(palette.text_secondary),
                    );
                    ui.label(
                        egui::RichText::new(format!(
                            "Corruption Detected: {}",
                            if rep.integrity.corruption_detected {
                                "YES (Structure Damaged)"
                            } else {
                                "No (Bit-Exact Valid)"
                            }
                        ))
                        .size(12.5)
                        .color(if rep.integrity.corruption_detected {
                            Color32::RED
                        } else {
                            palette.text_muted
                        }),
                    );
                });

                card_frame.show(&mut cols[1], |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("2. Parser & Decoder")
                                .size(13.5)
                                .strong()
                                .color(palette.text_primary),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(format!(
                                    "{:.1}%",
                                    rep.parseability.parseability_score
                                ))
                                .size(13.0)
                                .strong()
                                .color(
                                    if rep.parseability.can_parse {
                                        Color32::GREEN
                                    } else {
                                        Color32::RED
                                    },
                                ),
                            );
                        });
                    });
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(format!("Parser: {}", rep.parseability.parser_used))
                            .size(12.5)
                            .color(palette.text_secondary),
                    );
                    ui.label(
                        egui::RichText::new(format!(
                            "Decoded: {}",
                            rep.parseability.decoded_payload_summary
                        ))
                        .size(12.5)
                        .color(palette.text_secondary),
                    );
                    if !rep.parseability.parse_warnings.is_empty() {
                        ui.label(
                            egui::RichText::new(format!(
                                "Warnings: {}",
                                rep.parseability.parse_warnings.join(", ")
                            ))
                            .size(12.0)
                            .color(Color32::from_rgb(255, 189, 46)),
                        );
                    } else {
                        ui.label(
                            egui::RichText::new("Zero decode anomalies detected")
                                .size(12.0)
                                .color(palette.text_muted),
                        );
                    }
                });
            });

            ui.add_space(6.0);

            ui.columns(2, |cols| {
                card_frame.show(&mut cols[0], |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("3. Compatibility Matrix")
                                .size(13.5)
                                .strong()
                                .color(palette.text_primary),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(format!(
                                    "{:.1}%",
                                    rep.compatibility.compatibility_score
                                ))
                                .size(13.0)
                                .strong()
                                .color(Color32::GREEN),
                            );
                        });
                    });
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(format!(
                            "Web Browsers: {:.0}% | Mobile HW: {:.0}% | Archival: {:.0}%",
                            rep.compatibility.web_compatibility_score,
                            rep.compatibility.mobile_compatibility_score,
                            rep.compatibility.archival_compatibility_score
                        ))
                        .size(12.5)
                        .color(palette.text_secondary),
                    );
                    ui.label(
                        egui::RichText::new(format!(
                            "Standards: {}",
                            rep.compatibility.standards_compliance.join(" | ")
                        ))
                        .size(12.0)
                        .color(palette.text_muted),
                    );
                });

                card_frame.show(&mut cols[1], |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("4. Quality & Fidelity")
                                .size(13.5)
                                .strong()
                                .color(palette.text_primary),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(format!("{:.1}%", rep.quality.quality_score))
                                    .size(13.0)
                                    .strong()
                                    .color(Color32::GREEN),
                            );
                        });
                    });
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(format!("Rating: {}", rep.quality.fidelity_rating))
                            .size(12.5)
                            .color(palette.text_secondary),
                    );
                    ui.label(
                        egui::RichText::new(format!(
                            "Depth/Resolution: {}",
                            rep.quality.color_or_sample_depth
                        ))
                        .size(12.5)
                        .color(palette.text_secondary),
                    );
                    ui.label(
                        egui::RichText::new(format!(
                            "Clarity Heuristic: {:.1}%",
                            rep.quality.edge_clarity_score
                        ))
                        .size(12.0)
                        .color(palette.text_muted),
                    );
                });
            });

            // Benchmark Comparison Section
            if let Some(ref bench) = self.bench_report {
                ui.add_space(8.0);
                card_frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Performance Benchmark Results")
                                .size(14.0)
                                .strong()
                                .color(palette.text_primary),
                        );
                        ui.separator();
                        ui.label(
                            egui::RichText::new(format!(
                                "Duration: {:.2} ms ({} us)",
                                bench.duration_ms, bench.duration_micros
                            ))
                            .size(13.0)
                            .strong()
                            .color(palette.accent),
                        );
                        ui.separator();
                        ui.label(
                            egui::RichText::new(format!(
                                "Throughput: {:.1} MB/s",
                                bench.throughput_mb_per_sec
                            ))
                            .size(13.0)
                            .strong()
                            .color(Color32::GREEN),
                        );
                        ui.separator();
                        ui.label(
                            egui::RichText::new(if bench.hardware_accelerated {
                                "HW-ACCEL ACTIVE"
                            } else {
                                "CPU PIPELINE"
                            })
                            .size(12.5)
                            .color(PANTONE_GOLD),
                        );
                    });

                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new("Comparison Against Industry Converters:")
                            .size(13.0)
                            .strong()
                            .color(palette.text_secondary),
                    );

                    for comp in &bench.comparisons {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(format!("- {:<24}", comp.name))
                                    .size(12.5)
                                    .strong()
                                    .color(palette.text_primary),
                            );
                            ui.label(
                                egui::RichText::new(format!("({:<12})", comp.category))
                                    .size(12.0)
                                    .color(palette.text_muted),
                            );
                            ui.label(
                                egui::RichText::new(format!(
                                    "~{:.1} ms",
                                    comp.estimated_duration_ms
                                ))
                                .size(12.5)
                                .color(palette.text_secondary),
                            );
                            ui.label(
                                egui::RichText::new(format!(
                                    "-> {:.1}x FASTER",
                                    comp.speedup_factor
                                ))
                                .size(12.5)
                                .strong()
                                .color(Color32::GREEN),
                            );
                            ui.label(
                                egui::RichText::new(format!("- {}", comp.notes))
                                    .size(12.0)
                                    .color(palette.text_muted),
                            );
                        });
                    }

                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(&bench.summary)
                            .size(12.5)
                            .color(palette.text_primary),
                    );
                });
            }
        }
    }
}
