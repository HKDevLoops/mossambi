// tests/test_preferences_categories.rs
//
// Regression tests for Preferences 10-tab structure matching Filee v2.5.0.
// Pure 7-bit ASCII.

use mossambi::gui::preferences_view::{preference_categories, PreferencesView};

#[test]
fn test_preferences_has_exact_10_filee_tabs() {
    let cats = preference_categories();
    assert_eq!(cats.len(), 10, "Preferences must contain exactly 10 tabs");

    let expected_tabs = [
        ("home", "Home"),
        ("general", "General"),
        ("shortcuts", "Shortcuts"),
        ("donut", "Donut toolbar"),
        ("presets", "Presets"),
        ("fields", "Fields"),
        ("watch_folders", "Watch folders"),
        ("theme", "Theme"),
        ("engines", "Engines"),
        ("about", "About"),
    ];

    for (idx, (exp_id, exp_title)) in expected_tabs.iter().enumerate() {
        assert_eq!(cats[idx].id, *exp_id, "Tab index {} ID mismatch", idx);
        assert_eq!(
            cats[idx].title, *exp_title,
            "Tab index {} title mismatch",
            idx
        );
    }
}

#[test]
fn test_preferences_default_selected_tab_is_home_or_general() {
    let prefs = PreferencesView::default();
    assert_eq!(prefs.selected, 0);
}
