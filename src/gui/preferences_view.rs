// src/gui/preferences_view.rs
//
// macOS System Settings-style Preferences surface for Mossambi.
//
// Layout: a searchable LEFT category sidebar + a RIGHT detail pane that shows
// only the selected category's settings. Every setting row renders an explicit
// label (left) and a human-readable value (right) so the page scans like a
// settings app instead of a wall of sliders.
//
// Pure ASCII. No emoji. Uses the shared design tokens from crate::gui::design.

use crate::core::config::{
    AnimStyle, AnimationSettings, CheckboxMode, CornerStyle, CustomMenuItem, CustomMenuKind,
    CustomMenuScope, ExplorerStyle, GlassEffect, LookPreset, MenuStyle, MossambiConfig, OutputMode,
    RowDensity, SidebarStyle,
};
use crate::core::donut_config::{DonutConfig, DonutProfile, DonutSlicePreset};
use crate::core::fields_config::FieldsConfig;
use crate::core::icon_packs::IconPack;
use crate::core::scripting::{ScriptConsole, ScriptLang};
use crate::gui::design;
use crate::gui::macos_theme::ThemePalette;
use crate::gui::macos_theme::ThemeStyle;
use crate::gui::quick_settings::QuickSettingsState;
use eframe::egui::{
    self, Align, Align2, Color32, FontId, Frame, Layout, Margin, Pos2, Rect, Rounding, ScrollArea,
    Sense, Stroke, TextEdit, Ui, Vec2,
};
use std::path::PathBuf;

const CATEGORIES: &[&str] = &[
    "General",
    "Donut toolbar",
    "Shortcuts & Gestures",
    "Fields",
    "Appearance",
    "Explorer",
    "Behavior",
    "Performance",
    "Conversion & Engines",
    "Scripts",
    "About",
];

#[derive(Debug, Clone)]
pub struct PreferenceCategory {
    pub id: &'static str,
    pub title: &'static str,
}

pub fn preference_categories() -> &'static [PreferenceCategory] {
    // Static table kept in sync with CATEGORIES above. Returned as a slice of
    // owned-ish statics so the command palette can reference category titles.
    static CATS: [PreferenceCategory; 11] = [
        PreferenceCategory {
            id: "general",
            title: "General",
        },
        PreferenceCategory {
            id: "donut",
            title: "Donut toolbar",
        },
        PreferenceCategory {
            id: "shortcuts",
            title: "Shortcuts & Gestures",
        },
        PreferenceCategory {
            id: "fields",
            title: "Fields",
        },
        PreferenceCategory {
            id: "appearance",
            title: "Appearance",
        },
        PreferenceCategory {
            id: "explorer",
            title: "Explorer",
        },
        PreferenceCategory {
            id: "behavior",
            title: "Behavior",
        },
        PreferenceCategory {
            id: "performance",
            title: "Performance",
        },
        PreferenceCategory {
            id: "conversion",
            title: "Conversion & Engines",
        },
        PreferenceCategory {
            id: "scripts",
            title: "Scripts",
        },
        PreferenceCategory {
            id: "about",
            title: "About",
        },
    ];
    &CATS
}

#[derive(Debug, Clone)]
pub struct PreferenceHit {
    pub category_id: &'static str,
    pub category_title: &'static str,
    pub setting_label: &'static str,
    pub value_label: String,
}

pub struct PreferencesView {
    pub search: String,
    pub selected: usize,
    pub cfg: MossambiConfig,
    pub dirty: bool,
    pub selected_donut_profile_idx: usize,
    pub donut_profile_search: String,
    pub donut_preset_search: String,
    pub donut_preset_category: String,
    pub donut_view_back: bool,
    pub new_ext_input: String,
    pub new_profile_name: String,
    pub save_error: Option<String>,

    // Rebind capture: Some(shortcut action id) while waiting for the next key.
    pub capturing_action: Option<String>,
    // Script editor state (create mode when the id is None).
    pub script_edit_id: Option<String>,
    pub script_edit_title: String,
    pub script_edit_lang: ScriptLang,
    pub script_edit_body: String,
}

impl Default for PreferencesView {
    fn default() -> Self {
        Self {
            search: String::new(),
            selected: 0,
            cfg: MossambiConfig::load(),
            dirty: false,
            selected_donut_profile_idx: 0,
            donut_profile_search: String::new(),
            donut_preset_search: String::new(),
            donut_preset_category: "All".to_string(),
            donut_view_back: false,
            new_ext_input: String::new(),
            new_profile_name: String::new(),
            save_error: None,
            capturing_action: None,
            script_edit_id: None,
            script_edit_title: String::new(),
            script_edit_lang: ScriptLang::Rhai,
            script_edit_body: String::new(),
        }
    }
}

