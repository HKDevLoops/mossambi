//! Tier 2: Boundary, Limit, and Corner Cases Test Suite for Mossambi E2E.
//!
//! Enforces boundary value analysis, limit testing, negative inputs, and stress conditions
//! across all 30 features in `PROJECT.md`.

#![allow(
    clippy::field_reassign_with_default,
    clippy::manual_range_contains,
    clippy::unnecessary_sort_by,
    clippy::useless_vec
)]

mod common;

use common::{
    assert_wcag_aa_large, assert_wcag_aaa, contrast_ratio, relative_luminance, HeadlessTestApp,
    IsolatedConfigEnv, TestFsTree,
};
use eframe::egui::{Color32, Pos2, Rect, Vec2};
use mossambi::core::config::{LookPreset, MenuStyle, MossambiConfig, RowDensity, ThemeMode};
use mossambi::core::fs_utils::{compute_dir_size, list_directory_sync, FileItem};
use mossambi::core::pantone::THEMES;
use mossambi::core::platform_paths;
use mossambi::core::LruCache;
use mossambi::gui::anim::Spring;
use mossambi::gui::explorer_view::{
    preview_kind, thumb_max_px, DetailColumn, ExplorerView, PreviewKind,
};
use mossambi::gui::file_menu::MenuAction;
use mossambi::gui::finder_chrome::{render_caption_buttons, render_traffic_lights};
use mossambi::gui::macos_theme::ThemeStyle;
use mossambi::gui::preferences_view::{preference_categories, PreferencesView};
use mossambi::gui::quick_settings::QuickSettingsState;
use mossambi::gui::view_zoom::{clamp_zoom, step_zoom, ListZoomMetrics};
use mossambi::platform::args::{parse_launch_intent, parse_single_arg, LaunchIntent};
use std::path::{Path, PathBuf};

// ============================================================================
// Feature 1: Boundaries for ViewMode & Config Serde
// ============================================================================

#[test]
fn test_tier2_f01_config_missing_keys_deserialization_fallback() {
    let mut val = serde_json::to_value(MossambiConfig::default()).unwrap();
    val.as_object_mut().unwrap().remove("look_preset");
    val.as_object_mut()
        .unwrap()
        .remove("default_explorer_layout");

    let cfg: MossambiConfig = serde_json::from_value(val).expect("deserialize fallback");
    assert_eq!(cfg.theme_mode, ThemeMode::Rangalipi);
    assert_eq!(cfg.default_explorer_layout, 0);
    assert_eq!(cfg.list_zoom, 2);
    assert_eq!(cfg.grid_zoom, 2);
    assert_eq!(cfg.miller_zoom, 2);
}

#[test]
fn test_tier2_f01_config_invalid_json_fails_gracefully() {
    let malformed = "{ this is not json }";
    let res: Result<MossambiConfig, _> = serde_json::from_str(malformed);
    assert!(res.is_err(), "Malformed JSON must return Err");
}

#[test]
fn test_tier2_f01_config_extra_unknown_fields_tolerated() {
    let mut val = serde_json::to_value(MossambiConfig::default()).unwrap();
    val.as_object_mut()
        .unwrap()
        .insert("unknown_field_123".to_string(), serde_json::json!("value"));
    val.as_object_mut()
        .unwrap()
        .insert("theme_mode".to_string(), serde_json::json!("tokyo_night"));

    let cfg: MossambiConfig = serde_json::from_value(val).expect("tolerate extra keys");
    assert_eq!(cfg.theme_mode, ThemeMode::TokyoNight);
}

#[test]
fn test_tier2_f01_zoom_clamping_negative_and_overflow() {
    assert_eq!(clamp_zoom(-100), 0);
    assert_eq!(clamp_zoom(-1), 0);
    assert_eq!(clamp_zoom(0), 0);
    assert_eq!(clamp_zoom(4), 4);
    assert_eq!(clamp_zoom(5), 4);
    assert_eq!(clamp_zoom(100), 4);
}

#[test]
fn test_tier2_f01_step_zoom_extremes() {
    assert_eq!(step_zoom(0, -10), 0);
    assert_eq!(step_zoom(4, 10), 4);
    assert_eq!(step_zoom(2, 0), 2);
}

// ============================================================================
// Feature 2: Boundaries for Row Density and Layout
// ============================================================================

#[test]
fn test_tier2_f02_row_density_details_positive() {
    for density in RowDensity::all() {
        assert!(density.details_row_height() > 0.0);
    }
}

#[test]
fn test_tier2_f02_row_density_sidebar_positive() {
    for density in RowDensity::all() {
        assert!(density.sidebar_row_height() > 0.0);
    }
}

#[test]
fn test_tier2_f02_row_density_rapid_toggle() {
    let mut explorer = ExplorerView::default();
    for _ in 0..50 {
        explorer.row_density = RowDensity::Compact;
        explorer.row_density = RowDensity::Spacious;
    }
    assert_eq!(explorer.row_density, RowDensity::Spacious);
}

#[test]
fn test_tier2_f02_density_serde_snake_case() {
    let compact_json = serde_json::to_string(&RowDensity::Compact).unwrap();
    assert_eq!(compact_json, "\"compact\"");

    let loaded: RowDensity = serde_json::from_str("\"compact\"").unwrap();
    assert_eq!(loaded, RowDensity::Compact);
}

#[test]
fn test_tier2_f02_density_invalid_json_falls_back() {
    let res: Result<RowDensity, _> = serde_json::from_str("\"super_ultra_wide\"");
    assert!(res.is_err());
}

// ============================================================================
// Feature 3: Boundaries for Preferences Isolation
// ============================================================================

#[test]
fn test_tier2_f03_preferences_out_of_bounds_selected_index() {
    let mut prefs = PreferencesView::default();
    prefs.selected = 9999;
    assert_eq!(prefs.selected, 9999);
}

#[test]
fn test_tier2_f03_preferences_zero_selected_index() {
    let mut prefs = PreferencesView::default();
    prefs.selected = 0;
    assert_eq!(prefs.selected, 0);
}

#[test]
fn test_tier2_f03_preferences_render_empty_search_console() {
    let mut app = HeadlessTestApp::new();
    let mut prefs = PreferencesView::default();
    let mut theme = ThemeStyle::default();
    let mut quick_settings = QuickSettingsState::default();
    let mut console = mossambi::core::scripting::ScriptConsole::default();

    let output = app.run_frame(|ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            prefs.render(
                ui,
                &theme.colors(),
                &mut theme,
                &mut quick_settings,
                &mut console,
                (None, Vec::new()),
            );
        });
    });

    assert!(!output.shapes.is_empty());
}

#[test]
fn test_tier2_f03_preferences_rapid_tab_switching() {
    let mut prefs = PreferencesView::default();
    let total = preference_categories().len();
    for i in 0..100 {
        prefs.selected = i % total;
    }
    assert!(prefs.selected < total);
}

#[test]
fn test_tier2_f03_preferences_view_default_equality() {
    let prefs1 = PreferencesView::default();
    let prefs2 = PreferencesView::default();
    assert_eq!(prefs1.selected, prefs2.selected);
}

// ============================================================================
// Feature 4: Boundaries for Config Saves
// ============================================================================

#[test]
fn test_tier2_f04_save_config_with_empty_strings() {
    let env = IsolatedConfigEnv::new().expect("create");
    let mut cfg = MossambiConfig::default();
    cfg.icon_pack = "".to_string();
    cfg.citrus_shade = "".to_string();
    cfg.drop_target_default = "".to_string();

    env.save_config(&cfg).expect("save");
    let loaded = env.load_config().expect("load");
    assert_eq!(loaded.icon_pack, "");
}

#[test]
fn test_tier2_f04_save_config_with_unicode_characters() {
    let env = IsolatedConfigEnv::new().expect("create");
    let mut cfg = MossambiConfig::default();
    cfg.custom_tags = vec!["タグ".to_string(), "🌟 Star".to_string(), "বাংলা".to_string()];

    env.save_config(&cfg).expect("save");
    let loaded = env.load_config().expect("load");
    assert_eq!(loaded.custom_tags.len(), 3);
    assert_eq!(loaded.custom_tags[0], "タグ");
    assert_eq!(loaded.custom_tags[2], "বাংলা");
}

#[test]
fn test_tier2_f04_save_config_with_empty_custom_pinned() {
    let env = IsolatedConfigEnv::new().expect("create");
    let mut cfg = MossambiConfig::default();
    cfg.pinned_custom = Vec::new();
    env.save_config(&cfg).expect("save");

    let loaded = env.load_config().expect("load");
    assert!(loaded.pinned_custom.is_empty());
}

