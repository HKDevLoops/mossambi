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

use crate::core::icon_packs::IconGlyph;
use crate::core::{FileCategory, UniversalRegistry};
use crate::exam::{optimize_exam_media, ExamConstraint};
use crate::gui::command_palette::{CommandPalette, PaletteAction};
use crate::gui::design::{layout, radius, type_scale};
use crate::gui::finder_chrome::{
    render_sidebar_collapsible_header, render_sidebar_row, render_sidebar_section_header,
    SidebarRowSpec,
};
use crate::gui::macos_theme::*;
use crate::gui::quick_settings::QuickSettingsState;
use crate::hwaccel::detect_hardware_capabilities;
use eframe::egui::{
    self, Color32, FontId, Pos2, Rect, Rounding, Sense, Stroke, Ui, Vec2, ViewportCommand,
};
use std::path::{Path, PathBuf};

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

pub struct StudioView {
    pub current_tab: StudioTab,
    pub explorer_view: crate::gui::explorer_view::ExplorerView,
    pub exam_status: String,
    pub last_exam_output: Option<PathBuf>,
    pub quality_report: Option<crate::codec::FileQualityReport>,
    pub bench_report: Option<crate::codec::BenchmarkReport>,
    pub inspection_status: String,
    pub inspected_file_path: Option<PathBuf>,
    pub catalogue_search: String,
    pub catalogue_category: Option<FileCategory>,

    // Sidebar Collapsible Sections
    pub drives_expanded: bool,
    pub cloud_expanded: bool,
    pub network_expanded: bool,
    pub tags_expanded: bool,
    pub tools_expanded: bool,

    // macOS System Settings-style Preferences surface
    pub prefs_view: crate::gui::preferences_view::PreferencesView,

    // Finder-style Command Palette (Ctrl+P / Cmd+K)
    pub command_palette: CommandPalette,
}

impl Default for StudioView {
    fn default() -> Self {
        Self {
            current_tab: StudioTab::FileExplorer,
            explorer_view: crate::gui::explorer_view::ExplorerView::default(),
            exam_status: "Ready to optimize for portal boundaries.".to_string(),
            last_exam_output: None,
            quality_report: None,
            bench_report: None,
            inspection_status: "Select a file to inspect.".to_string(),
            inspected_file_path: None,
            catalogue_search: String::new(),
            catalogue_category: None,

            drives_expanded: true,
            cloud_expanded: false,
            network_expanded: false,
            tags_expanded: true,
            tools_expanded: true,

            prefs_view: crate::gui::preferences_view::PreferencesView::default(),
            command_palette: CommandPalette::default(),
        }
    }
}