impl PreferencesView {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn render(
        &mut self,
        ui: &mut Ui,
        palette: &ThemePalette,
        current_theme: &mut ThemeStyle,
        settings: &mut QuickSettingsState,
        console: &mut ScriptConsole,
        run_ctx: (Option<std::path::PathBuf>, Vec<std::path::PathBuf>),
    ) {
        // Keep cfg.theme_mode in sync with the live theme selection.
        self.cfg.theme_mode = (*current_theme).into();

        // Fluid macOS System Settings layout: strictly partitioned 210px sidebar + 1px divider + fluid detail pane
        let total_avail = ui.available_size();
        let sidebar_w = 210.0_f32;

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;

            // 1. Fluid macOS System Settings sidebar (210px) with dedicated vertical scroll area
            ui.allocate_ui_with_layout(
                Vec2::new(sidebar_w, total_avail.y),
                Layout::top_down(Align::Min),
                |ui| {
                    Frame::none()
                        .fill(palette.sidebar_bg)
                        .inner_margin(Margin::symmetric(10.0, 12.0))
                        .show(ui, |ui| {
                            // Restore automatic widget rhythm inside the sidebar
                            // partition; the outer horizontal keeps ZERO spacing so
                            // the sidebar, divider, and pane stay flush.
                            ui.spacing_mut().item_spacing =
                                Vec2::new(design::space::MD, design::space::SM);
                            ui.label(
                                egui::RichText::new("Settings")
                                    .size(design::type_scale::TITLE)
                                    .strong()
                                    .color(palette.text_primary),
                            );
                            ui.add_space(design::space::SM);
                            ui.add(
                                TextEdit::singleline(&mut self.search)
                                    .hint_text("Search settings")
                                    .desired_width(sidebar_w - 24.0),
                            );
                            ui.add_space(design::space::SM);

                            ScrollArea::vertical()
                                .id_salt("prefs_sidebar_scroll")
                                .auto_shrink([false; 2])
                                .show(ui, |ui| {
                                    if self.search.trim().is_empty() {
                                        for (i, cat) in CATEGORIES.iter().enumerate() {
                                            let is_sel = self.selected == i;
                                            let item_h = 32.0_f32;
                                            let (rect, resp) = ui.allocate_exact_size(
                                                Vec2::new(sidebar_w - 24.0, item_h),
                                                Sense::click(),
                                            );
                                            if resp.clicked() {
                                                self.selected = i;
                                            }
                                            if is_sel {
                                                ui.painter().rect_filled(
                                                    rect,
                                                    Rounding::same(design::radius::SM),
                                                    Color32::from_rgba_unmultiplied(
                                                        palette.accent.r(),
                                                        palette.accent.g(),
                                                        palette.accent.b(),
                                                        if palette.is_dark { 45 } else { 26 },
                                                    ),
                                                );
                                                ui.painter().rect_stroke(
                                                    rect,
                                                    Rounding::same(design::radius::SM),
                                                    Stroke::new(1.0_f32, palette.accent),
                                                );
                                            } else if resp.hovered() {
                                                ui.painter().rect_filled(
                                                    rect,
                                                    Rounding::same(design::radius::SM),
                                                    Color32::from_rgba_unmultiplied(
                                                        palette.text_primary.r(),
                                                        palette.text_primary.g(),
                                                        palette.text_primary.b(),
                                                        if palette.is_dark { 20 } else { 12 },
                                                    ),
                                                );
                                            }
                                            let text_x = rect.left() + 8.0;
                                            ui.painter().text(
                                                Pos2::new(text_x, rect.center().y),
                                                Align2::LEFT_CENTER,
                                                *cat,
                                                egui::FontId::proportional(design::type_scale::BODY_SM),
                                                // Accent text on the accent-tinted
                                                // selection fill was unreadable.
                                                palette.text_primary,
                                            );
                                            ui.add_space(2.0);
                                        }
                                    } else {
                                        // Filtered results across all categories.
                                        let q = self.search.to_lowercase();
                                        let hits = self.search_hits();
                                        if hits.is_empty() {
                                            ui.label(
                                                egui::RichText::new("No matching settings.")
                                                    .size(design::type_scale::BODY_SM)
                                                    .color(palette.text_muted),
                                            );
                                            ui.label(
                                                egui::RichText::new(
                                                    "Try a different word, e.g. \"opacity\" or \"tags\".",
                                                )
                                                .size(design::type_scale::CAPTION)
                                                .color(palette.text_muted),
                                            );
                                        } else {
                                            for hit in &hits {
                                                if !hit.setting_label.to_lowercase().contains(&q)
                                                    && !hit.value_label.to_lowercase().contains(&q)
                                                    && !hit.category_title.to_lowercase().contains(&q)
                                                {
                                                    continue;
                                                }
                                                ui.label(
                                                    egui::RichText::new(hit.setting_label)
                                                        .size(design::type_scale::BODY_SM)
                                                        .color(palette.text_primary),
                                                );
                                                ui.label(
                                                    egui::RichText::new(format!(
                                                        "{}  -  {}",
                                                        hit.category_title, hit.value_label
                                                    ))
                                                    .size(design::type_scale::CAPTION)
                                                    .color(palette.text_muted),
                                                );
                                                ui.add_space(design::space::XS);
                                            }
                                        }
                                    }
                                });
                        });
                },
            );

            // 2. Subtle 1px vertical separator
            let (div_rect, _) =
                ui.allocate_exact_size(Vec2::new(1.0, total_avail.y), Sense::hover());
            ui.painter()
                .rect_filled(div_rect, Rounding::ZERO, palette.border);

            // 3. Detail Content Pane
            let detail_w = (ui.available_width() - 8.0).max(320.0);
            ui.allocate_ui_with_layout(
                Vec2::new(detail_w, total_avail.y),
                Layout::top_down(Align::Min),
                |ui| {
                    let mut dirty = false;
                    if let Some(err) = self.save_error.clone() {
                        // Pinned save-status banner: visible regardless of
                        // scroll, because silent persistence failures are how
                        // settings get lost without the user ever knowing.
                        // Danger red is theme-independent on purpose:
                        // ThemePalette carries no error role, and a failed save
                        // must read as a failure in every theme.
                        let danger = Color32::from_rgb(196, 74, 66);
                        let tint = if palette.is_dark { 42 } else { 22 };
                        Frame::none()
                            .fill(Color32::from_rgba_unmultiplied(
                                danger.r(),
                                danger.g(),
                                danger.b(),
                                tint,
                            ))
                            .stroke(Stroke::new(1.0_f32, danger))
                            .rounding(Rounding::same(design::radius::MD))
                            .inner_margin(Margin::symmetric(12.0, 8.0))
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "Settings were NOT saved: {err}"
                                    ))
                                    .size(design::type_scale::BODY_SM)
                                    .strong()
                                    .color(palette.text_primary),
                                );
                                ui.label(
                                    egui::RichText::new(
                                        "Saving retries automatically; if this banner stays, check disk space or folder permissions.",
                                    )
                                    .size(design::type_scale::CAPTION)
                                    .color(palette.text_secondary),
                                );
                            });
                        ui.add_space(design::space::SM);
                    }
                    ScrollArea::vertical()
                        .id_salt("prefs_detail_scroll")
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            // Restore automatic widget rhythm inside the detail
                            // pane; the outer horizontal keeps ZERO spacing so the
                            // sidebar, divider, and pane stay flush.
                            ui.spacing_mut().item_spacing =
                                Vec2::new(design::space::MD, design::space::SM);
                            if self.selected == 1 {
                                // Donut toolbar: fluid 3-column layout centered with safety
                                // padding. Uncapped to 1280 so maximized windows use the
                                // width instead of leaving a large centering void.
                                let donut_w = 1280.0_f32.min(ui.available_width() - 32.0);
                                let pad_x = ((ui.available_width() - donut_w) * 0.5).max(16.0);
                                ui.horizontal(|ui| {
                                    ui.add_space(pad_x);
                                    ui.vertical(|ui| {
                                        ui.set_width(donut_w);
                                        // Bound the max rect too: set_width alone only
                                        // raises the minimum, so the inner 3-column
                                        // layout would still see the full pane width
                                        // and run past the right window edge when
                                        // maximized.
                                        ui.set_max_width(donut_w);
                                        ui.add_space(design::space::SM);
                                        let title = CATEGORIES[self.selected];
                                        ui.label(
                                            egui::RichText::new(title)
                                                .size(design::type_scale::TITLE_LG)
                                                .strong()
                                                .color(palette.text_primary),
                                        );
                                        ui.label(
                                            egui::RichText::new(self.category_subtitle(self.selected))
                                                .size(design::type_scale::BODY_SM)
                                                .color(palette.text_secondary),
                                        );
                                        ui.add_space(design::space::MD);
                                        self.render_donut_toolbar(ui, palette, &mut dirty);
                                    });
                                });
                            } else {
                                // System Settings layout: a bounded column next to the
                                // category list (centering left a ~300 px void between them).
                                let content_w = 680.0_f32.min(ui.available_width() - 56.0).max(320.0);
                                let pad_x = 32.0_f32;
                                ui.horizontal(|ui| {
                                    ui.add_space(pad_x);
                                    ui.vertical(|ui| {
                                        ui.set_width(content_w);
                                        ui.add_space(design::space::SM);
                                        let title = CATEGORIES[self.selected];
                                        ui.label(
                                            egui::RichText::new(title)
                                                .size(design::type_scale::TITLE_LG)
                                                .strong()
                                                .color(palette.text_primary),
                                        );
                                        ui.label(
                                            egui::RichText::new(self.category_subtitle(self.selected))
                                                .size(design::type_scale::BODY_SM)
                                                .color(palette.text_secondary),
                                        );
                                        ui.add_space(design::space::MD);

                                        match self.selected {
                                            0 => self.render_general(ui, palette, &mut dirty),
                                            1 => self.render_donut_toolbar(ui, palette, &mut dirty),
                                            2 => self.render_shortcuts(ui, palette, &mut dirty),
                                            3 => self.render_fields(ui, palette, &mut dirty),
                                            4 => self.render_appearance(ui, palette, current_theme, &mut dirty),
                                            5 => self.render_explorer(ui, palette, &mut dirty),
                                            6 => self.render_behavior(ui, palette, &mut dirty),
                                            7 => self.render_performance(ui, palette, &mut dirty),
                                            8 => self.render_conversion(ui, palette, settings, &mut dirty),
                                            9 => self.render_scripts(ui, palette, console, run_ctx, &mut dirty),
                                            _ => self.render_about(ui, palette),
                                        }
                                    });
                                });
                            }
                            // Propagate the frame-local dirty flag to the live
                            // persistence check below; the per-tab renders borrow
                            // a local because they already hold &mut self.
                            if dirty {
                                self.dirty = true;
                            }
                        });
                },
            );
        });

        if self.dirty {
            // Live persistence: every change writes immediately. A failure is
            // surfaced in the pane instead of being swallowed silently, and the
            // flag stays set so the next frame retries (transient locks heal).
            if let Err(e) = self.cfg.save() {
                self.save_error = Some(e.to_string());
            } else {
                self.dirty = false;
                self.save_error = None;
            }
        }
    }
    fn category_subtitle(&self, i: usize) -> &'static str {
        match i {
            0 => "Startup, default save locations, browser drag staging, and history.",
            1 => "Profiles, interactive radial donut preview, extension bindings, and appearance metrics.",
            2 => "Studio shortcut rebinds, gesture engine keys, and physics.",
            3 => "Toggle specialized domains on or off across the suite.",
            4 => "Theme, glass, opacity, menu and control style.",
            5 => "Layout, hidden files, tags and pinned folders.",
            6 => "Startup, sound, background work, and shell replacement.",
            7 => "Threads, DirectML hardware acceleration, and frame rate.",
            8 => "Zero-loss transcode engines, 255-format routing, and script engines.",
            9 => "Create, run, and monitor Rhai and Lua automation scripts.",
            _ => "Version, hardware telemetry, and credits.",
        }
    }

    fn search_hits(&self) -> Vec<PreferenceHit> {
        let mut out = Vec::new();
        // Appearance
        out.push(PreferenceHit {
            category_id: "appearance",
            category_title: "Appearance",
            setting_label: "Design style",
            value_label: self.cfg.design_style.label().to_string(),
        });
        out.push(PreferenceHit {
            category_id: "appearance",
            category_title: "Appearance",
            setting_label: "Glass effect",
            value_label: glass_label(self.cfg.glass_effect).to_string(),
        });
        out.push(PreferenceHit {
            category_id: "appearance",
            category_title: "Appearance",
            setting_label: "Window opacity",
            value_label: format!("{:.0}%", self.cfg.window_opacity * 100.0),
        });
        out.push(PreferenceHit {
            category_id: "appearance",
            category_title: "Appearance",
            setting_label: "Menu style",
            value_label: menu_style_label(self.cfg.menu_style).to_string(),
        });
        out.push(PreferenceHit {
            category_id: "appearance",
            category_title: "Appearance",
            setting_label: "Checkbox mode",
            value_label: checkbox_mode_label(self.cfg.checkbox_mode).to_string(),
        });
        out.push(PreferenceHit {
            category_id: "appearance",
            category_title: "Appearance",
            setting_label: "Icon pack",
            value_label: IconPack::from_config_id(&self.cfg.icon_pack)
                .label()
                .to_string(),
        });
        // Explorer
        out.push(PreferenceHit {
            category_id: "explorer",
            category_title: "Explorer",
            setting_label: "Show hidden files",
            value_label: on_off(self.cfg.show_hidden_files).to_string(),
        });
        out.push(PreferenceHit {
            category_id: "explorer",
            category_title: "Explorer",
            setting_label: "Show tags section",
            value_label: on_off(self.cfg.show_tags_section).to_string(),
        });
        out.push(PreferenceHit {
            category_id: "explorer",
            category_title: "Explorer",
            setting_label: "Calculate checksums",
            value_label: on_off(self.cfg.calculate_checksums).to_string(),
        });
        // Scripts
        out.push(PreferenceHit {
            category_id: "scripts",
            category_title: "Scripts",
            setting_label: "Script console",
            value_label: "Run Rhai and Lua scripts and watch live outcomes".to_string(),
        });
        out.push(PreferenceHit {
            category_id: "scripts",
            category_title: "Scripts",
            setting_label: "New script (Rhai or Lua)",
            value_label: "Saved to the custom commands database".to_string(),
        });
        // Behavior
        out.push(PreferenceHit {
            category_id: "behavior",
            category_title: "Behavior",
            setting_label: "Always on top",
            value_label: on_off(self.cfg.always_on_top).to_string(),
        });
        out.push(PreferenceHit {
            category_id: "behavior",
            category_title: "Behavior",
            setting_label: "Play sound",
            value_label: on_off(self.cfg.play_sound).to_string(),
        });
        out.push(PreferenceHit {
            category_id: "behavior",
            category_title: "Behavior",
            setting_label: "Launch at login",
            value_label: on_off(self.cfg.launch_at_login).to_string(),
        });
        out.push(PreferenceHit {
            category_id: "behavior",
            category_title: "Behavior",
            setting_label: "Background work",
            value_label: on_off(self.cfg.background_work).to_string(),
        });
        out.push(PreferenceHit {
            category_id: "behavior",
            category_title: "Behavior",
            setting_label: "Max frame rate",
            value_label: on_off(self.cfg.max_fps).to_string(),
        });
        // Input
        out.push(PreferenceHit {
            category_id: "input",
            category_title: "Input & Gestures",
            setting_label: "Gesture wake-up",
            value_label: on_off(self.cfg.enable_gesture_wakeup).to_string(),
        });
        out.push(PreferenceHit {
            category_id: "input",
            category_title: "Input & Gestures",
            setting_label: "Hotkey wake-up",
            value_label: on_off(self.cfg.enable_hotkey_wakeup).to_string(),
        });
        // Performance
        let topo = crate::core::qos::detect_cpu_topology();
        let wt_val = if self.cfg.worker_threads == 0 {
            format!("Auto ({})", topo.description())
        } else {
            format!("{} threads", self.cfg.worker_threads)
        };
        out.push(PreferenceHit {
            category_id: "performance",
            category_title: "Performance",
            setting_label: "Worker threads",
            value_label: wt_val,
        });
        // Conversion
        out.push(PreferenceHit {
            category_id: "conversion",
            category_title: "Conversion",
            setting_label: "Output mode",
            value_label: output_mode_label(self.cfg.output_mode).to_string(),
        });
        out.push(PreferenceHit {
            category_id: "conversion",
            category_title: "Conversion",
            setting_label: "Enforce zero loss",
            value_label: on_off(self.cfg.enforce_zero_loss).to_string(),
        });
        // General
        out.push(PreferenceHit {
            category_id: "general",
            category_title: "General",
            setting_label: "When dropping files",
            value_label: self.cfg.drop_action_pref.label().to_string(),
        });
        out
    }

    // ---- category renderers ----

    fn render_general(&mut self, ui: &mut Ui, palette: &ThemePalette, dirty: &mut bool) {
        // Only rows that do something. The static "Language" and "When I
        // close the window" labels and the no-op "Remove from the main
        // menu" button are gone.
        card(ui, palette, |ui| {
            toggle_row(
                ui,
                palette,
                "Start Mossambi when I sign in",
                "Runs the background helper (tray, radial widget hotkeys) at sign-in.",
                &mut self.cfg.launch_at_login,
                dirty,
            );
            ui.add_space(design::space::XS);
            toggle_row(
                ui,
                palette,
                "Show \"Convert with Mossambi\" in the Explorer context menu",
                "Adds the entry for files in Windows Explorer (Show more options on Windows 11).",
                &mut self.cfg.shell_context_menu,
                dirty,
            );
            ui.add_space(design::space::XS);
            toggle_row(
                ui,
                palette,
                "Use Mossambi as default file manager",
                "Open folders and drives in Mossambi instead of Windows Explorer.",
                &mut self.cfg.default_file_manager,
                dirty,
            );
            if let Some(err) = crate::platform::default_handler::last_error() {
                // Registry failure feedback shown inline next to the toggle.
                // Danger red is theme-independent (see save banner above):
                // ThemePalette carries no error role.
                ui.add_space(design::space::XS);
                ui.colored_label(
                    Color32::from_rgb(196, 74, 66),
                    format!("Could not update the default file manager: {err}"),
                );
            }
            ui.add_space(design::space::XS);
            toggle_row(
                ui,
                palette,
                "File dialog companion bar",
                "Dock a quick-jump bar to Open and Save file dialogs (#32770).",
                &mut self.cfg.dialog_companion,
                dirty,
            );
        });

        ui.add_space(design::space::MD);

        card(ui, palette, |ui| {
            ui.label(
                egui::RichText::new("When dropping files")
                    .size(design::type_scale::TITLE_SM)
                    .strong()
                    .color(palette.text_primary),
            );
            ui.add_space(design::space::XS);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Action on drop")
                        .size(design::type_scale::BODY)
                        .color(palette.text_primary),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let cur_label = match self.cfg.drop_action_pref {
                        crate::core::DropActionPreference::SmartDefault => {
                            "Smart default (default)"
                        }
                        other => other.label(),
                    };
                    egui::ComboBox::from_id_salt("pref_drop_action")
                        .selected_text(cur_label)
                        .show_ui(ui, |ui| {
                            for opt in crate::core::DropActionPreference::all() {
                                let text = match opt {
                                    crate::core::DropActionPreference::SmartDefault => {
                                        "Smart default (default)"
                                    }
                                    other => other.label(),
                                };
                                if ui
                                    .selectable_value(&mut self.cfg.drop_action_pref, opt, text)
                                    .changed()
                                {
                                    *dirty = true;
                                }
                            }
                        });
                });
            });
            ui.add_space(design::space::XS);
            ui.label(
                egui::RichText::new(
                    "Smart default moves on the same drive and copies across drives. Always ask shows an action menu at drop position.",
                )
                .size(design::type_scale::CAPTION)
                .color(palette.text_secondary),
            );
        });

        ui.add_space(design::space::MD);

        card(ui, palette, |ui| {
            ui.label(
                egui::RichText::new("Default save location")
                    .size(design::type_scale::TITLE_SM)
                    .strong()
                    .color(palette.text_primary),
            );
            ui.label(egui::RichText::new("Presets set to \"Default save location\" save here. A preset with its own location keeps it.").size(design::type_scale::CAPTION).color(palette.text_secondary));
            ui.add_space(design::space::XS);

            ui.horizontal(|ui| {
                let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::hover());
                crate::core::icon_packs::paint_icon(
                    ui,
                    IconPack::Fluent,
                    crate::core::icon_packs::IconGlyph::Folder,
                    icon_rect,
                    [palette.accent.r(), palette.accent.g(), palette.accent.b()],
                );
                ui.label(
                    egui::RichText::new("Same folder as the source")
                        .size(design::type_scale::BODY)
                        .strong()
                        .color(palette.text_primary),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let (pill_rect, _) =
                        ui.allocate_exact_size(Vec2::new(76.0, 22.0), Sense::hover());
                    ui.painter().rect_filled(
                        pill_rect,
                        Rounding::same(11.0),
                        Color32::from_rgba_unmultiplied(
                            palette.accent.r(),
                            palette.accent.g(),
                            palette.accent.b(),
                            if palette.is_dark { 36 } else { 20 },
                        ),
                    );
                    ui.painter().text(
                        pill_rect.center(),
                        Align2::CENTER_CENTER,
                        "Default",
                        FontId::proportional(11.0),
                        palette.accent,
                    );
                });
            });
            ui.add_space(design::space::SM);

            toggle_row(
                ui,
                palette,
                "Keep the original file dates for every conversion",
                "Converted files keep the original created and modified dates, with every preset.\nOff: only presets with \"Keep the original file dates\" do it.",
                &mut self.cfg.keep_original_file_dates,
                dirty,
            );

            toggle_row(
                ui,
                palette,
                "Save every geospatial result as a ZIP",
                "Shapefile, GeoJSON, KML, GPX and the other geospatial results are packed into one ZIP per file, with every preset.\nOff: only presets with \"Save the result as a ZIP\" do it.",
                &mut self.cfg.save_geospatial_as_zip,
                dirty,
            );

            toggle_row(
                ui,
                palette,
                "Show toast notifications",
                "Windows Action Center toasts when a batch finishes or fails while the window is in the background (or when a job runs longer than 10 seconds).\nOff: never show system notifications.",
                &mut self.cfg.notifications_enabled,
                dirty,
            );
        });

        ui.add_space(design::space::MD);

        card(ui, palette, |ui| {
            ui.label(
                egui::RichText::new("Files dragged from a browser")
                    .size(design::type_scale::TITLE_SM)
                    .strong()
                    .color(palette.text_primary),
            );
            ui.label(egui::RichText::new("Hold the key and drag a picture, a link to a file or a download out of Chrome, Edge, Firefox or Safari: the donut toolbar opens as it does for your own files. The file is saved in this folder, and presets that save next to the original put the converted file here too. Mossambi downloads a link itself, without your browser sign-in.").size(design::type_scale::CAPTION).color(palette.text_secondary));
            ui.add_space(design::space::SM);

            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Save to")
                        .size(design::type_scale::BODY_SM)
                        .color(palette.text_secondary),
                );
                let mut staging_str = self
                    .cfg
                    .browser_staging_dir
                    .as_ref()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();
                if ui
                    .add(
                        TextEdit::singleline(&mut staging_str)
                            .desired_width(ui.available_width() - 90.0),
                    )
                    .changed()
                {
                    self.cfg.browser_staging_dir = if staging_str.trim().is_empty() {
                        None
                    } else {
                        Some(PathBuf::from(staging_str))
                    };
                    *dirty = true;
                }
                if ui.button("Browse...").clicked() {
                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                        self.cfg.browser_staging_dir = Some(folder);
                        *dirty = true;
                    }
                }
            });

            ui.add_space(design::space::XS);
            toggle_row(
                ui,
                palette,
                "Save only the converted file",
                "The original waits in a temporary folder and is deleted once it is converted.",
                &mut self.cfg.browser_save_converted_only,
                dirty,
            );
        });

        ui.add_space(design::space::MD);

        card(ui, palette, |ui| {
            ui.label(
                egui::RichText::new("Conversion history")
                    .size(design::type_scale::TITLE_SM)
                    .strong()
                    .color(palette.text_primary),
            );
            ui.add_space(design::space::XS);

            toggle_row(
                ui,
                palette,
                "Don't keep a history of recent conversions",
                "",
                &mut self.cfg.disable_conversion_history,
                dirty,
            );

            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("History retention limit")
                        .size(design::type_scale::BODY)
                        .color(palette.text_primary),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let (badge_rect, _) =
                        ui.allocate_exact_size(Vec2::new(70.0, 22.0), Sense::hover());
                    ui.painter().rect_filled(
                        badge_rect,
                        Rounding::same(design::radius::XS),
                        palette.card_hover,
                    );
                    ui.painter().text(
                        badge_rect.center(),
                        Align2::CENTER_CENTER,
                        format!("{} items", self.cfg.conversion_history_limit),
                        FontId::proportional(12.0),
                        palette.accent,
                    );
                    ui.add_space(8.0);
                    let slider = egui::Slider::new(&mut self.cfg.conversion_history_limit, 0..=500)
                        .show_value(false);
                    if ui.add_sized(Vec2::new(140.0, 20.0), slider).changed() {
                        *dirty = true;
                    }
                });
            });

            ui.add_space(design::space::SM);
            let history_cnt = crate::core::conversion_history::conversion_history_count();
            ui.horizontal(|ui| {
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("Clear history")
                                .size(design::type_scale::BODY_SM)
                                .color(palette.text_primary),
                        )
                        .fill(palette.card_hover)
                        .stroke(Stroke::new(1.0_f32, palette.border))
                        .rounding(Rounding::same(design::radius::SM)),
                    )
                    .clicked()
                {
                    crate::core::conversion_history::clear_conversion_history();
                    *dirty = true;
                }
                ui.label(
                    egui::RichText::new(format!("({history_cnt} conversions logged)"))
                        .size(design::type_scale::CAPTION)
                        .color(palette.text_secondary),
                );
            });
        });
    }

    fn render_donut_toolbar(&mut self, ui: &mut Ui, palette: &ThemePalette, dirty: &mut bool) {
        let avail_w = ui.available_width();
        if avail_w < 760.0 {
            // Single column stacked layout when detail width < 760px
            let col_w = (avail_w - 8.0).max(280.0);
            self.render_donut_col1(ui, col_w, palette, dirty);
            ui.add_space(design::space::MD);
            let div1_rect = ui.allocate_space(Vec2::new(col_w, 1.0)).1;
            ui.painter()
                .rect_filled(div1_rect, Rounding::ZERO, palette.border);
            ui.add_space(design::space::MD);
            self.render_donut_col2(ui, col_w, palette, dirty);
            ui.add_space(design::space::MD);
            let div2_rect = ui.allocate_space(Vec2::new(col_w, 1.0)).1;
            ui.painter()
                .rect_filled(div2_rect, Rounding::ZERO, palette.border);
            ui.add_space(design::space::MD);
            self.render_donut_col3(ui, col_w, palette, dirty);
            return;
        }

        // Tailwind CSS-grade fluid 3-column responsive layout with horizontal scroll safety
        let (col1_w, col2_w, col3_w) = if avail_w >= 880.0 {
            let c1 = (avail_w * 0.22).clamp(170.0, 210.0);
            let c3 = (avail_w * 0.32).clamp(220.0, 300.0);
            let c2 = (avail_w - c1 - c3 - 36.0).max(300.0);
            (c1, c2, c3)
        } else {
            let c1 = 165.0;
            let c2 = 310.0;
            let c3 = (avail_w - c1 - c2 - 36.0).max(210.0);
            (c1, c2, c3)
        };

        ScrollArea::horizontal()
            .id_salt("donut_toolbar_h_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    self.render_donut_col1(ui, col1_w, palette, dirty);
                    ui.add_space(design::space::SM);
                    // Vertical divider line between col 1 and col 2
                    let div1_rect = ui.allocate_space(Vec2::new(1.0, 560.0)).1;
                    ui.painter()
                        .rect_filled(div1_rect, Rounding::ZERO, palette.border);
                    ui.add_space(design::space::SM);
                    self.render_donut_col2(ui, col2_w, palette, dirty);
                    ui.add_space(design::space::SM);
                    // Vertical divider line between col 2 and col 3
                    let div2_rect = ui.allocate_space(Vec2::new(1.0, 560.0)).1;
                    ui.painter()
                        .rect_filled(div2_rect, Rounding::ZERO, palette.border);
                    ui.add_space(design::space::SM);
                    self.render_donut_col3(ui, col3_w, palette, dirty);
                });
            });
    }

    fn render_donut_col1(
        &mut self,
        ui: &mut Ui,
        col1_w: f32,
        palette: &ThemePalette,
        dirty: &mut bool,
    ) {
        ui.allocate_ui_with_layout(Vec2::new(col1_w, ui.available_height()), Layout::top_down(Align::Min), |ui| {
            ui.label(
                egui::RichText::new("1. Choose a profile")
                    .size(design::type_scale::TITLE_SM)
                    .strong()
                    .color(palette.text_primary),
            );
            ui.add_space(design::space::XS);

            ui.add(
                TextEdit::singleline(&mut self.donut_profile_search)
                    .hint_text("Search")
                    .desired_width(col1_w - 12.0),
            );
            ui.add_space(design::space::SM);

            let search_q = self.donut_profile_search.trim().to_lowercase();
            let num_profiles = self.cfg.donut.profiles.len();

            ScrollArea::vertical()
                .id_salt("donut_profiles_scroll")
                .max_height(480.0)
                .show(ui, |ui| {
                    for i in 0..num_profiles {
                        let (name, front_len, back_len, is_mixed) = {
                            let p = &self.cfg.donut.profiles[i];
                            (p.name.clone(), p.front_slices.len(), p.back_slices.len(), p.is_mixed_selection)
                        };

                        if !search_q.is_empty() && !name.to_lowercase().contains(&search_q) {
                            continue;
                        }

                        let is_selected = self.selected_donut_profile_idx == i;
                        let card_rect = ui.allocate_space(Vec2::new(col1_w - 14.0, 32.0)).1;
                        let is_hovered = ui.rect_contains_pointer(card_rect);

                        let bg_color = if is_selected {
                            palette.card_hover
                        } else if is_hovered {
                            Color32::from_rgba_unmultiplied(palette.card_hover.r(), palette.card_hover.g(), palette.card_hover.b(), 70)
                        } else {
                            Color32::TRANSPARENT
                        };

                        ui.painter().rect_filled(card_rect, Rounding::same(design::radius::SM), bg_color);
                        if is_selected {
                            ui.painter().rect_stroke(card_rect, Rounding::same(design::radius::SM), Stroke::new(1.0_f32, palette.accent));
                        }

                        let text_color = if is_selected { palette.text_primary } else { palette.text_secondary };
                        let label_pos = egui::pos2(card_rect.min.x + 8.0, card_rect.center().y - 7.0);
                        ui.painter().text(
                            label_pos,
                            Align2::LEFT_TOP,
                            &name,
                            egui::FontId::proportional(design::type_scale::BODY_SM),
                            text_color,
                        );

                        let badge_str = if is_mixed {
                            format!("M {}-{}", front_len, back_len)
                        } else {
                            format!("{}-{}", front_len, back_len)
                        };
                        let badge_pos = egui::pos2(card_rect.max.x - 8.0, card_rect.center().y - 6.0);
                        ui.painter().text(
                            badge_pos,
                            Align2::RIGHT_TOP,
                            badge_str,
                            egui::FontId::proportional(design::type_scale::MICRO),
                            palette.text_muted,
                        );

                        if ui.interact(card_rect, ui.id().with(i), Sense::click()).clicked() {
                            self.selected_donut_profile_idx = i;
                        }
                        ui.add_space(2.0);
                    }
                });

            ui.add_space(design::space::SM);
            ui.horizontal_wrapped(|ui| {
                if ui.button("+ New profile").clicked() {
                    let new_id = format!("custom_{}", self.cfg.donut.profiles.len() + 1);
                    let new_prof = DonutProfile::new(
                        &new_id,
                        "New Profile",
                        &[".txt"],
                        &["PNG", "PDF", "WEBP", "JPG"],
                        &["CROP", "RENAME", "ZIP", "EXPLORE"],
                    );
                    self.cfg.donut.profiles.push(new_prof);
                    self.selected_donut_profile_idx = self.cfg.donut.profiles.len() - 1;
                    *dirty = true;
                }
                if ui.button("Alt+Up").clicked() && self.selected_donut_profile_idx > 0 {
                    self.cfg.donut.move_profile_up(self.selected_donut_profile_idx);
                    self.selected_donut_profile_idx -= 1;
                    *dirty = true;
                }
                if ui.button("Alt+Down").clicked()
                    && self.selected_donut_profile_idx + 1 < self.cfg.donut.profiles.len()
                {
                    self.cfg.donut.move_profile_down(self.selected_donut_profile_idx);
                    self.selected_donut_profile_idx += 1;
                    *dirty = true;
                }
            });

            ui.add_space(design::space::XS);
            ui.label(
                egui::RichText::new("When two profiles share an extension, the higher one opens. Alt+Up / Alt+Down moves the selected profile.")
                    .size(design::type_scale::MICRO)
                    .color(palette.text_muted),
            );
        });
    }

    fn render_donut_col2(
        &mut self,
        ui: &mut Ui,
        col2_w: f32,
        palette: &ThemePalette,
        dirty: &mut bool,
    ) {
        ui.allocate_ui_with_layout(
            Vec2::new(col2_w, ui.available_height()),
            Layout::top_down(Align::Min),
            |ui| {
                ui.label(
                    egui::RichText::new("2. Arrange presets")
                        .size(design::type_scale::TITLE_SM)
                        .strong()
                        .color(palette.text_primary),
                );
                ui.add_space(design::space::XS);

                if self.selected_donut_profile_idx >= self.cfg.donut.profiles.len() {
                    self.selected_donut_profile_idx = 0;
                }
                let (profile_name, profile_id, front_len, back_len, slices_copy) = {
                    let p = &self.cfg.donut.profiles[self.selected_donut_profile_idx];
                    let slices = if self.donut_view_back {
                        &p.back_slices
                    } else {
                        &p.front_slices
                    };
                    (
                        p.name.clone(),
                        p.id.clone(),
                        p.front_slices.len(),
                        p.back_slices.len(),
                        slices.clone(),
                    )
                };

                // Front / Back toggle pills
                ui.horizontal_wrapped(|ui| {
                    let is_front = !self.donut_view_back;
                    let front_label = format!("Front {}", front_len);
                    if design::segmented_pill(ui, palette, is_front, &front_label).clicked() {
                        self.donut_view_back = false;
                    }

                    let back_label = format!("Back {}", back_len);
                    if design::segmented_pill(ui, palette, !is_front, &back_label).clicked() {
                        self.donut_view_back = true;
                    }
                });

                ui.label(
                    egui::RichText::new("Flip the open donut: Tab / mouse wheel")
                        .size(design::type_scale::MICRO)
                        .color(palette.text_muted),
                );
                ui.add_space(design::space::SM);

                // Live Vector Radial Donut Preview (Dynamically scaled to col2_w)
                let canvas_w = (col2_w - 24.0).clamp(210.0, 270.0);
                let canvas_h = (canvas_w * 0.90).clamp(190.0, 245.0);
                let (preview_rect, _) =
                    ui.allocate_exact_size(Vec2::new(canvas_w, canvas_h), Sense::hover());
                let center = preview_rect.center();
                let painter = ui.painter_at(preview_rect);

                let n_slices = slices_copy.len().clamp(1, 8);
                let outer_r = (canvas_w * 0.38).clamp(70.0, 100.0);
                let inner_r = (outer_r * 0.44).clamp(32.0, 46.0);

                for (idx, slice) in slices_copy.iter().enumerate().take(n_slices) {
                    let span = std::f32::consts::PI * 2.0 / (n_slices as f32);
                    let r_mid = (inner_r + outer_r) * 0.5;
                    let gap = (crate::widget::DEFAULT_WEDGE_GAP_PX / r_mid).clamp(0.01, 0.04);
                    let mid_a = -std::f32::consts::PI / 2.0 + (idx as f32) * span;
                    let a0 = mid_a - span * 0.5 + gap * 0.5;
                    let a1 = mid_a + span * 0.5 - gap * 0.5;

                    let pts = crate::widget::generate_capsule_polygon(
                        center, inner_r, outer_r, a0, a1, 8.0, 5.0,
                    );
                    painter.add(egui::Shape::convex_polygon(
                        pts,
                        palette.card_bg,
                        Stroke::new(1.0_f32, palette.border),
                    ));

                    let text_pos = Pos2::new(
                        center.x + r_mid * mid_a.cos(),
                        center.y + r_mid * mid_a.sin(),
                    );
                    painter.text(
                        text_pos,
                        Align2::CENTER_CENTER,
                        &slice.label,
                        egui::FontId::proportional(design::type_scale::BODY_SM),
                        palette.text_primary,
                    );
                }

                // Center columella hub
                painter.circle_filled(center, inner_r, palette.card_hover);
                painter.circle_stroke(center, inner_r, Stroke::new(1.5_f32, palette.accent));
                painter.text(
                    center,
                    Align2::CENTER_CENTER,
                    format!("{}\n{}/8", profile_name, n_slices),
                    egui::FontId::proportional(design::type_scale::BODY_SM),
                    palette.accent,
                );

                ui.add_space(design::space::SM);

                let mut do_reset = false;
                let mut do_delete = false;

                // Profile Configuration Card
                card(ui, palette, |ui| {
                    let prof = &mut self.cfg.donut.profiles[self.selected_donut_profile_idx];
                    ui.label(
                        egui::RichText::new("Name")
                            .size(design::type_scale::BODY_SM)
                            .color(palette.text_secondary),
                    );
                    if ui
                        .add(TextEdit::singleline(&mut prof.name).desired_width(f32::INFINITY))
                        .changed()
                    {
                        *dirty = true;
                    }
                    ui.add_space(design::space::XS);

                    ui.horizontal_wrapped(|ui| {
                        if ui
                            .checkbox(
                                &mut prof.is_mixed_selection,
                                "Use for mixed selections (files of different types)",
                            )
                            .changed()
                        {
                            *dirty = true;
                        }
                    });
                    ui.add_space(design::space::XS);

                    ui.label(
                        egui::RichText::new("Used for these extensions")
                            .size(design::type_scale::BODY_SM)
                            .color(palette.text_secondary),
                    );
                    let mut ext_to_remove = None;

                    ScrollArea::vertical()
                        .id_salt("ext_tags_scroll")
                        .max_height(100.0)
                        .show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                for (ext_idx, ext) in prof.extensions.iter().enumerate() {
                                    let pill_btn = egui::Button::new(
                                        egui::RichText::new(format!("{} x", ext))
                                            .size(design::type_scale::BODY_SM)
                                            .color(palette.text_secondary),
                                    )
                                    .fill(palette.card_bg)
                                    .stroke(Stroke::new(1.0_f32, palette.border))
                                    .rounding(Rounding::same(design::radius::PILL));
                                    let pill_resp = ui.add(pill_btn);
                                    design::paint_pill_ring(
                                        ui.painter(),
                                        pill_resp.rect,
                                        Rounding::same(design::radius::PILL),
                                        design::pill_state(&pill_resp, false),
                                        palette.accent,
                                    );
                                    if pill_resp.clicked() {
                                        ext_to_remove = Some(ext_idx);
                                    }
                                }
                            });
                        });

                    if let Some(idx) = ext_to_remove {
                        if idx < prof.extensions.len() {
                            prof.extensions.remove(idx);
                            *dirty = true;
                        }
                    }

                    ui.horizontal_wrapped(|ui| {
                        ui.add(
                            TextEdit::singleline(&mut self.new_ext_input)
                                .hint_text(".jpg")
                                .desired_width(100.0),
                        );
                        if ui.button("+ Add extension").clicked() {
                            let clean = self.new_ext_input.trim();
                            if !clean.is_empty() {
                                let formatted = if clean.starts_with('.') {
                                    clean.to_string()
                                } else {
                                    format!(".{}", clean)
                                };
                                if !prof.extensions.contains(&formatted) {
                                    prof.extensions.push(formatted);
                                    self.new_ext_input.clear();
                                    *dirty = true;
                                }
                            }
                        }
                    });

                    ui.add_space(design::space::XS);
                    ui.horizontal_wrapped(|ui| {
                        if ui.button("Reset to default").clicked() {
                            do_reset = true;
                        }
                        if profile_id != "images" && ui.button("Delete profile").clicked() {
                            do_delete = true;
                        }
                    });
                });

                if do_reset {
                    let default_cfg = DonutConfig::default();
                    if let Some(def_p) = default_cfg.profiles.get(self.selected_donut_profile_idx) {
                        self.cfg.donut.profiles[self.selected_donut_profile_idx] = def_p.clone();
                        *dirty = true;
                    }
                }
                if do_delete && self.cfg.donut.profiles.len() > 1 {
                    self.cfg
                        .donut
                        .profiles
                        .remove(self.selected_donut_profile_idx);
                    self.selected_donut_profile_idx = 0;
                    *dirty = true;
                }

                ui.add_space(design::space::SM);

                // Appearance Card
                card(ui, palette, |ui| {
                    ui.label(
                        egui::RichText::new("Appearance")
                            .size(design::type_scale::TITLE_SM)
                            .strong()
                            .color(palette.text_primary),
                    );
                    ui.add_space(design::space::XS);

                    let slider_w = (col2_w - 90.0).clamp(110.0, 180.0);
                    ui.spacing_mut().slider_width = slider_w;

                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Size")
                                .size(design::type_scale::BODY_SM)
                                .color(palette.text_secondary),
                        );
                        if ui
                            .add(
                                egui::Slider::new(
                                    &mut self.cfg.donut.appearance.size,
                                    280.0..=480.0,
                                )
                                .suffix(" px"),
                            )
                            .changed()
                        {
                            *dirty = true;
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Hole")
                                .size(design::type_scale::BODY_SM)
                                .color(palette.text_secondary),
                        );
                        if ui
                            .add(
                                egui::Slider::new(&mut self.cfg.donut.appearance.hole, 30.0..=80.0)
                                    .suffix(" px"),
                            )
                            .changed()
                        {
                            *dirty = true;
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Opacity")
                                .size(design::type_scale::BODY_SM)
                                .color(palette.text_secondary),
                        );
                        if ui
                            .add(egui::Slider::new(
                                &mut self.cfg.donut.appearance.opacity,
                                0.20..=1.00,
                            ))
                            .changed()
                        {
                            *dirty = true;
                        }
                    });
                });
            },
        );
    }

    fn render_donut_col3(
        &mut self,
        ui: &mut Ui,
        col3_w: f32,
        palette: &ThemePalette,
        dirty: &mut bool,
    ) {
        ui.allocate_ui_with_layout(
            Vec2::new(col3_w, ui.available_height()),
            Layout::top_down(Align::Min),
            |ui| {
                ui.label(
                    egui::RichText::new("Available presets")
                        .size(design::type_scale::TITLE_SM)
                        .strong()
                        .color(palette.text_primary),
                );
                ui.add_space(design::space::XS);

                ui.horizontal_wrapped(|ui| {
                    let _ = ui.button("+ New preset");
                    let _ = ui.button("New group");
                    let _ = ui.button("Copy to folder");
                });
                ui.add_space(design::space::XS);

                ui.add(
                    TextEdit::singleline(&mut self.donut_preset_search)
                        .hint_text("Search presets")
                        .desired_width(col3_w - 12.0),
                );
                ui.add_space(design::space::XS);

                // Category filter pills
                let categories = [
                    "All",
                    "Image",
                    "Document",
                    "Video & audio",
                    "Edit",
                    "3D & CAD",
                ];
                ui.horizontal_wrapped(|ui| {
                    for cat in categories {
                        let active = self.donut_preset_category == cat;
                        if design::segmented_pill(ui, palette, active, cat).clicked() {
                            self.donut_preset_category = cat.to_string();
                        }
                    }
                });
                ui.add_space(design::space::SM);

                let available_catalog = [
                    ("Edit Group of 6", "Edit", true),
                    ("Pages per sheet Group of 5", "Document", true),
                    ("Flip Group of 2", "Edit", true),
                    ("Crop video Group of 6", "Video & audio", true),
                    ("JPG 70%", "Image", false),
                    ("TIFF", "Image", false),
                    ("BMP", "Image", false),
                    ("ICO", "Image", false),
                    ("AVIF", "Image", false),
                    ("HEIC", "Image", false),
                    ("Grayscale Same format", "Edit", true),
                    ("Trim transparent borders", "Edit", true),
                    ("Pixelate / blur Same format", "Edit", true),
                    ("Annotate Same format", "Edit", true),
                    ("Rotate left PDF", "Document", true),
                    ("Rotate right PDF", "Document", true),
                    ("2 pages per sheet PDF", "Document", true),
                    ("4 pages per sheet PDF", "Document", true),
                    ("6 pages per sheet PDF", "Document", true),
                    ("8 pages per sheet PDF", "Document", true),
                    ("9 pages per sheet PDF", "Document", true),
                    ("Rotate left Same format", "Edit", true),
                    ("Rotate right Same format", "Edit", true),
                    ("Rotate 180 Same format", "Edit", true),
                    ("Flip horizontal Same format", "Edit", true),
                    ("Flip vertical Same format", "Edit", true),
                    ("TIFF 300dpi", "Image", false),
                    ("Merge PDF", "Document", true),
                    ("Long image PNG", "Image", false),
                    ("Split PDF", "Document", true),
                    ("Compress PDF", "Document", true),
                    ("Searchable PDF", "Document", true),
                    ("STL", "3D & CAD", false),
                    ("OBJ", "3D & CAD", false),
                    ("PLY", "3D & CAD", false),
                    ("DXF", "3D & CAD", false),
                    ("DWG", "3D & CAD", false),
                ];

                let search_p = self.donut_preset_search.trim().to_lowercase();
                let cat_filter = &self.donut_preset_category;

                ScrollArea::vertical()
                    .id_salt("donut_presets_catalog_scroll")
                    .max_height(420.0)
                    .show(ui, |ui| {
                        for (preset_name, cat, is_action) in available_catalog {
                            if cat_filter != "All" && cat != cat_filter {
                                continue;
                            }
                            if !search_p.is_empty()
                                && !preset_name.to_lowercase().contains(&search_p)
                            {
                                continue;
                            }

                            let card_w = (col3_w - 16.0).max(180.0);
                            let btn = egui::Button::new(
                                egui::RichText::new(preset_name)
                                    .size(design::type_scale::BODY_SM)
                                    .color(palette.text_primary),
                            )
                            .wrap()
                            .fill(palette.card_bg)
                            .stroke(Stroke::new(1.0_f32, palette.border))
                            .rounding(Rounding::same(design::radius::SM));

                            if ui.add_sized(Vec2::new(card_w, 28.0), btn).clicked()
                                && self.selected_donut_profile_idx < self.cfg.donut.profiles.len()
                            {
                                let prof =
                                    &mut self.cfg.donut.profiles[self.selected_donut_profile_idx];
                                let target_list = if self.donut_view_back {
                                    &mut prof.back_slices
                                } else {
                                    &mut prof.front_slices
                                };
                                let max_s = if self.donut_view_back {
                                    prof.max_back_slices
                                } else {
                                    prof.max_front_slices
                                };
                                if target_list.len() < max_s {
                                    target_list.push(if is_action {
                                        DonutSlicePreset::action(
                                            preset_name,
                                            preset_name,
                                            preset_name,
                                        )
                                    } else {
                                        DonutSlicePreset::format(preset_name)
                                    });
                                    *dirty = true;
                                }
                            }
                            ui.add_space(2.0);
                        }
                    });
            },
        );
    }

    fn render_shortcuts(&mut self, ui: &mut Ui, palette: &ThemePalette, dirty: &mut bool) {
        // Rebind capture: while listening, the next pressed key (Esc cancels)
        // becomes the binding for the pending action. Raw events are read
        // here, and the studio dispatch skips its own shortcuts while a
        // capture is pending so the key lands in the binding instead.
        if let Some(action_id) = self.capturing_action.clone() {
            let events: Vec<egui::Event> = ui.ctx().input(|i| i.events.clone());
            for event in events {
                if let egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } = event
                {
                    if key == egui::Key::Escape {
                        self.capturing_action = None;
                    } else {
                        self.cfg
                            .shortcuts
                            .insert(action_id.clone(), key_binding_display(modifiers, key));
                        *dirty = true;
                        self.capturing_action = None;
                    }
                    break;
                }
            }
        }

        ui.label(
            egui::RichText::new(
                "Rebind the studio shortcuts and tune the global gesture engine. Bindings are saved to config.json.",
            )
            .size(design::type_scale::BODY_SM)
            .color(palette.text_secondary),
        );
        ui.add_space(design::space::XS);

        // App shortcuts card: the three rebindable studio bindings.
        card(ui, palette, |ui| {
            ui.label(
                egui::RichText::new("App shortcuts")
                    .size(design::type_scale::TITLE_SM)
                    .strong()
                    .color(palette.text_primary),
            );
            ui.label(
                egui::RichText::new(
                    "Click Rebind, then press the new key combination. Esc cancels.",
                )
                .size(design::type_scale::CAPTION)
                .color(palette.text_secondary),
            );
            ui.add_space(design::space::XS);

            for (action_id, action_label) in [
                ("palette", "Command palette"),
                ("new_tab", "New tab"),
                ("close_tab", "Close tab"),
            ] {
                let binding = self.cfg.shortcut_for(action_id);
                let capturing_this = self.capturing_action.as_deref() == Some(action_id);
                let shown = if capturing_this {
                    "Press keys... (Esc to cancel)".to_string()
                } else {
                    binding
                };
                ui.add_space(design::space::XS);
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new(action_label)
                                .size(design::type_scale::CAPTION)
                                .color(palette.text_muted),
                        );
                        ui.label(
                            egui::RichText::new(shown)
                                .size(design::type_scale::BODY)
                                .strong()
                                .color(if capturing_this {
                                    palette.accent
                                } else {
                                    palette.text_primary
                                }),
                        );
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let btn_label = if capturing_this {
                            "Listening..."
                        } else {
                            "Rebind"
                        };
                        if ui.button(btn_label).clicked() {
                            self.capturing_action = Some(action_id.to_string());
                        }
                    });
                });
            }
        });

        ui.add_space(design::space::MD);

        // Gestures card: the real gesture-engine keys from config.
        card(ui, palette, |ui| {
            ui.label(
                egui::RichText::new("Gestures")
                    .size(design::type_scale::TITLE_SM)
                    .strong()
                    .color(palette.text_primary),
            );
            ui.add_space(design::space::XS);

            let gtype_lbl = self.cfg.gesture_type.clone();
            let gtype_mut = &mut self.cfg.gesture_type;
            setting_row(
                ui,
                palette,
                "Gesture type",
                &gtype_lbl,
                dirty,
                |ui, dirty| {
                    let mut changed = false;
                    egui::ComboBox::from_id_salt("prefs_gesture_type")
                        .selected_text(gtype_lbl.clone())
                        .show_ui(ui, |ui| {
                            for (value, label) in [
                                ("shake", "Shake"),
                                ("circle", "Circle"),
                                ("both", "Shake or circle"),
                            ] {
                                if ui
                                    .selectable_value(gtype_mut, value.to_string(), label)
                                    .changed()
                                {
                                    changed = true;
                                }
                            }
                        });
                    if changed {
                        *dirty = true;
                    }
                },
            );

            let gw = self.cfg.enable_gesture_wakeup;
            let gw_lbl = on_off(gw).to_string();
            let gw_mut = &mut self.cfg.enable_gesture_wakeup;
            setting_row(
                ui,
                palette,
                "Gesture wake-up",
                &gw_lbl,
                dirty,
                |ui, dirty| {
                    if ui.checkbox(gw_mut, "").changed() {
                        *dirty = true;
                    }
                },
            );

            let hw = self.cfg.enable_hotkey_wakeup;
            let hw_lbl = on_off(hw).to_string();
            let hw_mut = &mut self.cfg.enable_hotkey_wakeup;
            setting_row(
                ui,
                palette,
                "Hotkey wake-up",
                &hw_lbl,
                dirty,
                |ui, dirty| {
                    if ui.checkbox(hw_mut, "").changed() {
                        *dirty = true;
                    }
                },
            );

            let sens = self.cfg.shake_sensitivity_px;
            let sens_lbl = format!("{sens} px");
            let sens_mut = &mut self.cfg.shake_sensitivity_px;
            setting_row(
                ui,
                palette,
                "Shake sensitivity",
                &sens_lbl,
                dirty,
                |ui, dirty| {
                    if ui
                        .add(egui::Slider::new(sens_mut, 5.0..=100.0).show_value(false))
                        .changed()
                    {
                        *dirty = true;
                    }
                },
            );

            let to = self.cfg.gesture_timeout_ms;
            let to_lbl = format!("{to} ms");
            let to_mut = &mut self.cfg.gesture_timeout_ms;
            setting_row(
                ui,
                palette,
                "Gesture timeout",
                &to_lbl,
                dirty,
                |ui, dirty| {
                    if ui
                        .add(egui::Slider::new(to_mut, 150..=1000).show_value(false))
                        .changed()
                    {
                        *dirty = true;
                    }
                },
            );

            let dd = self.cfg.deep_drag_detection_only;
            let dd_lbl = on_off(dd).to_string();
            let dd_mut = &mut self.cfg.deep_drag_detection_only;
            setting_row(
                ui,
                palette,
                "Deep drag detection only",
                &dd_lbl,
                dirty,
                |ui, dirty| {
                    if ui.checkbox(dd_mut, "").changed() {
                        *dirty = true;
                    }
                },
            );
        });
    }

    fn render_fields(&mut self, ui: &mut Ui, palette: &ThemePalette, dirty: &mut bool) {
        ui.label(
            egui::RichText::new("Turn off the fields you don't work in. A format that belongs only to fields that are off is left out of the donut's slices, \"Available presets\" in Settings -> Donut toolbar, the list in Settings -> Presets and a preset's \"Convert to\" list. Mossambi still recognizes its files and the command line still converts to it, and a file you drop still offers the formats of its own fields.")
                .size(design::type_scale::BODY_SM)
                .color(palette.text_secondary),
        );
        ui.add_space(design::space::MD);

        card(ui, palette, |ui| {
            for spec in FieldsConfig::SPECS {
                let current = self.cfg.fields.is_field_enabled(spec.key);

                let w = ui.available_width();
                ui.allocate_ui_with_layout(
                    Vec2::new(w, 44.0),
                    Layout::left_to_right(Align::Center),
                    |ui| {
                        ui.vertical(|ui| {
                            ui.label(
                                egui::RichText::new(spec.title)
                                    .size(design::type_scale::BODY)
                                    .color(palette.text_primary),
                            );
                            ui.label(
                                egui::RichText::new(format!("Formats: {}", spec.formats))
                                    .size(design::type_scale::CAPTION)
                                    .color(palette.text_muted),
                            );
                        });

                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let text = if current { "On" } else { "Off" };
                            if design::segmented_pill(ui, palette, current, text).clicked() {
                                self.cfg.fields.set_field_enabled(spec.key, !current);
                                *dirty = true;
                            }
                        });
                    },
                );
                ui.add_space(design::space::XS);
            }
        });

        ui.add_space(design::space::MD);
        ui.label(
            egui::RichText::new("Always shown: Image, PDF, Document, Spreadsheet, Presentation, HWP, Text, Video, Audio, E-book, Archive, Vector, Subtitle, Data")
                .size(design::type_scale::CAPTION)
                .color(palette.text_muted),
        );
    }

    fn render_look_preset_card(
        ui: &mut egui::Ui,
        palette: &ThemePalette,
        preset: LookPreset,
        is_active: bool,
    ) -> bool {
        let card_size = Vec2::new(140.0, 105.0);
        let (rect, resp) = ui.allocate_exact_size(card_size, Sense::click());
        let is_hovered = resp.hovered();
        let painter = ui.painter_at(rect);

        let bg = if is_active {
            Color32::from_rgba_unmultiplied(
                palette.accent.r(),
                palette.accent.g(),
                palette.accent.b(),
                if palette.is_dark { 35 } else { 20 },
            )
        } else if is_hovered {
            palette.card_hover
        } else {
            palette.card_bg
        };

        let stroke = if is_active {
            Stroke::new(2.0_f32, palette.accent)
        } else if is_hovered {
            Stroke::new(1.0_f32, palette.text_secondary)
        } else {
            Stroke::new(1.0_f32, palette.border)
        };

        painter.rect(rect, Rounding::same(design::radius::MD), bg, stroke);

        // Miniature wireframe preview (top 56px of card)
        let preview_rect =
            Rect::from_min_size(rect.min + Vec2::new(8.0, 8.0), Vec2::new(124.0, 52.0));
        painter.rect(
            preview_rect,
            Rounding::same(4.0),
            palette.window_bg,
            Stroke::new(1.0_f32, palette.border),
        );

        // Mini header bar
        let header_h = 12.0;
        let header_rect =
            Rect::from_min_size(preview_rect.min, Vec2::new(preview_rect.width(), header_h));
        painter.rect_filled(header_rect, Rounding::same(3.0), palette.sidebar_bg);

        match preset {
            LookPreset::Finder => {
                // Traffic lights on left
                painter.circle_filled(
                    header_rect.min + Vec2::new(6.0, 6.0),
                    2.0,
                    Color32::from_rgb(255, 95, 86),
                );
                painter.circle_filled(
                    header_rect.min + Vec2::new(12.0, 6.0),
                    2.0,
                    Color32::from_rgb(255, 189, 46),
                );
                painter.circle_filled(
                    header_rect.min + Vec2::new(18.0, 6.0),
                    2.0,
                    Color32::from_rgb(39, 201, 63),
                );
            }
            LookPreset::WindowsExplorer => {
                // Mini tabs on top left
                painter.rect_filled(
                    Rect::from_min_size(
                        header_rect.min + Vec2::new(4.0, 2.0),
                        Vec2::new(28.0, 8.0),
                    ),
                    Rounding::same(2.0),
                    palette.card_bg,
                );
                // Caption buttons on right
                painter.rect_filled(
                    Rect::from_min_size(
                        header_rect.max - Vec2::new(14.0, 8.0),
                        Vec2::new(6.0, 4.0),
                    ),
                    Rounding::ZERO,
                    palette.text_muted,
                );
            }
            LookPreset::Nautilus => {
                // Centered pill
                painter.rect_filled(
                    Rect::from_center_size(header_rect.center(), Vec2::new(32.0, 7.0)),
                    Rounding::same(3.0),
                    palette.card_bg,
                );
                // Caption buttons right
                painter.circle_filled(
                    header_rect.right_center() - Vec2::new(8.0, 0.0),
                    2.0,
                    palette.text_muted,
                );
            }
            LookPreset::Thunar => {
                // Menu bar dashes
                painter.line_segment(
                    [
                        header_rect.min + Vec2::new(4.0, 6.0),
                        header_rect.min + Vec2::new(34.0, 6.0),
                    ],
                    Stroke::new(1.5_f32, palette.text_muted),
                );
                // Caption button right
                painter.circle_filled(
                    header_rect.right_center() - Vec2::new(8.0, 0.0),
                    2.0,
                    palette.text_muted,
                );
            }
        }

        // Mini sidebar on left
        let mini_sidebar_w = 26.0;
        let mini_sidebar_rect = Rect::from_min_size(
            preview_rect.min + Vec2::new(0.0, header_h),
            Vec2::new(mini_sidebar_w, preview_rect.height() - header_h),
        );
        painter.rect_filled(mini_sidebar_rect, Rounding::ZERO, palette.sidebar_bg);

        // Mini content lines
        let line_x = preview_rect.min.x + mini_sidebar_w + 6.0;
        let line_w = preview_rect.max.x - line_x - 6.0;
        for i in 0..3 {
            let line_y = preview_rect.min.y + header_h + 8.0 + (i as f32 * 8.0);
            painter.line_segment(
                [
                    Pos2::new(line_x, line_y),
                    Pos2::new(line_x + line_w, line_y),
                ],
                Stroke::new(1.5_f32, palette.border),
            );
        }

        // Preset Label
        let label_y = rect.min.y + 68.0;
        painter.text(
            Pos2::new(rect.center().x, label_y),
            Align2::CENTER_TOP,
            preset.label(),
            FontId::proportional(design::type_scale::BODY_SM),
            if is_active {
                palette.accent
            } else {
                palette.text_primary
            },
        );

        // Preset Subtitle
        let sub_y = rect.min.y + 86.0;
        painter.text(
            Pos2::new(rect.center().x, sub_y),
            Align2::CENTER_TOP,
            match preset {
                LookPreset::Finder => "macOS Finder",
                LookPreset::WindowsExplorer => "Windows 11",
                LookPreset::Nautilus => "GNOME Nautilus",
                LookPreset::Thunar => "XFCE Thunar",
            },
            FontId::proportional(design::type_scale::SMALL),
            palette.text_muted,
        );

        resp.clicked()
    }

    fn render_appearance(
        &mut self,
        ui: &mut Ui,
        palette: &ThemePalette,
        current_theme: &mut ThemeStyle,
        dirty: &mut bool,
    ) {
        section(ui, palette, "Look Presets");
        ui.label(
            egui::RichText::new("Choose an operating-system look preset to set sidebar, toolbar, layout, density, icon pack, corner radius, and accent. Individual settings stay overridable.")
                .size(design::type_scale::BODY_SM)
                .color(palette.text_secondary),
        );
        ui.add_space(design::space::SM);
        card(ui, palette, |ui| {
            let active_preset = self.cfg.look_preset;
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(design::space::SM, design::space::SM);
                for preset in LookPreset::all() {
                    let is_active = active_preset == Some(preset);
                    if Self::render_look_preset_card(ui, palette, preset, is_active) {
                        self.cfg.apply_look_preset(preset);
                        *current_theme = ThemeStyle::from(self.cfg.theme_mode);
                        *dirty = true;
                    }
                }
            });
            if let Some(p) = self.cfg.look_preset {
                ui.add_space(design::space::SM);
                ui.label(
                    egui::RichText::new(p.description())
                        .size(design::type_scale::SMALL)
                        .color(palette.text_secondary),
                );
            }
        });
        ui.add_space(design::space::SM);

        section(ui, palette, "Theme");
        ui.label(
            egui::RichText::new("Pick a theme. Every theme has a light and a dark version.")
                .size(design::type_scale::BODY_SM)
                .color(palette.text_secondary),
        );
        ui.add_space(design::space::SM);
        card(ui, palette, |ui| {
            let ap_mut = &mut self.cfg.appearance;
            setting_row(ui, palette, "Appearance", "", dirty, |ui, dirty| {
                for v in crate::core::Appearance::all().into_iter().rev() {
                    let active = *ap_mut == v;
                    if design::segmented_pill(ui, palette, active, v.label()).clicked() {
                        *ap_mut = v;
                        *dirty = true;
                    }
                }
            });
            ui.add_space(design::space::XS);
            ui.separator();
            ui.add_space(design::space::XS);
            let ds_mut = &mut self.cfg.design_style;
            setting_row(
                ui,
                palette,
                "Design Style",
                ds_mut.description(),
                dirty,
                |ui, dirty| {
                    for s in crate::core::DesignStyle::all() {
                        let active = *ds_mut == s;
                        if design::segmented_pill(ui, palette, active, s.label()).clicked() {
                            *ds_mut = s;
                            *dirty = true;
                        }
                    }
                },
            );
        });
        ui.add_space(design::space::SM);
        // Compact swatch grid (2 responsive columns fitting cleanly in bounded card).
        let themes = ThemeStyle::all();
        let btn_w = ((ui.available_width() - 36.0) * 0.5).clamp(160.0, 235.0);
        for chunk in themes.chunks(2) {
            ui.horizontal(|ui| {
                for t in chunk {
                    let is_active = *current_theme == *t;
                    let pt = t.to_pantone();
                    let (bg_r, bg_g, bg_b) = pt.bg.to_rgb();
                    let (acc_r, acc_g, acc_b) = pt.accent.to_rgb();
                    let bg_col = Color32::from_rgb(bg_r, bg_g, bg_b);
                    let acc_col = Color32::from_rgb(acc_r, acc_g, acc_b);
                    let (tx_r, tx_g, tx_b) = pt.text_primary.to_rgb();
                    // Label in the swatch's own text colour: the app's
                    // secondary text was unreadable on light swatches.
                    let btn = egui::Button::new(
                        egui::RichText::new(pt.name)
                            .size(design::type_scale::MICRO)
                            .color(if is_active {
                                palette.text_primary
                            } else {
                                Color32::from_rgb(tx_r, tx_g, tx_b)
                            }),
                    )
                    .fill(if is_active {
                        palette.card_hover
                    } else {
                        bg_col
                    })
                    .stroke(Stroke::new(
                        if is_active { 1.5_f32 } else { 0.5_f32 },
                        if is_active {
                            palette.accent
                        } else {
                            palette.border
                        },
                    ))
                    .rounding(Rounding::same(design::radius::SM));
                    if ui.add_sized(Vec2::new(btn_w, 26.0), btn).clicked() {
                        *current_theme = *t;
                        self.cfg.theme_mode = (*t).into();
                        *dirty = true;
                    }
                    // accent dot
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
                    ui.painter().circle_filled(rect.center(), 4.0, acc_col);
                    ui.add_space(design::space::XS);
                }
            });
            ui.add_space(design::space::XS);
        }

        section(ui, palette, "Glass & window");
        card(ui, palette, |ui| {
            let sc = self.cfg.ui_scale;
            let sc_mut = &mut self.cfg.ui_scale;
            setting_row(
                ui,
                palette,
                "Interface size",
                &format!("{:.0}%", sc * 100.0),
                dirty,
                |ui, dirty| {
                    for v in [1.5_f32, 1.25, 1.1, 1.0, 0.9] {
                        let label = format!("{:.0}%", v * 100.0);
                        let active = (*sc_mut - v).abs() < 0.01;
                        if design::segmented_pill(ui, palette, active, &label).clicked() {
                            *sc_mut = v;
                            *dirty = true;
                        }
                    }
                },
            );
            let glass_mut = &mut self.cfg.glass_effect;
            setting_row(ui, palette, "Glass effect", "", dirty, |ui, dirty| {
                for (v, lab) in [
                    (GlassEffect::FrostedGlass, "Frosted"),
                    (GlassEffect::LiquidGlass, "Liquid"),
                    (GlassEffect::Acrylic, "Acrylic"),
                    (GlassEffect::Mica, "Mica"),
                    (GlassEffect::Opaque, "Opaque"),
                ] {
                    let active = *glass_mut == v;
                    if design::segmented_pill(ui, palette, active, lab).clicked() {
                        *glass_mut = v;
                        *dirty = true;
                    }
                }
            });

            let op = self.cfg.window_opacity;
            let op_lbl = format!("{:.0}%", op * 100.0);
            let op_mut = &mut self.cfg.window_opacity;
            setting_row(
                ui,
                palette,
                "Window opacity",
                &op_lbl,
                dirty,
                |ui, dirty| {
                    if ui
                        .add(egui::Slider::new(op_mut, 0.3..=1.0).show_value(false))
                        .changed()
                    {
                        *dirty = true;
                    }
                },
            );

            let ms_mut = &mut self.cfg.menu_style;
            setting_row(ui, palette, "Menu style", "", dirty, |ui, dirty| {
                for (v, lab) in [
                    (MenuStyle::Finder, "Finder"),
                    (MenuStyle::Windows, "Windows"),
                    (MenuStyle::Linux, "Linux"),
                ] {
                    let active = *ms_mut == v;
                    if design::segmented_pill(ui, palette, active, lab).clicked() {
                        *ms_mut = v;
                        *dirty = true;
                    }
                }
            });

            let cs_mut = &mut self.cfg.corner_style;
            setting_row(ui, palette, "Corner style", "", dirty, |ui, dirty| {
                for v in CornerStyle::all() {
                    let active = *cs_mut == v;
                    if design::segmented_pill(ui, palette, active, v.label()).clicked() {
                        *cs_mut = v;
                        *dirty = true;
                    }
                }
            });

            let rd_mut = &mut self.cfg.row_density;
            setting_row(ui, palette, "Row density", "", dirty, |ui, dirty| {
                for v in RowDensity::all() {
                    let active = *rd_mut == v;
                    if design::segmented_pill(ui, palette, active, v.label()).clicked() {
                        *rd_mut = v;
                        *dirty = true;
                    }
                }
            });

            let ss_mut = &mut self.cfg.sidebar_style;
            setting_row(ui, palette, "Sidebar style", "", dirty, |ui, dirty| {
                for v in SidebarStyle::all() {
                    let active = *ss_mut == v;
                    if design::segmented_pill(ui, palette, active, v.label()).clicked() {
                        *ss_mut = v;
                        *dirty = true;
                    }
                }
            });

            let es_mut = &mut self.cfg.explorer_style;
            setting_row(ui, palette, "Toolbar layout", "", dirty, |ui, dirty| {
                for v in ExplorerStyle::all() {
                    let active = *es_mut == *v;
                    if design::segmented_pill(ui, palette, active, v.name()).clicked() {
                        *es_mut = *v;
                        *dirty = true;
                    }
                }
            });

            // -- Context Menu customization (user-defined entries, Phase 4) --
            egui::CollapsingHeader::new(
                egui::RichText::new("Context Menu")
                    .size(design::type_scale::BODY_SM)
                    .strong()
                    .color(palette.text_primary),
            )
            .default_open(false)
            .show(ui, |ui| {
                ui.add_space(design::space::XS);
                let items = &mut self.cfg.context_menu_items;
                let mut removed: Option<usize> = None;
                for (idx, item) in items.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!("#{}", idx + 1))
                                .size(design::type_scale::SMALL)
                                .strong()
                                .color(palette.text_secondary),
                        );
                        ui.add(
                            TextEdit::singleline(&mut item.label)
                                .id_salt(format!("ctx_label_{idx}"))
                                .desired_width(140.0)
                                .hint_text("Menu label"),
                        );
                        ui.add(
                            TextEdit::singleline(&mut item.value)
                                .id_salt(format!("ctx_value_{idx}"))
                                .desired_width(200.0)
                                .hint_text("Command / app / script / URL"),
                        );
                        ui.add(
                            TextEdit::singleline(&mut item.args)
                                .id_salt(format!("ctx_args_{idx}"))
                                .desired_width(110.0)
                                .hint_text("Args (%p = path)"),
                        );
                    });
                    ui.horizontal(|ui| {
                        egui::ComboBox::from_id_salt(format!("ctx_kind_{idx}"))
                            .selected_text(item.kind.name())
                            .show_ui(ui, |ui| {
                                for k in CustomMenuKind::all() {
                                    ui.selectable_value(&mut item.kind, *k, k.name());
                                }
                            });
                        egui::ComboBox::from_id_salt(format!("ctx_scope_{idx}"))
                            .selected_text(item.show_in.name())
                            .show_ui(ui, |ui| {
                                for s in CustomMenuScope::all() {
                                    ui.selectable_value(&mut item.show_in, *s, s.name());
                                }
                            });
                        if ui
                            .button(egui::RichText::new("Remove").size(design::type_scale::SMALL))
                            .clicked()
                        {
                            removed = Some(idx);
                        }
                    });
                    ui.add_space(design::space::XS);
                }
                if let Some(idx) = removed {
                    items.remove(idx);
                    *dirty = true;
                }
                if ui.button("Add custom item").clicked() {
                    items.push(CustomMenuItem {
                        label: "New item".to_string(),
                        kind: CustomMenuKind::Command,
                        value: String::new(),
                        args: String::new(),
                        show_in: CustomMenuScope::All,
                    });
                    *dirty = true;
                }
            });

            let cm_mut = &mut self.cfg.checkbox_mode;
            setting_row(ui, palette, "Checkbox mode", "", dirty, |ui, dirty| {
                for (v, lab) in [
                    (CheckboxMode::Always, "Always"),
                    (CheckboxMode::Hover, "On hover"),
                    (CheckboxMode::Off, "Hidden"),
                ] {
                    let active = *cm_mut == v;
                    if design::segmented_pill(ui, palette, active, lab).clicked() {
                        *cm_mut = v;
                        *dirty = true;
                    }
                }
            });
        });

        section(ui, palette, "Icons & typography");
        card(ui, palette, |ui| {
            let icon_pack_id = self.cfg.icon_pack.clone();
            let ip_lbl = IconPack::from_config_id(&icon_pack_id).label().to_string();
            setting_row(ui, palette, "Icon pack", &ip_lbl, dirty, |ui, dirty| {
                // Show as a compact menu button listing all packs.
                ui.menu_button("Change", |menu_ui| {
                    for pack in IconPack::all() {
                        if menu_ui.button(pack.label()).clicked() {
                            self.cfg.icon_pack = pack.id().to_string();
                            *dirty = true;
                        }
                    }
                });
            });

            // A real picker (the free-text box was never read by the app).
            let font_now = self
                .cfg
                .font_family
                .clone()
                .unwrap_or_else(|| "System".to_string());
            let font_mut = &mut self.cfg.font_family;
            setting_row(ui, palette, "Font", "", dirty, |ui, dirty| {
                egui::ComboBox::from_id_salt("prefs_font_family")
                    .selected_text(font_now.as_str())
                    .width(220.0)
                    .show_ui(ui, |ui| {
                        for (label, _) in crate::gui::studio_app::ui_font_choices() {
                            if ui
                                .selectable_label(font_now.eq_ignore_ascii_case(label), label)
                                .clicked()
                            {
                                *font_mut = if label == "System" {
                                    None
                                } else {
                                    Some(label.clone())
                                };
                                *dirty = true;
                            }
                        }
                    });
            });

            let mut shade = self.cfg.citrus_shade.clone();
            let shade_lbl = if shade.is_empty() {
                "Default".to_string()
            } else {
                shade.clone()
            };
            setting_row(
                ui,
                palette,
                "Citrus shade",
                &shade_lbl,
                dirty,
                |ui, dirty| {
                    if ui
                        .add(TextEdit::singleline(&mut shade).desired_width(140.0))
                        .changed()
                    {
                        self.cfg.citrus_shade = shade.clone();
                        *dirty = true;
                    }
                },
            );

            let tui = self.cfg.tui_transparent_bg;
            let tui_lbl = on_off(tui).to_string();
            let tui_mut = &mut self.cfg.tui_transparent_bg;
            setting_row(
                ui,
                palette,
                "Transparent TUI background",
                &tui_lbl,
                dirty,
                |ui, dirty| {
                    if ui.checkbox(tui_mut, "").changed() {
                        *dirty = true;
                    }
                },
            );
        });

        section(ui, palette, "Motion & Animations");
        ui.label(
            egui::RichText::new(
                "Configure physical spring dynamics, motion curves, and animations across Widget, Explorer, and TUI.",
            )
            .size(design::type_scale::BODY_SM)
            .color(palette.text_secondary),
        );
        ui.add_space(design::space::SM);

        // Motion Curve Preset for General Surfaces
        let current_style = self.cfg.animations.widget;
        let style_lbl = current_style.name().to_string();
        setting_row(
            ui,
            palette,
            "Motion curve",
            &style_lbl,
            dirty,
            |ui, dirty| {
                egui::ComboBox::from_id_salt("prefs_master_motion_curve_combo")
                    .selected_text(self.cfg.animations.widget.name())
                    .width(180.0)
                    .show_ui(ui, |ui| {
                        for &style in AnimStyle::all() {
                            let is_active = self.cfg.animations.widget == style;
                            if ui.selectable_label(is_active, style.name()).clicked() {
                                self.cfg.animations.hover = style;
                                self.cfg.animations.selection = style;
                                self.cfg.animations.open = style;
                                self.cfg.animations.peek = style;
                                self.cfg.animations.toolbar = style;
                                self.cfg.animations.widget = style;
                                self.cfg.animations.scroll = style;
                                self.cfg.animations.tui = style;
                                *dirty = true;
                            }
                        }
                    });
            },
        );

        // Independent Per-Surface Motion Tuning (Unlimited Animation Customization)
        egui::CollapsingHeader::new(
            egui::RichText::new("Independent Surface Motion Overrides")
                .size(design::type_scale::BODY_SM)
                .strong()
                .color(palette.text_primary),
        )
        .default_open(false)
        .show(ui, |ui| {
            ui.add_space(design::space::XS);
            for (id, label, accessor) in AnimationSettings::all_fields() {
                let cur = *accessor(&mut self.cfg.animations);
                let cur_name = cur.name().to_string();
                setting_row(ui, palette, label, &cur_name, dirty, |ui, dirty| {
                    egui::ComboBox::from_id_salt(format!("prefs_motion_override_{}", id))
                        .selected_text(cur.name())
                        .width(180.0)
                        .show_ui(ui, |ui| {
                            for &style in AnimStyle::all() {
                                let is_active = *accessor(&mut self.cfg.animations) == style;
                                if ui.selectable_label(is_active, style.name()).clicked() {
                                    *accessor(&mut self.cfg.animations) = style;
                                    *dirty = true;
                                }
                            }
                        });
                });
            }
        });

        // Spring Physics Controls
        let stiffness = self.cfg.animations.spring_stiffness;
        let stiff_lbl = format!("{:.0}", stiffness);
        setting_row(
            ui,
            palette,
            "Spring stiffness",
            &stiff_lbl,
            dirty,
            |ui, dirty| {
                if ui
                    .add(
                        egui::Slider::new(&mut self.cfg.animations.spring_stiffness, 80.0..=500.0)
                            .show_value(false),
                    )
                    .changed()
                {
                    *dirty = true;
                }
            },
        );

        let damping = self.cfg.animations.spring_damping;
        let damp_lbl = format!("{:.2}", damping);
        setting_row(
            ui,
            palette,
            "Damping ratio",
            &damp_lbl,
            dirty,
            |ui, dirty| {
                if ui
                    .add(
                        egui::Slider::new(&mut self.cfg.animations.spring_damping, 0.40..=2.00)
                            .show_value(false),
                    )
                    .changed()
                {
                    *dirty = true;
                }
            },
        );

        let mass = self.cfg.animations.spring_mass;
        let mass_lbl = format!("{:.2}", mass);
        setting_row(
            ui,
            palette,
            "Virtual mass",
            &mass_lbl,
            dirty,
            |ui, dirty| {
                if ui
                    .add(
                        egui::Slider::new(&mut self.cfg.animations.spring_mass, 0.50..=3.00)
                            .show_value(false),
                    )
                    .changed()
                {
                    *dirty = true;
                }
            },
        );

        toggle_row(
            ui,
            palette,
            "Reduced motion (Accessibility)",
            "Disables all spring and easing animations across the application",
            &mut self.cfg.animations.reduced_motion,
            dirty,
        );

        toggle_row(
            ui,
            palette,
            "Micro-interactions",
            "Delightful button presses, icon lifts, and row hover glows",
            &mut self.cfg.animations.micro_interactions,
            dirty,
        );

        toggle_row(
            ui,
            palette,
            "Shimmer & progress wave effects",
            "Animated specular light sweeps over active progress bars",
            &mut self.cfg.animations.shimmer_effects,
            dirty,
        );

        toggle_row(
            ui,
            palette,
            "Radial widget group slice outer fan-out",
            "Fans out outer sub-ring when hovering grouped preset petals",
            &mut self.cfg.animations.widget_group_fanout,
            dirty,
        );

        toggle_row(
            ui,
            palette,
            "Radial widget progress ring mode",
            "Sucks petals into center and shows animated progress ring during conversions",
            &mut self.cfg.animations.widget_progress_ring,
            dirty,
        );

        toggle_row(
            ui,
            palette,
            "Radial widget elastic hover pop-out",
            "Translates active hovered petal 7px outward along its spoke vector",
            &mut self.cfg.animations.widget_elastic_hover,
            dirty,
        );

        section(ui, palette, "Window & Explorer Motion");
        ui.label(
            egui::RichText::new(
                "Authentic OS-level window minimize transitions and fluid spatial explorer navigation.",
            )
            .size(design::type_scale::BODY_SM)
            .color(palette.text_secondary),
        );
        ui.add_space(design::space::SM);

        let cur_win_anim = self.cfg.animations.window_anim;
        let win_anim_lbl = cur_win_anim.label().to_string();
        setting_row(
            ui,
            palette,
            "Window minimize effect",
            &win_anim_lbl,
            dirty,
            |ui, dirty| {
                egui::ComboBox::from_id_salt("prefs_window_anim_effect_combo")
                    .selected_text(cur_win_anim.label())
                    .width(180.0)
                    .show_ui(ui, |ui| {
                        for anim in crate::core::window_anim::WindowAnimType::ALL {
                            let is_active = self.cfg.animations.window_anim == anim;
                            if ui.selectable_label(is_active, anim.label()).clicked() {
                                self.cfg.animations.window_anim = anim;
                                *dirty = true;
                            }
                        }
                    });
            },
        );

        let dur_lbl = format!("{:.2} s", self.cfg.animations.window_anim_duration);
        setting_row(
            ui,
            palette,
            "Animation duration",
            &dur_lbl,
            dirty,
            |ui, dirty| {
                if ui
                    .add(
                        egui::Slider::new(
                            &mut self.cfg.animations.window_anim_duration,
                            0.15..=0.60,
                        )
                        .show_value(false),
                    )
                    .changed()
                {
                    *dirty = true;
                }
            },
        );

        toggle_row(
            ui,
            palette,
            "Folder navigation drift",
            "Subtle 14px directional drift when entering or backing out of directories",
            &mut self.cfg.animations.folder_nav_motion,
            dirty,
        );

        toggle_row(
            ui,
            palette,
            "Tactile click feedback",
            "Subtle 0.985 scale depression on file, folder, and button clicks",
            &mut self.cfg.animations.click_feedback,
            dirty,
        );

        card(ui, palette, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Collapsible Sidebar:")
                        .size(design::type_scale::BODY_SM)
                        .strong()
                        .color(palette.text_primary),
                );
                ui.label(
                    egui::RichText::new(
                        "Toggle with Ctrl+B (Cmd+B) or the top toolbar Hamburger button.",
                    )
                    .size(design::type_scale::BODY_SM)
                    .color(palette.text_secondary),
                );
            });
        });
        ui.add_space(design::space::SM);
    }

    fn render_explorer(&mut self, ui: &mut Ui, palette: &ThemePalette, dirty: &mut bool) {
        section(ui, palette, "Layout & views");
        card(ui, palette, |ui| {
            let layouts = ["Details", "List", "Grid", "Columns"];
            let lay = self.cfg.default_explorer_layout as usize;
            let lay_lbl = layouts.get(lay).copied().unwrap_or("Details").to_string();
            let lay_mut = &mut self.cfg.default_explorer_layout;
            setting_row(
                ui,
                palette,
                "Default layout",
                &lay_lbl,
                dirty,
                |ui, dirty| {
                    for (i, name) in layouts.iter().enumerate() {
                        let active = *lay_mut as usize == i;
                        if design::segmented_pill(ui, palette, active, name).clicked() {
                            *lay_mut = i as u8;
                            *dirty = true;
                        }
                    }
                },
            );

            let gis = self.cfg.grid_icon_size;
            let gis_lbl = format!("{} px", gis);
            let gis_mut = &mut self.cfg.grid_icon_size;
            setting_row(
                ui,
                palette,
                "Grid icon size",
                &gis_lbl,
                dirty,
                |ui, dirty| {
                    if ui
                        .add(egui::Slider::new(gis_mut, 64..=300).show_value(false))
                        .changed()
                    {
                        *dirty = true;
                    }
                },
            );

            let ez = self.cfg.explorer_zoom;
            let ez_lbl = format!("Level {}", ez);
            let ez_mut = &mut self.cfg.explorer_zoom;
            setting_row(ui, palette, "Explorer zoom", &ez_lbl, dirty, |ui, dirty| {
                if ui
                    .add(egui::Slider::new(ez_mut, 0..=4).show_value(false))
                    .changed()
                {
                    *dirty = true;
                }
            });
        });

        section(ui, palette, "Visibility & sections");
        card(ui, palette, |ui| {
            let sh = self.cfg.show_hidden_files;
            let sh_lbl = on_off(sh).to_string();
            let sh_mut = &mut self.cfg.show_hidden_files;
            setting_row(
                ui,
                palette,
                "Show hidden files",
                &sh_lbl,
                dirty,
                |ui, dirty| {
                    if ui.checkbox(sh_mut, "").changed() {
                        *dirty = true;
                    }
                },
            );

            let cc = self.cfg.calculate_checksums;
            let cc_lbl = on_off(cc).to_string();
            let cc_mut = &mut self.cfg.calculate_checksums;
            setting_row(
                ui,
                palette,
                "Calculate checksums",
                &cc_lbl,
                dirty,
                |ui, dirty| {
                    if ui.checkbox(cc_mut, "").changed() {
                        *dirty = true;
                    }
                },
            );

            let sts = self.cfg.show_tags_section;
            let sts_lbl = on_off(sts).to_string();
            let sts_mut = &mut self.cfg.show_tags_section;
            setting_row(
                ui,
                palette,
                "Show tags section",
                &sts_lbl,
                dirty,
                |ui, dirty| {
                    if ui.checkbox(sts_mut, "").changed() {
                        *dirty = true;
                    }
                },
            );
        });

        section(ui, palette, "Custom pinned folders");
        card(ui, palette, |ui| {
            let mut to_remove: Option<usize> = None;
            for (i, p) in self.cfg.pinned_custom.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(p)
                            .size(design::type_scale::BODY_SM)
                            .color(palette.text_primary),
                    );
                    if ui.small_button("Remove").clicked() {
                        to_remove = Some(i);
                        *dirty = true;
                    }
                });
            }
            if let Some(i) = to_remove {
                self.cfg.pinned_custom.remove(i);
            }
            let mut new_pin = String::new();
            ui.horizontal(|ui| {
                ui.add(
                    TextEdit::singleline(&mut new_pin)
                        .hint_text("Path to pin")
                        .desired_width(260.0),
                );
                if ui.button("Add").clicked() && !new_pin.trim().is_empty() {
                    self.cfg.pinned_custom.push(new_pin.trim().to_string());
                    *dirty = true;
                }
            });
        });
    }

    fn render_behavior(&mut self, ui: &mut Ui, palette: &ThemePalette, dirty: &mut bool) {
        section(ui, palette, "Window & system");
        card(ui, palette, |ui| {
            let aot = self.cfg.always_on_top;
            let aot_lbl = on_off(aot).to_string();
            let aot_mut = &mut self.cfg.always_on_top;
            setting_row(
                ui,
                palette,
                "Always on top",
                &aot_lbl,
                dirty,
                |ui, dirty| {
                    if ui.checkbox(aot_mut, "").changed() {
                        *dirty = true;
                    }
                },
            );

            let ps = self.cfg.play_sound;
            let ps_lbl = on_off(ps).to_string();
            let ps_mut = &mut self.cfg.play_sound;
            setting_row(ui, palette, "Play sound", &ps_lbl, dirty, |ui, dirty| {
                if ui.checkbox(ps_mut, "").changed() {
                    *dirty = true;
                }
            });
        });

        section(ui, palette, "Execution & performance");
        card(ui, palette, |ui| {
            let bw = self.cfg.background_work;
            let bw_lbl = on_off(bw).to_string();
            let bw_mut = &mut self.cfg.background_work;
            setting_row(
                ui,
                palette,
                "Background work",
                &bw_lbl,
                dirty,
                |ui, dirty| {
                    if ui.checkbox(bw_mut, "").changed() {
                        *dirty = true;
                    }
                },
            );

            let mf = self.cfg.max_fps;
            let mf_lbl = on_off(mf).to_string();
            let mf_mut = &mut self.cfg.max_fps;
            setting_row(
                ui,
                palette,
                "Max frame rate",
                &mf_lbl,
                dirty,
                |ui, dirty| {
                    if ui.checkbox(mf_mut, "").changed() {
                        *dirty = true;
                    }
                },
            );

            let wt = self.cfg.worker_threads;
            let topo = crate::core::qos::detect_cpu_topology();
            let wt_lbl = if wt == 0 {
                format!("Auto ({})", topo.description())
            } else {
                format!("{} threads", wt)
            };
            let wt_mut = &mut self.cfg.worker_threads;
            setting_row(
                ui,
                palette,
                "Worker threads",
                &wt_lbl,
                dirty,
                |ui, dirty| {
                    if ui
                        .add(egui::Slider::new(wt_mut, 0..=32).show_value(false))
                        .changed()
                    {
                        *dirty = true;
                    }
                },
            );
        });

        section(ui, palette, "Shell integration & maintenance");
        card(ui, palette, |ui| {
            let status = crate::core::default_explorer::status();
            let status_lbl = match status {
                crate::core::default_explorer::TakeoverState::Active => {
                    "Active (Mossambi is Default)"
                }
                crate::core::default_explorer::TakeoverState::NotActive => {
                    "Not Active (Windows Explorer)"
                }
                crate::core::default_explorer::TakeoverState::Unknown => "Unknown",
            };
            setting_row(
                ui,
                palette,
                "Default file manager",
                status_lbl,
                dirty,
                |ui, _dirty| {
                    ui.horizontal(|ui| {
                        if ui.button("Take Over Win+E").clicked() {
                            let _ = crate::core::default_explorer::take_over();
                        }
                        if ui.button("Restore Explorer").clicked() {
                            let _ = crate::core::default_explorer::restore();
                        }
                    });
                },
            );

            let ret = self.cfg.retention_days;
            let ret_lbl = format!("{} days", ret);
            let ret_mut = &mut self.cfg.retention_days;
            setting_row(
                ui,
                palette,
                "Trash retention",
                &ret_lbl,
                dirty,
                |ui, dirty| {
                    if ui
                        .add(egui::Slider::new(ret_mut, 1..=365).suffix(" d"))
                        .changed()
                    {
                        *dirty = true;
                    }
                },
            );
        });
    }

    fn render_performance(&mut self, ui: &mut Ui, palette: &ThemePalette, dirty: &mut bool) {
        let topo = crate::core::qos::detect_cpu_topology();

        section(ui, palette, "Processor & Core Architecture");
        card(ui, palette, |ui| {
            ui.horizontal(|ui| {
                let (c_rect, _) = ui.allocate_exact_size(Vec2::splat(24.0), Sense::hover());
                crate::core::icon_packs::paint_icon(
                    ui,
                    IconPack::Fluent,
                    crate::core::icon_packs::IconGlyph::ActionShield,
                    c_rect,
                    [palette.accent.r(), palette.accent.g(), palette.accent.b()],
                );
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new(&topo.cpu_name)
                            .size(design::type_scale::BODY)
                            .strong()
                            .color(palette.text_primary),
                    );
                    ui.label(
                        egui::RichText::new(format!(
                            "{} physical cores  |  {} logical threads  |  {}",
                            topo.physical_cores,
                            topo.total_logical_cores,
                            if topo.is_hybrid {
                                "Heterogeneous Hybrid Architecture"
                            } else {
                                "Classical Multicore Architecture (Ryzen / Core)"
                            }
                        ))
                        .size(design::type_scale::CAPTION)
                        .color(palette.text_secondary),
                    );
                });
            });

            ui.add_space(design::space::SM);

            // Visual Core Partitioning breakdown
            let (part_rect, _) =
                ui.allocate_exact_size(Vec2::new(ui.available_width(), 20.0), Sense::hover());
            let painter = ui.painter_at(part_rect);
            painter.rect_filled(
                part_rect,
                Rounding::same(design::radius::XS),
                palette.card_hover,
            );

            let total_t = topo.total_logical_cores.max(1) as f32;
            let p_ratio = (topo.p_cores as f32 / total_t).clamp(0.05, 0.9);
            let e_ratio = (topo.e_cores as f32 / total_t).clamp(0.05, 0.9);

            let w = part_rect.width();
            let p_w = w * p_ratio;
            let e_w = w * e_ratio;
            let s_w = (w - p_w - e_w).max(4.0);

            let p_rect = Rect::from_min_size(part_rect.min, Vec2::new(p_w, part_rect.height()));
            let e_rect = Rect::from_min_size(
                part_rect.min + Vec2::new(p_w, 0.0),
                Vec2::new(e_w, part_rect.height()),
            );
            let s_rect = Rect::from_min_size(
                part_rect.min + Vec2::new(p_w + e_w, 0.0),
                Vec2::new(s_w, part_rect.height()),
            );

            painter.rect_filled(p_rect, Rounding::ZERO, Color32::from_rgb(70, 140, 240));
            painter.rect_filled(e_rect, Rounding::ZERO, Color32::from_rgb(50, 180, 120));
            painter.rect_filled(s_rect, Rounding::ZERO, Color32::from_rgb(240, 160, 40));

            ui.add_space(design::space::XS);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!("P-Cores (Compute): {}", topo.p_cores))
                        .size(design::type_scale::SMALL)
                        .color(Color32::from_rgb(70, 140, 240)),
                );
                ui.label(
                    egui::RichText::new(format!("E-Cores (EcoQoS): {}", topo.e_cores))
                        .size(design::type_scale::SMALL)
                        .color(Color32::from_rgb(50, 180, 120)),
                );
                ui.label(
                    egui::RichText::new(format!("Super-Core (UI 144Hz): {}", topo.super_cores))
                        .size(design::type_scale::SMALL)
                        .color(Color32::from_rgb(240, 160, 40)),
                );
            });
        });

        section(ui, palette, "Worker Threads & Execution");
        card(ui, palette, |ui| {
            let wt = self.cfg.worker_threads;
            let wt_lbl = if wt == 0 {
                format!("Auto ({} P-cores)", topo.p_cores)
            } else {
                format!("{} threads", wt)
            };
            let wt_mut = &mut self.cfg.worker_threads;
            setting_row(
                ui,
                palette,
                "Worker thread pool",
                &wt_lbl,
                dirty,
                |ui, dirty| {
                    if ui
                        .add(egui::Slider::new(wt_mut, 0..=32).show_value(false))
                        .changed()
                    {
                        *dirty = true;
                    }
                },
            );

            ui.add_space(design::space::XS);
            toggle_row(
                ui,
                palette,
                "EcoQoS Power Throttling",
                "Marks background indexing and metadata workers with Windows EcoQoS to preserve thermal headroom.",
                &mut self.cfg.background_work,
                dirty,
            );

            ui.add_space(design::space::XS);
            toggle_row(
                ui,
                palette,
                "Hardware Video Acceleration",
                "Uses GPU hardware acceleration (NVENC / AMF / QSV) for media conversions.",
                &mut self.cfg.conv_video_hw_accel,
                dirty,
            );
        });

        section(ui, palette, "Display Refresh & Responsiveness");
        card(ui, palette, |ui| {
            let mf = self.cfg.max_fps;
            let mf_lbl = on_off(mf).to_string();
            let mf_mut = &mut self.cfg.max_fps;
            setting_row(
                ui,
                palette,
                "High refresh rate sync (144Hz / V-Sync)",
                &mf_lbl,
                dirty,
                |ui, dirty| {
                    if ui.checkbox(mf_mut, "").changed() {
                        *dirty = true;
                    }
                },
            );
        });
    }

    fn render_conversion(
        &mut self,
        ui: &mut Ui,
        palette: &ThemePalette,
        settings: &mut QuickSettingsState,
        dirty: &mut bool,
    ) {
        section(ui, palette, "Output destination & policy");
        card(ui, palette, |ui| {
            let om = self.cfg.output_mode;
            let om_lbl = output_mode_label(om).to_string();
            let om_mut = &mut self.cfg.output_mode;
            setting_row(ui, palette, "Output mode", &om_lbl, dirty, |ui, dirty| {
                for (v, lab) in [
                    (OutputMode::Sibling, "Sibling"),
                    (OutputMode::Custom, "Custom"),
                ] {
                    let active = *om_mut == v;
                    if design::segmented_pill(ui, palette, active, lab).clicked() {
                        *om_mut = v;
                        *dirty = true;
                    }
                }
            });

            let mut dir = self
                .cfg
                .custom_output_dir
                .clone()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            let dir_lbl = if dir.trim().is_empty() {
                "None".to_string()
            } else {
                dir.clone()
            };
            setting_row(
                ui,
                palette,
                "Custom output folder",
                &dir_lbl,
                dirty,
                |ui, dirty| {
                    if ui
                        .add(TextEdit::singleline(&mut dir).desired_width(220.0))
                        .changed()
                    {
                        self.cfg.custom_output_dir = if dir.trim().is_empty() {
                            None
                        } else {
                            Some(PathBuf::from(dir.trim()))
                        };
                        *dirty = true;
                    }
                },
            );

            let ezl = self.cfg.enforce_zero_loss;
            let ezl_lbl = on_off(ezl).to_string();
            let ezl_mut = &mut self.cfg.enforce_zero_loss;
            setting_row(
                ui,
                palette,
                "Enforce zero loss",
                &ezl_lbl,
                dirty,
                |ui, dirty| {
                    if ui.checkbox(ezl_mut, "").changed() {
                        *dirty = true;
                    }
                },
            );
        });

        section(ui, palette, "Conversion defaults");
        card(ui, palette, |ui| {
            let iq = settings.image_quality;
            let iq_lbl = format!("{}%", iq);
            let iq_mut = &mut settings.image_quality;
            let iq_conv = &mut self.cfg.conv_image_quality;
            setting_row(ui, palette, "Image quality", &iq_lbl, dirty, |ui, dirty| {
                if ui
                    .add(egui::Slider::new(iq_mut, 1..=100).show_value(false))
                    .changed()
                {
                    *iq_conv = *iq_mut;
                    *dirty = true;
                }
            });

            let il = settings.image_lossless;
            let il_lbl = on_off(il).to_string();
            let il_mut = &mut settings.image_lossless;
            let il_conv = &mut self.cfg.conv_image_lossless;
            setting_row(
                ui,
                palette,
                "Image lossless",
                &il_lbl,
                dirty,
                |ui, dirty| {
                    if ui.checkbox(il_mut, "").changed() {
                        *il_conv = *il_mut;
                        *dirty = true;
                    }
                },
            );

            let idpi = settings.image_dpi;
            let idpi_lbl = format!("{}", idpi);
            let idpi_mut = &mut settings.image_dpi;
            let idpi_conv = &mut self.cfg.conv_image_dpi;
            setting_row(ui, palette, "Image DPI", &idpi_lbl, dirty, |ui, dirty| {
                if ui
                    .add(egui::Slider::new(idpi_mut, 72..=600).show_value(false))
                    .changed()
                {
                    *idpi_conv = *idpi_mut;
                    *dirty = true;
                }
            });

            let ab = settings.audio_bitrate.clone();
            let ab_lbl = if ab.is_empty() {
                "Default".to_string()
            } else {
                ab.clone()
            };
            let ab_mut = &mut settings.audio_bitrate;
            let ab_conv = &mut self.cfg.conv_audio_bitrate;
            setting_row(ui, palette, "Audio bitrate", &ab_lbl, dirty, |ui, dirty| {
                for v in ["320k", "256k", "192k", "128k"] {
                    let active = ab_mut.as_str() == v;
                    if design::segmented_pill(ui, palette, active, v).clicked() {
                        *ab_mut = v.to_string();
                        *ab_conv = v.to_string();
                        *dirty = true;
                    }
                }
            });

            let asl = settings.audio_strict_lossless;
            let asl_lbl = on_off(asl).to_string();
            let asl_mut = &mut settings.audio_strict_lossless;
            let asl_conv = &mut self.cfg.conv_audio_strict_lossless;
            setting_row(
                ui,
                palette,
                "Audio strict lossless",
                &asl_lbl,
                dirty,
                |ui, dirty| {
                    if ui.checkbox(asl_mut, "").changed() {
                        *asl_conv = *asl_mut;
                        *dirty = true;
                    }
                },
            );

            let vcrf = settings.video_crf;
            let vcrf_lbl = format!("{}", vcrf);
            let vcrf_mut = &mut settings.video_crf;
            let vcrf_conv = &mut self.cfg.conv_video_crf;
            setting_row(ui, palette, "Video CRF", &vcrf_lbl, dirty, |ui, dirty| {
                if ui
                    .add(egui::Slider::new(vcrf_mut, 0..=51).show_value(false))
                    .changed()
                {
                    *vcrf_conv = *vcrf_mut;
                    *dirty = true;
                }
            });

            let vp = settings.video_preset.clone();
            let vp_lbl = if vp.is_empty() {
                "Default".to_string()
            } else {
                vp.clone()
            };
            let vp_mut = &mut settings.video_preset;
            let vp_conv = &mut self.cfg.conv_video_preset;
            setting_row(ui, palette, "Video preset", &vp_lbl, dirty, |ui, dirty| {
                for v in ["High Quality", "Balanced", "Fast"] {
                    let active = vp_mut.as_str() == v;
                    if design::segmented_pill(ui, palette, active, v).clicked() {
                        *vp_mut = v.to_string();
                        *vp_conv = v.to_string();
                        *dirty = true;
                    }
                }
            });

            let vhw = settings.video_hw_accel;
            let vhw_lbl = on_off(vhw).to_string();
            let vhw_mut = &mut settings.video_hw_accel;
            let vhw_conv = &mut self.cfg.conv_video_hw_accel;
            setting_row(
                ui,
                palette,
                "Video HW accel",
                &vhw_lbl,
                dirty,
                |ui, dirty| {
                    if ui.checkbox(vhw_mut, "").changed() {
                        *vhw_conv = *vhw_mut;
                        *dirty = true;
                    }
                },
            );

            let ac = settings.archive_compression.clone();
            let ac_lbl = if ac.is_empty() {
                "Default".to_string()
            } else {
                ac.clone()
            };
            let ac_mut = &mut settings.archive_compression;
            let ac_conv = &mut self.cfg.conv_archive_compression;
            setting_row(
                ui,
                palette,
                "Archive compression",
                &ac_lbl,
                dirty,
                |ui, dirty| {
                    for v in ["Ultra", "Normal", "Fast"] {
                        let active = ac_mut.as_str() == v;
                        if design::segmented_pill(ui, palette, active, v).clicked() {
                            *ac_mut = v.to_string();
                            *ac_conv = v.to_string();
                            *dirty = true;
                        }
                    }
                },
            );

            let em = settings.exam_mode_enabled;
            let em_lbl = on_off(em).to_string();
            let em_mut = &mut settings.exam_mode_enabled;
            let em_conv = &mut self.cfg.conv_exam_mode_enabled;
            setting_row(
                ui,
                palette,
                "Exam mode enabled",
                &em_lbl,
                dirty,
                |ui, dirty| {
                    if ui.checkbox(em_mut, "").changed() {
                        *em_conv = *em_mut;
                        *dirty = true;
                    }
                },
            );
        });
    }

    /// Scripts page: script manager (disk + database), the Rhai/Lua editor,
    /// and the live execution console.
    fn render_scripts(
        &mut self,
        ui: &mut Ui,
        palette: &ThemePalette,
        console: &mut ScriptConsole,
        run_ctx: (Option<std::path::PathBuf>, Vec<std::path::PathBuf>),
        _dirty: &mut bool,
    ) {
        ui.label(
            egui::RichText::new(
                "Run Rhai or Lua scripts with sandboxed mossambi primitives. Disk scripts live in the scripts folder; saved scripts are stored in the custom commands database.",
            )
            .size(design::type_scale::BODY_SM)
            .color(palette.text_secondary),
        );
        ui.add_space(design::space::SM);

        // Manager card
        card(ui, palette, |ui| {
            ui.label(
                egui::RichText::new("Installed scripts")
                    .size(design::type_scale::TITLE_SM)
                    .strong()
                    .color(palette.text_primary),
            );
            ui.add_space(design::space::XS);

            let scripts = crate::core::scripting::load_all_user_scripts();
            if scripts.is_empty() {
                ui.label(
                    egui::RichText::new("No scripts yet. Create one below.")
                        .size(design::type_scale::CAPTION)
                        .color(palette.text_muted),
                );
            }
            for script in scripts {
                ui.add_space(design::space::XS);
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new(script.title.clone())
                                .size(design::type_scale::BODY)
                                .strong()
                                .color(palette.text_primary),
                        );
                        let source_label = match &script.source {
                            crate::core::scripting::ScriptSource::Disk(path) => {
                                format!("disk - {}", path.display())
                            }
                            crate::core::scripting::ScriptSource::Database => {
                                "database".to_string()
                            }
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} - {}",
                                source_label,
                                script.lang.label()
                            ))
                            .size(design::type_scale::CAPTION)
                            .color(palette.text_muted),
                        );
                    });
                    ui.add_space(design::space::MD);
                    if ui.button("Run").clicked() {
                        let sender = console.sender();
                        let (current_file, selected) = run_ctx.clone();
                        let title = script.title.clone();
                        crate::core::scripting::execute_script_async(
                            script.body.clone(),
                            script.lang,
                            current_file,
                            selected,
                            move |outcome| {
                                let _ = sender.send((title, outcome));
                            },
                        );
                    }
                    if let crate::core::scripting::ScriptSource::Database = script.source {
                        if ui.button("Edit").clicked() {
                            self.script_edit_id =
                                Some(script.id.trim_start_matches("db:").to_string());
                            self.script_edit_title = script.title.clone();
                            self.script_edit_lang = script.lang;
                            self.script_edit_body = script.body.clone();
                        }
                        if ui.button("Delete").clicked() {
                            let record_id = script.id.trim_start_matches("db:").to_string();
                            let _ = crate::core::state_db::delete_command(&record_id);
                        }
                    }
                });
            }
        });
        ui.add_space(design::space::MD);

        // Editor card
        card(ui, palette, |ui| {
            ui.label(
                egui::RichText::new("Script editor")
                    .size(design::type_scale::TITLE_SM)
                    .strong()
                    .color(palette.text_primary),
            );
            ui.add_space(design::space::XS);

            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Title")
                        .size(design::type_scale::CAPTION)
                        .color(palette.text_muted),
                );
                ui.add(
                    TextEdit::singleline(&mut self.script_edit_title)
                        .hint_text("e.g. Rename screenshots by date")
                        .desired_width(260.0),
                );
                ui.label(
                    egui::RichText::new("Language")
                        .size(design::type_scale::CAPTION)
                        .color(palette.text_muted),
                );
                for lang in [ScriptLang::Rhai, ScriptLang::Lua] {
                    if design::segmented_pill(
                        ui,
                        palette,
                        self.script_edit_lang == lang,
                        lang.label(),
                    )
                    .clicked()
                    {
                        self.script_edit_lang = lang;
                    }
                }
            });
            ui.add_space(design::space::XS);

            ui.add(
                TextEdit::multiline(&mut self.script_edit_body)
                    .hint_text("-- Lua example: print(mossambi_file_size(current_file))")
                    .desired_width(f32::INFINITY)
                    .code_editor(),
            );
            ui.add_space(design::space::XS);

            ui.horizontal(|ui| {
                if ui.button("New").clicked() {
                    self.script_edit_id = None;
                    self.script_edit_title.clear();
                    self.script_edit_body.clear();
                    self.script_edit_lang = ScriptLang::Rhai;
                }
                if ui.button("Save").clicked() {
                    let title = self.script_edit_title.trim().to_string();
                    if !title.is_empty() && !self.script_edit_body.trim().is_empty() {
                        let record_id = match &self.script_edit_id {
                            Some(id) => id.clone(),
                            None => format!(
                                "cmd-{:x}",
                                std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .map(|d| d.as_nanos())
                                    .unwrap_or(0)
                            ),
                        };
                        let record = crate::core::state_db::CustomCommandRecord {
                            id: record_id,
                            title,
                            script_lang: self.script_edit_lang.as_str().to_string(),
                            script_body: self.script_edit_body.clone(),
                            shortcut: None,
                        };
                        if crate::core::state_db::save_command(&record).is_ok() {
                            self.script_edit_id = None;
                            self.script_edit_title.clear();
                            self.script_edit_body.clear();
                        }
                    }
                }
                match &self.script_edit_id {
                    Some(id) => {
                        ui.label(
                            egui::RichText::new(format!("Editing {}", id))
                                .size(design::type_scale::CAPTION)
                                .color(palette.text_muted),
                        );
                    }
                    None => {
                        ui.label(
                            egui::RichText::new("New script")
                                .size(design::type_scale::CAPTION)
                                .color(palette.text_muted),
                        );
                    }
                }
            });
        });
        ui.add_space(design::space::MD);

        // Console card: live outcomes, newest first.
        console.drain();
        card(ui, palette, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Script console")
                        .size(design::type_scale::TITLE_SM)
                        .strong()
                        .color(palette.text_primary),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("Clear").clicked() {
                        console.entries.clear();
                    }
                });
            });
            ui.add_space(design::space::XS);

            if console.entries.is_empty() {
                ui.label(
                    egui::RichText::new(
                        "No runs yet. Run a script above or from the command palette.",
                    )
                    .size(design::type_scale::CAPTION)
                    .color(palette.text_muted),
                );
            }
            for entry in console.entries.iter().rev() {
                ui.add_space(design::space::XS);
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(if entry.outcome.success {
                                "OK"
                            } else {
                                "FAILED"
                            })
                            .size(design::type_scale::CAPTION)
                            .strong()
                            .color(if entry.outcome.success {
                                palette.accent
                            } else {
                                palette.text_primary
                            }),
                        );
                        ui.label(
                            egui::RichText::new(entry.title.clone())
                                .size(design::type_scale::BODY)
                                .strong()
                                .color(palette.text_primary),
                        );
                        ui.label(
                            egui::RichText::new(entry.time.clone())
                                .size(design::type_scale::CAPTION)
                                .color(palette.text_muted),
                        );
                    });
                    if entry.outcome.success {
                        ui.label(
                            egui::RichText::new(format!(
                                "Returned: {}",
                                entry.outcome.return_value
                            ))
                            .size(design::type_scale::CAPTION)
                            .color(palette.text_secondary),
                        );
                    }
                    if let Some(err) = &entry.outcome.error {
                        ui.label(
                            egui::RichText::new(format!("Error: {}", err))
                                .size(design::type_scale::CAPTION)
                                .color(palette.text_secondary),
                        );
                    }
                    for log in &entry.outcome.logs {
                        ui.label(
                            egui::RichText::new(format!("  {}", log))
                                .size(design::type_scale::CAPTION)
                                .color(palette.text_secondary),
                        );
                    }
                });
            }
        });
    }

    fn render_about(&mut self, ui: &mut Ui, palette: &ThemePalette) {
        let topo = crate::core::qos::detect_cpu_topology();

        card(ui, palette, |ui| {
            ui.horizontal(|ui| {
                let (b_rect, _) = ui.allocate_exact_size(Vec2::splat(40.0), Sense::hover());
                crate::core::icon_packs::paint_icon(
                    ui,
                    IconPack::Fluent,
                    crate::core::icon_packs::IconGlyph::ActionShield,
                    b_rect,
                    [palette.accent.r(), palette.accent.g(), palette.accent.b()],
                );
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new("Mossambi Fluent Studio")
                            .size(design::type_scale::TITLE)
                            .strong()
                            .color(palette.text_primary),
                    );
                    ui.label(
                        egui::RichText::new("Version 2.1.0  |  Production Build (MSVC x86_64)")
                            .size(design::type_scale::BODY_SM)
                            .color(palette.text_secondary),
                    );
                });
            });
        });

        section(ui, palette, "Host System Diagnostics");
        card(ui, palette, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Processor:")
                        .size(design::type_scale::BODY_SM)
                        .strong()
                        .color(palette.text_primary),
                );
                ui.label(
                    egui::RichText::new(&topo.cpu_name)
                        .size(design::type_scale::BODY_SM)
                        .color(palette.text_primary),
                );
            });
            ui.label(
                egui::RichText::new(format!(
                    "Topology: {} physical cores, {} logical threads ({})",
                    topo.physical_cores,
                    topo.total_logical_cores,
                    if topo.is_hybrid {
                        "Modern Hybrid P/E"
                    } else {
                        "Classical Multicore"
                    }
                ))
                .size(design::type_scale::CAPTION)
                .color(palette.text_secondary),
            );
            ui.add_space(design::space::XS);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Core Partition:")
                        .size(design::type_scale::BODY_SM)
                        .strong()
                        .color(palette.text_primary),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "P-Cores: {}  |  E-Cores (EcoQoS): {}  |  Super-Core UI: {}",
                        topo.p_cores, topo.e_cores, topo.super_cores
                    ))
                    .size(design::type_scale::BODY_SM)
                    .color(palette.text_secondary),
                );
            });
            ui.add_space(design::space::XS);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Compositor:")
                        .size(design::type_scale::BODY_SM)
                        .strong()
                        .color(palette.text_primary),
                );
                ui.label(
                    egui::RichText::new("Windows DirectComposition DWM (Mica / Acrylic Glass)")
                        .size(design::type_scale::BODY_SM)
                        .color(palette.text_secondary),
                );
            });
            ui.add_space(design::space::XS);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("File System:")
                        .size(design::type_scale::BODY_SM)
                        .strong()
                        .color(palette.text_primary),
                );
                ui.label(
                    egui::RichText::new(
                        "Windows Native Recycle Bin (SHFileOperation + Undo Ctrl+Z)",
                    )
                    .size(design::type_scale::BODY_SM)
                    .color(palette.text_secondary),
                );
            });
        });

        section(ui, palette, "Architecture & Standards");
        card(ui, palette, |ui| {
            ui.label(
                egui::RichText::new(
                    "Standards: Pure 7-bit ASCII verified, Apple HIG & Windows 11 Fluent design.",
                )
                .size(design::type_scale::BODY_SM)
                .color(palette.text_secondary),
            );
            ui.label(
                egui::RichText::new("Conversion Engine: 255 formats with zero-loss guarantee and GPU DirectML acceleration.")
                    .size(design::type_scale::BODY_SM)
                    .color(palette.text_secondary),
            );
        });
    }
}