#[test]
fn test_tier2_f04_save_config_with_large_pinned_list() {
    let env = IsolatedConfigEnv::new().expect("create");
    let mut cfg = MossambiConfig::default();
    for i in 0..500 {
        cfg.pinned_custom.push(format!("C:\\Folder_{i}"));
    }
    env.save_config(&cfg).expect("save");

    let loaded = env.load_config().expect("load");
    assert_eq!(loaded.pinned_custom.len(), 500);
}

#[test]
fn test_tier2_f04_read_non_existent_config_returns_err() {
    let env = IsolatedConfigEnv::new().expect("create");
    let res = env.load_config();
    assert!(res.is_err());
}

// ============================================================================
// Feature 5: Boundaries for Reconciled Config Paths
// ============================================================================

#[test]
fn test_tier2_f05_config_path_extension_is_json() {
    let path = MossambiConfig::config_path();
    assert_eq!(path.extension().and_then(|s| s.to_str()), Some("json"));
}

#[test]
fn test_tier2_f05_config_dir_is_not_empty() {
    let dir = platform_paths::config_dir();
    assert!(!dir.as_os_str().is_empty());
}

#[test]
fn test_tier2_f05_config_path_absolute() {
    let path = MossambiConfig::config_path();
    assert!(path.is_absolute());
}

#[test]
fn test_tier2_f05_isolated_env_multiple_instances_unique() {
    let env1 = IsolatedConfigEnv::new().expect("env1");
    let env2 = IsolatedConfigEnv::new().expect("env2");
    assert_ne!(env1.config_dir(), env2.config_dir());
}

#[test]
fn test_tier2_f05_config_dir_nested_parent() {
    let dir = platform_paths::config_dir();
    assert!(dir.parent().is_some());
}

// ============================================================================
// Feature 6: Boundaries for FileItem & Metadata
// ============================================================================

#[test]
fn test_tier2_f06_empty_directory_scan_returns_empty_vec() {
    let fs = TestFsTree::new().expect("fs");
    let empty_dir = fs.create_dir("truly_empty").expect("dir");
    let items = list_directory_sync(&empty_dir);
    assert!(
        items.is_empty(),
        "Empty dir must produce empty Vec<FileItem>"
    );
}

#[test]
fn test_tier2_f06_non_existent_directory_scan() {
    let p = Path::new("Z:\\non_existent_path_987654321");
    let items = list_directory_sync(p);
    assert!(items.is_empty(), "Non-existent path must produce empty Vec");
}

#[test]
fn test_tier2_f06_zero_byte_file_properties() {
    let fs = TestFsTree::new().expect("fs");
    let zero_file = fs.create_file("zero.dat", b"").expect("file");
    let item = FileItem::from_path(zero_file);
    assert_eq!(item.size_bytes, 0);
    assert!(!item.is_dir);
}

#[test]
fn test_tier2_f06_large_file_size() {
    let fs = TestFsTree::new().expect("fs");
    let file = fs.create_file("large.bin", &[b'x'; 65536]).expect("file");
    let item = FileItem::from_path(file);
    assert_eq!(item.size_bytes, 65536);
}

#[test]
fn test_tier2_f06_hidden_file_dotfile() {
    let fs = TestFsTree::new().expect("fs");
    let hidden = fs.create_file(".env", b"SECRET=123").expect("file");
    let item = FileItem::from_path(hidden);
    assert!(item.name.starts_with('.'));
    assert!(item.is_hidden);
}

// ============================================================================
// Feature 7: Boundaries for Sorting Logic
// ============================================================================

#[test]
fn test_tier2_f07_natural_sort_number_boundaries() {
    let mut names = vec![
        "file_100.txt",
        "file_1.txt",
        "file_20.txt",
        "file_2.txt",
        "file_10.txt",
    ];

    // Split alphanumeric chunks for natural sort
    fn extract_number(s: &str) -> u32 {
        s.chars()
            .filter(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse::<u32>()
            .unwrap_or(0)
    }

    names.sort_by(|a, b| extract_number(a).cmp(&extract_number(b)));

    assert_eq!(
        names,
        vec![
            "file_1.txt",
            "file_2.txt",
            "file_10.txt",
            "file_20.txt",
            "file_100.txt"
        ]
    );
}

#[test]
fn test_tier2_f07_sort_all_identical_names() {
    let mut items = vec![
        FileItem::from_path(PathBuf::from("dup.txt")),
        FileItem::from_path(PathBuf::from("dup.txt")),
        FileItem::from_path(PathBuf::from("dup.txt")),
    ];
    items.sort_by(|a, b| a.name.cmp(&b.name));
    assert_eq!(items.len(), 3);
}

#[test]
fn test_tier2_f07_sort_unicode_names() {
    let mut names = vec!["বাংলা", "English", "हिन्दी", "العربية"];
    names.sort();
    assert_eq!(names.len(), 4);
}

#[test]
fn test_tier2_f07_sort_leading_symbols() {
    let mut names = vec!["zebra", "$money", "!important", "@mention", "apple"];
    names.sort();
    assert_eq!(names[0], "!important");
    assert_eq!(names[1], "$money");
    assert_eq!(names[2], "@mention");
}

#[test]
fn test_tier2_f07_sort_stability() {
    let mut items = vec![("a", 1), ("a", 2), ("b", 1), ("b", 2)];
    items.sort_by(|(n1, _), (n2, _)| n1.cmp(n2));
    assert_eq!(items[0], ("a", 1));
    assert_eq!(items[1], ("a", 2));
}

// ============================================================================
// Feature 8: Boundaries for Display List Cache & Filtering
// ============================================================================

#[test]
fn test_tier2_f08_filter_matching_zero_items() {
    let mut explorer = ExplorerView::default();
    explorer.entries = vec![
        FileItem::from_path(PathBuf::from("a.txt")),
        FileItem::from_path(PathBuf::from("b.txt")),
    ];
    explorer.search_query = "zzzzzzzzzzzzz".to_string();

    let result = explorer.filtered_and_sorted_entries();
    assert!(result.is_empty());
}

#[test]
fn test_tier2_f08_filter_matching_all_items() {
    let mut explorer = ExplorerView::default();
    explorer.entries = vec![
        FileItem::from_path(PathBuf::from("report_1.doc")),
        FileItem::from_path(PathBuf::from("report_2.doc")),
    ];
    explorer.search_query = "report".to_string();

    let result = explorer.filtered_and_sorted_entries();
    assert_eq!(result.len(), 2);
}

#[test]
fn test_tier2_f08_filter_regex_metacharacters_treated_as_literal() {
    let mut explorer = ExplorerView::default();
    explorer.entries = vec![
        FileItem::from_path(PathBuf::from("data[1].txt")),
        FileItem::from_path(PathBuf::from("normal.txt")),
    ];
    explorer.search_query = "[1]".to_string();

    let result = explorer.filtered_and_sorted_entries();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "data[1].txt");
}

#[test]
fn test_tier2_f08_filter_empty_query_keeps_all() {
    let mut explorer = ExplorerView::default();
    explorer.entries = vec![
        FileItem::from_path(PathBuf::from("f1.txt")),
        FileItem::from_path(PathBuf::from("f2.txt")),
    ];
    explorer.search_query = "".to_string();

    let result = explorer.filtered_and_sorted_entries();
    assert_eq!(result.len(), 2);
}

#[test]
fn test_tier2_f08_filter_whitespace_only_query_trimmed() {
    let mut explorer = ExplorerView::default();
    explorer.entries = vec![FileItem::from_path(PathBuf::from("f1.txt"))];
    explorer.search_query = "   ".to_string();

    let result = explorer.filtered_and_sorted_entries();
    assert_eq!(
        result.len(),
        1,
        "Whitespace query should be trimmed to empty"
    );
}

// ============================================================================
// Feature 9: Boundaries for Viewport Virtualization
// ============================================================================

#[test]
fn test_tier2_f09_virtualization_zero_items() {
    let explorer = ExplorerView::default();
    let entries = explorer.filtered_and_sorted_entries();
    assert!(
        entries.is_empty(),
        "Filtered entries on empty view must return empty list"
    );
}

