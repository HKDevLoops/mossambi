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

use crate::core::config::{CheckboxMode, GlassEffect, MenuStyle, MossambiConfig, OutputMode};
use crate::core::donut_config::{DonutConfig, DonutProfile, DonutSlicePreset};
use crate::core::fields_config::FieldsConfig;
use crate::core::icon_packs::IconPack;
use crate::gui::design;
use crate::gui::macos_theme::ThemePalette;
use crate::gui::macos_theme::ThemeStyle;
use crate::gui::quick_settings::QuickSettingsState;
use eframe::egui::{
    self, Align, Align2, Color32, Frame, Layout, Margin, Rounding, ScrollArea, Sense, Stroke,
    TextEdit, Ui, Vec2,
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
    static CATS: [PreferenceCategory; 10] = [
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
    pub exclusion_input: String,
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
            exclusion_input: String::new(),
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
    ) {
        // Keep cfg.theme_mode in sync with the live theme selection.
        self.cfg.theme_mode = (*current_theme).into();

        // Sidebar.
        egui::SidePanel::left("prefs_sidebar_panel")
            .exact_width(208.0)
            .frame(
                Frame::none()
                    .fill(palette.sidebar_bg)
                    .inner_margin(Margin::symmetric(10.0, 10.0)),
            )
            .show_inside(ui, |ui| {
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
                        .desired_width(f32::INFINITY),
                );
                ui.add_space(design::space::MD);

                if self.search.trim().is_empty() {
                    for (i, cat) in CATEGORIES.iter().enumerate() {
                        let is_sel = self.selected == i;
                        let resp = ui.selectable_label(
                            is_sel,
                            egui::RichText::new(*cat)
                                .size(design::type_scale::BODY)
                                .color(if is_sel {
                                    palette.accent
                                } else {
                                    palette.text_primary
                                }),
                        );
                        if resp.clicked() {
                            self.selected = i;
                        }
                        ui.label(
                            egui::RichText::new(self.category_summary(i))
                                .size(design::type_scale::CAPTION)
                                .color(palette.text_muted),
                        );
                        ui.add_space(design::space::XS);
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

        // Detail pane.
        let mut dirty = false;
        ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
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
                    _ => self.render_about(ui, palette),
                }
                let _ = dirty;
            });

        if self.dirty {
            let _ = self.cfg.save();
            self.dirty = false;
        }
    }

    fn category_summary(&self, i: usize) -> String {
        match i {
            0 => format!(
                "Dates: {} - history: {}",
                if self.cfg.keep_original_file_dates {
                    "kept"
                } else {
                    "normal"
                },
                self.cfg.conversion_history_limit
            ),
            1 => format!(
                "{} profiles - size {:.0}px",
                self.cfg.donut.profiles.len(),
                self.cfg.donut.appearance.size
            ),
            2 => format!(
                "Tab flip - gestures {}",
                if self.cfg.enable_gesture_wakeup {
                    "on"
                } else {
                    "off"
                }
            ),
            3 => "12 domain toggles".to_string(),
            4 => format!(
                "{} - {}",
                glass_label(self.cfg.glass_effect),
                if self.cfg.window_opacity >= 0.99 {
                    "solid".to_string()
                } else {
                    format!("{:.0}%", self.cfg.window_opacity * 100.0)
                }
            ),
            5 => format!(
                "Layout {} - tags {}",
                self.cfg.default_explorer_layout,
                if self.cfg.show_tags_section {
                    "on"
                } else {
                    "off"
                }
            ),
            6 => format!(
                "Sound {} - autostart {}",
                if self.cfg.play_sound { "on" } else { "off" },
                if self.cfg.launch_at_login {
                    "on"
                } else {
                    "off"
                }
            ),
            7 => format!("{} threads", self.cfg.worker_threads),
            8 => format!(
                "{} - {}",
                output_mode_label(self.cfg.output_mode),
                if self.cfg.enforce_zero_loss {
                    "lossless"
                } else {
                    "standard"
                }
            ),
            _ => "Mossambi v2.1".to_string(),
        }
    }

    fn category_subtitle(&self, i: usize) -> &'static str {
        match i {
            0 => "Startup, default save locations, browser drag staging, and history.",
            1 => "Profiles, interactive radial donut preview, extension bindings, and appearance metrics.",
            2 => "Modifier drag gestures, hotkeys, flip keys, and app exclusion list.",
            3 => "Toggle specialized domains on or off across the suite.",
            4 => "Theme, glass, opacity, menu and control style.",
            5 => "Layout, hidden files, tags and pinned folders.",
            6 => "Startup, sound, background work, and shell replacement.",
            7 => "Threads, DirectML hardware acceleration, and frame rate.",
            8 => "Zero-loss transcode engines, 255-format routing, and local AI catalog.",
            _ => "Version, hardware telemetry, and credits.",
        }
    }

    fn search_hits(&self) -> Vec<PreferenceHit> {
        let mut out = Vec::new();
        // Appearance
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
            setting_label: "Show custom pinned folders",
            value_label: on_off(self.cfg.show_custom_pinned).to_string(),
        });
        out.push(PreferenceHit {
            category_id: "explorer",
            category_title: "Explorer",
            setting_label: "Calculate checksums",
            value_label: on_off(self.cfg.calculate_checksums).to_string(),
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
        out.push(PreferenceHit {
            category_id: "performance",
            category_title: "Performance",
            setting_label: "Worker threads",
            value_label: format!("{}", self.cfg.worker_threads),
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
        out
    }

    // ---- category renderers ----

    fn render_general(&mut self, ui: &mut Ui, palette: &ThemePalette, dirty: &mut bool) {
        card(ui, palette, |ui| {
            ui.label(
                egui::RichText::new("Language")
                    .size(design::type_scale::BODY)
                    .color(palette.text_primary),
            );
            ui.label(
                egui::RichText::new("System default")
                    .size(design::type_scale::BODY_SM)
                    .color(palette.text_muted),
            );
            ui.add_space(design::space::SM);

            toggle_row(
                ui,
                palette,
                "Start Mossambi when I sign in",
                "",
                &mut self.cfg.launch_at_login,
                dirty,
            );

            ui.add_space(design::space::XS);
            ui.label(
                egui::RichText::new("When I close the window")
                    .size(design::type_scale::BODY)
                    .color(palette.text_primary),
            );
            ui.label(
                egui::RichText::new("Ask every time")
                    .size(design::type_scale::BODY_SM)
                    .color(palette.text_muted),
            );
            ui.add_space(design::space::SM);

            let mut context_menu = true;
            toggle_row(
                ui,
                palette,
                "Show \"Convert with Mossambi\" in the Explorer context menu",
                "\"Convert with Mossambi\" is registered in the native shell menu.",
                &mut context_menu,
                dirty,
            );

            if ui.button("Remove from the main menu").clicked() {
                // Shell hook update
            }
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

            ui.label(
                egui::RichText::new("Same folder as the source")
                    .size(design::type_scale::BODY)
                    .color(palette.accent),
            );
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
                    egui::RichText::new("Conversions to keep in the history")
                        .size(design::type_scale::BODY)
                        .color(palette.text_primary),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("+50").clicked() {
                        self.cfg.conversion_history_limit += 50;
                        *dirty = true;
                    }
                    if ui.button("-50").clicked() && self.cfg.conversion_history_limit >= 50 {
                        self.cfg.conversion_history_limit -= 50;
                        *dirty = true;
                    }
                    ui.label(
                        egui::RichText::new(format!("{}", self.cfg.conversion_history_limit))
                            .size(design::type_scale::BODY)
                            .color(palette.accent),
                    );
                });
            });

            ui.add_space(design::space::SM);
            ui.horizontal(|ui| {
                if ui.button("Clear history").clicked() {
                    // clears history
                }
                ui.label(
                    egui::RichText::new("0 conversion(s) in the history")
                        .size(design::type_scale::BODY_SM)
                        .color(palette.text_muted),
                );
            });
        });
    }

    fn render_donut_toolbar(&mut self, ui: &mut Ui, palette: &ThemePalette, dirty: &mut bool) {
        let total_w = ui.available_width();
        let col1_w = 210.0;
        let col2_w = 360.0;
        let col3_w = (total_w - col1_w - col2_w - 32.0).max(220.0);

        ui.horizontal_top(|ui| {
            // ---- COLUMN 1: 1. Choose a profile ----
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
                        .desired_width(col1_w - 10.0),
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
                            let card_rect = ui.allocate_space(Vec2::new(col1_w - 16.0, 32.0)).1;
                            let is_hovered = ui.rect_contains_pointer(card_rect);

                            let bg_color = if is_selected {
                                palette.card_hover
                            } else if is_hovered {
                                Color32::from_rgba_premultiplied(palette.card_hover.r(), palette.card_hover.g(), palette.card_hover.b(), 70)
                            } else {
                                Color32::TRANSPARENT
                            };

                            ui.painter().rect_filled(card_rect, Rounding::same(design::radius::SM), bg_color);
                            if is_selected {
                                ui.painter().rect_stroke(card_rect, Rounding::same(design::radius::SM), Stroke::new(1.0_f32, palette.accent));
                            }

                            let text_color = if is_selected { palette.accent } else { palette.text_primary };
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
                ui.horizontal(|ui| {
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

            ui.add_space(design::space::SM);
            // Vertical divider line between col 1 and col 2
            let div1_rect = ui.allocate_space(Vec2::new(1.0, 560.0)).1;
            ui.painter().rect_filled(div1_rect, Rounding::ZERO, palette.border);
            ui.add_space(design::space::SM);

            // ---- COLUMN 2: 2. Arrange presets ----
            ui.allocate_ui_with_layout(Vec2::new(col2_w, ui.available_height()), Layout::top_down(Align::Min), |ui| {
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
                    let slices = if self.donut_view_back { &p.back_slices } else { &p.front_slices };
                    (p.name.clone(), p.id.clone(), p.front_slices.len(), p.back_slices.len(), slices.clone())
                };

                // Front / Back toggle pills
                ui.horizontal(|ui| {
                    let front_btn = egui::Button::new(
                        egui::RichText::new(format!("Front {}", front_len))
                            .size(design::type_scale::BODY_SM)
                            .color(if !self.donut_view_back { palette.accent } else { palette.text_secondary }),
                    )
                    .fill(if !self.donut_view_back { palette.card_hover } else { palette.card_bg })
                    .stroke(Stroke::new(1.0_f32, if !self.donut_view_back { palette.accent } else { palette.border }))
                    .rounding(Rounding::same(design::radius::SM));
                    if ui.add(front_btn).clicked() {
                        self.donut_view_back = false;
                    }

                    let back_btn = egui::Button::new(
                        egui::RichText::new(format!("Back {}", back_len))
                            .size(design::type_scale::BODY_SM)
                            .color(if self.donut_view_back { palette.accent } else { palette.text_secondary }),
                    )
                    .fill(if self.donut_view_back { palette.card_hover } else { palette.card_bg })
                    .stroke(Stroke::new(1.0_f32, if self.donut_view_back { palette.accent } else { palette.border }))
                    .rounding(Rounding::same(design::radius::SM));
                    if ui.add(back_btn).clicked() {
                        self.donut_view_back = true;
                    }
                });

                ui.label(
                    egui::RichText::new("Flip the open donut: Tab / mouse wheel")
                        .size(design::type_scale::MICRO)
                        .color(palette.text_muted),
                );
                ui.add_space(design::space::SM);

                // Live Vector Radial Donut Preview
                let (preview_rect, _) = ui.allocate_exact_size(Vec2::new(260.0, 240.0), Sense::hover());
                let center = preview_rect.center();
                let painter = ui.painter_at(preview_rect);

                let n_slices = slices_copy.len().max(1);
                let outer_r = 95.0f32;
                let inner_r = 42.0f32;

                use std::f32::consts::PI;
                let step_angle = 2.0 * PI / (n_slices as f32);

                for (idx, slice) in slices_copy.iter().enumerate() {
                    let a0 = -PI / 2.0 + (idx as f32) * step_angle;
                    let a1 = a0 + step_angle;
                    let seg_steps = 16;
                    let mut points = Vec::new();

                    for s in 0..=seg_steps {
                        let t = (s as f32) / (seg_steps as f32);
                        let a = a0 + t * (a1 - a0);
                        points.push(egui::pos2(center.x + outer_r * a.cos(), center.y + outer_r * a.sin()));
                    }
                    for s in (0..=seg_steps).rev() {
                        let t = (s as f32) / (seg_steps as f32);
                        let a = a0 + t * (a1 - a0);
                        points.push(egui::pos2(center.x + inner_r * a.cos(), center.y + inner_r * a.sin()));
                    }

                    painter.add(egui::Shape::convex_polygon(
                        points,
                        Color32::from_rgb(28, 36, 32),
                        Stroke::new(1.0_f32, Color32::from_rgb(55, 75, 65)),
                    ));

                    let mid_a = (a0 + a1) * 0.5;
                    let label_r = (inner_r + outer_r) * 0.5;
                    let text_pos = egui::pos2(center.x + label_r * mid_a.cos(), center.y + label_r * mid_a.sin());
                    painter.text(
                        text_pos,
                        Align2::CENTER_CENTER,
                        &slice.label,
                        egui::FontId::proportional(design::type_scale::MICRO),
                        Color32::from_rgb(220, 235, 230),
                    );
                }

                // Center columella hub
                painter.circle_filled(center, inner_r, Color32::from_rgb(18, 24, 21));
                painter.circle_stroke(center, inner_r, Stroke::new(1.5_f32, palette.accent));
                painter.text(
                    center,
                    Align2::CENTER_CENTER,
                    format!("{}\n{}/12", profile_name, slices_copy.len()),
                    egui::FontId::proportional(design::type_scale::MICRO),
                    palette.accent,
                );

                ui.add_space(design::space::SM);

                let mut do_reset = false;
                let mut do_delete = false;

                // Profile Configuration Card
                card(ui, palette, |ui| {
                    let prof = &mut self.cfg.donut.profiles[self.selected_donut_profile_idx];
                    ui.label(egui::RichText::new("Name").size(design::type_scale::BODY_SM).color(palette.text_secondary));
                    if ui.add(TextEdit::singleline(&mut prof.name).desired_width(f32::INFINITY)).changed() {
                        *dirty = true;
                    }
                    ui.add_space(design::space::XS);

                    ui.horizontal(|ui| {
                        if ui.checkbox(&mut prof.is_mixed_selection, "Use for mixed selections (files of different types)").changed() {
                            *dirty = true;
                        }
                    });
                    ui.add_space(design::space::XS);

                    ui.label(egui::RichText::new("Used for these extensions").size(design::type_scale::BODY_SM).color(palette.text_secondary));
                    let mut ext_to_remove = None;

                    ScrollArea::vertical()
                        .id_salt("ext_tags_scroll")
                        .max_height(100.0)
                        .show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                for (ext_idx, ext) in prof.extensions.iter().enumerate() {
                                    let pill_resp = ui.button(format!("{} x", ext));
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

                    ui.horizontal(|ui| {
                        ui.add(TextEdit::singleline(&mut self.new_ext_input).hint_text(".jpg").desired_width(120.0));
                        if ui.button("+ Add extension").clicked() {
                            let clean = self.new_ext_input.trim();
                            if !clean.is_empty() {
                                let formatted = if clean.starts_with('.') { clean.to_string() } else { format!(".{}", clean) };
                                if !prof.extensions.contains(&formatted) {
                                    prof.extensions.push(formatted);
                                    self.new_ext_input.clear();
                                    *dirty = true;
                                }
                            }
                        }
                    });

                    ui.add_space(design::space::XS);
                    ui.horizontal(|ui| {
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
                    self.cfg.donut.profiles.remove(self.selected_donut_profile_idx);
                    self.selected_donut_profile_idx = 0;
                    *dirty = true;
                }

                ui.add_space(design::space::SM);

                // Appearance Card
                card(ui, palette, |ui| {
                    ui.label(egui::RichText::new("Appearance").size(design::type_scale::TITLE_SM).strong().color(palette.text_primary));
                    ui.add_space(design::space::XS);

                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Size").size(design::type_scale::BODY_SM).color(palette.text_secondary));
                        if ui.add(egui::Slider::new(&mut self.cfg.donut.appearance.size, 320.0..=540.0).suffix(" px")).changed() {
                            *dirty = true;
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Hole").size(design::type_scale::BODY_SM).color(palette.text_secondary));
                        if ui.add(egui::Slider::new(&mut self.cfg.donut.appearance.hole, 30.0..=80.0).suffix(" px")).changed() {
                            *dirty = true;
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Opacity").size(design::type_scale::BODY_SM).color(palette.text_secondary));
                        if ui.add(egui::Slider::new(&mut self.cfg.donut.appearance.opacity, 0.20..=1.00)).changed() {
                            *dirty = true;
                        }
                    });
                });
            });

            ui.add_space(design::space::SM);
            // Vertical divider line between col 2 and col 3
            let div2_rect = ui.allocate_space(Vec2::new(1.0, 560.0)).1;
            ui.painter().rect_filled(div2_rect, Rounding::ZERO, palette.border);
            ui.add_space(design::space::SM);

            // ---- COLUMN 3: Available presets catalog ----
            ui.allocate_ui_with_layout(Vec2::new(col3_w, ui.available_height()), Layout::top_down(Align::Min), |ui| {
                ui.label(
                    egui::RichText::new("Available presets")
                        .size(design::type_scale::TITLE_SM)
                        .strong()
                        .color(palette.text_primary),
                );
                ui.add_space(design::space::XS);

                ui.horizontal(|ui| {
                    let _ = ui.button("+ New preset");
                    let _ = ui.button("New group");
                    let _ = ui.button("Copy to folder");
                });
                ui.add_space(design::space::XS);

                ui.add(
                    TextEdit::singleline(&mut self.donut_preset_search)
                        .hint_text("Search presets")
                        .desired_width(col3_w - 10.0),
                );
                ui.add_space(design::space::XS);

                // Category filter pills
                let categories = ["All", "Image", "Document", "Video & audio", "Edit", "3D & CAD"];
                ui.horizontal_wrapped(|ui| {
                    for cat in categories {
                        let active = self.donut_preset_category == cat;
                        let btn = egui::Button::new(
                            egui::RichText::new(cat)
                                .size(design::type_scale::MICRO)
                                .color(if active { palette.accent } else { palette.text_secondary }),
                        )
                        .fill(if active { palette.card_hover } else { palette.card_bg })
                        .stroke(Stroke::new(1.0_f32, if active { palette.accent } else { palette.border }))
                        .rounding(Rounding::same(design::radius::SM));
                        if ui.add(btn).clicked() {
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
                            if !search_p.is_empty() && !preset_name.to_lowercase().contains(&search_p) {
                                continue;
                            }

                            let btn = egui::Button::new(
                                egui::RichText::new(preset_name)
                                    .size(design::type_scale::BODY_SM)
                                    .color(palette.text_primary),
                            )
                            .fill(palette.card_bg)
                            .stroke(Stroke::new(1.0_f32, palette.border))
                            .rounding(Rounding::same(design::radius::SM));

                            if ui.add_sized(Vec2::new(col3_w - 20.0, 28.0), btn).clicked()
                                && self.selected_donut_profile_idx < self.cfg.donut.profiles.len()
                            {
                                let prof = &mut self.cfg.donut.profiles[self.selected_donut_profile_idx];
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
            });
        });
    }

    fn render_shortcuts(&mut self, ui: &mut Ui, palette: &ThemePalette, dirty: &mut bool) {
        ui.label(
            egui::RichText::new(
                "Choose how the donut toolbar opens. You can add as many gestures as you like.",
            )
            .size(design::type_scale::BODY_SM)
            .color(palette.text_secondary),
        );
        ui.add_space(design::space::XS);

        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Global input detection is running.")
                    .size(design::type_scale::BODY_SM)
                    .color(palette.accent),
            );
            ui.button("Retry gestures").clicked();
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let _ = ui.button("+ Add gesture");
            });
        });
        ui.add_space(design::space::SM);

        // Card 1: Ctrl + Left
        card(ui, palette, |ui| {
            ui.horizontal(|ui| {
                let mut on = true;
                let _ = ui.checkbox(&mut on, "");
                ui.label(
                    egui::RichText::new("Ctrl + Left")
                        .size(design::type_scale::BODY)
                        .strong()
                        .color(palette.text_primary),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let _ = ui.button("Delete");
                });
            });
            ui.add_space(design::space::XS);

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new("Gesture")
                            .size(design::type_scale::CAPTION)
                            .color(palette.text_muted),
                    );
                    ui.label(
                        egui::RichText::new("Modifier + drag files")
                            .size(design::type_scale::BODY_SM)
                            .color(palette.text_primary),
                    );
                });
                ui.add_space(design::space::MD);
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new("Active in")
                            .size(design::type_scale::CAPTION)
                            .color(palette.text_muted),
                    );
                    ui.label(
                        egui::RichText::new("File managers and desktop")
                            .size(design::type_scale::BODY_SM)
                            .color(palette.text_primary),
                    );
                });
            });
            ui.add_space(design::space::XS);

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new("Modifier keys")
                            .size(design::type_scale::CAPTION)
                            .color(palette.text_muted),
                    );
                    ui.horizontal(|ui| {
                        let _ = ui.button("Ctrl");
                        let _ = ui.button("Alt");
                        let _ = ui.button("Shift");
                        let _ = ui.button("Win");
                    });
                });
                ui.add_space(design::space::MD);
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new("Mouse button")
                            .size(design::type_scale::CAPTION)
                            .color(palette.text_muted),
                    );
                    ui.label(
                        egui::RichText::new("Left")
                            .size(design::type_scale::BODY_SM)
                            .color(palette.text_primary),
                    );
                });
            });
            ui.add_space(design::space::XS);

            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Drag distance")
                        .size(design::type_scale::CAPTION)
                        .color(palette.text_muted),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new("System default")
                            .size(design::type_scale::CAPTION)
                            .color(palette.text_secondary),
                    );
                });
            });
        });

        ui.add_space(design::space::MD);

        // Card 2: Ctrl + Alt + Space
        card(ui, palette, |ui| {
            ui.horizontal(|ui| {
                let mut on = true;
                let _ = ui.checkbox(&mut on, "");
                ui.label(
                    egui::RichText::new("Ctrl + Alt + Space")
                        .size(design::type_scale::BODY)
                        .strong()
                        .color(palette.text_primary),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let _ = ui.button("Delete");
                });
            });
            ui.add_space(design::space::XS);

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new("Gesture")
                            .size(design::type_scale::CAPTION)
                            .color(palette.text_muted),
                    );
                    ui.label(
                        egui::RichText::new("Keyboard shortcut (selected files)")
                            .size(design::type_scale::BODY_SM)
                            .color(palette.text_primary),
                    );
                });
                ui.add_space(design::space::MD);
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new("Active in")
                            .size(design::type_scale::CAPTION)
                            .color(palette.text_muted),
                    );
                    ui.label(
                        egui::RichText::new("File managers and desktop")
                            .size(design::type_scale::BODY_SM)
                            .color(palette.text_primary),
                    );
                });
            });
            ui.add_space(design::space::XS);

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new("Modifier keys")
                            .size(design::type_scale::CAPTION)
                            .color(palette.text_muted),
                    );
                    ui.horizontal(|ui| {
                        let _ = ui.button("Ctrl");
                        let _ = ui.button("Alt");
                        let _ = ui.button("Shift");
                        let _ = ui.button("Win");
                    });
                });
                ui.add_space(design::space::MD);
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new("Key")
                            .size(design::type_scale::CAPTION)
                            .color(palette.text_muted),
                    );
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Space")
                                .size(design::type_scale::BODY_SM)
                                .color(palette.text_primary),
                        );
                        let _ = ui.button("Record");
                        let _ = ui.button("Clear");
                    });
                });
            });
        });

        ui.add_space(design::space::MD);

        // Card 3: Flip the donut
        card(ui, palette, |ui| {
            ui.label(
                egui::RichText::new("Flip the donut")
                    .size(design::type_scale::TITLE_SM)
                    .strong()
                    .color(palette.text_primary),
            );
            ui.label(egui::RichText::new("While the donut is open, this key or a turn of the mouse wheel shows its back. Each donut gets its back under Donut toolbar.").size(design::type_scale::CAPTION).color(palette.text_secondary));
            ui.add_space(design::space::SM);

            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Key:")
                        .size(design::type_scale::BODY_SM)
                        .color(palette.text_secondary),
                );
                ui.label(
                    egui::RichText::new("Tab")
                        .size(design::type_scale::BODY)
                        .strong()
                        .color(palette.accent),
                );
                let _ = ui.button("Record");
                let _ = ui.button("Use Tab");
                let _ = ui.button("Clear");
            });
            ui.add_space(design::space::XS);

            let mut wheel_flip = true;
            toggle_row(
                ui,
                palette,
                "Mouse wheel flips the donut",
                "",
                &mut wheel_flip,
                dirty,
            );
        });

        ui.add_space(design::space::MD);

        // Card 4: Never trigger in these apps
        card(ui, palette, |ui| {
            ui.label(
                egui::RichText::new("Never trigger in these apps")
                    .size(design::type_scale::TITLE_SM)
                    .strong()
                    .color(palette.text_primary),
            );
            ui.add_space(design::space::XS);

            ui.add(
                TextEdit::singleline(&mut self.exclusion_input)
                    .hint_text("Type an app name and press Enter, e.g. photoshop")
                    .desired_width(f32::INFINITY),
            );
            ui.add_space(design::space::XS);

            ui.horizontal(|ui| {
                let _ = ui.button("+ Pick a running app");
            });
            ui.add_space(design::space::XS);

            ui.label(
                egui::RichText::new(
                    "Gestures and shortcuts do nothing while the pointer is over these apps.",
                )
                .size(design::type_scale::CAPTION)
                .color(palette.text_muted),
            );
        });

        ui.add_space(design::space::MD);
        section(ui, palette, "Gesture physics & sensitivity");
        card(ui, palette, |ui| {
            self.render_input(ui, palette, dirty);
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
                            let btn = egui::Button::new(
                                egui::RichText::new(text)
                                    .size(design::type_scale::BODY_SM)
                                    .color(if current {
                                        palette.accent
                                    } else {
                                        palette.text_secondary
                                    }),
                            )
                            .fill(if current {
                                palette.card_hover
                            } else {
                                palette.card_bg
                            })
                            .stroke(Stroke::new(
                                1.0_f32,
                                if current {
                                    palette.accent
                                } else {
                                    palette.border
                                },
                            ))
                            .rounding(Rounding::same(design::radius::SM));
                            if ui.add_sized(Vec2::new(50.0, 24.0), btn).clicked() {
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

    fn render_appearance(
        &mut self,
        ui: &mut Ui,
        palette: &ThemePalette,
        current_theme: &mut ThemeStyle,
        dirty: &mut bool,
    ) {
        section(ui, palette, "Theme");
        ui.label(
            egui::RichText::new("Pick a certified Pantone theme.")
                .size(design::type_scale::BODY_SM)
                .color(palette.text_secondary),
        );
        ui.add_space(design::space::SM);
        // Compact swatch grid.
        let themes = ThemeStyle::all();
        for chunk in themes.chunks(4) {
            ui.horizontal(|ui| {
                for t in chunk {
                    let is_active = *current_theme == *t;
                    let pt = t.to_pantone();
                    let (bg_r, bg_g, bg_b) = pt.bg.to_rgb();
                    let (acc_r, acc_g, acc_b) = pt.accent.to_rgb();
                    let bg_col = Color32::from_rgb(bg_r, bg_g, bg_b);
                    let acc_col = Color32::from_rgb(acc_r, acc_g, acc_b);
                    let btn = egui::Button::new(
                        egui::RichText::new(t.name())
                            .size(design::type_scale::MICRO)
                            .color(if is_active {
                                palette.accent
                            } else {
                                palette.text_secondary
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
                    if ui.add_sized(Vec2::new(150.0, 26.0), btn).clicked() {
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
        let glass = self.cfg.glass_effect;
        let glass_lbl = glass_label(glass).to_string();
        let glass_mut = &mut self.cfg.glass_effect;
        setting_row(
            ui,
            palette,
            "Glass effect",
            &glass_lbl,
            dirty,
            |ui, dirty| {
                for (v, lab) in [
                    (GlassEffect::FrostedGlass, "Frosted"),
                    (GlassEffect::LiquidGlass, "Liquid"),
                    (GlassEffect::Opaque, "Opaque"),
                ] {
                    let active = *glass_mut == v;
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new(lab)
                                    .size(design::type_scale::SMALL)
                                    .color(if active {
                                        palette.accent
                                    } else {
                                        palette.text_secondary
                                    }),
                            )
                            .fill(if active {
                                palette.card_hover
                            } else {
                                Color32::TRANSPARENT
                            })
                            .stroke(Stroke::new(
                                1.0_f32,
                                if active {
                                    palette.accent
                                } else {
                                    palette.border
                                },
                            ))
                            .rounding(Rounding::same(design::radius::XS)),
                        )
                        .clicked()
                    {
                        *glass_mut = v;
                        *dirty = true;
                    }
                }
            },
        );

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
                    .add(egui::Slider::new(op_mut, 0.5..=1.0).show_value(false))
                    .changed()
                {
                    *dirty = true;
                }
            },
        );

        let ms = self.cfg.menu_style;
        let ms_lbl = menu_style_label(ms).to_string();
        let ms_mut = &mut self.cfg.menu_style;
        setting_row(ui, palette, "Menu style", &ms_lbl, dirty, |ui, dirty| {
            for (v, lab) in [
                (MenuStyle::Finder, "Finder"),
                (MenuStyle::Windows, "Windows"),
                (MenuStyle::Linux, "Linux"),
            ] {
                let active = *ms_mut == v;
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new(lab)
                                .size(design::type_scale::SMALL)
                                .color(if active {
                                    palette.accent
                                } else {
                                    palette.text_secondary
                                }),
                        )
                        .fill(if active {
                            palette.card_hover
                        } else {
                            Color32::TRANSPARENT
                        })
                        .stroke(Stroke::new(
                            1.0_f32,
                            if active {
                                palette.accent
                            } else {
                                palette.border
                            },
                        ))
                        .rounding(Rounding::same(design::radius::XS)),
                    )
                    .clicked()
                {
                    *ms_mut = v;
                    *dirty = true;
                }
            }
        });

        let cm = self.cfg.checkbox_mode;
        let cm_lbl = checkbox_mode_label(cm).to_string();
        let cm_mut = &mut self.cfg.checkbox_mode;
        setting_row(ui, palette, "Checkbox mode", &cm_lbl, dirty, |ui, dirty| {
            for (v, lab) in [
                (CheckboxMode::Always, "Always"),
                (CheckboxMode::Hover, "On hover"),
                (CheckboxMode::Off, "Hidden"),
            ] {
                let active = *cm_mut == v;
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new(lab)
                                .size(design::type_scale::SMALL)
                                .color(if active {
                                    palette.accent
                                } else {
                                    palette.text_secondary
                                }),
                        )
                        .fill(if active {
                            palette.card_hover
                        } else {
                            Color32::TRANSPARENT
                        })
                        .stroke(Stroke::new(
                            1.0_f32,
                            if active {
                                palette.accent
                            } else {
                                palette.border
                            },
                        ))
                        .rounding(Rounding::same(design::radius::XS)),
                    )
                    .clicked()
                {
                    *cm_mut = v;
                    *dirty = true;
                }
            }
        });

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

        let mut font = self.cfg.font_family.clone().unwrap_or_default();
        let font_lbl = if font.is_empty() {
            "System".to_string()
        } else {
            font.clone()
        };
        setting_row(ui, palette, "Font family", &font_lbl, dirty, |ui, dirty| {
            if ui
                .add(TextEdit::singleline(&mut font).desired_width(140.0))
                .changed()
            {
                self.cfg.font_family = if font.trim().is_empty() {
                    None
                } else {
                    Some(font.trim().to_string())
                };
                *dirty = true;
            }
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
    }

    fn render_explorer(&mut self, ui: &mut Ui, palette: &ThemePalette, dirty: &mut bool) {
        section(ui, palette, "Layout");
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
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new(*name)
                                    .size(design::type_scale::SMALL)
                                    .color(if active {
                                        palette.accent
                                    } else {
                                        palette.text_secondary
                                    }),
                            )
                            .fill(if active {
                                palette.card_hover
                            } else {
                                Color32::TRANSPARENT
                            })
                            .stroke(Stroke::new(
                                1.0_f32,
                                if active {
                                    palette.accent
                                } else {
                                    palette.border
                                },
                            ))
                            .rounding(Rounding::same(design::radius::XS)),
                        )
                        .clicked()
                    {
                        *lay_mut = i as u8;
                        *dirty = true;
                    }
                }
            },
        );

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
                    .add(egui::Slider::new(gis_mut, 32..=128).show_value(false))
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

        let scp = self.cfg.show_custom_pinned;
        let scp_lbl = on_off(scp).to_string();
        let scp_mut = &mut self.cfg.show_custom_pinned;
        setting_row(
            ui,
            palette,
            "Show custom pinned folders",
            &scp_lbl,
            dirty,
            |ui, dirty| {
                if ui.checkbox(scp_mut, "").changed() {
                    *dirty = true;
                }
            },
        );

        section(ui, palette, "Custom pinned folders");
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
    }

    fn render_behavior(&mut self, ui: &mut Ui, palette: &ThemePalette, dirty: &mut bool) {
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

        let lal = self.cfg.launch_at_login;
        let lal_lbl = on_off(lal).to_string();
        let lal_mut = &mut self.cfg.launch_at_login;
        setting_row(
            ui,
            palette,
            "Launch at login",
            &lal_lbl,
            dirty,
            |ui, dirty| {
                if ui.checkbox(lal_mut, "").changed() {
                    *dirty = true;
                }
            },
        );

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
        let wt_lbl = format!("{}", wt);
        let wt_mut = &mut self.cfg.worker_threads;
        setting_row(
            ui,
            palette,
            "Worker threads",
            &wt_lbl,
            dirty,
            |ui, dirty| {
                if ui
                    .add(egui::Slider::new(wt_mut, 1..=16).show_value(false))
                    .changed()
                {
                    *dirty = true;
                }
            },
        );

        let status = crate::core::default_explorer::status();
        let status_lbl = match status {
            crate::core::default_explorer::TakeoverState::Active => "Active (Mossambi is Default)",
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
    }

    fn render_input(&mut self, ui: &mut Ui, palette: &ThemePalette, dirty: &mut bool) {
        let egw = self.cfg.enable_gesture_wakeup;
        let egw_lbl = on_off(egw).to_string();
        let egw_mut = &mut self.cfg.enable_gesture_wakeup;
        setting_row(
            ui,
            palette,
            "Gesture wake-up",
            &egw_lbl,
            dirty,
            |ui, dirty| {
                if ui.checkbox(egw_mut, "").changed() {
                    *dirty = true;
                }
            },
        );

        let ehw = self.cfg.enable_hotkey_wakeup;
        let ehw_lbl = on_off(ehw).to_string();
        let ehw_mut = &mut self.cfg.enable_hotkey_wakeup;
        setting_row(
            ui,
            palette,
            "Hotkey wake-up",
            &ehw_lbl,
            dirty,
            |ui, dirty| {
                if ui.checkbox(ehw_mut, "").changed() {
                    *dirty = true;
                }
            },
        );

        let gt = self.cfg.gesture_type.clone();
        let gt_lbl = gesture_type_label(&gt).to_string();
        let gt_mut = &mut self.cfg.gesture_type;
        setting_row(ui, palette, "Gesture type", &gt_lbl, dirty, |ui, dirty| {
            for (v, lab) in [("shake", "Shake"), ("circle", "Circle"), ("both", "Both")] {
                let active = *gt_mut == v;
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new(lab)
                                .size(design::type_scale::SMALL)
                                .color(if active {
                                    palette.accent
                                } else {
                                    palette.text_secondary
                                }),
                        )
                        .fill(if active {
                            palette.card_hover
                        } else {
                            Color32::TRANSPARENT
                        })
                        .stroke(Stroke::new(
                            1.0_f32,
                            if active {
                                palette.accent
                            } else {
                                palette.border
                            },
                        ))
                        .rounding(Rounding::same(design::radius::XS)),
                    )
                    .clicked()
                {
                    *gt_mut = v.to_string();
                    *dirty = true;
                }
            }
        });

        let ss = self.cfg.shake_sensitivity_px;
        let ss_lbl = format!("{:.0} px", ss);
        let ss_mut = &mut self.cfg.shake_sensitivity_px;
        setting_row(
            ui,
            palette,
            "Shake sensitivity",
            &ss_lbl,
            dirty,
            |ui, dirty| {
                if ui
                    .add(egui::Slider::new(ss_mut, 4.0..=40.0).show_value(false))
                    .changed()
                {
                    *dirty = true;
                }
            },
        );

        let gt2 = self.cfg.gesture_timeout_ms;
        let gt2_lbl = format!("{} ms", gt2);
        let gt2_mut = &mut self.cfg.gesture_timeout_ms;
        setting_row(
            ui,
            palette,
            "Gesture timeout",
            &gt2_lbl,
            dirty,
            |ui, dirty| {
                if ui
                    .add(egui::Slider::new(gt2_mut, 200..=2000).show_value(false))
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
            "Deep-drag detection only",
            &dd_lbl,
            dirty,
            |ui, dirty| {
                if ui.checkbox(dd_mut, "").changed() {
                    *dirty = true;
                }
            },
        );

        let ws = self.cfg.wheel_size;
        let ws_lbl = format!("{}", ws);
        let ws_mut = &mut self.cfg.wheel_size;
        setting_row(ui, palette, "Wheel size", &ws_lbl, dirty, |ui, dirty| {
            if ui
                .add(egui::Slider::new(ws_mut, 1..=8).show_value(false))
                .changed()
            {
                *dirty = true;
            }
        });

        let dtd = self.cfg.drop_target_default.clone();
        let dtd_lbl = if dtd.is_empty() {
            "Auto".to_string()
        } else {
            dtd.clone()
        };
        let dtd_mut = &mut self.cfg.drop_target_default;
        setting_row(
            ui,
            palette,
            "Drop target default",
            &dtd_lbl,
            dirty,
            |ui, dirty| {
                if ui
                    .add(TextEdit::singleline(dtd_mut).desired_width(120.0))
                    .changed()
                {
                    *dirty = true;
                }
            },
        );

        let kp = self.cfg.keybinding_profile;
        let kp_lbl = if kp == 0 {
            "macOS Finder"
        } else {
            "Windows Explorer"
        };
        let kp_mut = &mut self.cfg.keybinding_profile;
        setting_row(
            ui,
            palette,
            "Keybinding profile",
            kp_lbl,
            dirty,
            |ui, dirty| {
                if ui.selectable_label(*kp_mut == 0, "macOS Finder").clicked() {
                    *kp_mut = 0;
                    *dirty = true;
                }
                if ui
                    .selectable_label(*kp_mut == 1, "Windows Explorer")
                    .clicked()
                {
                    *kp_mut = 1;
                    *dirty = true;
                }
            },
        );
    }

    fn render_performance(&mut self, ui: &mut Ui, palette: &ThemePalette, dirty: &mut bool) {
        let wt = self.cfg.worker_threads;
        let wt_lbl = format!("{}", wt);
        let wt_mut = &mut self.cfg.worker_threads;
        setting_row(
            ui,
            palette,
            "Worker threads",
            &wt_lbl,
            dirty,
            |ui, dirty| {
                if ui
                    .add(egui::Slider::new(wt_mut, 1..=16).show_value(false))
                    .changed()
                {
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

        let bgw = self.cfg.background_work;
        let bgw_lbl = on_off(bgw).to_string();
        let bgw_mut = &mut self.cfg.background_work;
        setting_row(
            ui,
            palette,
            "Background work",
            &bgw_lbl,
            dirty,
            |ui, dirty| {
                if ui.checkbox(bgw_mut, "").changed() {
                    *dirty = true;
                }
            },
        );

        let ai = self.cfg.local_ai_enabled;
        let ai_lbl = on_off(ai).to_string();
        let ai_mut = &mut self.cfg.local_ai_enabled;
        setting_row(
            ui,
            palette,
            "Local AI (Ollama/LM Studio)",
            &ai_lbl,
            dirty,
            |ui, dirty| {
                if ui.checkbox(ai_mut, "").changed() {
                    *dirty = true;
                }
            },
        );

        let ep_lbl = self.cfg.local_ai_endpoint.clone();
        let ep_mut = &mut self.cfg.local_ai_endpoint;
        setting_row(
            ui,
            palette,
            "Local AI endpoint",
            &ep_lbl,
            dirty,
            |ui, dirty| {
                if ui
                    .add(TextEdit::singleline(ep_mut).desired_width(180.0))
                    .changed()
                {
                    *dirty = true;
                }
            },
        );

        let mdl_lbl = self.cfg.local_ai_model.clone();
        let mdl_mut = &mut self.cfg.local_ai_model;
        setting_row(
            ui,
            palette,
            "Local AI model",
            &mdl_lbl,
            dirty,
            |ui, dirty| {
                if ui
                    .add(TextEdit::singleline(mdl_mut).desired_width(140.0))
                    .changed()
                {
                    *dirty = true;
                }
            },
        );
    }

    fn render_conversion(
        &mut self,
        ui: &mut Ui,
        palette: &ThemePalette,
        settings: &mut QuickSettingsState,
        dirty: &mut bool,
    ) {
        section(ui, palette, "Output");
        let om = self.cfg.output_mode;
        let om_lbl = output_mode_label(om).to_string();
        let om_mut = &mut self.cfg.output_mode;
        setting_row(ui, palette, "Output mode", &om_lbl, dirty, |ui, dirty| {
            for (v, lab) in [
                (OutputMode::Sibling, "Sibling"),
                (OutputMode::Custom, "Custom"),
            ] {
                let active = *om_mut == v;
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new(lab)
                                .size(design::type_scale::SMALL)
                                .color(if active {
                                    palette.accent
                                } else {
                                    palette.text_secondary
                                }),
                        )
                        .fill(if active {
                            palette.card_hover
                        } else {
                            Color32::TRANSPARENT
                        })
                        .stroke(Stroke::new(
                            1.0_f32,
                            if active {
                                palette.accent
                            } else {
                                palette.border
                            },
                        ))
                        .rounding(Rounding::same(design::radius::XS)),
                    )
                    .clicked()
                {
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

        section(ui, palette, "Conversion defaults");
        ui.label(
            egui::RichText::new("Quick conversion settings (image, audio, video, archive, exam).")
                .size(design::type_scale::BODY_SM)
                .color(palette.text_secondary),
        );
        ui.add_space(design::space::SM);

        let iq = settings.image_quality;
        let iq_lbl = format!("{}%", iq);
        let iq_mut = &mut settings.image_quality;
        setting_row(ui, palette, "Image quality", &iq_lbl, dirty, |ui, dirty| {
            if ui
                .add(egui::Slider::new(iq_mut, 1..=100).show_value(false))
                .changed()
            {
                *dirty = true;
            }
        });

        let il = settings.image_lossless;
        let il_lbl = on_off(il).to_string();
        let il_mut = &mut settings.image_lossless;
        setting_row(
            ui,
            palette,
            "Image lossless",
            &il_lbl,
            dirty,
            |ui, dirty| {
                if ui.checkbox(il_mut, "").changed() {
                    *dirty = true;
                }
            },
        );

        let idpi = settings.image_dpi;
        let idpi_lbl = format!("{}", idpi);
        let idpi_mut = &mut settings.image_dpi;
        setting_row(ui, palette, "Image DPI", &idpi_lbl, dirty, |ui, dirty| {
            if ui
                .add(egui::Slider::new(idpi_mut, 72..=600).show_value(false))
                .changed()
            {
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
        setting_row(ui, palette, "Audio bitrate", &ab_lbl, dirty, |ui, dirty| {
            if ui
                .add(TextEdit::singleline(ab_mut).desired_width(120.0))
                .changed()
            {
                *dirty = true;
            }
        });

        let asl = settings.audio_strict_lossless;
        let asl_lbl = on_off(asl).to_string();
        let asl_mut = &mut settings.audio_strict_lossless;
        setting_row(
            ui,
            palette,
            "Audio strict lossless",
            &asl_lbl,
            dirty,
            |ui, dirty| {
                if ui.checkbox(asl_mut, "").changed() {
                    *dirty = true;
                }
            },
        );

        let vcrf = settings.video_crf;
        let vcrf_lbl = format!("{}", vcrf);
        let vcrf_mut = &mut settings.video_crf;
        setting_row(ui, palette, "Video CRF", &vcrf_lbl, dirty, |ui, dirty| {
            if ui
                .add(egui::Slider::new(vcrf_mut, 0..=51).show_value(false))
                .changed()
            {
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
        setting_row(ui, palette, "Video preset", &vp_lbl, dirty, |ui, dirty| {
            if ui
                .add(TextEdit::singleline(vp_mut).desired_width(120.0))
                .changed()
            {
                *dirty = true;
            }
        });

        let vhw = settings.video_hw_accel;
        let vhw_lbl = on_off(vhw).to_string();
        let vhw_mut = &mut settings.video_hw_accel;
        setting_row(
            ui,
            palette,
            "Video HW accel",
            &vhw_lbl,
            dirty,
            |ui, dirty| {
                if ui.checkbox(vhw_mut, "").changed() {
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
        setting_row(
            ui,
            palette,
            "Archive compression",
            &ac_lbl,
            dirty,
            |ui, dirty| {
                if ui
                    .add(TextEdit::singleline(ac_mut).desired_width(120.0))
                    .changed()
                {
                    *dirty = true;
                }
            },
        );

        let em = settings.exam_mode_enabled;
        let em_lbl = on_off(em).to_string();
        let em_mut = &mut settings.exam_mode_enabled;
        setting_row(
            ui,
            palette,
            "Exam mode enabled",
            &em_lbl,
            dirty,
            |ui, dirty| {
                if ui.checkbox(em_mut, "").changed() {
                    *dirty = true;
                }
            },
        );
    }

    fn render_about(&mut self, ui: &mut Ui, palette: &ThemePalette) {
        ui.label(
            egui::RichText::new("Mossambi")
                .size(design::type_scale::TITLE_LG)
                .strong()
                .color(palette.text_primary),
        );
        ui.label(
            egui::RichText::new("File explorer and conversion platform.")
                .size(design::type_scale::BODY)
                .color(palette.text_secondary),
        );
        ui.add_space(design::space::MD);
        ui.label(
            egui::RichText::new("Version: 0.1.0")
                .size(design::type_scale::BODY_SM)
                .color(palette.text_muted),
        );
        ui.label(
            egui::RichText::new("Design language: macOS Finder / System Settings.")
                .size(design::type_scale::BODY_SM)
                .color(palette.text_muted),
        );
    }
}

// ---- helpers ----

fn section(ui: &mut Ui, palette: &ThemePalette, title: &str) {
    ui.add_space(design::space::SM);
    ui.label(
        egui::RichText::new(title)
            .size(design::type_scale::TITLE_SM)
            .strong()
            .color(palette.text_primary),
    );
    ui.add_space(design::space::XS);
}

fn card<R>(ui: &mut Ui, palette: &ThemePalette, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
    Frame::none()
        .fill(palette.card_bg)
        .stroke(Stroke::new(1.0_f32, palette.border))
        .rounding(Rounding::same(design::radius::MD))
        .inner_margin(Margin::same(design::space::SM))
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
    ui.allocate_ui_with_layout(
        Vec2::new(w, 36.0),
        Layout::left_to_right(Align::Center),
        |ui| {
            ui.vertical(|ui| {
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
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let text = if *val { "On" } else { "Off" };
                let btn = egui::Button::new(
                    egui::RichText::new(text)
                        .size(design::type_scale::BODY_SM)
                        .color(if *val {
                            palette.accent
                        } else {
                            palette.text_secondary
                        }),
                )
                .fill(if *val {
                    palette.card_hover
                } else {
                    palette.card_bg
                })
                .stroke(Stroke::new(
                    1.0_f32,
                    if *val { palette.accent } else { palette.border },
                ))
                .rounding(Rounding::same(design::radius::SM));
                if ui.add_sized(Vec2::new(50.0, 24.0), btn).clicked() {
                    *val = !*val;
                    *dirty = true;
                }
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
    let pos = ui.input(|i| i.pointer.hover_pos());
    ui.allocate_ui_with_layout(
        Vec2::new(w, 30.0),
        Layout::left_to_right(Align::Center),
        |ui| {
            let r = ui.max_rect();
            if let Some(p) = pos {
                if r.contains(p) {
                    ui.painter().rect_filled(
                        r,
                        Rounding::same(design::radius::SM),
                        Color32::from_rgba_premultiplied(
                            palette.card_hover.r(),
                            palette.card_hover.g(),
                            palette.card_hover.b(),
                            60,
                        ),
                    );
                }
            }
            ui.horizontal(|ui| {
                ui.add_space(design::space::XS);
                ui.label(
                    egui::RichText::new(label)
                        .size(design::type_scale::BODY)
                        .color(palette.text_primary),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
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
            });
        },
    );
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
        "worker_threads" => Some(format!("{}", cfg.worker_threads)),
        "icon_pack" => Some(IconPack::from_config_id(&cfg.icon_pack).label().to_string()),
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
        ("Show custom pinned folders", "show_custom_pinned"),
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
    ] {
        let v = setting_value_label(key, cfg).unwrap_or_default();
        if q.is_empty() || label.to_lowercase().contains(&q) || v.to_lowercase().contains(&q) {
            // map key -> category title
            let cat_title = match key {
                "glass_effect" | "window_opacity" | "menu_style" | "checkbox_mode"
                | "icon_pack" => "Appearance",
                "show_hidden_files"
                | "show_tags_section"
                | "show_custom_pinned"
                | "calculate_checksums" => "Explorer",
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

fn gesture_type_label(s: &str) -> &'static str {
    match s {
        "shake" => "Shake",
        "circle" => "Circle",
        "both" => "Both",
        _ => "Both",
    }
}