// ---- helpers ----

fn section(ui: &mut Ui, palette: &ThemePalette, title: &str) {
    // More space above a heading than below it keeps groups reading as groups.
    ui.add_space(design::space::LG);
    ui.label(
        egui::RichText::new(title)
            .size(design::type_scale::TITLE_SM)
            .strong()
            .color(palette.text_primary),
    );
    ui.add_space(design::space::XS);
}

/// Renders an egui key press as the stored display form ("Ctrl+P").
fn key_binding_display(modifiers: egui::Modifiers, key: egui::Key) -> String {
    let key_name = format!("{:?}", key);
    let mut out = String::new();
    if modifiers.ctrl {
        out.push_str("Ctrl+");
    }
    if modifiers.alt {
        out.push_str("Alt+");
    }
    if modifiers.shift {
        out.push_str("Shift+");
    }
    out.push_str(&key_name);
    out
}

fn card<R>(ui: &mut Ui, palette: &ThemePalette, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
    let mut bg = palette.card_bg;
    bg = Color32::from_rgb(bg.r(), bg.g(), bg.b());
    let avail = ui.available_width();
    ui.set_max_width(avail);
    Frame::none()
        .fill(bg)
        .stroke(Stroke::new(1.0_f32, palette.border))
        .rounding(Rounding::same(10.0))
        .inner_margin(Margin::symmetric(16.0, 12.0))
        .show(ui, add_contents)
        .inner
}