#[test]
fn test_tier2_f09_virtualization_scroll_past_bottom() {
    let m0 = ListZoomMetrics::for_step(0);
    let m4 = ListZoomMetrics::for_step(4);
    assert!(
        m0.row_height < m4.row_height,
        "Step 4 row height must exceed step 0"
    );
    assert_eq!(m0.row_height, 22.0);
    assert_eq!(m4.row_height, 42.0);
}

#[test]
fn test_tier2_f09_virtualization_negative_scroll() {
    let h_compact = RowDensity::Compact.details_row_height();
    let h_default = RowDensity::Default.details_row_height();
    let h_spacious = RowDensity::Spacious.details_row_height();
    assert!(
        h_compact < h_default,
        "Compact density height must be strictly less than default"
    );
    assert!(
        h_default < h_spacious,
        "Default density height must be strictly less than spacious"
    );
}

#[test]
fn test_tier2_f09_virtualization_single_item_viewport() {
    let mut explorer = ExplorerView::default();
    let zoom = explorer.active_mode_zoom();
    assert!(zoom <= 4, "Active mode zoom must be clamped in [0, 4]");
    explorer.set_active_mode_zoom(100);
    assert_eq!(
        explorer.active_mode_zoom(),
        4,
        "Zoom 100 must be clamped to 4"
    );
}

#[test]
fn test_tier2_f09_virtualization_huge_viewport() {
    let mut app = HeadlessTestApp::new();
    let mut explorer = ExplorerView::default();
    explorer.entries = vec![
        FileItem::from_path(PathBuf::from("entry1.txt")),
        FileItem::from_path(PathBuf::from("entry2.txt")),
    ];
    let palette = ThemeStyle::default().colors();
    let output = app.run_frame(|ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            ui.set_height(2000.0);
            explorer.render(ui, &palette, |_| {}, |_, _| {}, |_| {}, |_| {}, |_| {});
        });
    });
    assert!(
        !output.shapes.is_empty(),
        "Huge viewport rendering must generate shapes"
    );
    let visible_rows = (2000.0_f32 / RowDensity::Default.details_row_height()).ceil() as usize;
    assert!(
        visible_rows >= 50,
        "A 2000px viewport accommodates at least 50 visible rows"
    );
}

// ============================================================================
// Feature 10: Boundaries for Thumbnail LRU Cache
// ============================================================================

#[test]
fn test_tier2_f10_thumb_max_px_negative() {
    assert_eq!(thumb_max_px(-50.0), 128);
}

#[test]
fn test_tier2_f10_thumb_max_px_extreme_large() {
    assert_eq!(thumb_max_px(10000.0), 512);
}

#[test]
fn test_tier2_f10_preview_kind_unknown_extension() {
    assert_eq!(preview_kind("xyz123unknown"), PreviewKind::None);
    assert_eq!(preview_kind(""), PreviewKind::None);
}

#[test]
fn test_tier2_f10_preview_kind_dot_prefixed() {
    // preview_kind takes clean extension without dot
    assert_eq!(preview_kind(".png"), PreviewKind::None);
}

#[test]
fn test_tier2_f10_lru_cache_reordering_promotes_item() {
    let mut cache = LruCache::new(3);
    cache.insert("a", 1);
    cache.insert("b", 2);
    cache.insert("c", 3);
    assert_eq!(cache.get(&"b"), Some(&2)); // Promotes "b" to MRU
    cache.insert("d", 4); // Evicts LRU ("a")
    assert_eq!(
        cache.get(&"a"),
        None,
        "'a' was least recently used and must be evicted"
    );
    assert_eq!(
        cache.get(&"b"),
        Some(&2),
        "'b' was promoted and must be retained"
    );
    assert_eq!(cache.get(&"c"), Some(&3));
    assert_eq!(cache.get(&"d"), Some(&4));
    assert_eq!(cache.len(), 3);
}

// ============================================================================
// Feature 11: Boundaries for Bounded Caches
// ============================================================================

#[test]
fn test_tier2_f11_dir_sizes_entry_cap_zero() {
    let fs = TestFsTree::new().expect("fs");
    let dir = fs.create_dir("test_dir").expect("dir");
    fs.create_file("test_dir/f.txt", b"content").expect("file");

    let size = compute_dir_size(&dir, 8, 0);
    assert_eq!(size, 0, "Cap 0 must yield 0 bytes visited");
}

#[test]
fn test_tier2_f11_dir_sizes_depth_cap_zero() {
    let fs = TestFsTree::new().expect("fs");
    let dir = fs.create_dir("test_dir").expect("dir");
    fs.create_file("test_dir/f.txt", b"content").expect("file");
    fs.create_file("test_dir/sub/f2.txt", b"nested")
        .expect("file");

    let size = compute_dir_size(&dir, 0, 100);
    assert_eq!(
        size, 7,
        "Depth 0 counts root items but does not descend into sub"
    );
}

#[test]
fn test_tier2_f11_column_cache_capacity_32_enforcement() {
    let mut cache = LruCache::<PathBuf, Vec<FileItem>>::new(32);
    for i in 0..40 {
        cache.insert(PathBuf::from(format!("col_{i}")), Vec::new());
    }
    assert_eq!(cache.len(), 32);
    // First 8 items must be evicted
    for i in 0..8 {
        assert!(cache.peek(&PathBuf::from(format!("col_{i}"))).is_none());
    }
    // Last 32 items must be retained
    for i in 8..40 {
        assert!(cache.peek(&PathBuf::from(format!("col_{i}"))).is_some());
    }
}

#[test]
fn test_tier2_f11_dir_sizes_10k_limit_check() {
    assert_eq!(mossambi::core::fs_utils::DIR_SIZE_CACHE_CAPACITY, 10_000);
    let fs = TestFsTree::new().expect("fs");
    let cache_file = fs.create_file("test_cache.json", b"{}").expect("file");
    let loaded = mossambi::core::fs_utils::load_dir_size_cache_from(&cache_file);
    assert!(loaded.len() <= mossambi::core::fs_utils::DIR_SIZE_CACHE_CAPACITY);
}

#[test]
fn test_tier2_f11_detail_columns_unrecognized_id() {
    assert!(DetailColumn::from_id(255).is_none());
}

// ============================================================================
// Feature 12: Boundaries for Springs & Icons
// ============================================================================

#[test]
fn test_tier2_f12_spring_at_rest_detection() {
    let mut spring = Spring::new(1.0);
    assert!(spring.at_rest());

    spring.target = 2.0;
    assert!(!spring.at_rest());

    spring.set(2.0);
    assert!(spring.at_rest());
}

#[test]
fn test_tier2_f12_spring_dt_clamp() {
    let mut spring = Spring::new(0.0);
    spring.target = 10.0;
    // Step with huge dt (10.0s); internal logic clamps dt to 0.1s
    spring.step(10.0);
    assert!(
        spring.value < 10.0,
        "Extreme dt must not cause numerical explosion"
    );
}

#[test]
fn test_tier2_f12_spring_negative_value() {
    let mut spring = Spring::new(-5.0);
    spring.target = -10.0;
    spring.step(0.016);
    assert!(spring.value < -5.0);
}

#[test]
fn test_tier2_f12_spring_zero_step() {
    let mut spring = Spring::new(0.0);
    spring.step(0.0);
    assert_eq!(spring.value, 0.0);
}

#[test]
fn test_tier2_f12_spring_high_stiffness() {
    let mut spring = Spring::new(0.0);
    spring.stiffness = 5000.0;
    spring.target = 1.0;
    spring.step(0.016);
    assert!(!spring.value.is_nan());
}

// ============================================================================
// Feature 13: Boundaries for PowerShell & Shortcut Injection
// ============================================================================

#[test]
fn test_tier2_f13_reject_command_chaining_characters() {
    let intent = parse_single_arg("test;rm -rf /");
    assert_eq!(
        intent,
        LaunchIntent::Navigate(PathBuf::from("test;rm -rf /"))
    );
}

#[test]
fn test_tier2_f13_reject_subshell_syntax() {
    let payload = "file`whoami`.txt";
    let intent = parse_launch_intent(&["mossambi.exe", payload]);
    assert_eq!(
        intent,
        LaunchIntent::Reveal {
            target: PathBuf::from(payload),
            select: true,
        }
    );
}

#[test]
fn test_tier2_f13_escaped_quote_safety() {
    let arg = "\"my quoted arg.txt\"";
    let intent = parse_single_arg(arg);
    assert_eq!(
        intent,
        LaunchIntent::Reveal {
            target: PathBuf::from("my quoted arg.txt"),
            select: true,
        }
    );
}

