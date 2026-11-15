//! Finder-Style Command Palette (Ctrl+P / Cmd+K)
//!
//! A fast, keyboard-first modal launcher for Mossambi actions, format conversions,
//! view modes, and navigation destinations. Pure ASCII.

use crate::core::icon_packs::{IconGlyph, IconPack};
use crate::core::scripting::ScriptLang;
use crate::gui::design::{radius, space, type_scale};
use crate::gui::macos_theme::ThemePalette;
use eframe::egui::{self, Color32, FontId, Key, Pos2, Rect, Rounding, Sense, Stroke, Vec2};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaletteAction {
    NavigateHome,
    NavigateDesktop,
    NavigateDownloads,
    NavigateDocuments,
    NavigatePictures,
    NavigateVideos,
    NavigateMusic,
    NavigateRecycleBin,
    NavigateTo(PathBuf),
    OpenFile(PathBuf),
    NewFolder,
    NewWindow,
    NewTab,
    CloseTab,
    Refresh,
    FocusSearch,
    ShowProperties,
    DeleteSelected,
    EmptyRecycleBin,
    Copy,
    Cut,
    Paste,
    ViewDetails,
    ViewGrid,
    ViewColumns,
    ToggleSplitView,
    ToggleInspectorPane,
    TabTranscoder,
    TabExamStudio,
    TabQualityInspector,
    TabArchiveSuite,
    TabHardwareTelemetry,
    TabPreferences,
    ToggleHiddenFiles,
    SelectAll,
    ClearSelection,
    Undo,
    Redo,
    ConvertTarget(&'static str),
    ExamConform,
    PackZip,
    RunUserScript {
        title: String,
        body: String,
        lang: ScriptLang,
    },
}

#[derive(Debug, Clone)]
pub struct CommandEntry {
    pub title: &'static str,
    pub category: &'static str,
    pub shortcut: &'static str,
    pub glyph: IconGlyph,
    pub action: PaletteAction,
}

pub const ALL_COMMANDS: &[CommandEntry] = &[
    // Navigation
    CommandEntry {
        title: "Go to Home",
        category: "Navigation",
        shortcut: "Alt+H",
        glyph: IconGlyph::Home,
        action: PaletteAction::NavigateHome,
    },
    CommandEntry {
        title: "Go to Desktop",
        category: "Navigation",
        shortcut: "",
        glyph: IconGlyph::FolderDesktop,
        action: PaletteAction::NavigateDesktop,
    },
    CommandEntry {
        title: "Go to Downloads",
        category: "Navigation",
        shortcut: "",
        glyph: IconGlyph::FolderDownloads,
        action: PaletteAction::NavigateDownloads,
    },
    CommandEntry {
        title: "Go to Documents",
        category: "Navigation",
        shortcut: "",
        glyph: IconGlyph::FolderDocuments,
        action: PaletteAction::NavigateDocuments,
    },
    CommandEntry {
        title: "Go to Pictures",
        category: "Navigation",
        shortcut: "",
        glyph: IconGlyph::FolderPictures,
        action: PaletteAction::NavigatePictures,
    },
    CommandEntry {
        title: "Go to Videos",
        category: "Navigation",
        shortcut: "",
        glyph: IconGlyph::FolderVideos,
        action: PaletteAction::NavigateVideos,
    },
    CommandEntry {
        title: "Go to Music",
        category: "Navigation",
        shortcut: "",
        glyph: IconGlyph::FolderMusic,
        action: PaletteAction::NavigateMusic,
    },
    CommandEntry {
        title: "Go to Recycle Bin",
        category: "Navigation",
        shortcut: "",
        glyph: IconGlyph::RecycleBin,
        action: PaletteAction::NavigateRecycleBin,
    },
    // View Modes
    CommandEntry {
        title: "View: Details Table",
        category: "View",
        shortcut: "Ctrl+2",
        glyph: IconGlyph::ActionList,
        action: PaletteAction::ViewDetails,
    },
    CommandEntry {
        title: "View: Grid / Icons",
        category: "View",
        shortcut: "Ctrl+1",
        glyph: IconGlyph::ActionView,
        action: PaletteAction::ViewGrid,
    },
    CommandEntry {
        title: "View: Miller Columns",
        category: "View",
        shortcut: "Ctrl+3",
        glyph: IconGlyph::ActionColumns,
        action: PaletteAction::ViewColumns,
    },
    CommandEntry {
        title: "Toggle Dual-Pane Split View",
        category: "View",
        shortcut: "F3",
        glyph: IconGlyph::ActionColumns,
        action: PaletteAction::ToggleSplitView,
    },
    CommandEntry {
        title: "Toggle Inspector Preview Pane",
        category: "View",
        shortcut: "Ctrl+Shift+P",
        glyph: IconGlyph::Search,
        action: PaletteAction::ToggleInspectorPane,
    },
    // Tools & Converters
    CommandEntry {
        title: "Open Transcoder Studio",
        category: "Tools",
        shortcut: "",
        glyph: IconGlyph::ActionRefresh,
        action: PaletteAction::TabTranscoder,
    },
    CommandEntry {
        title: "Open Custom Conformer (Exam Mode)",
        category: "Tools",
        shortcut: "",
        glyph: IconGlyph::ActionEdit,
        action: PaletteAction::TabExamStudio,
    },
    CommandEntry {
        title: "Open Quality Inspector",
        category: "Tools",
        shortcut: "",
        glyph: IconGlyph::Search,
        action: PaletteAction::TabQualityInspector,
    },
    CommandEntry {
        title: "Open Archive Suite",
        category: "Tools",
        shortcut: "",
        glyph: IconGlyph::PageArchive,
        action: PaletteAction::TabArchiveSuite,
    },
    CommandEntry {
        title: "Open Hardware & AI Telemetry",
        category: "Tools",
        shortcut: "",
        glyph: IconGlyph::ActionShield,
        action: PaletteAction::TabHardwareTelemetry,
    },
    CommandEntry {
        title: "Open Preferences",
        category: "Tools",
        shortcut: "Ctrl+,",
        glyph: IconGlyph::Gear,
        action: PaletteAction::TabPreferences,
    },
    // Actions & Conversions
    CommandEntry {
        title: "Toggle Hidden Files (.dotfiles)",
        category: "Actions",
        shortcut: "Ctrl+H",
        glyph: IconGlyph::ActionEye,
        action: PaletteAction::ToggleHiddenFiles,
    },
    CommandEntry {
        title: "Convert Selected to PNG",
        category: "Convert",
        shortcut: "",
        glyph: IconGlyph::PageImage,
        action: PaletteAction::ConvertTarget("PNG"),
    },
    CommandEntry {
        title: "Convert Selected to JPG",
        category: "Convert",
        shortcut: "",
        glyph: IconGlyph::PageImage,
        action: PaletteAction::ConvertTarget("JPG"),
    },
    CommandEntry {
        title: "Convert Selected to WEBP",
        category: "Convert",
        shortcut: "",
        glyph: IconGlyph::PageImage,
        action: PaletteAction::ConvertTarget("WEBP"),
    },
    CommandEntry {
        title: "Convert Selected to PDF",
        category: "Convert",
        shortcut: "",
        glyph: IconGlyph::PagePdf,
        action: PaletteAction::ConvertTarget("PDF"),
    },
    CommandEntry {
        title: "Convert Selected to MP4",
        category: "Convert",
        shortcut: "",
        glyph: IconGlyph::PageVideo,
        action: PaletteAction::ConvertTarget("MP4"),
    },
    CommandEntry {
        title: "Conform for Exam Envelope",
        category: "Convert",
        shortcut: "",
        glyph: IconGlyph::ActionCheck,
        action: PaletteAction::ExamConform,
    },
    CommandEntry {
        title: "Pack into ZIP Archive",
        category: "Actions",
        shortcut: "",
        glyph: IconGlyph::PageArchive,
        action: PaletteAction::PackZip,
    },
    CommandEntry {
        title: "Select All Files",
        category: "Selection",
        shortcut: "Ctrl+A",
        glyph: IconGlyph::ActionSelectAll,
        action: PaletteAction::SelectAll,
    },
    CommandEntry {
        title: "Clear Selection",
        category: "Selection",
        shortcut: "Esc",
        glyph: IconGlyph::ActionClose,
        action: PaletteAction::ClearSelection,
    },
    CommandEntry {
        title: "Undo Last Action",
        category: "Edit",
        shortcut: "Ctrl+Z",
        glyph: IconGlyph::ActionRefresh,
        action: PaletteAction::Undo,
    },
    CommandEntry {
        title: "Redo Last Action",
        category: "Edit",
        shortcut: "Ctrl+Y",
        glyph: IconGlyph::ActionRefresh,
        action: PaletteAction::Redo,
    },
    CommandEntry {
        title: "New Folder",
        category: "File",
        shortcut: "Ctrl+Shift+N",
        glyph: IconGlyph::FolderFavorites,
        action: PaletteAction::NewFolder,
    },
    CommandEntry {
        title: "New Explorer Window",
        category: "File",
        shortcut: "Ctrl+N",
        glyph: IconGlyph::ActionOpen,
        action: PaletteAction::NewWindow,
    },
    CommandEntry {
        title: "New Tab",
        category: "File",
        shortcut: "Ctrl+T",
        glyph: IconGlyph::ActionOpen,
        action: PaletteAction::NewTab,
    },
    CommandEntry {
        title: "Close Current Tab",
        category: "File",
        shortcut: "Ctrl+W",
        glyph: IconGlyph::ActionClose,
        action: PaletteAction::CloseTab,
    },
    CommandEntry {
        title: "Refresh Directory",
        category: "View",
        shortcut: "Ctrl+R",
        glyph: IconGlyph::ActionRefresh,
        action: PaletteAction::Refresh,
    },
    CommandEntry {
        title: "Find / Search Files",
        category: "Search",
        shortcut: "Ctrl+F",
        glyph: IconGlyph::Search,
        action: PaletteAction::FocusSearch,
    },
    CommandEntry {
        title: "Properties & Icon Customization",
        category: "File",
        shortcut: "Alt+Enter",
        glyph: IconGlyph::ActionProperties,
        action: PaletteAction::ShowProperties,
    },
    CommandEntry {
        title: "Delete to Recycle Bin",
        category: "File",
        shortcut: "Delete",
        glyph: IconGlyph::RecycleBin,
        action: PaletteAction::DeleteSelected,
    },
    CommandEntry {
        title: "Empty Recycle Bin",
        category: "System",
        shortcut: "Ctrl+Shift+Del",
        glyph: IconGlyph::RecycleBin,
        action: PaletteAction::EmptyRecycleBin,
    },
    CommandEntry {
        title: "Copy Selection",
        category: "Edit",
        shortcut: "Ctrl+C",
        glyph: IconGlyph::ActionCopy,
        action: PaletteAction::Copy,
    },
    CommandEntry {
        title: "Cut Selection",
        category: "Edit",
        shortcut: "Ctrl+X",
        glyph: IconGlyph::ActionCut,
        action: PaletteAction::Cut,
    },
    CommandEntry {
        title: "Paste from Clipboard",
        category: "Edit",
        shortcut: "Ctrl+V",
        glyph: IconGlyph::ActionPaste,
        action: PaletteAction::Paste,
    },
];

#[derive(Debug, Clone)]
pub struct DynamicCommand {
    pub title: String,
    pub category: String,
    pub shortcut: String,
    pub glyph: IconGlyph,
    pub action: PaletteAction,
}

#[derive(Debug, Clone)]
pub struct PaletteItem {
    pub title: String,
    pub subtitle: String,
    pub category: String,
    pub shortcut: String,
    pub glyph: IconGlyph,
    pub action: PaletteAction,
    pub is_file: bool,
}

#[derive(Debug, Default)]
pub struct CommandPalette {
    pub is_open: bool,
    pub query: String,
    pub selected_index: usize,
}

impl CommandPalette {
    pub fn get_combined_commands() -> Vec<DynamicCommand> {
        let mut list: Vec<DynamicCommand> = ALL_COMMANDS
            .iter()
            .map(|c| DynamicCommand {
                title: c.title.to_string(),
                category: c.category.to_string(),
                shortcut: c.shortcut.to_string(),
                glyph: c.glyph,
                action: c.action.clone(),
            })
            .collect();

        for script in crate::core::scripting::load_all_user_scripts() {
            list.push(DynamicCommand {
                title: format!("Run: {}", script.title),
                category: "Scripts".to_string(),
                shortcut: script.shortcut.unwrap_or_default(),
                glyph: IconGlyph::PageCode,
                action: PaletteAction::RunUserScript {
                    title: script.title,
                    body: script.body,
                    lang: script.lang,
                },
            });
        }

        list
    }

    pub fn toggle(&mut self) {
        self.is_open = !self.is_open;
        if self.is_open {
            self.query.clear();
            self.selected_index = 0;
        }
    }

    pub fn open(&mut self) {
        self.is_open = true;
        self.query.clear();
        self.selected_index = 0;
    }

    pub fn open_with_query(&mut self, query: &str) {
        self.is_open = true;
        self.query = query.to_string();
        self.selected_index = 0;
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.query.clear();
    }

    pub fn render(
        &mut self,
        ctx: &egui::Context,
        palette: &ThemePalette,
        icon_pack: IconPack,
        _current_dir: Option<&std::path::Path>,
        current_files: &[crate::core::FileItem],
    ) -> Option<PaletteAction> {
        if !self.is_open {
            return None;
        }

        let mut executed_action: Option<PaletteAction> = None;

        // Modal overlay backdrop
        egui::Area::new(egui::Id::new("command_palette_backdrop"))
            .interactable(true)
            .order(egui::Order::Foreground)
            .fixed_pos(Pos2::ZERO)
            .show(ctx, |ui| {
                let screen_rect = ctx.screen_rect();
                let bg_col = Color32::from_black_alpha(110);
                ui.painter()
                    .rect_filled(screen_rect, Rounding::ZERO, bg_col);

                let panel_w = 540.0_f32.min(screen_rect.width() - 32.0);
                let panel_h = 360.0_f32.min(screen_rect.height() - 64.0);
                let panel_pos = Pos2::new(
                    screen_rect.center().x - panel_w * 0.5,
                    (screen_rect.top() + 80.0).min(screen_rect.bottom() - panel_h - 20.0),
                );
                let panel_rect = Rect::from_min_size(panel_pos, Vec2::new(panel_w, panel_h));

                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(panel_rect), |ui| {
                    let frame = egui::Frame::none()
                        .fill(palette.window_bg)
                        .stroke(Stroke::new(1.0_f32, palette.border))
                        .rounding(Rounding::same(radius::LG))
                        .inner_margin(egui::Margin::symmetric(space::MD, space::MD));

                    frame.show(ui, |ui| {
                        // 1. Search Box
                        ui.horizontal(|ui| {
                            let icon_side = 16.0;
                            let (irect, _) =
                                ui.allocate_exact_size(Vec2::splat(icon_side), Sense::hover());
                            crate::core::icon_packs::paint_icon(
                                ui,
                                icon_pack,
                                IconGlyph::Search,
                                irect,
                                [palette.accent.r(), palette.accent.g(), palette.accent.b()],
                            );
                            ui.add_space(space::XS);

                            let search_id = egui::Id::new("command_palette_input");
                            let te = egui::TextEdit::singleline(&mut self.query)
                                .id(search_id)
                                .hint_text("Type '>' for commands, or search files...")
                                .desired_width(ui.available_width() - 20.0)
                                .font(FontId::proportional(type_scale::BODY));
                            let resp = ui.add(te);
                            resp.request_focus();

                            if ui.input(|i| i.key_pressed(Key::Escape)) {
                                self.is_open = false;
                            }
                        });

                        ui.add_space(space::XS);
                        ui.separator();
                        ui.add_space(space::XS);

                        // Determine items based on query
                        let mut items: Vec<PaletteItem> = Vec::new();
                        let q = self.query.trim();

                        if q.starts_with('>') {
                            // Command mode (VS Code parity: '>' filters commands)
                            let cmd_q = q.trim_start_matches('>').trim().to_lowercase();
                            let all_cmds = Self::get_combined_commands();
                            for cmd in all_cmds {
                                if cmd_q.is_empty()
                                    || cmd.title.to_lowercase().contains(&cmd_q)
                                    || cmd.category.to_lowercase().contains(&cmd_q)
                                {
                                    items.push(PaletteItem {
                                        title: cmd.title,
                                        subtitle: String::new(),
                                        category: cmd.category,
                                        shortcut: cmd.shortcut,
                                        glyph: cmd.glyph,
                                        action: cmd.action,
                                        is_file: false,
                                    });
                                }
                            }
                        } else {
                            // Quick Open / File Search mode (typing searches current folder files & subfolders)
                            let file_q = q.to_lowercase();
                            if !file_q.is_empty() {
                                for item in current_files {
                                    let name_match = item.name.to_lowercase().contains(&file_q);
                                    let ext_match = item.extension.to_lowercase().contains(&file_q);
                                    if name_match || ext_match {
                                        let (glyph, _) = IconGlyph::resolve_with_path(
                                            Some(&item.path),
                                            &item.name,
                                            &item.extension,
                                            item.is_dir,
                                            false,
                                            false,
                                            false,
                                            false,
                                            false,
                                            [120, 120, 120],
                                        );
                                        let subtitle = if item.is_dir {
                                            "Folder".to_string()
                                        } else {
                                            format!("{} - {}", item.extension, item.formatted_size())
                                        };
                                        let action = if item.is_dir {
                                            PaletteAction::NavigateTo(item.path.clone())
                                        } else {
                                            PaletteAction::OpenFile(item.path.clone())
                                        };
                                        items.push(PaletteItem {
                                            title: item.name.clone(),
                                            subtitle,
                                            category: if item.is_dir {
                                                "Folder".to_string()
                                            } else {
                                                "File".to_string()
                                            },
                                            shortcut: String::new(),
                                            glyph,
                                            action,
                                            is_file: true,
                                        });
                                        if items.len() >= 40 {
                                            break;
                                        }
                                    }
                                }
                            }

                            // Also append matching commands
                            let all_cmds = Self::get_combined_commands();
                            for cmd in all_cmds {
                                if file_q.is_empty()
                                    || cmd.title.to_lowercase().contains(&file_q)
                                    || cmd.category.to_lowercase().contains(&file_q)
                                {
                                    items.push(PaletteItem {
                                        title: cmd.title,
                                        subtitle: String::new(),
                                        category: cmd.category,
                                        shortcut: cmd.shortcut,
                                        glyph: cmd.glyph,
                                        action: cmd.action,
                                        is_file: false,
                                    });
                                    if items.len() >= 80 {
                                        break;
                                    }
                                }
                            }
                        }

                        let match_count = items.len();
                        if match_count == 0 {
                            ui.vertical_centered(|ui| {
                                ui.add_space(space::XL);
                                ui.label(
                                    egui::RichText::new("No matching commands or files")
                                        .size(type_scale::SMALL)
                                        .color(palette.text_muted),
                                );
                            });
                            return;
                        }

                        // Arrow navigation
                        if ui.input(|i| i.key_pressed(Key::ArrowDown)) {
                            self.selected_index = (self.selected_index + 1) % match_count;
                        }
                        if ui.input(|i| i.key_pressed(Key::ArrowUp)) {
                            self.selected_index = if self.selected_index == 0 {
                                match_count.saturating_sub(1)
                            } else {
                                self.selected_index - 1
                            };
                        }
                        if self.selected_index >= match_count {
                            self.selected_index = 0;
                        }

                        // Enter execution
                        if ui.input(|i| i.key_pressed(Key::Enter)) {
                            if let Some(item) = items.get(self.selected_index) {
                                executed_action = Some(item.action.clone());
                                self.is_open = false;
                            }
                        }

                        // Items rows scroll area
                        egui::ScrollArea::vertical()
                            .auto_shrink([false; 2])
                            .show(ui, |ui| {
                                for (idx, item) in items.iter().enumerate() {
                                    let is_active = idx == self.selected_index;
                                    let row_h = 30.0;
                                    let (rrect, rresp) = ui.allocate_exact_size(
                                        Vec2::new(ui.available_width(), row_h),
                                        Sense::click(),
                                    );
                                    let is_hovered = rresp.hovered();

                                    let painter = ui.painter_at(rrect);
                                    if is_active {
                                        painter.rect_filled(
                                            rrect,
                                            Rounding::same(radius::SM),
                                            Color32::from_rgba_unmultiplied(
                                                palette.accent.r(),
                                                palette.accent.g(),
                                                palette.accent.b(),
                                                if palette.is_dark { 45 } else { 26 },
                                            ),
                                        );
                                        painter.rect_stroke(
                                            rrect,
                                            Rounding::same(radius::SM),
                                            Stroke::new(1.0_f32, palette.accent),
                                        );
                                    } else if is_hovered {
                                        painter.rect_filled(
                                            rrect,
                                            Rounding::same(radius::SM),
                                            palette.card_hover,
                                        );
                                    }

                                    // Icon
                                    let irect = Rect::from_center_size(
                                        Pos2::new(rrect.left() + 15.0, rrect.center().y),
                                        Vec2::splat(16.0),
                                    );
                                    crate::core::icon_packs::paint_icon(
                                        ui,
                                        icon_pack,
                                        item.glyph,
                                        irect,
                                        if is_active {
                                            [
                                                palette.accent.r(),
                                                palette.accent.g(),
                                                palette.accent.b(),
                                            ]
                                        } else {
                                            [
                                                palette.text_secondary.r(),
                                                palette.text_secondary.g(),
                                                palette.text_secondary.b(),
                                            ]
                                        },
                                    );

                                    // Title + Subtitle
                                    let text_color = if is_active {
                                        if palette.is_dark {
                                            palette.accent
                                        } else {
                                            palette.text_primary
                                        }
                                    } else {
                                        palette.text_primary
                                    };

                                    let title_x = rrect.left() + 34.0;
                                    painter.text(
                                        Pos2::new(title_x, rrect.center().y),
                                        egui::Align2::LEFT_CENTER,
                                        &item.title,
                                        FontId::proportional(type_scale::BODY_SM),
                                        text_color,
                                    );

                                    if !item.subtitle.is_empty() {
                                        let offset = item.title.len() as f32 * 7.5 + 16.0;
                                        painter.text(
                                            Pos2::new(title_x + offset, rrect.center().y),
                                            egui::Align2::LEFT_CENTER,
                                            &item.subtitle,
                                            FontId::proportional(type_scale::CAPTION),
                                            palette.text_muted,
                                        );
                                    }

                                    // Category / Shortcut on right
                                    let meta_label = if !item.shortcut.is_empty() {
                                        format!("{}  {}", item.category, item.shortcut)
                                    } else {
                                        item.category.clone()
                                    };
                                    painter.text(
                                        Pos2::new(rrect.right() - space::MD, rrect.center().y),
                                        egui::Align2::RIGHT_CENTER,
                                        meta_label,
                                        FontId::proportional(type_scale::CAPTION),
                                        palette.text_muted,
                                    );

                                    if rresp.clicked() {
                                        executed_action = Some(item.action.clone());
                                        self.is_open = false;
                                    }
                                }
                            });
                    });
                });
            });

        executed_action
    }
}