fn toggle_row(
    ui: &mut Ui,
    palette: &ThemePalette,
    label: &str,
    description: &str,
    val: &mut bool,
    dirty: &mut bool,
) {
    let w = ui.available_width();
    let row_height = if description.is_empty() { 36.0 } else { 48.0 };
    ui.allocate_ui_with_layout(
        Vec2::new(w, row_height),
        Layout::left_to_right(Align::Center),
        |ui| {
            let ctrl_width = 80.0_f32;
            let left_w = (ui.available_width() - ctrl_width - 8.0).max(120.0);
            ui.allocate_ui_with_layout(
                Vec2::new(left_w, row_height),
                Layout::top_down(Align::Min),
                |ui| {
                    ui.add_space(2.0);
                    ui.label(
                        egui::RichText::new(label)
                            .size(design::type_scale::BODY)
                            .color(palette.text_primary),
                    );
                    if !description.is_empty() {
                        ui.label(
                            egui::RichText::new(description)
                                .size(design::type_scale::CAPTION)
                                .color(palette.text_secondary),
                        );
                    }
                },
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                // Authentic Apple HIG Pill Toggle Switch (38x22 px)
                let track_size = Vec2::new(38.0, 22.0);
                let (rect, resp) = ui.allocate_exact_size(track_size, Sense::click());
                if resp.clicked() {
                    *val = !*val;
                    *dirty = true;
                }
                let track_radius = 11.0;
                let track_color = if *val { palette.accent } else { palette.border };
                ui.painter()
                    .rect_filled(rect, Rounding::same(track_radius), track_color);

                // Sliding thumb circle (radius 8.5)
                let thumb_x = if *val {
                    rect.max.x - 11.0
                } else {
                    rect.min.x + 11.0
                };
                let thumb_center = egui::pos2(thumb_x, rect.center().y);
                ui.painter()
                    .circle_filled(thumb_center, 8.5, Color32::from_rgb(255, 255, 255));

                // Ring highlight: soft on hover, full accent while on,
                // strongest while pressed or keyboard-focused.
                let ring_state = design::pill_state(&resp, *val);
                design::paint_pill_ring(
                    ui.painter(),
                    rect,
                    Rounding::same(track_radius),
                    ring_state,
                    palette.accent,
                );

                let text = if *val { "On" } else { "Off" };
                ui.label(
                    egui::RichText::new(text)
                        .size(design::type_scale::BODY_SM)
                        .color(if *val {
                            palette.accent
                        } else {
                            palette.text_muted
                        }),
                );
            });
        },
    );
    ui.add_space(design::space::XS);
}