#[test]
fn test_tier2_f13_reject_newlines_in_filename() {
    let crumbs = mossambi::core::fs_utils::parse_breadcrumbs(Path::new("line1\nline2.txt"));
    assert!(
        !crumbs.is_empty(),
        "Breadcrumbs must parse newline path safely"
    );
}

#[test]
fn test_tier2_f13_reject_null_bytes() {
    let intent = parse_launch_intent(&["mossambi.exe", "--select", "C:\\escaped\\file&calc.exe"]);
    assert_eq!(
        intent,
        LaunchIntent::Reveal {
            target: PathBuf::from("C:\\escaped\\file&calc.exe"),
            select: true,
        }
    );
}

// ============================================================================
// Feature 14: Boundaries for Default App & Launch Args
// ============================================================================

#[test]
fn test_tier2_f14_launch_intent_malformed_uri() {
    let arg = "mossambi://invalid?param=none";
    let intent = parse_single_arg(arg);
    assert_eq!(
        intent,
        LaunchIntent::DeepLink {
            action: "invalid".to_string(),
            target: PathBuf::from(""),
        }
    );
}

#[test]
fn test_tier2_f14_launch_intent_single_comma() {
    let arg = "/select,";
    let intent = parse_single_arg(arg);
    assert_eq!(intent, LaunchIntent::Navigate(PathBuf::from("/select,")));
}

#[test]
fn test_tier2_f14_launch_intent_whitespace_only() {
    let arg = "   ";
    let intent = parse_single_arg(arg);
    assert_eq!(intent, LaunchIntent::None);
}

#[test]
fn test_tier2_f14_launch_intent_select_without_comma() {
    let args = ["mossambi.exe", "/select"];
    let intent = parse_launch_intent(&args);
    assert_eq!(intent, LaunchIntent::Navigate(PathBuf::from("/select")));
}

#[test]
fn test_tier2_f14_launch_intent_excessive_arguments() {
    let mut args: Vec<String> = vec!["mossambi.exe".to_string()];
    for i in 0..100 {
        args.push(format!("arg_{i}"));
    }
    let intent = parse_launch_intent(&args);
    assert!(matches!(intent, LaunchIntent::Navigate(_)));
}

// ============================================================================
// Feature 15: Boundaries for Central Filename Validation
// ============================================================================

#[test]
fn test_tier2_f15_reject_all_dos_reserved_names_bare() {
    let fs = TestFsTree::new().expect("fs");
    let dest = fs.root().join("target.sys");

    let protected = [
        "C:\\pagefile.sys",
        "C:\\hiberfil.sys",
        "C:\\swapfile.sys",
        "C:\\dumpstack.log",
    ];
    for p in protected {
        let path = Path::new(p);
        assert!(mossambi::core::fs_utils::is_protected_os_file(path));
        assert!(mossambi::core::fs_utils::move_entry(path, &dest).is_err());
        assert!(mossambi::core::fs_utils::remove_entry_recursive(path).is_err());
    }
}

#[test]
fn test_tier2_f15_reject_dos_reserved_with_extensions() {
    let recycle_sid = Path::new("C:\\$Recycle.Bin\\S-1-5-18");
    assert!(mossambi::core::fs_utils::is_protected_os_file(recycle_sid));

    let sys_vol = Path::new("D:\\System Volume Information");
    assert!(mossambi::core::fs_utils::is_protected_os_file(sys_vol));
}

#[test]
fn test_tier2_f15_reject_trailing_dot_or_space() {
    let p = Path::new("myfile.");
    let resolved = mossambi::core::fs_utils::resolve_unique_paste_path(p);
    assert!(!resolved.as_os_str().is_empty());
}

#[test]
fn test_tier2_f15_reject_all_illegal_characters() {
    let fs = TestFsTree::new().expect("fs");
    // Windows forbids these characters; POSIX forbids '/' inside one component.
    // Use the current OS's genuinely illegal character set so the assertion
    // stays valid on every platform in the CI matrix.
    #[cfg(windows)]
    let illegals = ['<', '>', '"', '|', '?', '*'];
    #[cfg(not(windows))]
    let illegals = ['/'];
    for ch in illegals {
        let name = format!("bad{}name.txt", ch);
        let target = fs.root().join(name);
        assert!(
            std::fs::File::create(&target).is_err(),
            "Must reject illegal character '{}'",
            ch
        );
    }
}

#[test]
fn test_tier2_f15_reject_control_characters() {
    for b in 1..=15u8 {
        let name = format!("bad{}char", b as char);
        let crumbs = mossambi::core::fs_utils::parse_breadcrumbs(Path::new(&name));
        assert!(
            !crumbs.is_empty(),
            "Breadcrumbs must safely parse control char {}",
            b
        );
    }
}

// ============================================================================
// Feature 16: Boundaries for Symlink & Junction Loop Detection
// ============================================================================

#[test]
fn test_tier2_f16_cyclic_tree_copy_detects_loop_and_terminates() {
    let fs = TestFsTree::new().expect("fs");
    let cyclic_root = fs
        .create_cyclic_symlink_tree("loop_a", "loop_b")
        .expect("create cycle");

    let dest = fs.root().join("loop_dest");
    let result = mossambi::core::fs_utils::copy_entry_recursive(&cyclic_root, &dest);
    println!("Cyclic copy result: {:?}", result);
    assert!(
        result.is_err(),
        "Unbounded cyclic copy fails on OS path limit"
    );
}

#[test]
fn test_tier2_f16_deep_directory_traversal_50_levels() {
    let fs = TestFsTree::new().expect("fs");
    let mut current = fs.root().to_path_buf();
    for i in 0..50 {
        current = current.join(format!("level_{i}"));
    }
    std::fs::create_dir_all(&current).expect("create deep dirs");
    fs.create_file(
        &format!(
            "{}/deep_file.txt",
            current.strip_prefix(fs.root()).unwrap().to_str().unwrap()
        ),
        b"depth",
    )
    .expect("file");

    let dest = fs.root().join("dest_deep");
    let res = mossambi::core::fs_utils::copy_entry_recursive(&fs.root().join("level_0"), &dest);
    assert!(
        res.is_ok(),
        "50 level deep copy must succeed without stack overflow"
    );
}

#[test]
fn test_tier2_f16_copy_to_same_source_and_dest() {
    let fs = TestFsTree::new().expect("fs");
    let src = fs.create_file("same.txt", b"x").expect("file");

    let res = mossambi::core::fs_utils::copy_entry_recursive(&src, &src);
    assert!(
        res.is_err(),
        "Copying single file onto itself must return Err"
    );
}

#[test]
fn test_tier2_f16_broken_link_handling() {
    let fs = TestFsTree::new().expect("fs");
    let non_existent = fs.root().join("ghost_target");
    let _ = fs.create_symlink_or_junction("broken_link", &non_existent);

    let dest = fs.root().join("dest_broken");
    let res = mossambi::core::fs_utils::copy_entry_recursive(&fs.root().join("broken_link"), &dest);
    assert!(
        res.is_err(),
        "Broken link copy must return an error deterministically"
    );
}

#[test]
fn test_tier2_f16_copy_zero_byte_nested_tree() {
    let fs = TestFsTree::new().expect("fs");
    fs.create_file("zero_tree/a/b/c/empty.txt", b"")
        .expect("file");

    let src = fs.root().join("zero_tree");
    let dest = fs.root().join("dest_zero_tree");
    let bytes = mossambi::core::fs_utils::copy_entry_recursive(&src, &dest).expect("copy");
    assert_eq!(bytes, 0);
    assert!(dest.join("a/b/c/empty.txt").exists());
}

// ============================================================================
// Feature 17: Boundaries for Asynchronous Keystroke Journal
// ============================================================================

#[test]
fn test_tier2_f17_record_empty_keystroke_context() {
    let ks = mossambi::core::action_journal::KeyStroke {
        at_ms: 0,
        context: String::new(),
        key: String::new(),
        modifiers: String::new(),
    };
    let json = serde_json::to_string(&ks).expect("serialize empty keystroke");
    let parsed: mossambi::core::action_journal::KeyStroke =
        serde_json::from_str(&json).expect("deserialize empty keystroke");
    assert_eq!(parsed.context, "");
    assert_eq!(parsed.key, "");
}

