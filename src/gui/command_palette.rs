//! Finder-Style Command Palette (Ctrl+P / Cmd+K)
//!
//! A fast, keyboard-first modal launcher for Mossambi actions, format conversions,
//! view modes, and navigation destinations. Pure ASCII.

use crate::core::icon_packs::{IconGlyph, IconPack};
use crate::gui::design::{radius, space, type_scale};
use crate::gui::macos_theme::ThemePalette;
use eframe::egui::{self, Color32, FontId, Key, Pos2, Rect, Rounding, Sense, Stroke, Vec2};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaletteAction {
    NavigateHome,
    NavigateDesktop,
    NavigateDownloads,
    NavigateDocuments,
    NavigatePictures,
    NavigateVideos,
    NavigateMusic,
    NavigateRecycleBin,
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
    ConvertTarget(&'static str),
    ExamConform,
    PackZip,
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
];

#[derive(Debug, Default)]
pub struct CommandPalette {
    pub is_open: bool,
    pub query: String,
    pub selected_index: usize,
}

impl CommandPalette {
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

    pub fn close(&mut self) {
        self.is_open = false;
        self.query.clear();
    }

    pub fn render(
        &mut self,
        ctx: &egui::Context,
        palette: &ThemePalette,
        icon_pack: IconPack,
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

                let panel_w = 480.0_f32.min(screen_rect.width() - 32.0);
                let panel_h = 320.0_f32.min(screen_rect.height() - 64.0);
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
                                .hint_text("Type a command, format, or place...")
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

                        // Filter matching commands
                        let q = self.query.trim().to_lowercase();
                        let matches: Vec<&CommandEntry> = ALL_COMMANDS
                            .iter()
                            .filter(|cmd| {
                                if q.is_empty() {
                                    true
                                } else {
                                    cmd.title.to_lowercase().contains(&q)
                                        || cmd.category.to_lowercase().contains(&q)
                                }
                            })
                            .collect();

                        let match_count = matches.len();
                        if match_count == 0 {
                            ui.vertical_centered(|ui| {
                                ui.add_space(space::XL);
                                ui.label(
                                    egui::RichText::new("No matching commands")
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
                            if let Some(cmd) = matches.get(self.selected_index) {
                                executed_action = Some(cmd.action);
                                self.is_open = false;
                            }
                        }

                        // Command rows scroll area
                        egui::ScrollArea::vertical()
                            .auto_shrink([false; 2])
                            .show(ui, |ui| {
                                for (idx, cmd) in matches.iter().enumerate() {
                                    let is_active = idx == self.selected_index;
                                    let row_h = 28.0;
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
                                            Color32::from_rgba_premultiplied(
                                                palette.accent.r(),
                                                palette.accent.g(),
                                                palette.accent.b(),
                                                if palette.is_dark { 50 } else { 30 },
                                            ),
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
                                        Pos2::new(rrect.left() + 14.0, rrect.center().y),
                                        Vec2::splat(15.0),
                                    );
                                    crate::core::icon_packs::paint_icon(
                                        ui,
                                        icon_pack,
                                        cmd.glyph,
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

                                    // Title
                                    let text_color = if is_active {
                                        if palette.is_dark {
                                            palette.accent
                                        } else {
                                            palette.text_primary
                                        }
                                    } else {
                                        palette.text_primary
                                    };
                                    painter.text(
                                        Pos2::new(rrect.left() + 32.0, rrect.center().y),
                                        egui::Align2::LEFT_CENTER,
                                        cmd.title,
                                        FontId::proportional(type_scale::BODY_SM),
                                        text_color,
                                    );

                                    // Category / Shortcut on right
                                    let meta_label = if !cmd.shortcut.is_empty() {
                                        format!("{}  {}", cmd.category, cmd.shortcut)
                                    } else {
                                        cmd.category.to_string()
                                    };
                                    painter.text(
                                        Pos2::new(rrect.right() - space::MD, rrect.center().y),
                                        egui::Align2::RIGHT_CENTER,
                                        meta_label,
                                        FontId::proportional(type_scale::CAPTION),
                                        palette.text_muted,
                                    );

                                    if rresp.clicked() {
                                        executed_action = Some(cmd.action);
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