fn setting_row(
    ui: &mut Ui,
    palette: &ThemePalette,
    label: &str,
    value: &str,
    dirty: &mut bool,
    body: impl FnOnce(&mut Ui, &mut bool),
) {
    let w = ui.available_width();
    if w < 440.0 {
        // Narrow viewports: wrap into two stacked rows
        ui.vertical(|ui| {
            ui.add_space(2.0);
            ui.label(
                egui::RichText::new(label)
                    .size(design::type_scale::BODY)
                    .color(palette.text_primary),
            );
            ui.add_space(2.0);
            ui.horizontal_wrapped(|ui| {
                body(ui, dirty);
                if !value.is_empty() {
                    ui.add_space(design::space::XS);
                    ui.label(
                        egui::RichText::new(value)
                            .size(design::type_scale::SMALL)
                            .color(palette.text_muted),
                    );
                }
            });
            ui.add_space(4.0);
        });
    } else {
        // Standard wide viewport: label left, control right-aligned with ample breathing room
        ui.allocate_ui_with_layout(
            Vec2::new(w, 34.0),
            Layout::left_to_right(Align::Center),
            |ui| {
                let label_w = 170.0_f32.min(w * 0.32);
                ui.allocate_ui_with_layout(
                    Vec2::new(label_w, 34.0),
                    Layout::left_to_right(Align::Center),
                    |ui| {
                        ui.add_space(design::space::XS);
                        ui.label(
                            egui::RichText::new(label)
                                .size(design::type_scale::BODY)
                                .color(palette.text_primary),
                        );
                    },
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add_space(4.0);
                    let x0 = ui.cursor().right();
                    body(ui, dirty);
                    let control_w = x0 - ui.cursor().right();
                    if !value.is_empty() && control_w < 150.0 {
                        ui.add_space(design::space::XS);
                        ui.label(
                            egui::RichText::new(value)
                                .size(design::type_scale::SMALL)
                                .color(palette.text_muted),
                        );
                    }
                });
            },
        );
    }
    ui.add_space(design::space::XS);
}