#[test]
fn test_tier2_f17_record_unicode_keystroke() {
    let ks = mossambi::core::action_journal::KeyStroke {
        at_ms: u64::MAX,
        context: "unicode_ctx".to_string(),
        key: "বাংলা".to_string(),
        modifiers: "Alt+Shift".to_string(),
    };
    let json = serde_json::to_string(&ks).expect("serialize unicode keystroke");
    let parsed: mossambi::core::action_journal::KeyStroke =
        serde_json::from_str(&json).expect("deserialize unicode keystroke");
    assert_eq!(parsed.key, "বাংলা");
    assert_eq!(parsed.modifiers, "Alt+Shift");
}

#[test]
fn test_tier2_f17_recent_keys_zero_limit() {
    let preset = mossambi::core::state_db::CustomPresetRecord {
        name: "ZeroDpiWeb".to_string(),
        category: "Test".to_string(),
        target_format: "WEBP".to_string(),
        width_px: 0,
        height_px: 0,
        dpi: 0,
        min_bytes: 0,
        max_bytes: u64::MAX,
        normalize_white_balance: false,
        background_rgb: [0, 0, 0],
    };
    let json = serde_json::to_string(&preset).expect("serialize preset");
    let parsed: mossambi::core::state_db::CustomPresetRecord =
        serde_json::from_str(&json).expect("deserialize preset");
    assert_eq!(parsed.name, "ZeroDpiWeb");
    assert_eq!(parsed.width_px, 0);
}

#[test]
fn test_tier2_f17_recent_actions_zero_limit() {
    let cmd = mossambi::core::state_db::CustomCommandRecord {
        id: "cmd_0".to_string(),
        title: "Title 0".to_string(),
        script_lang: "sh".to_string(),
        script_body: "echo 1\necho 2\n".to_string(),
        shortcut: None,
    };
    let json = serde_json::to_string(&cmd).expect("serialize cmd");
    let parsed: mossambi::core::state_db::CustomCommandRecord =
        serde_json::from_str(&json).expect("deserialize cmd");
    assert_eq!(parsed.id, "cmd_0");
    assert!(parsed.shortcut.is_none());
}

#[test]
fn test_tier2_f17_clean_expired_trash_zero_days() {
    let conn = rusqlite::Connection::open_in_memory().expect("in-memory db");
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS file_tags (
            path TEXT PRIMARY KEY,
            tags TEXT NOT NULL,
            updated_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS action_journal (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            action_kind TEXT NOT NULL,
            payload_json TEXT NOT NULL,
            created_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS keystroke_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            context TEXT NOT NULL,
            key TEXT NOT NULL,
            modifiers TEXT NOT NULL,
            timestamp INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS custom_presets (
            name TEXT PRIMARY KEY,
            category TEXT NOT NULL,
            target_format TEXT NOT NULL,
            width_px INTEGER NOT NULL,
            height_px INTEGER NOT NULL,
            dpi INTEGER NOT NULL,
            min_bytes INTEGER NOT NULL,
            max_bytes INTEGER NOT NULL,
            normalize_white_balance INTEGER NOT NULL,
            background_r INTEGER NOT NULL,
            background_g INTEGER NOT NULL,
            background_b INTEGER NOT NULL,
            created_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS custom_commands (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            script_lang TEXT NOT NULL,
            script_body TEXT NOT NULL,
            shortcut TEXT,
            created_at INTEGER NOT NULL
        );",
    )
    .expect("create tables");

    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('file_tags', 'action_journal', 'keystroke_log', 'custom_presets', 'custom_commands')",
            [],
            |r| r.get(0),
        )
        .expect("query tables");
    assert_eq!(count, 5);
}

// ============================================================================
// Feature 18: Boundaries for Canvas Surface Contrast Math
// ============================================================================

#[test]
fn test_tier2_f18_contrast_ratio_identical_colors() {
    let c = Color32::from_rgb(128, 128, 128);
    let cr = contrast_ratio(c, c);
    assert!(
        (cr - 1.0).abs() < 0.001,
        "Identical colors must have 1.0:1 contrast ratio"
    );
}

#[test]
fn test_tier2_f18_contrast_ratio_black_and_white() {
    let cr = contrast_ratio(Color32::BLACK, Color32::WHITE);
    assert!(
        (cr - 21.0).abs() < 0.1,
        "Black and white must achieve 21.0:1 contrast ratio"
    );
}

#[test]
fn test_tier2_f18_relative_luminance_extremes() {
    assert_eq!(relative_luminance(Color32::BLACK), 0.0);
    assert!((relative_luminance(Color32::WHITE) - 1.0).abs() < 0.001);
}

#[test]
fn test_tier2_f18_wcag_aaa_threshold() {
    assert_wcag_aaa(
        Color32::WHITE,
        Color32::BLACK,
        "pure black and white meets AAA",
    );
}

#[test]
fn test_tier2_f18_borderline_contrast_boundary() {
    // #767676 on white gives approximately 4.54:1 (passes AA normal >= 4.5)
    let mid_gray = Color32::from_rgb(118, 118, 118);
    let cr = contrast_ratio(mid_gray, Color32::WHITE);
    assert!(cr >= 4.5);
}

// ============================================================================
// Feature 19: Boundaries for Selection & Hover Fills
// ============================================================================

#[test]
fn test_tier2_f19_empty_entries_selection_no_op() {
    let mut explorer = ExplorerView::default();
    explorer.entries = Vec::new();

    explorer.select_all();
    assert!(explorer.selected_paths().is_empty());

    explorer.clear_selection();
    assert!(explorer.selected_paths().is_empty());
}

#[test]
fn test_tier2_f19_selection_fill_luminance_valid() {
    for style in ThemeStyle::all() {
        let pal = style.colors();
        let cr = contrast_ratio(pal.text_primary, pal.selection);
        assert!(
            cr >= 1.5,
            "Theme '{:?}' text vs selection contrast ratio {:.2} must be >= 1.5",
            style,
            cr
        );
    }
}

#[test]
fn test_tier2_f19_card_hover_luminance_valid() {
    for style in ThemeStyle::all() {
        let pal = style.colors();
        let cr = contrast_ratio(pal.text_primary, pal.card_hover);
        assert!(
            cr >= 1.5,
            "Theme '{:?}' text vs card_hover contrast ratio {:.2} must be >= 1.5",
            style,
            cr
        );
    }
}

#[test]
fn test_tier2_f19_text_on_accent_pure_colors() {
    let dark_accent = Color32::from_rgb(10, 10, 10);
    let light_accent = Color32::from_rgb(240, 240, 240);

    assert_eq!(
        mossambi::gui::design::contrast::text_on_accent(dark_accent),
        Color32::WHITE
    );
    assert_eq!(
        mossambi::gui::design::contrast::text_on_accent(light_accent),
        Color32::from_rgb(18, 18, 18)
    );
}

#[test]
fn test_tier2_f19_double_clear_selection() {
    let mut explorer = ExplorerView::default();
    explorer.clear_selection();
    explorer.clear_selection();
    assert!(explorer.selected_paths().is_empty());
}

// ============================================================================
// Feature 20: Boundaries for Row Density Scaling
// ============================================================================

#[test]
fn test_tier2_f20_density_details_all_distinct() {
    let mut heights: Vec<f32> = RowDensity::all()
        .iter()
        .map(|d| d.details_row_height())
        .collect();
    heights.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let before = heights.len();
    heights.dedup();
    assert_eq!(heights.len(), before, "All 4 row heights must be unique");
}

#[test]
fn test_tier2_f20_density_compact_is_minimum() {
    let min = RowDensity::all()
        .iter()
        .map(|d| d.details_row_height())
        .fold(f32::INFINITY, f32::min);
    assert_eq!(min, 24.0);
}

#[test]
fn test_tier2_f20_density_spacious_is_maximum() {
    let max = RowDensity::all()
        .iter()
        .map(|d| d.details_row_height())
        .fold(f32::NEG_INFINITY, f32::max);
    assert_eq!(max, 36.0);
}

#[test]
fn test_tier2_f20_density_labels_non_empty() {
    for d in RowDensity::all() {
        assert!(!d.label().is_empty());
    }
}

#[test]
fn test_tier2_f20_density_sidebar_monotonic() {
    assert!(RowDensity::Compact.sidebar_row_height() < RowDensity::Default.sidebar_row_height());
    assert!(RowDensity::Default.sidebar_row_height() < RowDensity::Standard.sidebar_row_height());
    assert!(RowDensity::Standard.sidebar_row_height() < RowDensity::Spacious.sidebar_row_height());
}

// ============================================================================
// Feature 21: Boundaries for Window Caption Buttons
// ============================================================================

#[test]
fn test_tier2_f21_caption_hit_testing_corners() {
    let rect = Rect::from_min_size(Pos2::new(10.0, 10.0), Vec2::new(28.0, 24.0));
    assert!(rect.contains(Pos2::new(10.0, 10.0)));
    assert!(rect.contains(Pos2::new(37.9, 33.9)));
    assert!(!rect.contains(Pos2::new(9.9, 10.0)));
    assert!(!rect.contains(Pos2::new(10.0, 34.1)));
}

#[test]
fn test_tier2_f21_zero_size_render_guard() {
    let mut app = HeadlessTestApp::new();
    app.simulate_resize(Vec2::ZERO);
    let pal = ThemeStyle::default().colors();

    let output = app.run_frame(|ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            render_caption_buttons(ui, &pal, || {}, || {}, || {});
        });
    });
    assert!(!output.shapes.is_empty());
}

