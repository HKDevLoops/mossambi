//! Headless smoke tests for Mossambi GUI components using `egui::Context::run`.
//!
//! Renders preferences tabs, studio chrome presets, and main view tabs
//! in a purely headless environment to ensure complete layout stability
//! and raise structural test coverage.

use eframe::egui::{Context, RawInput};
use mossambi::core::config::{LookPreset, MossambiConfig};
use mossambi::core::scripting::ScriptConsole;
use mossambi::gui::macos_theme::ThemeStyle;
use mossambi::gui::preferences_view::{preference_categories, PreferencesView};
use mossambi::gui::quick_settings::QuickSettingsState;
use mossambi::gui::studio_view::{StudioTab, StudioView};

#[test]
fn test_preferences_all_categories_render_headless() {
    let ctx = Context::default();
    let mut prefs = PreferencesView::default();
    let mut current_theme = ThemeStyle::default();
    let mut quick_settings = QuickSettingsState::default();
    let mut console = ScriptConsole::default();

    let categories = preference_categories();
    for (idx, cat) in categories.iter().enumerate() {
        prefs.selected = idx;

        let _output = ctx.run(RawInput::default(), |ctx| {
            eframe::egui::CentralPanel::default().show(ctx, |ui| {
                let palette = current_theme.colors();
                prefs.render(
                    ui,
                    &palette,
                    &mut current_theme,
                    &mut quick_settings,
                    &mut console,
                    (None, Vec::new()),
                );
            });
        });

        assert_eq!(
            prefs.selected, idx,
            "Category {} must remain active",
            cat.title
        );
    }
}

#[test]
fn test_studio_view_all_presets_render_headless() {
    let ctx = Context::default();
    let mut studio = StudioView::default();
    let mut current_theme = ThemeStyle::default();
    let mut quick_settings = QuickSettingsState::default();

    for preset in LookPreset::all() {
        let mut config = MossambiConfig::default();
        config.apply_look_preset(preset);

        let _output = ctx.run(RawInput::default(), |ctx| {
            studio.render(
                ctx,
                &mut current_theme,
                &mut config,
                &None,
                &mut quick_settings,
                || {},
                |_| {},
                |_, _| {},
                |_| {},
            );
        });

        assert_eq!(config.look_preset, Some(preset));
    }
}

#[test]
fn test_studio_view_all_tabs_render_headless() {
    let ctx = Context::default();
    let mut studio = StudioView::default();
    let mut current_theme = ThemeStyle::default();
    let mut config = MossambiConfig::default();
    let mut quick_settings = QuickSettingsState::default();

    let all_tabs = [
        StudioTab::FileExplorer,
        StudioTab::Transcoder,
        StudioTab::ExamStudio,
        StudioTab::QualityInspector,
        StudioTab::ArchiveSuite,
        StudioTab::HardwareTelemetry,
        StudioTab::Preferences,
    ];

    for tab in all_tabs {
        studio.current_tab = tab;

        let _output = ctx.run(RawInput::default(), |ctx| {
            studio.render(
                ctx,
                &mut current_theme,
                &mut config,
                &None,
                &mut quick_settings,
                || {},
                |_| {},
                |_, _| {},
                |_| {},
            );
        });

        assert_eq!(studio.current_tab, tab);
    }
}

#[test]
fn test_explorer_view_all_modes_render_headless() {
    let ctx = Context::default();
    let mut explorer = mossambi::gui::explorer_view::ExplorerView::default();
    let current_theme = ThemeStyle::default();
    let palette = current_theme.colors();

    let modes = [
        mossambi::gui::explorer_view::ViewMode::Details,
        mossambi::gui::explorer_view::ViewMode::Grid,
        mossambi::gui::explorer_view::ViewMode::Columns,
    ];

    for mode in modes {
        explorer.view_mode = mode;
        explorer.is_home_view = false;

        let _output = ctx.run(RawInput::default(), |ctx| {
            eframe::egui::CentralPanel::default().show(ctx, |ui| {
                explorer.render(ui, &palette, |_| {}, |_, _| {}, |_| {}, |_| {}, |_| {});
            });
        });

        assert_eq!(explorer.view_mode, mode);
    }

    // Home view rendering
    explorer.is_home_view = true;
    let _output = ctx.run(RawInput::default(), |ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            explorer.render(ui, &palette, |_| {}, |_, _| {}, |_| {}, |_| {}, |_| {});
        });
    });
    assert!(explorer.is_home_view);
}