pub fn setting_value_label(key: &str, cfg: &MossambiConfig) -> Option<String> {
    match key {
        "show_tags_section" => Some(if cfg.show_tags_section {
            "Shown".into()
        } else {
            "Hidden".into()
        }),
        "show_custom_pinned" => Some(if cfg.show_custom_pinned {
            "Shown".into()
        } else {
            "Hidden".into()
        }),
        "menu_style" => Some(menu_style_label(cfg.menu_style).to_string()),
        "checkbox_mode" => Some(checkbox_mode_label(cfg.checkbox_mode).to_string()),
        "window_opacity" => Some(format!("{:.0}%", cfg.window_opacity * 100.0)),
        "always_on_top" => Some(on_off(cfg.always_on_top).to_string()),
        "play_sound" => Some(on_off(cfg.play_sound).to_string()),
        "launch_at_login" => Some(on_off(cfg.launch_at_login).to_string()),
        "max_fps" => Some(on_off(cfg.max_fps).to_string()),
        "background_work" => Some(on_off(cfg.background_work).to_string()),
        "enable_gesture_wakeup" => Some(on_off(cfg.enable_gesture_wakeup).to_string()),
        "enable_hotkey_wakeup" => Some(on_off(cfg.enable_hotkey_wakeup).to_string()),
        "enforce_zero_loss" => Some(on_off(cfg.enforce_zero_loss).to_string()),
        "show_hidden_files" => Some(on_off(cfg.show_hidden_files).to_string()),
        "calculate_checksums" => Some(on_off(cfg.calculate_checksums).to_string()),
        "tui_transparent_bg" => Some(on_off(cfg.tui_transparent_bg).to_string()),
        "deep_drag_detection_only" => Some(on_off(cfg.deep_drag_detection_only).to_string()),
        "glass_effect" => Some(glass_label(cfg.glass_effect).to_string()),
        "output_mode" => Some(output_mode_label(cfg.output_mode).to_string()),
        "worker_threads" => {
            let topo = crate::core::qos::detect_cpu_topology();
            Some(if cfg.worker_threads == 0 {
                format!("Auto ({})", topo.description())
            } else {
                format!("{} threads", cfg.worker_threads)
            })
        }
        "icon_pack" => Some(IconPack::from_config_id(&cfg.icon_pack).label().to_string()),
        "drop_action_pref" => Some(cfg.drop_action_pref.label().to_string()),
        _ => None,
    }
}