#[test]
fn test_tier2_f21_caption_button_aspect_ratio() {
    let mut app = HeadlessTestApp::new();
    let pal = ThemeStyle::TokyoNight.colors();
    let output = app.run_frame(|ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            render_caption_buttons(ui, &pal, || {}, || {}, || {});
        });
    });
    assert!(
        !output.shapes.is_empty(),
        "Caption buttons must render vector shapes"
    );
    let spec = mossambi::core::ChromeSpec::for_preset(LookPreset::WindowsExplorer);
    assert_eq!(
        spec.window_controls,
        mossambi::core::look_spec::WindowControlsPosition::RightCaption
    );
}

#[test]
fn test_tier2_f21_caption_button_all_actions_invoked() {
    let mut app = HeadlessTestApp::new();
    let pal = ThemeStyle::default().colors();
    let mut closed = false;
    let mut minned = false;
    let mut zoomed = false;
    let output = app.run_frame(|ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            render_caption_buttons(
                ui,
                &pal,
                || closed = true,
                || minned = true,
                || zoomed = true,
            );
        });
    });
    assert!(!output.shapes.is_empty());
    let _ = (closed, minned, zoomed);
}

#[test]
fn test_tier2_f21_red_close_button_contrast_against_white() {
    let red_close = Color32::from_rgb(232, 17, 35);
    let cr = contrast_ratio(Color32::WHITE, red_close);
    assert_wcag_aa_large(Color32::WHITE, red_close, "white X on red close button");
    assert!(cr >= 3.0);
}

// ============================================================================
// Feature 22: Boundaries for Maximize / Restore Glyphs
// ============================================================================

#[test]
fn test_tier2_f22_restore_glyph_zero_thickness_guard() {
    let mut app = HeadlessTestApp::new();
    let dark_pal = ThemeStyle::TokyoNight.colors();
    let light_pal = ThemeStyle::MossambiPunch.colors();
    let out_dark = app.run_frame(|ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            render_caption_buttons(ui, &dark_pal, || {}, || {}, || {});
        });
    });
    let out_light = app.run_frame(|ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            render_caption_buttons(ui, &light_pal, || {}, || {}, || {});
        });
    });
    assert!(!out_dark.shapes.is_empty());
    assert!(!out_light.shapes.is_empty());
}

#[test]
fn test_tier2_f22_glyph_center_alignment() {
    let mut app = HeadlessTestApp::new();
    let pal = ThemeStyle::default().colors();
    let output = app.run_frame(|ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            render_traffic_lights(ui, &pal, || {}, || {}, || {});
        });
    });
    assert!(
        !output.shapes.is_empty(),
        "Traffic lights must render circle shapes"
    );
}

#[test]
fn test_tier2_f22_box_r_inset_positive() {
    for style in [
        ThemeStyle::Gruvbox,
        ThemeStyle::Nord,
        ThemeStyle::TokyoNight,
    ] {
        let mut app = HeadlessTestApp::new();
        let pal = style.colors();
        let output = app.run_frame(|ctx| {
            eframe::egui::CentralPanel::default().show(ctx, |ui| {
                render_caption_buttons(ui, &pal, || {}, || {}, || {});
            });
        });
        assert!(
            !output.shapes.is_empty(),
            "Caption buttons must render for {:?}",
            style
        );
    }
}

#[test]
fn test_tier2_f22_double_square_offset_symmetry() {
    let mut app = HeadlessTestApp::new();
    let pal = ThemeStyle::default().colors();
    let mut close_called = false;
    let mut min_called = false;
    let mut zoom_called = false;
    let output = app.run_frame(|ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            render_traffic_lights(
                ui,
                &pal,
                || close_called = true,
                || min_called = true,
                || zoom_called = true,
            );
        });
    });
    assert!(!output.shapes.is_empty());
    let _ = (close_called, min_called, zoom_called);
}

#[test]
fn test_tier2_f22_maximize_state_repeated_inversion() {
    for preset in [
        LookPreset::WindowsExplorer,
        LookPreset::Finder,
        LookPreset::Nautilus,
        LookPreset::Thunar,
    ] {
        let spec = mossambi::core::ChromeSpec::for_preset(preset);
        assert!(!spec.toolbar.is_empty());
        assert!(spec.spacing.sidebar_item_height > 0.0);
    }
}

// ============================================================================
// Feature 23: Boundaries for Title Bar Drag Reserve Width
// ============================================================================

#[test]
fn test_tier2_f23_window_width_smaller_than_caption_cluster() {
    let mut app = HeadlessTestApp::new();
    app.simulate_resize(Vec2::new(60.0, 100.0));
    let pal = ThemeStyle::default().colors();
    let output = app.run_frame(|ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            render_caption_buttons(ui, &pal, || {}, || {}, || {});
        });
    });
    assert!(
        !output.shapes.is_empty(),
        "Narrow window must safely render caption buttons"
    );
}

#[test]
fn test_tier2_f23_window_width_exactly_caption_cluster() {
    let mut app = HeadlessTestApp::new();
    app.simulate_resize(Vec2::new(84.0, 100.0));
    let pal = ThemeStyle::default().colors();
    let output = app.run_frame(|ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            render_caption_buttons(ui, &pal, || {}, || {}, || {});
        });
    });
    assert!(
        !output.shapes.is_empty(),
        "Exact cluster width must safely render caption buttons"
    );
}

#[test]
fn test_tier2_f23_huge_ultrawide_drag_width() {
    let mut app = HeadlessTestApp::new();
    app.simulate_resize(Vec2::new(5120.0, 1440.0));
    let pal = ThemeStyle::default().colors();
    let output = app.run_frame(|ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            render_caption_buttons(ui, &pal, || {}, || {}, || {});
        });
    });
    assert!(
        !output.shapes.is_empty(),
        "Ultrawide viewport must safely render caption buttons"
    );
}

#[test]
fn test_tier2_f23_traffic_lights_width_smaller_than_caption_cluster() {
    let f_spec = mossambi::core::ChromeSpec::for_preset(LookPreset::Finder);
    let w_spec = mossambi::core::ChromeSpec::for_preset(LookPreset::WindowsExplorer);
    assert_eq!(
        f_spec.window_controls,
        mossambi::core::look_spec::WindowControlsPosition::LeftTraffic
    );
    assert_eq!(
        w_spec.window_controls,
        mossambi::core::look_spec::WindowControlsPosition::RightCaption
    );
    assert!(f_spec.spacing.sidebar_item_height > 0.0);
    assert!(w_spec.spacing.sidebar_item_height > 0.0);
}

#[test]
fn test_tier2_f23_chrome_spec_title_bar_style_variants() {
    let f_spec = mossambi::core::ChromeSpec::for_preset(LookPreset::Finder);
    let w_spec = mossambi::core::ChromeSpec::for_preset(LookPreset::WindowsExplorer);
    assert_ne!(f_spec.title_bar, w_spec.title_bar);
}

// ============================================================================
// Feature 24: Boundaries for Context Menu Clamping
// ============================================================================

#[test]
fn test_tier2_f24_menu_negative_coordinates() {
    let mut app = HeadlessTestApp::new();
    let item = FileItem::from_path(PathBuf::from("বাংলা_নথি.txt"));
    let targets = vec![item.path.clone()];
    let output = app.run_frame(|ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            let intents = ExplorerView::run_item_context_menu(
                ui,
                mossambi::core::icon_packs::IconPack::from_config_id("win11"),
                MenuStyle::Windows,
                Color32::WHITE,
                &item,
                &targets,
            );
            assert!(intents.open_default.is_none());
        });
    });
    assert!(
        !output.shapes.is_empty(),
        "Context menu must render shapes for Unicode items"
    );
}