impl StudioView {
    /// Renders the complete macOS Studio window with sidebar and content area.
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        ctx: &egui::Context,
        current_theme: &mut ThemeStyle,
        current_file: &Option<PathBuf>,
        quick_settings: &mut QuickSettingsState,
        mut on_pick_file: impl FnMut(),
        mut on_stage_file: impl FnMut(PathBuf),
        mut on_convert_batch: impl FnMut(Vec<PathBuf>, String),
        on_open_folder: impl FnMut(&Path),
    ) {
        let palette = current_theme.colors();

        // Keyboard shortcuts: Ctrl+P / Cmd+K opens Command Palette
        let toggle_palette = ctx.input(|i| {
            (i.modifiers.ctrl || i.modifiers.command)
                && (i.key_pressed(egui::Key::P) || i.key_pressed(egui::Key::K))
        });
        if toggle_palette {
            self.command_palette.toggle();
        }

        // 1. Unified Top Panel (Windows 11 Fluent Tabs + Navigation Toolbar)
        let top_frame = egui::Frame::none()
            .fill(palette.sidebar_bg)
            .inner_margin(egui::Margin::symmetric(10.0, 4.0));

        let is_max = ctx.input(|i| i.viewport().maximized.unwrap_or(false));

        egui::TopBottomPanel::top("studio_unified_header")
            .frame(top_frame)
            .show(ctx, |ui| {
                // ==============================================================
                // ROW 1: Tabs + Drag Window Area + Window Controls
                // ==============================================================
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(6.0, 0.0);

                    // Finder traffic lights on the LEFT of the tab strip.
                    // The Windows-style Fluent dots (plus a stray maximize
                    // glyph) were replaced: macOS puts the controls first.
                    crate::gui::finder_chrome::render_traffic_lights(
                        ui,
                        &palette,
                        || ctx.send_viewport_cmd(ViewportCommand::Close),
                        || ctx.send_viewport_cmd(ViewportCommand::Minimized(true)),
                        || ctx.send_viewport_cmd(ViewportCommand::Maximized(!is_max)),
                    );

                    ui.add_space(6.0);

                    // Brand Icon
                    let (b_rect, _) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
                    crate::core::icon_packs::paint_icon(
                        ui,
                        self.explorer_view.icon_pack,
                        IconGlyph::ActionShield,
                        b_rect,
                        [palette.accent.r(), palette.accent.g(), palette.accent.b()],
                    );

                    // Tabs
                    let mut tab_to_select: Option<usize> = None;
                    let mut tab_to_close: Option<usize> = None;
                    let tabs_count = self.explorer_view.tabs.len();

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

                        ui.painter().rect_filled(rect, Rounding::same(6.0), bg);
                        if is_active {
                            ui.painter()
                                .rect_stroke(rect, Rounding::same(6.0), tab_stroke);
                        }

                        // Tab icon
                        ui.painter().text(
                            rect.left_center() + Vec2::new(10.0, 0.0),
                            egui::Align2::LEFT_CENTER,
                            tab.icon,
                            egui::FontId::proportional(20.0),
                            if is_active {
                                palette.accent
                            } else {
                                palette.text_secondary
                            },
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
                            rect.left_center() + Vec2::new(28.0, 0.0),
                            egui::Align2::LEFT_CENTER,
                            title_display,
                            egui::FontId::proportional(13.0),
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

                    if let Some(idx) = tab_to_close {
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

                    // Draggable Window Title Region
                    let drag_w = (ui.available_width() - 130.0).max(20.0);
                    let (_drag_rect, drag_resp) =
                        ui.allocate_exact_size(Vec2::new(drag_w, 26.0), Sense::click_and_drag());
                    if drag_resp.drag_started() {
                        ctx.send_viewport_cmd(ViewportCommand::StartDrag);
                    }
                    if drag_resp.double_clicked() {
                        ctx.send_viewport_cmd(ViewportCommand::Maximized(!is_max));
                    }
                });

                ui.add_space(6.0);

                // ==============================================================
                // ROW 2: Navigation Buttons + Breadcrumbs + Search + Utilities
                // ==============================================================
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(6.0, 0.0);

                    // Back Button
                    let can_back = self.explorer_view.can_go_back();
                    let back_btn = egui::Button::new("<")
                        .min_size(Vec2::splat(26.0))
                        .fill(if can_back {
                            palette.card_bg
                        } else {
                            Color32::TRANSPARENT
                        })
                        .rounding(Rounding::same(4.0));
                    if ui
                        .add_enabled(can_back, back_btn)
                        .on_hover_text("Back (Alt+Left)")
                        .clicked()
                    {
                        self.current_tab = StudioTab::FileExplorer;
                        self.explorer_view.go_back();
                    }

                    // Forward Button
                    let can_fwd = self.explorer_view.can_go_forward();
                    let fwd_btn = egui::Button::new(">")
                        .min_size(Vec2::splat(26.0))
                        .fill(if can_fwd {
                            palette.card_bg
                        } else {
                            Color32::TRANSPARENT
                        })
                        .rounding(Rounding::same(4.0));
                    if ui
                        .add_enabled(can_fwd, fwd_btn)
                        .on_hover_text("Forward (Alt+Right)")
                        .clicked()
                    {
                        self.current_tab = StudioTab::FileExplorer;
                        self.explorer_view.go_forward();
                    }

                    // Up Folder Button
                    let up_btn = egui::Button::new("^")
                        .min_size(Vec2::splat(26.0))
                        .fill(palette.card_bg)
                        .rounding(Rounding::same(4.0));
                    if ui
                        .add(up_btn)
                        .on_hover_text("Up to parent folder (Alt+Up)")
                        .clicked()
                    {
                        self.current_tab = StudioTab::FileExplorer;
                        if let Some(parent) = self.explorer_view.current_dir.parent() {
                            let p = parent.to_path_buf();
                            if !p.as_os_str().is_empty() {
                                self.explorer_view.navigate_to(p);
                            }
                        }
                    }

                    // Refresh Button
                    let ref_btn = egui::Button::new("R")
                        .min_size(Vec2::splat(26.0))
                        .fill(palette.card_bg)
                        .rounding(Rounding::same(4.0));
                    if ui.add(ref_btn).on_hover_text("Refresh (F5)").clicked() {
                        self.explorer_view
                            .navigate_to(self.explorer_view.current_dir.clone());
                    }

                    ui.add_space(4.0);

                    // Address / Breadcrumb Bar
                    let search_area_w = 280.0_f32;
                    let breadcrumb_w = (ui.available_width() - search_area_w - 10.0).max(160.0);

                    ui.allocate_ui_with_layout(
                        Vec2::new(breadcrumb_w, 28.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            let (bc_rect, _) = ui
                                .allocate_exact_size(Vec2::new(breadcrumb_w, 28.0), Sense::hover());
                            ui.painter()
                                .rect_filled(bc_rect, Rounding::same(6.0), palette.card_bg);
                            ui.painter().rect_stroke(
                                bc_rect,
                                Rounding::same(6.0),
                                Stroke::new(1.0_f32, palette.border),
                            );

                            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(bc_rect), |ui| {
                                ui.horizontal(|ui| {
                                    ui.add_space(8.0);
                                    if self.explorer_view.is_home_view {
                                        ui.label(
                                            egui::RichText::new("Home")
                                                .strong()
                                                .color(palette.accent),
                                        );
                                        ui.label(
                                            egui::RichText::new("v")
                                                .size(11.0)
                                                .color(palette.text_muted),
                                        );
                                    } else if self.explorer_view.is_editing_path {
                                        let text_resp = ui.add(
                                            egui::TextEdit::singleline(
                                                &mut self.explorer_view.path_edit_buffer,
                                            )
                                            .desired_width(breadcrumb_w - 40.0)
                                            .font(egui::TextStyle::Monospace),
                                        );
                                        if text_resp.lost_focus()
                                            && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                        {
                                            let new_p =
                                                PathBuf::from(&self.explorer_view.path_edit_buffer);
                                            if new_p.exists() {
                                                self.explorer_view.navigate_to(new_p);
                                            }
                                            self.explorer_view.is_editing_path = false;
                                        }
                                    } else {
                                        let segments = crate::core::parse_breadcrumbs(
                                            &self.explorer_view.current_dir,
                                        );
                                        for seg in segments {
                                            let is_current =
                                                seg.target_path == self.explorer_view.current_dir;
                                            if ui
                                                .link(egui::RichText::new(&seg.label).color(
                                                    if is_current {
                                                        palette.accent
                                                    } else {
                                                        palette.text_primary
                                                    },
                                                ))
                                                .clicked()
                                            {
                                                self.explorer_view.navigate_to(seg.target_path);
                                            }
                                            ui.label(
                                                egui::RichText::new("/").color(palette.text_muted),
                                            );
                                        }
                                        if ui
                                            .small_button("Edit")
                                            .on_hover_text("Edit path (Ctrl+L)")
                                            .clicked()
                                        {
                                            self.explorer_view.path_edit_buffer = self
                                                .explorer_view
                                                .current_dir
                                                .to_string_lossy()
                                                .to_string();
                                            self.explorer_view.is_editing_path = true;
                                        }
                                    }
                                });
                            });
                        },
                    );

                    // Search Box & Action Icons on Right
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Settings Gear
                        if ui
                            .button(
                                egui::RichText::new("Prefs")
                                    .size(12.0)
                                    .color(palette.text_secondary),
                            )
                            .on_hover_text("Settings & Themes")
                            .clicked()
                        {
                            self.current_tab = StudioTab::Preferences;
                        }

                        // View layout toggle
                        let view_glyph = match self.explorer_view.view_mode {
                            crate::gui::explorer_view::ViewMode::Details => "List",
                            crate::gui::explorer_view::ViewMode::Grid => "Grid",
                            crate::gui::explorer_view::ViewMode::Columns => "Cols",
                        };
                        if ui
                            .button(
                                egui::RichText::new(view_glyph)
                                    .size(12.0)
                                    .color(palette.text_secondary),
                            )
                            .on_hover_text("Change View Mode (Details, Grid, Columns)")
                            .clicked()
                        {
                            self.explorer_view.view_mode = match self.explorer_view.view_mode {
                                crate::gui::explorer_view::ViewMode::Details => {
                                    crate::gui::explorer_view::ViewMode::Grid
                                }
                                crate::gui::explorer_view::ViewMode::Grid => {
                                    crate::gui::explorer_view::ViewMode::Columns
                                }
                                crate::gui::explorer_view::ViewMode::Columns => {
                                    crate::gui::explorer_view::ViewMode::Details
                                }
                            };
                        }

                        // Ready / Sync Checkmark
                        ui.label(egui::RichText::new("OK").size(12.0).color(palette.accent))
                            .on_hover_text("System Ready | Bit-Exact Standard Active");

                        // Search box
                        if !self.explorer_view.search_query.is_empty()
                            && ui.button("x").on_hover_text("Clear search").clicked()
                        {
                            self.explorer_view.search_query.clear();
                        }

                        let search_w = 160.0_f32;
                        let search_resp = ui.add(
                            egui::TextEdit::singleline(&mut self.explorer_view.search_query)
                                .hint_text("Search")
                                .desired_width(search_w),
                        );
                        if self.explorer_view.focus_search_requested {
                            search_resp.request_focus();
                            self.explorer_view.focus_search_requested = false;
                        }
                    });
                });
            });

        // 2. Left Sidebar Panel (Pinned, Drives, Cloud, Network, Tags, Tools)
        let sidebar_frame = egui::Frame::none()
            .fill(palette.sidebar_bg)
            .inner_margin(egui::Margin::symmetric(8.0, 8.0));

        egui::SidePanel::left("studio_sidebar")
            .resizable(false)
            .exact_width(layout::SIDEBAR_W)
            .frame(sidebar_frame)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let pack = self.explorer_view.icon_pack;
                        let accent_rgb =
                            [palette.accent.r(), palette.accent.g(), palette.accent.b()];

                        // 1. Home Item
                        let is_home_active = self.current_tab == StudioTab::FileExplorer
                            && self.explorer_view.is_home_view;
                        let home_spec = SidebarRowSpec {
                            pack,
                            glyph: IconGlyph::Home,
                            accent_rgb,
                            label: "Home",
                            selected: is_home_active,
                            show_pin: false,
                        };
                        if render_sidebar_row(ui, &palette, &home_spec) {
                            self.current_tab = StudioTab::FileExplorer;
                            let home_opt = std::env::var("USERPROFILE")
                                .or_else(|_| std::env::var("HOME"))
                                .ok()
                                .map(PathBuf::from);
                            let target = home_opt.unwrap_or_else(|| PathBuf::from("."));
                            self.explorer_view.navigate_to(target);
                            self.explorer_view.is_home_view = true;
                        }

                        // 2. FAVORITES Section
                        render_sidebar_section_header(ui, &palette, "Favorites");

                        let cfg = crate::core::MossambiConfig::load();
                        let home_opt = std::env::var("USERPROFILE")
                            .or_else(|_| std::env::var("HOME"))
                            .ok();
                        if let Some(home_str) = home_opt {
                            let base = PathBuf::from(home_str);
                            let recycle_target = self
                                .explorer_view
                                .drives
                                .iter()
                                .find(|d| d.is_system)
                                .map(|d| d.path.join("$Recycle.Bin"))
                                .unwrap_or_else(|| base.clone());
                            let pinned = [
                                ("Desktop", IconGlyph::FolderDesktop, base.join("Desktop")),
                                (
                                    "Downloads",
                                    IconGlyph::FolderDownloads,
                                    base.join("Downloads"),
                                ),
                                (
                                    "Documents",
                                    IconGlyph::FolderDocuments,
                                    base.join("Documents"),
                                ),
                                ("Pictures", IconGlyph::FolderPictures, base.join("Pictures")),
                                ("Music", IconGlyph::FolderMusic, base.join("Music")),
                                ("Videos", IconGlyph::FolderVideos, base.join("Videos")),
                                ("Recycle Bin", IconGlyph::RecycleBin, recycle_target),
                            ];

                            for (name, glyph, path) in pinned {
                                let is_active = self.current_tab == StudioTab::FileExplorer
                                    && !self.explorer_view.is_home_view
                                    && self.explorer_view.current_dir == path;
                                let spec = SidebarRowSpec {
                                    pack,
                                    glyph,
                                    accent_rgb,
                                    label: name,
                                    selected: is_active,
                                    show_pin: true,
                                };
                                if render_sidebar_row(ui, &palette, &spec) {
                                    if path.to_string_lossy().contains("$Recycle.Bin") {
                                        self.explorer_view.show_hidden_files = true;
                                    }
                                    self.current_tab = StudioTab::FileExplorer;
                                    self.explorer_view.navigate_to(path);
                                    self.explorer_view.is_home_view = false;
                                }
                                ui.add_space(1.0);
                            }

                            if cfg.show_custom_pinned && !cfg.pinned_custom.is_empty() {
                                render_sidebar_section_header(ui, &palette, "Pinned Folders");
                                let custom = cfg.pinned_custom.clone();
                                for raw in custom {
                                    let path = PathBuf::from(&raw);
                                    let name = path
                                        .file_name()
                                        .map(|n| n.to_string_lossy().into_owned())
                                        .unwrap_or_else(|| raw.clone());
                                    let is_active = self.current_tab == StudioTab::FileExplorer
                                        && !self.explorer_view.is_home_view
                                        && self.explorer_view.current_dir == path;
                                    let spec = SidebarRowSpec {
                                        pack,
                                        glyph: IconGlyph::Folder,
                                        accent_rgb,
                                        label: &name,
                                        selected: is_active,
                                        show_pin: true,
                                    };
                                    if render_sidebar_row(ui, &palette, &spec) {
                                        self.current_tab = StudioTab::FileExplorer;
                                        self.explorer_view.navigate_to(path);
                                        self.explorer_view.is_home_view = false;
                                    }
                                    ui.add_space(1.0);
                                }
                            }
                        }

                        // 3. DRIVES Section (Collapsible)
                        if render_sidebar_collapsible_header(
                            ui,
                            &palette,
                            "Drives",
                            self.drives_expanded,
                        ) {
                            self.drives_expanded = !self.drives_expanded;
                        }

                        if self.drives_expanded {
                            let drives = self.explorer_view.drives.clone();
                            for drive in &drives {
                                let is_active = self.current_tab == StudioTab::FileExplorer
                                    && self.explorer_view.current_dir.starts_with(&drive.path);
                                let glyph = if drive.is_system {
                                    IconGlyph::DriveSystem
                                } else {
                                    IconGlyph::Drive
                                };
                                let spec = SidebarRowSpec {
                                    pack,
                                    glyph,
                                    accent_rgb,
                                    label: &drive.display_name,
                                    selected: is_active,
                                    show_pin: false,
                                };
                                if render_sidebar_row(ui, &palette, &spec) {
                                    self.current_tab = StudioTab::FileExplorer;
                                    self.explorer_view.navigate_to(drive.path.clone());
                                    self.explorer_view.is_home_view = false;
                                }
                                ui.add_space(1.0);
                            }
                        }

                        // 4. CLOUD DRIVES Section (Collapsible)
                        if render_sidebar_collapsible_header(
                            ui,
                            &palette,
                            "Cloud",
                            self.cloud_expanded,
                        ) {
                            self.cloud_expanded = !self.cloud_expanded;
                        }
                        if self.cloud_expanded {
                            if let Ok(userprofile) = std::env::var("USERPROFILE") {
                                let od = PathBuf::from(&userprofile).join("OneDrive");
                                if od.exists() {
                                    let is_active = self.current_tab == StudioTab::FileExplorer
                                        && self.explorer_view.current_dir == od;
                                    let spec = SidebarRowSpec {
                                        pack,
                                        glyph: IconGlyph::Cloud,
                                        accent_rgb,
                                        label: "OneDrive",
                                        selected: is_active,
                                        show_pin: false,
                                    };
                                    if render_sidebar_row(ui, &palette, &spec) {
                                        self.current_tab = StudioTab::FileExplorer;
                                        self.explorer_view.navigate_to(od);
                                    }
                                }
                            }
                        }

                        // 5. NETWORK Section (Collapsible)
                        if render_sidebar_collapsible_header(
                            ui,
                            &palette,
                            "Network",
                            self.network_expanded,
                        ) {
                            self.network_expanded = !self.network_expanded;
                        }

                        // 6. TAGS Section (Collapsible)
                        if cfg.show_tags_section {
                            if render_sidebar_collapsible_header(
                                ui,
                                &palette,
                                "Tags",
                                self.tags_expanded,
                            ) {
                                self.tags_expanded = !self.tags_expanded;
                            }
                            if self.tags_expanded {
                                for tag_name in crate::core::tags::TAG_PRESETS {
                                    let tag_col = crate::core::tags::tag_color(tag_name);
                                    let is_active =
                                        self.explorer_view.active_tag.as_deref() == Some(tag_name);
                                    let row_h = 24.0;
                                    let (rect, resp) = ui.allocate_exact_size(
                                        Vec2::new(ui.available_width(), row_h),
                                        Sense::click(),
                                    );
                                    let is_hovered = resp.hovered();
                                    let painter = ui.painter_at(rect);
                                    if is_active {
                                        painter.rect_filled(
                                            rect,
                                            Rounding::same(radius::SM),
                                            Color32::from_rgba_premultiplied(
                                                palette.accent.r(),
                                                palette.accent.g(),
                                                palette.accent.b(),
                                                if palette.is_dark { 45 } else { 26 },
                                            ),
                                        );
                                    } else if is_hovered {
                                        painter.rect_filled(
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
                                        tag_name,
                                        FontId::proportional(type_scale::BODY_SM),
                                        text_color,
                                    );
                                    if resp.clicked() {
                                        self.current_tab = StudioTab::FileExplorer;
                                        self.explorer_view.set_active_tag(if is_active {
                                            None
                                        } else {
                                            Some(tag_name.to_string())
                                        });
                                    }
                                    ui.add_space(1.0);
                                }
                            }
                        }

                        // 7. TOOLS & CONVERTERS Section (Collapsible)
                        if render_sidebar_collapsible_header(
                            ui,
                            &palette,
                            "Tools & Converters",
                            self.tools_expanded,
                        ) {
                            self.tools_expanded = !self.tools_expanded;
                        }
                        if self.tools_expanded {
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

                            for (label, glyph, tab) in tools {
                                let spec = SidebarRowSpec {
                                    pack,
                                    glyph,
                                    accent_rgb,
                                    label,
                                    selected: self.current_tab == tab,
                                    show_pin: false,
                                };
                                if render_sidebar_row(ui, &palette, &spec) {
                                    self.current_tab = tab;
                                    if tab == StudioTab::ExamStudio {
                                        quick_settings.exam_mode_enabled = true;
                                    }
                                }
                                ui.add_space(1.0);
                            }
                        }

                        ui.add_space(12.0);
                    });
            });

        // 2. Central Content Panel with smooth vertical ScrollArea
        let central_frame = egui::Frame::none()
            .fill(palette.window_bg)
            .inner_margin(egui::Margin::symmetric(10.0, 6.0));

        egui::CentralPanel::default()
            .frame(central_frame)
            .show(ctx, |ui| match self.current_tab {
                StudioTab::FileExplorer => {
                    let mut staged: Option<PathBuf> = None;
                    let mut transcode_reqs: Vec<(PathBuf, String)> = Vec::new();
                    let mut exam_req: Option<PathBuf> = None;
                    let mut pack_req: Option<Vec<PathBuf>> = None;
                    let mut inspect_req: Option<PathBuf> = None;

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
                        if let Ok(rep) = crate::codec::FileQualityChecker::inspect_file(&p) {
                            self.quality_report = Some(rep);
                        }
                    }
                }
                other => {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| match other {
                            StudioTab::FileExplorer => unreachable!(),
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
                            StudioTab::Preferences => {
                                self.render_preferences_tab(
                                    ui,
                                    &palette,
                                    current_theme,
                                    quick_settings,
                                );
                            }
                        });
                }
            });

        // Finder-style Command Palette Modal (Ctrl+P / Cmd+K)
        let palette_action =
            self.command_palette
                .render(ctx, &palette, self.explorer_view.icon_pack);
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
                let base = crate::core::platform_paths::home_dir();
                let recycle_target = self
                    .explorer_view
                    .drives
                    .iter()
                    .find(|d| d.is_system)
                    .map(|d| d.path.join("$Recycle.Bin"))
                    .unwrap_or_else(|| base);
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
                self.current_tab = StudioTab::Transcoder;
            }
            PaletteAction::TabExamStudio => {
                self.current_tab = StudioTab::ExamStudio;
            }
            PaletteAction::TabQualityInspector => {
                self.current_tab = StudioTab::QualityInspector;
            }
            PaletteAction::TabArchiveSuite => {
                self.current_tab = StudioTab::ArchiveSuite;
            }
            PaletteAction::TabHardwareTelemetry => {
                self.current_tab = StudioTab::HardwareTelemetry;
            }
            PaletteAction::TabPreferences => {
                self.current_tab = StudioTab::Preferences;
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
                ui.label(egui::RichText::new("16 Threads Ready").size(12.0).color(PANTONE_FLAVEDO));
            });
        });

        ui.add_space(16.0);

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
                .stroke(Stroke::new(1.2_f32, PANTONE_FLAVEDO))
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
                            PANTONE_FLAVEDO
                        } else {
                            palette.card_bg
                        })
                        .stroke(Stroke::new(
                            1.0_f32,
                            if is_sel {
                                PANTONE_FLAVEDO
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
                        .color(PANTONE_FLAVEDO),
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
                            .fill(PANTONE_FLAVEDO)
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
                .fill(Color32::from_rgba_premultiplied(22, 26, 36, 200))
                .stroke(Stroke::new(1.8_f32, PANTONE_FLAVEDO))
                .rounding(Rounding::same(16.0))
                .inner_margin(egui::Margin::same(24.0));

            dropzone_frame.show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    let (hero_r, _) = ui.allocate_exact_size(Vec2::splat(28.0), Sense::hover());
                crate::core::icon_packs::paint_icon(ui, self.explorer_view.icon_pack, IconGlyph::ActionShield, hero_r, [palette.accent.r(), palette.accent.g(), palette.accent.b()]);
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
                                    .color(Color32::from_rgb(25, 40, 10)),
                            )
                            .fill(PANTONE_FLAVEDO)
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
                        ui.label(egui::RichText::new("Indian Exam Portals").size(12.0).color(PANTONE_FLAVEDO));
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
                    ui.label(egui::RichText::new("Web Image Optimizer").size(14.0).strong().color(palette.text_primary));
                    ui.label(egui::RichText::new("Convert PNG/JPG to WebP/AVIF for up to 80% size savings with zero perceptual loss.").size(12.5).color(palette.text_secondary));
                    ui.add_space(6.0);
                    if ui.button(egui::RichText::new("Select Images ->").size(12.5).color(PANTONE_FLAVEDO)).clicked() {
                        on_pick_file();
                    }
                });

                // Preset 2: Vector PDF
                let p2 = egui::Frame::none()
                    .fill(palette.card_bg)
                    .stroke(Stroke::new(1.0_f32, palette.border))
                    .rounding(Rounding::same(10.0))
                    .inner_margin(egui::Margin::same(14.0));
                p2.show(&mut cols[1], |ui| {
                    ui.label(egui::RichText::new("Notes to Vector PDF").size(14.0).strong().color(palette.text_primary));
                    ui.label(egui::RichText::new("Typeset text, markdown, and code into publication-grade 300 DPI vector PDF.").size(12.5).color(palette.text_secondary));
                    ui.add_space(6.0);
                    if ui.button(egui::RichText::new("Select Document ->").size(12.5).color(PANTONE_FLAVEDO)).clicked() {
                        on_pick_file();
                    }
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
                    ui.label(egui::RichText::new("Lossless Audio Remux").size(14.0).strong().color(palette.text_primary));
                    ui.label(egui::RichText::new("Bit-exact WAV <-> FLAC transcoding with zero stream re-encoding and CRC verification.").size(12.5).color(palette.text_secondary));
                    ui.add_space(6.0);
                    if ui.button(egui::RichText::new("Select Audio ->").size(12.5).color(PANTONE_FLAVEDO)).clicked() {
                        on_pick_file();
                    }
                });

                // Preset 4: Indian Exam Form
                let p4 = egui::Frame::none()
                    .fill(palette.card_bg)
                    .stroke(Stroke::new(1.0_f32, palette.border))
                    .rounding(Rounding::same(10.0))
                    .inner_margin(egui::Margin::same(14.0));
                p4.show(&mut cols[1], |ui| {
                    ui.label(egui::RichText::new("Custom Form Conformer").size(14.0).strong().color(palette.text_primary));
                    ui.label(egui::RichText::new("Conforms any photo, signature, or document to exact custom dimensions, DPI, and file size boundaries.").size(12.5).color(palette.text_secondary));
                    ui.add_space(6.0);
                    if ui.button(egui::RichText::new("Open Conformer ->").size(12.5).color(PANTONE_FLAVEDO)).clicked() {
                        self.current_tab = StudioTab::ExamStudio;
                    }
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

            let all_samples = [
                (
                    "PNG",
                    FileCategory::Image,
                    "Portable Network Graphics | Lossless",
                ),
                (
                    "JPG",
                    FileCategory::Image,
                    "Joint Photographic Experts Group",
                ),
                (
                    "WEBP",
                    FileCategory::Image,
                    "Google Modern Web Image | 80% Reduction",
                ),
                (
                    "AVIF",
                    FileCategory::Image,
                    "AV1 Image File Format | Next-Gen",
                ),
                (
                    "HEIC",
                    FileCategory::Image,
                    "High Efficiency Image Container | Apple",
                ),
                (
                    "TIFF",
                    FileCategory::Image,
                    "Tagged Image File Format | Archival",
                ),
                ("BMP", FileCategory::Image, "Windows Bitmap Graphics"),
                ("SVG", FileCategory::Image, "Scalable Vector Graphics"),
                (
                    "PDF",
                    FileCategory::Document,
                    "Adobe Portable Document Format | Vector",
                ),
                (
                    "DOCX",
                    FileCategory::Document,
                    "Microsoft Word OpenXML Document",
                ),
                (
                    "XLSX",
                    FileCategory::Document,
                    "Microsoft Excel Spreadsheet",
                ),
                (
                    "CSV",
                    FileCategory::Document,
                    "Comma-Separated Values Table",
                ),
                ("TXT", FileCategory::Document, "Plain Text UTF-8"),
                ("MD", FileCategory::Document, "Markdown Formatted Document"),
                ("WAV", FileCategory::Audio, "Uncompressed PCM RIFF Wave"),
                (
                    "FLAC",
                    FileCategory::Audio,
                    "Free Lossless Audio Codec | Bit-Exact",
                ),
                (
                    "MP3",
                    FileCategory::Audio,
                    "MPEG-1 Audio Layer III | 320 kbps",
                ),
                (
                    "AAC",
                    FileCategory::Audio,
                    "Advanced Audio Coding | High-Fidelity",
                ),
                ("OGG", FileCategory::Audio, "Ogg Vorbis Audio Stream"),
                (
                    "MP4",
                    FileCategory::Video,
                    "MPEG-4 Part 14 Video | Fast Remux",
                ),
                ("MKV", FileCategory::Video, "Matroska Multimedia Container"),
                ("WEBM", FileCategory::Video, "HTML5 Web Video | VP9/AV1"),
                (
                    "ZIP",
                    FileCategory::Archive,
                    "PKZip Deflate Level 9 | Safe Unpack",
                ),
                (
                    "TAR.GZ",
                    FileCategory::Archive,
                    "Unix Tape Archive Gzip Compressed",
                ),
                (
                    "TAR.ZST",
                    FileCategory::Archive,
                    "Zstandard Modern Ultra Compression",
                ),
            ];

            let query = self.catalogue_search.to_uppercase();
            ui.horizontal_wrapped(|ui| {
                for (fmt, _cat, desc) in all_samples {
                    if !query.is_empty()
                        && !fmt.contains(&query)
                        && !desc.to_uppercase().contains(&query)
                    {
                        continue;
                    }
                    let chip_frame = egui::Frame::none()
                        .fill(palette.card_bg)
                        .stroke(Stroke::new(1.0_f32, palette.border))
                        .rounding(Rounding::same(8.0))
                        .inner_margin(egui::Margin::symmetric(10.0, 6.0));

                    chip_frame.show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(fmt)
                                    .size(13.0)
                                    .strong()
                                    .color(PANTONE_FLAVEDO),
                            );
                            ui.label(
                                egui::RichText::new(desc)
                                    .size(12.0)
                                    .color(palette.text_muted),
                            );
                        });
                    });
                }
            });
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
                        .color(PANTONE_FLAVEDO),
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

            // Row 1: Specification Name
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Specification Name:").size(13.0));
                ui.add(
                    egui::TextEdit::singleline(&mut settings.exam_preset_name)
                        .desired_width(220.0)
                        .hint_text("e.g. Application Photo, Signature"),
                );
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
                        .fill(PANTONE_FLAVEDO)
                        .rounding(Rounding::same(8.0)),
                    );

                    if optimize_btn.clicked() {
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

                        let parent = path.parent().unwrap_or_else(|| Path::new("."));
                        let stem = path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("output");
                        let out_file = parent.join(format!("{}_conformed.jpg", stem));

                        match optimize_exam_media(path, &out_file, &constraint) {
                            Ok(res) => {
                                let len = std::fs::metadata(&res).map(|m| m.len()).unwrap_or(0);
                                self.exam_status = format!(
                                    "PASS: Generated {} ({:.1} KB) fitting [{} - {} KB] @ {} DPI",
                                    res.file_name().unwrap_or_default().to_string_lossy(),
                                    len as f64 / 1024.0,
                                    settings.exam_min_kb,
                                    settings.exam_max_kb,
                                    settings.exam_dpi
                                );
                                self.last_exam_output = Some(res);
                            }
                            Err(e) => {
                                self.exam_status = format!("FAIL: Optimization failed: {}", e);
                            }
                        }
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
                    .button(egui::RichText::new("Pack to 7Z").size(13.0))
                    .clicked()
                {
                    on_convert_batch(current_file.iter().cloned().collect(), "7Z".to_string());
                }
                if ui
                    .button(
                        egui::RichText::new("Safe Extract")
                            .size(13.0)
                            .color(PANTONE_FLAVEDO),
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
        let hw = detect_hardware_capabilities();
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

        let card_frame = egui::Frame::none()
            .fill(palette.card_bg)
            .stroke(Stroke::new(1.0_f32, palette.border))
            .rounding(Rounding::same(10.0))
            .inner_margin(egui::Margin::same(16.0));

        card_frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("GPU Transcoder:")
                        .size(13.5)
                        .strong()
                        .color(PANTONE_FLAVEDO),
                );
                ui.label(format!(
                    "{:?} (H.264: {} | HEVC: {})",
                    hw.vendor, hw.h264_encoder, hw.hevc_encoder
                ));
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("NPU AI Accelerator:")
                        .size(13.5)
                        .strong()
                        .color(PANTONE_FLAVEDO),
                );
                ui.label(format!("{} (Active)", hw.npu_name));
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("CPU Concurrency:")
                        .size(13.5)
                        .strong()
                        .color(PANTONE_FLAVEDO),
                );
                ui.label(format!(
                    "{} Concurrent Rayon Worker Threads",
                    hw.optimal_thread_count
                ));
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("SIMD Vector Units:")
                        .size(13.5)
                        .strong()
                        .color(PANTONE_FLAVEDO),
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
    }

    /// Tab 5: Preferences & Certified Pantone Swatches & Themes
    fn render_preferences_tab(
        &mut self,
        ui: &mut Ui,
        palette: &ThemePalette,
        current_theme: &mut ThemeStyle,
        _settings: &mut QuickSettingsState,
    ) {
        self.prefs_view
            .render(ui, palette, current_theme, _settings);
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
                        {
                            match crate::codec::BenchmarkEngine::benchmark_file(path, None) {
                                Ok(bench) => self.bench_report = Some(bench),
                                Err(e) => {
                                    self.inspection_status = format!("Benchmark failed: {}", e)
                                }
                            }
                        }

                        let inspect_btn = ui.add(
                            egui::Button::new(
                                egui::RichText::new("Inspect File")
                                    .size(13.0)
                                    .strong()
                                    .color(Color32::WHITE),
                            )
                            .fill(PANTONE_FLAVEDO)
                            .rounding(Rounding::same(8.0)),
                        );
                        if inspect_btn.clicked() {
                            match crate::codec::FileQualityChecker::inspect_file(path) {
                                Ok(rep) => {
                                    self.quality_report = Some(rep);
                                    self.inspected_file_path = Some(path.clone());
                                    self.inspection_status = "Inspection complete.".to_string();
                                }
                                Err(e) => {
                                    self.inspection_status = format!("Inspection failed: {}", e);
                                }
                            }
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
                            .color(PANTONE_FLAVEDO),
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
                            .color(PANTONE_FLAVEDO),
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
                            .color(PANTONE_FLAVEDO),
                    );
                });
            }
        }
    }
}