pub fn search_preferences(query: &str, cfg: &MossambiConfig) -> Vec<PreferenceHit> {
    let q = query.to_lowercase();
    let mut out = Vec::new();
    let cats = preference_categories();
    // Build a minimal hit list from setting_value_label keys.
    for (label, key) in [
        ("Glass effect", "glass_effect"),
        ("Window opacity", "window_opacity"),
        ("Menu style", "menu_style"),
        ("Checkbox mode", "checkbox_mode"),
        ("Icon pack", "icon_pack"),
        ("Show hidden files", "show_hidden_files"),
        ("Show tags section", "show_tags_section"),
        ("Calculate checksums", "calculate_checksums"),
        ("Always on top", "always_on_top"),
        ("Play sound", "play_sound"),
        ("Launch at login", "launch_at_login"),
        ("Background work", "background_work"),
        ("Max frame rate", "max_fps"),
        ("Gesture wake-up", "enable_gesture_wakeup"),
        ("Hotkey wake-up", "enable_hotkey_wakeup"),
        ("Enforce zero loss", "enforce_zero_loss"),
        ("Worker threads", "worker_threads"),
        ("Output mode", "output_mode"),
        ("When dropping files", "drop_action_pref"),
    ] {
        let v = setting_value_label(key, cfg).unwrap_or_default();
        if q.is_empty() || label.to_lowercase().contains(&q) || v.to_lowercase().contains(&q) {
            // map key -> category title
            let cat_title = match key {
                "drop_action_pref" => "General",
                "glass_effect" | "window_opacity" | "menu_style" | "checkbox_mode"
                | "icon_pack" => "Appearance",
                "show_hidden_files" | "show_tags_section" | "calculate_checksums" => "Explorer",
                "always_on_top" | "play_sound" | "launch_at_login" | "background_work"
                | "max_fps" => "Behavior",
                "enable_gesture_wakeup" | "enable_hotkey_wakeup" => "Input & Gestures",
                "worker_threads" => "Performance",
                "enforce_zero_loss" | "output_mode" => "Conversion",
                _ => "Appearance",
            };
            let cat_id = cats
                .iter()
                .find(|c| c.title == cat_title)
                .map(|c| c.id)
                .unwrap_or("appearance");
            out.push(PreferenceHit {
                category_id: cat_id,
                category_title: cat_title,
                setting_label: label,
                value_label: v,
            });
        }
    }
    out
}

// ---- value label helpers ----

fn on_off(b: bool) -> &'static str {
    if b {
        "On"
    } else {
        "Off"
    }
}

fn glass_label(g: GlassEffect) -> &'static str {
    match g {
        GlassEffect::FrostedGlass => "Frosted Glass",
        GlassEffect::LiquidGlass => "Liquid Glass",
        GlassEffect::Acrylic => "Acrylic",
        GlassEffect::Mica => "Mica",
        GlassEffect::Opaque => "Opaque",
    }
}

fn menu_style_label(m: MenuStyle) -> &'static str {
    match m {
        MenuStyle::Finder => "Finder",
        MenuStyle::Windows => "Windows",
        MenuStyle::Linux => "Linux",
    }
}

fn checkbox_mode_label(c: CheckboxMode) -> &'static str {
    match c {
        CheckboxMode::Always => "Always",
        CheckboxMode::Hover => "On hover",
        CheckboxMode::Off => "Hidden",
    }
}

fn output_mode_label(o: OutputMode) -> &'static str {
    match o {
        OutputMode::Sibling => "Sibling folder",
        OutputMode::Custom => "Custom folder",
    }
}