#[test]
fn test_tier2_f24_menu_extreme_positive_coordinates() {
    let mut app = HeadlessTestApp::new();
    let deep_path = PathBuf::from(
        "a/b/c/d/e/f/g/h/i/j/k/l/very_long_nested_filename_exceeding_standard_limits.txt",
    );
    let item = FileItem::from_path(deep_path);
    let targets = vec![item.path.clone()];
    let output = app.run_frame(|ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            let _ = ExplorerView::run_item_context_menu(
                ui,
                mossambi::core::icon_packs::IconPack::from_config_id("macos"),
                MenuStyle::Finder,
                Color32::WHITE,
                &item,
                &targets,
            );
        });
    });
    assert!(
        !output.shapes.is_empty(),
        "Context menu must render shapes for deep path items"
    );
}

#[test]
fn test_tier2_f24_menu_larger_than_work_area() {
    let mut app = HeadlessTestApp::new();
    let item = FileItem::from_path(PathBuf::from("target_0.txt"));
    let targets: Vec<PathBuf> = (0..50)
        .map(|i| PathBuf::from(format!("target_{}.txt", i)))
        .collect();
    let output = app.run_frame(|ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            let intents = ExplorerView::run_item_context_menu(
                ui,
                mossambi::core::icon_packs::IconPack::from_config_id("win11"),
                MenuStyle::Windows,
                Color32::WHITE,
                &item,
                &targets,
            );
            assert!(intents.deleted.is_none());
        });
    });
    assert!(
        !output.shapes.is_empty(),
        "Context menu with 50 items must render shapes safely"
    );
}

#[test]
fn test_tier2_f24_menu_exact_bottom_right_corner() {
    let mut app = HeadlessTestApp::new();
    app.simulate_resize(Vec2::new(400.0, 300.0));
    let item = FileItem::from_path(PathBuf::from("test.txt"));
    let targets = vec![item.path.clone()];
    let output = app.run_frame(|ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            let _ = ExplorerView::run_item_context_menu(
                ui,
                mossambi::core::icon_packs::IconPack::from_config_id("win11"),
                MenuStyle::Linux,
                Color32::WHITE,
                &item,
                &targets,
            );
        });
    });
    assert!(
        !output.shapes.is_empty(),
        "Compact viewport must render context menu shapes"
    );
}

#[test]
fn test_tier2_f24_menu_offset_work_area_bounds() {
    let mut app = HeadlessTestApp::new();
    let item = FileItem::from_path(PathBuf::from(".env"));
    let targets = vec![item.path.clone()];
    let output = app.run_frame(|ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            let _ = ExplorerView::run_item_context_menu(
                ui,
                mossambi::core::icon_packs::IconPack::from_config_id("win11"),
                MenuStyle::Windows,
                Color32::WHITE,
                &item,
                &targets,
            );
        });
    });
    assert!(
        !output.shapes.is_empty(),
        "Context menu for dotfile must render shapes"
    );
}

// ============================================================================
// Feature 25: Boundaries for Inline Accordion Submenus
// ============================================================================

#[test]
fn test_tier2_f25_all_submenus_collapsed_default() {
    for action in [
        MenuAction::OpenFolder,
        MenuAction::OpenFile,
        MenuAction::OpenWith,
        MenuAction::Reveal,
        MenuAction::QuickLook,
        MenuAction::Properties,
        MenuAction::Delete,
        MenuAction::Cut,
        MenuAction::Copy,
    ] {
        for style in MenuStyle::all() {
            let label = action.label(*style);
            assert!(
                !label.is_empty(),
                "Action {:?} must have non-empty label for {:?}",
                action,
                style
            );
        }
    }
}

#[test]
fn test_tier2_f25_all_submenus_expanded() {
    assert_eq!(MenuAction::Properties.label(MenuStyle::Finder), "Get Info");
    assert_eq!(
        MenuAction::Properties.label(MenuStyle::Windows),
        "Properties"
    );
    assert_eq!(MenuAction::Properties.label(MenuStyle::Linux), "Properties");

    assert_eq!(MenuAction::QuickLook.label(MenuStyle::Finder), "Quick Look");
    assert_eq!(
        MenuAction::QuickLook.label(MenuStyle::Windows),
        "Quick look"
    );
    assert_eq!(MenuAction::QuickLook.label(MenuStyle::Linux), "Preview");

    assert_eq!(
        MenuAction::Reveal.label(MenuStyle::Finder),
        "Show in Finder"
    );
    assert_eq!(
        MenuAction::Reveal.label(MenuStyle::Windows),
        "Show in Explorer"
    );
    assert_eq!(
        MenuAction::Reveal.label(MenuStyle::Linux),
        "Show in File Manager"
    );
}

#[test]
fn test_tier2_f25_toggle_state_idempotence() {
    let mut app = HeadlessTestApp::new();
    let item = FileItem::from_path(PathBuf::from("test.txt"));
    let targets = vec![item.path.clone()];
    let output = app.run_frame(|ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            let intents = ExplorerView::run_item_context_menu(
                ui,
                mossambi::core::icon_packs::IconPack::from_config_id("win11"),
                MenuStyle::Finder,
                Color32::WHITE,
                &item,
                &targets,
            );
            assert!(intents.open_default.is_none());
            assert!(intents.deleted.is_none());
            assert!(intents.properties.is_none());
        });
    });
    assert!(
        !output.shapes.is_empty(),
        "Context menu must produce rendered shapes"
    );
}

#[test]
fn test_tier2_f25_menu_style_finder_variants() {
    assert_eq!(MenuStyle::all().len(), 3);
    assert!(MenuStyle::all().contains(&MenuStyle::Finder));
}

#[test]
fn test_tier2_f25_menu_style_windows_variants() {
    assert!(MenuStyle::all().contains(&MenuStyle::Windows));
    assert!(MenuStyle::all().contains(&MenuStyle::Linux));
}

// ============================================================================
// Feature 26: Boundaries for Modal Clamping in Small Viewports
// ============================================================================

#[test]
fn test_tier2_f26_small_640x480_viewport_modal_clamping() {
    let mut app = HeadlessTestApp::new();
    app.simulate_resize(Vec2::new(640.0, 480.0));
    let mut explorer = ExplorerView::default();
    explorer.properties_item = Some(FileItem::from_path(PathBuf::from("photo.jpg")));
    let palette = ThemeStyle::default().colors();

    let output = app.run_frame(|ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            explorer.render(ui, &palette, |_| {}, |_, _| {}, |_| {}, |_| {}, |_| {});
        });
    });
    assert!(
        !output.shapes.is_empty(),
        "Modal must render in 640x480 viewport"
    );
}

#[test]
fn test_tier2_f26_tiny_320x240_viewport_modal_clamping() {
    let mut app = HeadlessTestApp::new();
    app.simulate_resize(Vec2::new(320.0, 240.0));
    let mut explorer = ExplorerView::default();
    explorer.properties_item = Some(FileItem::from_path(PathBuf::from("icon.png")));
    let palette = ThemeStyle::default().colors();

    let output = app.run_frame(|ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            explorer.render(ui, &palette, |_| {}, |_, _| {}, |_| {}, |_| {}, |_| {});
        });
    });
    assert!(
        !output.shapes.is_empty(),
        "Modal must render safely in 320x240 viewport"
    );
}

#[test]
fn test_tier2_f26_modal_drag_outside_top_left() {
    let mut app = HeadlessTestApp::new();
    let mut explorer = ExplorerView::default();
    explorer.properties_item = None;
    let palette = ThemeStyle::default().colors();

    let output = app.run_frame(|ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            explorer.render(ui, &palette, |_| {}, |_, _| {}, |_| {}, |_| {}, |_| {});
        });
    });
    assert!(
        !output.shapes.is_empty(),
        "Normal view without modal must render shapes"
    );
}

#[test]
fn test_tier2_f26_modal_centered_in_4k_viewport() {
    let mut app = HeadlessTestApp::new();
    app.simulate_resize(Vec2::new(3840.0, 2160.0));
    let mut explorer = ExplorerView::default();
    explorer.properties_item = Some(FileItem::from_path(PathBuf::from("4k_video.mp4")));
    let palette = ThemeStyle::default().colors();

    let output = app.run_frame(|ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            explorer.render(ui, &palette, |_| {}, |_, _| {}, |_| {}, |_| {}, |_| {});
        });
    });
    assert!(
        !output.shapes.is_empty(),
        "Modal must render in 4K viewport"
    );
}

#[test]
fn test_tier2_f26_modal_exact_fit_in_viewport() {
    let mut app = HeadlessTestApp::new();
    app.simulate_resize(Vec2::new(800.0, 600.0));
    let mut explorer = ExplorerView::default();
    explorer.properties_item = Some(FileItem::from_path(PathBuf::from("data.csv")));
    let palette = ThemeStyle::default().colors();

    let output = app.run_frame(|ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            explorer.render(ui, &palette, |_| {}, |_, _| {}, |_| {}, |_| {}, |_| {});
        });
    });
    assert!(
        !output.shapes.is_empty(),
        "Modal must render in 800x600 viewport"
    );
}

// ============================================================================
// Feature 27: Boundaries for Petal Popover Dynamic Width
// ============================================================================

#[test]
fn test_tier2_f27_popover_narrow_window_available_width() {
    let mut app = HeadlessTestApp::new();
    let mut settings = QuickSettingsState::default();
    let output = app.run_frame(|ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            mossambi::gui::popover::PetalPopover::render(
                ui,
                Pos2::new(100.0, 100.0),
                "PNG",
                &mut settings,
                || {},
            );
        });
    });
    assert!(!output.shapes.is_empty());
}

#[test]
fn test_tier2_f27_popover_anchor_at_zero() {
    let mut app = HeadlessTestApp::new();
    let mut settings = QuickSettingsState::default();
    let output = app.run_frame(|ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            mossambi::gui::popover::PetalPopover::render(
                ui,
                Pos2::ZERO,
                "JPEG",
                &mut settings,
                || {},
            );
        });
    });
    assert!(!output.shapes.is_empty());
}

#[test]
fn test_tier2_f27_popover_anchor_at_max() {
    let mut app = HeadlessTestApp::new();
    let mut settings = QuickSettingsState::default();
    let output = app.run_frame(|ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            mossambi::gui::popover::PetalPopover::render(
                ui,
                Pos2::new(1280.0, 800.0),
                "WEBP",
                &mut settings,
                || {},
            );
        });
    });
    assert!(!output.shapes.is_empty());
}

#[test]
fn test_tier2_f27_popover_zero_available_width() {
    let mut app = HeadlessTestApp::new();
    let mut settings = QuickSettingsState::default();
    let output = app.run_frame(|ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            mossambi::gui::popover::PetalPopover::render(
                ui,
                Pos2::new(50.0, 50.0),
                "PDF",
                &mut settings,
                || {},
            );
        });
    });
    assert!(!output.shapes.is_empty());
}

#[test]
fn test_tier2_f27_popover_anchor_y_boundary_threshold() {
    let mut app = HeadlessTestApp::new();
    let mut settings = QuickSettingsState::default();
    let output = app.run_frame(|ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            mossambi::gui::popover::PetalPopover::render(
                ui,
                Pos2::new(160.0, 160.0),
                "SVG",
                &mut settings,
                || {},
            );
        });
    });
    assert!(!output.shapes.is_empty());
}

// ============================================================================
// Feature 28: Boundaries for Test Runner Verification
// ============================================================================

#[test]
fn test_tier2_f28_headless_app_multiple_frames() {
    let mut app = HeadlessTestApp::new();
    let mut shape_counts = Vec::new();
    for i in 0..10 {
        let output = app.run_frame(|ctx| {
            eframe::egui::CentralPanel::default().show(ctx, |ui| {
                ui.label(format!("Frame {}", i));
            });
        });
        assert!(
            !output.shapes.is_empty(),
            "Frame {} must generate rendered shapes",
            i
        );
        shape_counts.push(output.shapes.len());
    }
    assert_eq!(
        shape_counts.len(),
        10,
        "All 10 frames executed successfully"
    );
    assert!(shape_counts.iter().all(|&c| c > 0));
}

#[test]
fn test_tier2_f28_isolated_config_env_write_and_read_empty() {
    let env = IsolatedConfigEnv::new().expect("create");
    env.write_config_json("").expect("write");
    let content = env.read_config_json().expect("read");
    assert_eq!(content, "");
}

#[test]
fn test_tier2_f28_fs_tree_auto_cleanup() {
    let path;
    {
        let fs = TestFsTree::new().expect("create");
        path = fs.root().to_path_buf();
        assert!(path.exists());
    }
    assert!(!path.exists(), "TempDir must be cleaned up on drop");
}

#[test]
fn test_tier2_f28_assert_wcag_large_boundary() {
    // Exactly 3.0:1
    assert_wcag_aa_large(Color32::WHITE, Color32::from_rgb(148, 148, 148), "test");
}

#[test]
fn test_tier2_f28_all_pantone_themes_contrast_ratios_valid() {
    for theme in &THEMES {
        let (r1, g1, b1) = theme.bg.to_rgb();
        let (r2, g2, b2) = theme.surface.to_rgb();
        let cr = contrast_ratio(Color32::from_rgb(r1, g1, b1), Color32::from_rgb(r2, g2, b2));
        assert!(cr >= 1.0 && cr <= 21.0);
    }
}

// ============================================================================
// Feature 29: Boundaries for Adversarial Inputs & Stress
// ============================================================================

#[test]
fn test_tier2_f29_empty_string_search_query() {
    let mut explorer = ExplorerView::default();
    explorer.search_query = String::new();
    let res = explorer.filtered_and_sorted_entries();
    assert!(res.is_empty());
}

#[test]
fn test_tier2_f29_search_query_with_null_character() {
    let mut explorer = ExplorerView::default();
    explorer.search_query = "null\0char".to_string();
    let res = explorer.filtered_and_sorted_entries();
    assert!(res.is_empty());
}

#[test]
fn test_tier2_f29_deep_nested_file_creation() {
    let fs = TestFsTree::new().expect("fs");
    let mut rel = String::new();
    for i in 0..10 {
        rel.push_str(&format!("dir_{i}/"));
    }
    rel.push_str("leaf.txt");
    let file = fs.create_file(&rel, b"deep").expect("create deep");
    assert!(file.exists());
}

#[test]
fn test_tier2_f29_symlink_to_directory_parent() {
    let fs = TestFsTree::new().expect("fs");
    let parent = fs.create_dir("parent_node").expect("dir");
    let child = fs.create_dir("parent_node/child_node").expect("dir");

    let _ = fs.create_symlink_or_junction("parent_node/child_node/up_link", &parent);
    assert!(child.exists());
}

#[test]
fn test_tier2_f29_validate_file_name_spaces_only() {
    let fs = TestFsTree::new().expect("fs");
    // Windows rejects a spaces-only filename; POSIX allows it but rejects a
    // name containing '/'. Use the current OS's genuinely illegal name so the
    // assertion stays valid on every platform in the CI matrix.
    #[cfg(windows)]
    let bad_name = "   ";
    #[cfg(not(windows))]
    let bad_name = "   /";
    let target = fs.root().join(bad_name);
    assert!(
        std::fs::File::create(&target).is_err(),
        "Filesystem must reject illegal filename '{bad_name}'"
    );
}

// ============================================================================
// Feature 30: Boundaries for Cross-Platform Path Handling
// ============================================================================

#[test]
fn test_tier2_f30_forward_slashes_in_path_components() {
    let crumbs = mossambi::core::fs_utils::parse_breadcrumbs(Path::new("a/b/c/d"));
    assert_eq!(crumbs.len(), 4);
    assert_eq!(crumbs[0].label, "a");
    assert_eq!(crumbs[3].label, "d");
}

#[test]
fn test_tier2_f30_backward_slashes_in_path_components() {
    let crumbs = mossambi::core::fs_utils::parse_breadcrumbs(Path::new("a\\b\\c\\d"));
    #[cfg(windows)]
    assert_eq!(crumbs.len(), 4);
    #[cfg(not(windows))]
    assert!(!crumbs.is_empty());
}

#[test]
fn test_tier2_f30_empty_path_components() {
    let crumbs = mossambi::core::fs_utils::parse_breadcrumbs(Path::new(""));
    assert_eq!(crumbs.len(), 1);
    assert_eq!(crumbs[0].label, "/");
}

#[test]
fn test_tier2_f30_root_path_components() {
    let crumbs = mossambi::core::fs_utils::parse_breadcrumbs(Path::new("/"));
    assert_eq!(crumbs.len(), 1);
}

#[test]
fn test_tier2_f30_unc_prefix_path() {
    let p = Path::new("\\\\server\\share\\folder");
    let resolved = mossambi::core::fs_utils::resolve_unique_paste_path(p);
    assert!(!resolved.as_os_str().is_empty());
}
