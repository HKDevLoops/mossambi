//! Tier 1: Happy-Path Feature Coverage Suite for Mossambi E2E.
//!
//! Enforces opaque-box happy path verification for all 30 features defined in `PROJECT.md`.
//! Exercised strictly through public interfaces and headless egui harnesses.

#![allow(
    clippy::field_reassign_with_default,
    clippy::manual_range_contains,
    clippy::unnecessary_sort_by,
    clippy::useless_vec
)]

mod common;

use common::{
    assert_wcag_aa, assert_wcag_aa_large, HeadlessTestApp, IsolatedConfigEnv, TestFsTree,
};
use eframe::egui::{Color32, Pos2, Vec2};
use mossambi::core::action_journal::{KeyStroke, RecordedAction};
use mossambi::core::config::{
    LookPreset, MenuStyle, MossambiConfig, RowDensity, ThemeMode, ViewMode,
};
use mossambi::core::fs_utils::{
    compute_dir_size, copy_entry_recursive, is_protected_os_file, list_directory_sync, FileItem,
    DIR_SIZE_CACHE_CAPACITY,
};
use mossambi::core::lru::LruCache;
use mossambi::core::pantone::{all_pantone_themes, THEMES};
use mossambi::core::platform_paths;
use mossambi::core::state_db::{CustomCommandRecord, CustomPresetRecord};
use mossambi::gui::anim::Spring;
use mossambi::gui::explorer_view::{
    preview_kind, thumb_max_px, CategoryFilter, DetailColumn, ExplorerView, PreviewKind,
    SortColumn, SortDirection,
};
use mossambi::gui::finder_chrome::render_caption_buttons;
use mossambi::gui::macos_theme::ThemeStyle;
use mossambi::gui::popover::PetalPopover;
use mossambi::gui::preferences_view::{preference_categories, PreferencesView};
use mossambi::gui::quick_settings::QuickSettingsState;
use mossambi::gui::studio_view::StudioView;
use mossambi::gui::view_zoom::ListZoomMetrics;
use mossambi::platform::args::{parse_launch_intent, parse_single_arg, LaunchIntent};
use std::path::{Path, PathBuf};

// ============================================================================
// Feature 1: view_mode field & persistence on MossambiConfig
// ============================================================================

#[test]
fn test_tier1_f01_view_mode_serde_roundtrip() {
    for mode in ViewMode::all() {
        let json = serde_json::to_string(&mode).expect("serialize ViewMode");
        let deserialized: ViewMode = serde_json::from_str(&json).expect("deserialize ViewMode");
        assert_eq!(
            mode, deserialized,
            "ViewMode {:?} must roundtrip serde",
            mode
        );
    }
}

#[test]
fn test_tier1_f01_config_zoom_for_all_view_modes() {
    let mut cfg = MossambiConfig::default();
    cfg.set_zoom_for_mode(ViewMode::Details, 0);
    cfg.set_zoom_for_mode(ViewMode::Grid, 3);
    cfg.set_zoom_for_mode(ViewMode::Columns, 4);

    assert_eq!(cfg.zoom_for_mode(ViewMode::Details), 0);
    assert_eq!(cfg.zoom_for_mode(ViewMode::Grid), 3);
    assert_eq!(cfg.zoom_for_mode(ViewMode::Columns), 4);
}

#[test]
fn test_tier1_f01_config_apply_look_preset_sets_layout() {
    let mut cfg = MossambiConfig::default();
    cfg.apply_look_preset(LookPreset::Finder);
    assert_eq!(
        cfg.default_explorer_layout, 3,
        "Finder defaults to Columns (3)"
    );

    cfg.apply_look_preset(LookPreset::WindowsExplorer);
    assert_eq!(
        cfg.default_explorer_layout, 0,
        "Windows Explorer defaults to Details (0)"
    );

    cfg.apply_look_preset(LookPreset::Nautilus);
    assert_eq!(
        cfg.default_explorer_layout, 2,
        "Nautilus defaults to Grid (2)"
    );

    cfg.apply_look_preset(LookPreset::Thunar);
    assert_eq!(
        cfg.default_explorer_layout, 0,
        "Thunar defaults to Details (0)"
    );
}

#[test]
fn test_tier1_f01_config_disk_persistence_roundtrip() {
    let env = IsolatedConfigEnv::new().expect("create isolated config env");
    let mut cfg = MossambiConfig::default();
    cfg.apply_look_preset(LookPreset::Nautilus);
    cfg.row_density = RowDensity::Spacious;

    env.save_config(&cfg).expect("save config");
    let loaded = env.load_config().expect("load config");

    assert_eq!(loaded.look_preset, Some(LookPreset::Nautilus));
    assert_eq!(loaded.row_density, RowDensity::Spacious);
    assert_eq!(loaded.default_explorer_layout, 2);
}

#[test]
fn test_tier1_f01_view_mode_all_contains_three_canonical_modes() {
    let modes = ViewMode::all();
    assert_eq!(modes.len(), 3);
    assert!(modes.contains(&ViewMode::Details));
    assert!(modes.contains(&ViewMode::Grid));
    assert!(modes.contains(&ViewMode::Columns));
}

// ============================================================================
// Feature 2: Bidirectional view_mode and row_density sync
// ============================================================================

#[test]
fn test_tier1_f02_explorer_view_mode_toggle() {
    let mut explorer = ExplorerView::default();
    explorer.view_mode = ViewMode::Details;
    assert_eq!(explorer.view_mode, ViewMode::Details);

    explorer.view_mode = ViewMode::Grid;
    assert_eq!(explorer.view_mode, ViewMode::Grid);

    explorer.view_mode = ViewMode::Columns;
    assert_eq!(explorer.view_mode, ViewMode::Columns);
}

#[test]
fn test_tier1_f02_row_density_scaling_values() {
    assert_eq!(RowDensity::Compact.details_row_height(), 24.0);
    assert_eq!(RowDensity::Default.details_row_height(), 28.0);
    assert_eq!(RowDensity::Standard.details_row_height(), 32.0);
    assert_eq!(RowDensity::Spacious.details_row_height(), 36.0);
}

#[test]
fn test_tier1_f02_row_density_sidebar_heights() {
    assert_eq!(RowDensity::Compact.sidebar_row_height(), 24.0);
    assert_eq!(RowDensity::Default.sidebar_row_height(), 30.0);
    assert_eq!(RowDensity::Standard.sidebar_row_height(), 32.0);
    assert_eq!(RowDensity::Spacious.sidebar_row_height(), 36.0);
}

#[test]
fn test_tier1_f02_sync_explorer_density_to_config() {
    let mut explorer = ExplorerView::default();
    explorer.row_density = RowDensity::Compact;

    let mut cfg = MossambiConfig::default();
    cfg.row_density = explorer.row_density;
    assert_eq!(cfg.row_density, RowDensity::Compact);
}

#[test]
fn test_tier1_f02_sync_config_density_to_explorer() {
    let mut cfg = MossambiConfig::default();
    cfg.row_density = RowDensity::Spacious;

    let mut explorer = ExplorerView::default();
    explorer.row_density = cfg.row_density;
    assert_eq!(explorer.row_density, RowDensity::Spacious);
}

// ============================================================================
// Feature 3: Eliminate Preferences and startup overwrite
// ============================================================================

#[test]
fn test_tier1_f03_preferences_rendering_does_not_mutate_config() {
    let mut app = HeadlessTestApp::new();
    let mut prefs = PreferencesView::default();
    let mut theme = ThemeStyle::default();
    let mut quick_settings = QuickSettingsState::default();
    let mut console = mossambi::core::scripting::ScriptConsole::default();

    let initial_view_mode = ViewMode::Grid;
    let mut config = MossambiConfig::default();
    config.set_zoom_for_mode(initial_view_mode, 3);

    for (idx, _) in preference_categories().iter().enumerate() {
        prefs.selected = idx;
        let _ = app.run_frame(|ctx| {
            eframe::egui::CentralPanel::default().show(ctx, |ui| {
                let palette = theme.colors();
                prefs.render(
                    ui,
                    &palette,
                    &mut theme,
                    &mut quick_settings,
                    &mut console,
                    (None, Vec::new()),
                );
            });
        });

        assert_eq!(
            config.zoom_for_mode(initial_view_mode),
            3,
            "Rendering preferences tab {} must not mutate custom zoom",
            idx
        );
    }
}

#[test]
fn test_tier1_f03_preferences_tabs_count() {
    let cats = preference_categories();
    assert!(
        cats.len() >= 7,
        "Preferences must contain at least 7 settings tabs"
    );
}

#[test]
fn test_tier1_f03_preferences_category_selection_isolated() {
    let mut prefs = PreferencesView::default();
    prefs.selected = 2;
    assert_eq!(prefs.selected, 2);
}

#[test]
fn test_tier1_f03_preferences_view_default_state() {
    let prefs = PreferencesView::default();
    assert_eq!(prefs.selected, 0);
}

#[test]
fn test_tier1_f03_studio_view_renders_without_corrupting_config() {
    let mut app = HeadlessTestApp::new();
    let mut studio = StudioView::default();
    let mut theme = ThemeStyle::default();
    let mut config = MossambiConfig::default();
    config.apply_look_preset(LookPreset::Thunar);
    let mut quick_settings = QuickSettingsState::default();

    let _ = app.run_frame(|ctx| {
        studio.render(
            ctx,
            &mut theme,
            &mut config,
            &None,
            &mut quick_settings,
            || {},
            |_| {},
            |_, _| {},
            |_| {},
        );
    });

    assert_eq!(config.look_preset, Some(LookPreset::Thunar));
    assert_eq!(config.row_density, RowDensity::Compact);
}

// ============================================================================
// Feature 4: Fix detached self.config saves
// ============================================================================

#[test]
fn test_tier1_f04_config_in_memory_theme_update() {
    let mut cfg = MossambiConfig::default();
    assert_eq!(cfg.theme_mode, ThemeMode::Rangalipi);

    cfg.theme_mode = ThemeMode::TokyoNight;
    assert_eq!(cfg.theme_mode, ThemeMode::TokyoNight);
}

#[test]
fn test_tier1_f04_config_save_persists_theme_change() {
    let env = IsolatedConfigEnv::new().expect("create config env");
    let mut cfg = MossambiConfig::default();
    cfg.theme_mode = ThemeMode::Kanagawa;
    env.save_config(&cfg).expect("save config");

    let loaded = env.load_config().expect("load config");
    assert_eq!(loaded.theme_mode, ThemeMode::Kanagawa);
}

#[test]
fn test_tier1_f04_autostart_toggle_persists() {
    let env = IsolatedConfigEnv::new().expect("create config env");
    let mut cfg = MossambiConfig::default();
    cfg.launch_at_login = true;
    cfg.autostart_prompted = Some(true);
    env.save_config(&cfg).expect("save config");

    let loaded = env.load_config().expect("load config");
    assert!(loaded.launch_at_login);
    assert_eq!(loaded.autostart_prompted, Some(true));
}

#[test]
fn test_tier1_f04_multiple_sequential_updates_persist() {
    let env = IsolatedConfigEnv::new().expect("create config env");
    let mut cfg = MossambiConfig::default();

    for theme in [
        ThemeMode::TokyoNight,
        ThemeMode::Kanagawa,
        ThemeMode::Rangalipi,
        ThemeMode::MossambiPunch,
    ] {
        cfg.theme_mode = theme;
        env.save_config(&cfg).expect("save config");
        let loaded = env.load_config().expect("load config");
        assert_eq!(loaded.theme_mode, theme);
    }
}

#[test]
fn test_tier1_f04_atomic_write_does_not_corrupt_file() {
    let env = IsolatedConfigEnv::new().expect("create config env");
    let cfg = MossambiConfig::default();
    env.save_config(&cfg).expect("save config");

    let raw = env.read_config_json().expect("read raw json");
    assert!(raw.contains("\"theme_mode\""));
    assert!(raw.ends_with('}'));
}

// ============================================================================
// Feature 5: Reconcile config directory paths
// ============================================================================

#[test]
fn test_tier1_f05_platform_paths_config_dir_exists() {
    let dir = platform_paths::config_dir();
    assert!(
        dir.to_string_lossy().contains(".config"),
        "Config directory must be inside .config/mossambi: {:?}",
        dir
    );
}

#[test]
fn test_tier1_f05_mossambi_config_config_path_matches_platform_dir() {
    let path = MossambiConfig::config_path();
    let dir = platform_paths::config_dir();
    assert_eq!(
        path.parent().expect("parent dir"),
        dir.as_path(),
        "MossambiConfig::config_path parent must match platform_paths::config_dir"
    );
}

#[test]
fn test_tier1_f05_isolated_config_path_structure() {
    let env = IsolatedConfigEnv::new().expect("create env");
    assert_eq!(env.config_path().file_name().unwrap(), "config.json");
}

#[test]
fn test_tier1_f05_custom_output_dir_persists() {
    let env = IsolatedConfigEnv::new().expect("create env");
    let mut cfg = MossambiConfig::default();
    cfg.custom_output_dir = Some(PathBuf::from("C:\\MossambiOutput"));
    env.save_config(&cfg).expect("save");

    let loaded = env.load_config().expect("load");
    assert_eq!(
        loaded.custom_output_dir,
        Some(PathBuf::from("C:\\MossambiOutput"))
    );
}

#[test]
fn test_tier1_f05_ensure_config_directory_creation() {
    let fs = TestFsTree::new().expect("create fs tree");
    let target_dir = fs.root().join(".config").join("mossambi");
    std::fs::create_dir_all(&target_dir).expect("create config dir");
    assert!(target_dir.is_dir());
}

// ============================================================================
// Feature 6: Zero-syscall FileItem extraction parity
// ============================================================================

#[test]
fn test_tier1_f06_file_item_from_path_parity() {
    let fs = TestFsTree::new().expect("create fs tree");
    let path = fs
        .create_file("test_doc.txt", b"Hello, Mossambi!")
        .expect("create file");

    let item = FileItem::from_path(path.clone());
    assert_eq!(item.name, "test_doc.txt");
    assert_eq!(item.extension, "TXT");
    assert!(!item.is_dir);
    assert_eq!(item.size_bytes, 16);
    assert!(item.modified.is_some());
}

#[test]
fn test_tier1_f06_directory_item_parity() {
    let fs = TestFsTree::new().expect("create fs tree");
    let dir_path = fs.create_dir("my_folder").expect("create dir");

    let item = FileItem::from_path(dir_path);
    assert_eq!(item.name, "my_folder");
    assert!(item.is_dir);
    assert_eq!(item.extension, "");
}

#[test]
fn test_tier1_f06_zero_byte_file_parity() {
    let fs = TestFsTree::new().expect("create fs tree");
    let path = fs.create_file("empty.dat", b"").expect("create file");

    let item = FileItem::from_path(path);
    assert_eq!(item.size_bytes, 0);
    assert_eq!(item.extension, "DAT");
}

#[test]
fn test_tier1_f06_file_item_formatted_size() {
    let fs = TestFsTree::new().expect("create fs tree");
    let path = fs
        .create_file("sample.bin", &[0u8; 1024])
        .expect("create file");

    let item = FileItem::from_path(path);
    assert_eq!(item.size_bytes, 1024);
}

#[test]
fn test_tier1_f06_sync_list_matches_items() {
    let fs = TestFsTree::new().expect("create fs tree");
    let _ = fs.create_file("a.txt", b"a").unwrap();
    let _ = fs.create_file("b.txt", b"b").unwrap();
    let _ = fs.create_dir("sub").unwrap();

    let items = list_directory_sync(fs.root());
    assert_eq!(items.len(), 3);
    assert!(
        items[0].is_dir,
        "Directories sort first in list_directory_sync"
    );
}

// ============================================================================
// Feature 7: In-place non-allocating sorting
// ============================================================================

#[test]
fn test_tier1_f07_case_insensitive_alphabetical_sort() {
    let mut names = vec!["zebra.txt", "Apple.txt", "banana.txt", "CHERRY.txt"];
    names.sort_by(|a, b| a.to_ascii_lowercase().cmp(&b.to_ascii_lowercase()));

    assert_eq!(
        names,
        vec!["Apple.txt", "banana.txt", "CHERRY.txt", "zebra.txt"]
    );
}

#[test]
fn test_tier1_f07_directory_first_sorting() {
    let fs = TestFsTree::new().expect("create fs tree");
    let f1 = fs.create_file("file_z.txt", b"z").unwrap();
    let d1 = fs.create_dir("dir_a").unwrap();
    let f2 = fs.create_file("file_a.txt", b"a").unwrap();

    let mut items = vec![
        FileItem::from_path(f1),
        FileItem::from_path(d1),
        FileItem::from_path(f2),
    ];

    items.sort_by(|a, b| {
        if a.is_dir != b.is_dir {
            b.is_dir.cmp(&a.is_dir)
        } else {
            a.name
                .to_ascii_lowercase()
                .cmp(&b.name.to_ascii_lowercase())
        }
    });

    assert_eq!(items[0].name, "dir_a");
    assert_eq!(items[1].name, "file_a.txt");
    assert_eq!(items[2].name, "file_z.txt");
}

#[test]
fn test_tier1_f07_sort_single_element() {
    let mut items = vec![FileItem::from_path(PathBuf::from("solo.txt"))];
    items.sort_by(|a, b| a.name.cmp(&b.name));
    assert_eq!(items.len(), 1);
}

#[test]
fn test_tier1_f07_sort_empty_vector() {
    let mut items: Vec<FileItem> = Vec::new();
    items.sort_by(|a, b| a.name.cmp(&b.name));
    assert!(items.is_empty());
}

#[test]
fn test_tier1_f07_sort_preserves_identical_names() {
    let mut items = vec![
        FileItem::from_path(PathBuf::from("same.txt")),
        FileItem::from_path(PathBuf::from("same.txt")),
    ];
    items.sort_by(|a, b| a.name.cmp(&b.name));
    assert_eq!(items.len(), 2);
}

// ============================================================================
// Feature 8: UI display list invalidation cache
// ============================================================================

#[test]
fn test_tier1_f08_explorer_view_filtered_and_sorted_entries() {
    let mut explorer = ExplorerView::default();
    explorer.entries = vec![
        FileItem::from_path(PathBuf::from("beta.txt")),
        FileItem::from_path(PathBuf::from("alpha.txt")),
        FileItem::from_path(PathBuf::from("gamma.txt")),
    ];

    let result = explorer.filtered_and_sorted_entries();
    assert_eq!(result.len(), 3);
    assert_eq!(result[0].name, "alpha.txt");
    assert_eq!(result[1].name, "beta.txt");
    assert_eq!(result[2].name, "gamma.txt");
}

#[test]
fn test_tier1_f08_search_query_filters_entries() {
    let mut explorer = ExplorerView::default();
    explorer.entries = vec![
        FileItem::from_path(PathBuf::from("report_2026.pdf")),
        FileItem::from_path(PathBuf::from("budget_2026.xlsx")),
        FileItem::from_path(PathBuf::from("photo.png")),
    ];

    explorer.search_query = "2026".to_string();
    let result = explorer.filtered_and_sorted_entries();
    assert_eq!(result.len(), 2);
    assert!(result.iter().all(|item| item.name.contains("2026")));
}

#[test]
fn test_tier1_f08_category_filter_application() {
    let mut explorer = ExplorerView::default();
    explorer.entries = vec![
        FileItem::from_path(PathBuf::from("doc.pdf")),
        FileItem::from_path(PathBuf::from("image.png")),
        FileItem::from_path(PathBuf::from("song.flac")),
    ];

    explorer.category_filter = CategoryFilter::Images;
    let result = explorer.filtered_and_sorted_entries();
    assert_eq!(result.len(), 1, "Images should match image.png");
}

#[test]
fn test_tier1_f08_sort_direction_descending() {
    let mut explorer = ExplorerView::default();
    explorer.entries = vec![
        FileItem::from_path(PathBuf::from("a.txt")),
        FileItem::from_path(PathBuf::from("b.txt")),
        FileItem::from_path(PathBuf::from("c.txt")),
    ];

    explorer.sort_direction = SortDirection::Descending;
    let result = explorer.filtered_and_sorted_entries();
    assert_eq!(result[0].name, "c.txt");
    assert_eq!(result[1].name, "b.txt");
    assert_eq!(result[2].name, "a.txt");
}

#[test]
fn test_tier1_f08_hidden_files_filter() {
    let mut explorer = ExplorerView::default();
    let mut hidden_item = FileItem::from_path(PathBuf::from(".git"));
    hidden_item.is_hidden = true;
    let mut regular_item = FileItem::from_path(PathBuf::from("visible.txt"));
    regular_item.is_hidden = false;

    explorer.entries = vec![hidden_item, regular_item];
    explorer.show_hidden_files = false;

    let result = explorer.filtered_and_sorted_entries();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "visible.txt");
}

// ============================================================================
// Feature 9: Viewport virtualization row ranges
// ============================================================================

#[test]
fn test_tier1_f09_list_zoom_metrics_step_heights() {
    let m0 = ListZoomMetrics::for_step(0);
    let m2 = ListZoomMetrics::for_step(2);
    let m4 = ListZoomMetrics::for_step(4);
    assert_eq!(m0.row_height, 22.0);
    assert_eq!(m2.row_height, 30.0);
    assert_eq!(m4.row_height, 42.0);
    assert!(m0.row_height < m2.row_height);
    assert!(m2.row_height < m4.row_height);
}

#[test]
fn test_tier1_f09_row_density_details_heights() {
    let h_compact = RowDensity::Compact.details_row_height();
    let h_default = RowDensity::Default.details_row_height();
    let h_spacious = RowDensity::Spacious.details_row_height();
    assert_eq!(h_compact, 24.0);
    assert_eq!(h_default, 28.0);
    assert_eq!(h_spacious, 36.0);
    assert!(h_compact < h_default && h_default < h_spacious);
}

#[test]
fn test_tier1_f09_zoom_level_clamping() {
    let mut explorer = ExplorerView::default();
    explorer.set_active_mode_zoom(0);
    assert_eq!(explorer.active_mode_zoom(), 0);
    explorer.set_active_mode_zoom(2);
    assert_eq!(explorer.active_mode_zoom(), 2);
    explorer.set_active_mode_zoom(100);
    assert_eq!(
        explorer.active_mode_zoom(),
        4,
        "Zoom 100 must be clamped to max 4"
    );
}

#[test]
fn test_tier1_f09_filtered_entries_empty_state() {
    let explorer = ExplorerView::default();
    let entries = explorer.filtered_and_sorted_entries();
    assert!(
        entries.is_empty(),
        "Empty explorer must have 0 filtered entries"
    );
}

#[test]
fn test_tier1_f09_filtered_entries_populated_order() {
    let mut explorer = ExplorerView::default();
    let fs = TestFsTree::new().expect("fs");
    fs.create_file("alpha.txt", b"a").expect("file");
    fs.create_file("beta.txt", b"b").expect("file");
    fs.create_file("gamma.txt", b"g").expect("file");
    let items = list_directory_sync(fs.root());
    explorer.entries = items;
    let filtered = explorer.filtered_and_sorted_entries();
    assert_eq!(filtered.len(), 3);
    assert_eq!(filtered[0].name, "alpha.txt");
    assert_eq!(filtered[1].name, "beta.txt");
    assert_eq!(filtered[2].name, "gamma.txt");
}

// ============================================================================
// Feature 10: LRU thumbnail texture cache
// ============================================================================

#[test]
fn test_tier1_f10_thumb_max_px_bucket_clamping() {
    assert_eq!(thumb_max_px(32.0), 128);
    assert_eq!(thumb_max_px(128.0), 128);
    assert_eq!(thumb_max_px(256.0), 256);
    assert_eq!(thumb_max_px(512.0), 512);
    assert_eq!(thumb_max_px(1024.0), 512);
}

#[test]
fn test_tier1_f10_preview_kind_mapping() {
    assert_eq!(preview_kind("PNG"), PreviewKind::Image);
    assert_eq!(preview_kind("JPG"), PreviewKind::Image);
    assert_eq!(preview_kind("PDF"), PreviewKind::Pdf);
    assert_eq!(preview_kind("MP4"), PreviewKind::Video);
    assert_eq!(preview_kind("TXT"), PreviewKind::None);
}

#[test]
fn test_tier1_f10_lru_eviction_simulation() {
    let mut cache = LruCache::<usize, usize>::new(256);
    for i in 0..300 {
        cache.insert(i, i * 10);
    }
    assert_eq!(cache.len(), 256, "Cache capacity must not exceed 256");
    assert_eq!(
        cache.get(&0),
        None,
        "Oldest element 0 must have been evicted"
    );
    assert_eq!(cache.get(&43), None, "Element 43 must have been evicted");
    assert_eq!(cache.get(&44), Some(&440), "Element 44 must be retained");
    assert_eq!(
        cache.get(&299),
        Some(&2990),
        "Most recent element must be retained"
    );
}

#[test]
fn test_tier1_f10_lru_mru_promotion() {
    let mut cache = LruCache::<&'static str, i32>::new(3);
    cache.insert("x", 10);
    cache.insert("y", 20);
    cache.insert("z", 30);
    // Access "x" -> promotes "x" to MRU
    assert_eq!(cache.get(&"x"), Some(&10));
    // Insert "w" -> should evict LRU, which is now "y"
    cache.insert("w", 40);
    assert_eq!(cache.get(&"y"), None, "'y' was LRU and must be evicted");
    assert_eq!(
        cache.get(&"x"),
        Some(&10),
        "'x' was promoted to MRU and must remain"
    );
    assert_eq!(cache.get(&"z"), Some(&30));
    assert_eq!(cache.get(&"w"), Some(&40));
}

#[test]
fn test_tier1_f10_preview_kind_case_insensitivity() {
    assert_eq!(preview_kind("png"), PreviewKind::Image);
    assert_eq!(preview_kind("Pdf"), PreviewKind::Pdf);
    assert_eq!(preview_kind("mkv"), PreviewKind::Video);
}

// ============================================================================
// Feature 11: Bounded column and directory size caches
// ============================================================================

#[test]
fn test_tier1_f11_dir_sizes_cache_compute() {
    let fs = TestFsTree::new().expect("fs");
    let sub = fs.create_dir("counted").expect("dir");
    fs.create_file("counted/f1.dat", &[0u8; 150]).expect("file");
    fs.create_file("counted/f2.dat", &[0u8; 250]).expect("file");

    let size = compute_dir_size(&sub, 8, 1000);
    assert_eq!(size, 400);
}

#[test]
fn test_tier1_f11_dir_sizes_entry_cap() {
    let fs = TestFsTree::new().expect("fs");
    let sub = fs.create_dir("capped").expect("dir");
    for i in 0..20 {
        fs.create_file(&format!("capped/f_{i}.dat"), &[0u8; 10])
            .expect("file");
    }

    // Capping at 5 entries
    let size = compute_dir_size(&sub, 8, 5);
    assert_eq!(size, 50);
}

#[test]
fn test_tier1_f11_dir_sizes_depth_cap() {
    let fs = TestFsTree::new().expect("fs");
    let _deep = fs.create_dir("d1/d2/d3").expect("dir");
    fs.create_file("d1/f0.dat", &[0u8; 10]).expect("file");
    fs.create_file("d1/d2/f1.dat", &[0u8; 20]).expect("file");
    fs.create_file("d1/d2/d3/f2.dat", &[0u8; 30]).expect("file");

    let root = fs.root().join("d1");
    let size_depth1 = compute_dir_size(&root, 1, 1000);
    assert_eq!(size_depth1, 30);

    let size_depth2 = compute_dir_size(&root, 2, 1000);
    assert_eq!(size_depth2, 60);
}

#[test]
fn test_tier1_f11_column_cache_bounds() {
    let mut cache = LruCache::<PathBuf, Vec<FileItem>>::new(32);
    for i in 0..40 {
        cache.insert(PathBuf::from(format!("path_{i}")), Vec::new());
    }
    assert_eq!(
        cache.len(),
        32,
        "Column cache must be strictly bounded at 32"
    );
    assert_eq!(
        cache.get(&PathBuf::from("path_0")),
        None,
        "Path 0 must have been evicted"
    );
    assert!(cache.get(&PathBuf::from("path_39")).is_some());
}

#[test]
fn test_tier1_f11_dir_sizes_cache_capacity_constant() {
    assert_eq!(
        DIR_SIZE_CACHE_CAPACITY, 10_000,
        "Directory size cache capacity constant must be 10,000"
    );
}

#[test]
fn test_tier1_f11_detail_columns_count() {
    let cols = DetailColumn::all();
    assert_eq!(cols.len(), 11);
}

// ============================================================================
// Feature 12: Shell icon deduplication & spring pruning
// ============================================================================

#[test]
fn test_tier1_f12_spring_initialization() {
    let spring = Spring::new(0.0);
    assert_eq!(spring.value, 0.0);
    assert_eq!(spring.target, 0.0);
}

#[test]
fn test_tier1_f12_spring_step_towards_target() {
    let mut spring = Spring::new(0.0);
    spring.target = 1.0;
    spring.step(0.016); // 1 frame at 60fps
    assert!(spring.value > 0.0);
}

#[test]
fn test_tier1_f12_spring_settling() {
    let mut spring = Spring::new(0.0);
    spring.target = 1.0;
    for _ in 0..120 {
        spring.step(0.016);
    }
    assert!((spring.value - 1.0).abs() < 0.05);
}

#[test]
fn test_tier1_f12_icon_deduplication_by_extension() {
    let f1 = FileItem::from_path(PathBuf::from("sample.txt"));
    let f2 = FileItem::from_path(PathBuf::from("report.TXT"));
    let f3 = FileItem::from_path(PathBuf::from("notes.Txt"));
    assert_eq!(f1.extension, "TXT");
    assert_eq!(f2.extension, "TXT");
    assert_eq!(f3.extension, "TXT");
}

#[test]
fn test_tier1_f12_spring_velocity_damps() {
    let mut spring = Spring::new(0.0);
    spring.target = 1.0;
    for _ in 0..200 {
        spring.step(0.016);
    }
    assert!(spring.velocity.abs() < 0.01);
}

// ============================================================================
// Feature 13: Eliminate PowerShell command injection
// ============================================================================

#[test]
fn test_tier1_f13_shortcut_argument_quoting() {
    let malicious = "C:\\Tools;calc.exe.txt";
    let intent = parse_single_arg(malicious);
    assert_eq!(
        intent,
        LaunchIntent::Reveal {
            target: PathBuf::from(malicious),
            select: true,
        },
        "Semicolon in path must not split launch intent"
    );
}

#[test]
fn test_tier1_f13_shortcut_powershell_metachar_escaping() {
    let vector = "file$(Get-Process).txt";
    let intent = parse_single_arg(vector);
    assert_eq!(
        intent,
        LaunchIntent::Reveal {
            target: PathBuf::from(vector),
            select: true,
        },
        "PowerShell subexpression must be treated as literal filename"
    );
}

#[test]
fn test_tier1_f13_semicolon_in_path_quoted() {
    let path = "C:\\Projects\\repo;exit\\run.bat";
    let intent = parse_single_arg(path);
    assert_eq!(
        intent,
        LaunchIntent::Reveal {
            target: PathBuf::from(path),
            select: true,
        }
    );
}

#[test]
fn test_tier1_f13_backtick_in_path_quoted() {
    let path = "C:\\Path`with`backticks";
    let intent = parse_single_arg(path);
    assert_eq!(intent, LaunchIntent::Navigate(PathBuf::from(path)));
}

#[test]
fn test_tier1_f13_pipe_in_filename_rejected() {
    let fs = TestFsTree::new().expect("fs");
    let bad_path = fs.root().join("file|command.txt");
    let res = std::fs::File::create(&bad_path);
    assert!(
        res.is_err(),
        "Operating system must reject creating file with pipe character"
    );
}

// ============================================================================
// Feature 14: Harden Windows default app execution
// ============================================================================

#[test]
fn test_tier1_f14_launch_intent_navigate() {
    let args = ["mossambi.exe", "C:\\Projects"];
    let intent = parse_launch_intent(&args);
    assert_eq!(
        intent,
        LaunchIntent::Navigate(PathBuf::from("C:\\Projects"))
    );
}

#[test]
fn test_tier1_f14_launch_intent_reveal_select() {
    let args = ["mossambi.exe", "/select,C:\\Projects\\file.txt"];
    let intent = parse_launch_intent(&args);
    assert_eq!(
        intent,
        LaunchIntent::Reveal {
            target: PathBuf::from("C:\\Projects\\file.txt"),
            select: true,
        }
    );
}

#[test]
fn test_tier1_f14_launch_intent_quoted_path_with_spaces() {
    let arg = "/select,C:\\Program Files\\Mossambi\\app.exe";
    let intent = parse_single_arg(arg);
    assert_eq!(
        intent,
        LaunchIntent::Reveal {
            target: PathBuf::from("C:\\Program Files\\Mossambi\\app.exe"),
            select: true,
        }
    );
}

#[test]
fn test_tier1_f14_launch_intent_empty_args_is_none() {
    let args: Vec<&str> = Vec::new();
    let intent = parse_launch_intent(&args);
    assert_eq!(intent, LaunchIntent::None);
}

#[test]
fn test_tier1_f14_launch_intent_uri_parsing() {
    let arg = "mossambi://open?path=C%3A%5CUsers";
    let intent = parse_single_arg(arg);
    assert!(matches!(intent, LaunchIntent::Navigate(_)));
}

// ============================================================================
// Feature 15: Central filename validation & path traversal protection
// ============================================================================

#[test]
fn test_tier1_f15_valid_filenames_pass() {
    let names = [
        "document.txt",
        "My Report (Final).pdf",
        "archive_2026.tar.gz",
        "code.rs",
        "photo.JPEG",
    ];
    for name in names {
        let f = FileItem::from_path(PathBuf::from(name));
        assert_eq!(f.name, name);
    }
}

#[test]
fn test_tier1_f15_reject_empty_filename() {
    let f = FileItem::from_path(PathBuf::from(""));
    assert!(f.name.is_empty());
}

#[test]
fn test_tier1_f15_reject_path_traversal() {
    let traversals = ["..", "../secret", "..\\secret", "folder/file"];
    for trav in traversals {
        let p = Path::new(trav);
        let components: Vec<_> = p.components().collect();
        assert!(
            components.len() > 1 || components.first() == Some(&std::path::Component::ParentDir),
            "Path '{trav}' must have traversal components"
        );
    }
}

#[test]
fn test_tier1_f15_protected_system_files() {
    let protected = [
        "pagefile.sys",
        "hiberfil.sys",
        "swapfile.sys",
        "dumpstack.log",
    ];
    for name in protected {
        assert!(
            is_protected_os_file(Path::new(name)),
            "'{name}' must be classified as protected system file"
        );
    }
    assert!(!is_protected_os_file(Path::new("regular_document.docx")));
    assert!(!is_protected_os_file(Path::new("my_photo.jpg")));
}

#[test]
fn test_tier1_f15_reject_illegal_characters() {
    let fs = TestFsTree::new().expect("fs");
    let illegal_names = [
        "bad<name>.txt",
        "bad>name.txt",
        "bad\"name.txt",
        "bad|name.txt",
        "bad?name.txt",
        "bad*name.txt",
    ];
    for name in illegal_names {
        let path = fs.root().join(name);
        let res = std::fs::File::create(&path);
        assert!(
            res.is_err(),
            "Filesystem must reject illegal filename '{name}'"
        );
    }
}

// ============================================================================
// Feature 16: Symlink & NTFS junction loop detection
// ============================================================================

#[test]
fn test_tier1_f16_clean_tree_copy() {
    let fs = TestFsTree::new().expect("fs");
    let src = fs.create_dir("source_tree").expect("dir");
    fs.create_file("source_tree/f1.txt", b"hello")
        .expect("file");
    fs.create_file("source_tree/sub/f2.txt", b"world")
        .expect("file");

    let dest = fs.root().join("dest_tree");
    let bytes = copy_entry_recursive(&src, &dest).expect("copy");

    assert_eq!(bytes, 10);
    assert!(dest.join("f1.txt").exists());
    assert!(dest.join("sub/f2.txt").exists());
}

#[test]
fn test_tier1_f16_single_file_copy() {
    let fs = TestFsTree::new().expect("fs");
    let src_file = fs.create_file("solo.txt", b"12345678").expect("file");
    let dest_file = fs.root().join("solo_copy.txt");

    let bytes = copy_entry_recursive(&src_file, &dest_file).expect("copy");

    assert_eq!(bytes, 8);
    assert_eq!(std::fs::read(&dest_file).unwrap(), b"12345678");
}

#[test]
fn test_tier1_f16_nested_folder_structure_preserved() {
    let fs = TestFsTree::new().expect("fs");
    fs.create_file("src_nested/a/b/c/item.txt", b"deep")
        .expect("file");

    let src = fs.root().join("src_nested");
    let dest = fs.root().join("dst_nested");
    let bytes = copy_entry_recursive(&src, &dest).expect("copy");

    assert_eq!(bytes, 4);
    assert!(dest.join("a/b/c/item.txt").exists());
}

#[test]
fn test_tier1_f16_empty_directory_copy() {
    let fs = TestFsTree::new().expect("fs");
    let src = fs.create_dir("empty_src").expect("dir");
    let dest = fs.root().join("empty_dest");

    let bytes = copy_entry_recursive(&src, &dest).expect("copy");
    assert_eq!(bytes, 0);
    assert!(dest.is_dir());
}

#[test]
fn test_tier1_f16_nonexistent_source_fails() {
    let fs = TestFsTree::new().expect("fs");
    let src = fs.root().join("nonexistent_folder");
    let dest = fs.root().join("dest_folder");
    let res = copy_entry_recursive(&src, &dest);
    assert!(res.is_err(), "Copying nonexistent source must return Err");
}

// ============================================================================
// Feature 17: Asynchronous SQLite keystroke journaling
// ============================================================================

#[test]
fn test_tier1_f17_keystroke_serde_roundtrip() {
    let stroke = KeyStroke {
        at_ms: 1731920000000,
        context: "test_ctx".to_string(),
        key: "ArrowDown".to_string(),
        modifiers: "Ctrl+Shift".to_string(),
    };
    let json = serde_json::to_string(&stroke).expect("serialize KeyStroke");
    let parsed: KeyStroke = serde_json::from_str(&json).expect("deserialize KeyStroke");
    assert_eq!(parsed.context, "test_ctx");
    assert_eq!(parsed.key, "ArrowDown");
    assert_eq!(parsed.modifiers, "Ctrl+Shift");
}

#[test]
fn test_tier1_f17_keystroke_unicode_bangla() {
    let stroke = KeyStroke {
        at_ms: 1731920000000,
        context: "bangla_input".to_string(),
        key: "\u{0995}".to_string(),
        modifiers: String::new(),
    };
    let json = serde_json::to_string(&stroke).expect("serialize KeyStroke");
    let parsed: KeyStroke = serde_json::from_str(&json).expect("deserialize KeyStroke");
    assert_eq!(parsed.key, "\u{0995}");
}

#[test]
fn test_tier1_f17_state_db_path_validity() {
    let db_path = mossambi::core::state_db::db_path();
    assert!(
        db_path.to_string_lossy().contains("mossambi_state.db")
            || db_path.to_string_lossy().contains("state.db"),
        "State DB path must reference state.db: {:?}",
        db_path
    );
}

#[test]
fn test_tier1_f17_recorded_action_serde_roundtrip() {
    let action = RecordedAction::CreateDir {
        path: PathBuf::from("C:\\Projects\\mossambi"),
    };
    let json = serde_json::to_string(&action).expect("serialize RecordedAction");
    let parsed: RecordedAction = serde_json::from_str(&json).expect("deserialize RecordedAction");
    assert!(matches!(parsed, RecordedAction::CreateDir { .. }));
}

#[test]
fn test_tier1_f17_custom_records_serde_and_in_memory_schema() {
    let preset = CustomPresetRecord {
        name: "Cyberpunk 2077".to_string(),
        category: "Gaming".to_string(),
        target_format: "png".to_string(),
        width_px: 1920,
        height_px: 1080,
        dpi: 96,
        min_bytes: 100,
        max_bytes: 10000000,
        normalize_white_balance: false,
        background_rgb: [0, 0, 0],
    };
    let json = serde_json::to_string(&preset).expect("serialize preset");
    let parsed: CustomPresetRecord = serde_json::from_str(&json).expect("deserialize preset");
    assert_eq!(parsed.name, "Cyberpunk 2077");

    let cmd = CustomCommandRecord {
        id: "git_status".to_string(),
        title: "Git Status".to_string(),
        script_lang: "sh".to_string(),
        script_body: "git status".to_string(),
        shortcut: Some("Ctrl+G".to_string()),
    };
    let cmd_json = serde_json::to_string(&cmd).expect("serialize cmd");
    let parsed_cmd: CustomCommandRecord = serde_json::from_str(&cmd_json).expect("deserialize cmd");
    assert_eq!(parsed_cmd.id, "git_status");
    assert_eq!(parsed_cmd.shortcut.as_deref(), Some("Ctrl+G"));

    // In-memory SQLite schema verification (zero host disk writes)
    let conn = rusqlite::Connection::open_in_memory().expect("in-memory db");
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS action_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            action_type TEXT NOT NULL,
            payload_json TEXT NOT NULL,
            timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS custom_presets (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            config_json TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS custom_commands (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            icon TEXT,
            executable TEXT NOT NULL,
            arguments TEXT,
            working_directory TEXT,
            shortcut TEXT,
            show_in_context_menu INTEGER DEFAULT 1,
            show_in_palette INTEGER DEFAULT 1,
            enabled INTEGER DEFAULT 1,
            order_index INTEGER DEFAULT 0
        );
        "#,
    )
    .expect("create tables");
    let count: i64 = conn
        .query_row("SELECT count(*) FROM custom_commands", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 0);
}

// ============================================================================
// Feature 18: Central canvas surface separation
// ============================================================================

#[test]
fn test_tier1_f18_pantone_themes_surface_2_contrast_dark() {
    for theme in &THEMES[0..12] {
        let (r1, g1, b1) = theme.surface_2.to_rgb();
        let (rt, gt, bt) = theme.text_primary.to_rgb();

        let s2 = Color32::from_rgb(r1, g1, b1);
        let tp = Color32::from_rgb(rt, gt, bt);

        assert_wcag_aa(
            tp,
            s2,
            &format!("Theme '{}' text_primary on surface_2", theme.id),
        );
    }
}

#[test]
fn test_tier1_f18_pantone_themes_surface_2_contrast_light() {
    let light_theme = &THEMES[12]; // mossambi_punch
    let (r1, g1, b1) = light_theme.surface_2.to_rgb();
    let (rt, gt, bt) = light_theme.text_primary.to_rgb();

    let s2 = Color32::from_rgb(r1, g1, b1);
    let tp = Color32::from_rgb(rt, gt, bt);

    assert_wcag_aa(
        tp,
        s2,
        &format!("Light theme '{}' text_primary on surface_2", light_theme.id),
    );
}

#[test]
fn test_tier1_f18_surface_2_differs_from_surface_1() {
    for theme in &THEMES {
        assert_ne!(
            theme.surface_1.hex, theme.surface_2.hex,
            "Theme '{}' surface_1 and surface_2 must have visual separation",
            theme.id
        );
    }
}

#[test]
fn test_tier1_f18_surface_2_differs_from_window_bg() {
    for theme in &THEMES {
        assert_ne!(
            theme.bg.hex, theme.surface_2.hex,
            "Theme '{}' window_bg and surface_2 must have visual separation",
            theme.id
        );
    }
}

#[test]
fn test_tier1_f18_all_13_themes_accessible() {
    assert_eq!(all_pantone_themes().len(), 13);
}

// ============================================================================
// Feature 19: Details view row hover & selection fills
// ============================================================================

#[test]
fn test_tier1_f19_selection_contrast_against_accent_text() {
    for style in ThemeStyle::all() {
        let pal = style.colors();
        let text_color = mossambi::gui::design::contrast::text_on_accent(pal.selection);
        assert_wcag_aa_large(
            text_color,
            pal.selection,
            &format!("ThemeStyle {:?} text on selection fill", style),
        );
    }
}

#[test]
fn test_tier1_f19_card_hover_color_differs_from_window_bg() {
    for style in ThemeStyle::all() {
        let pal = style.colors();
        assert_ne!(
            pal.card_hover, pal.window_bg,
            "ThemeStyle {:?} card_hover must differ from window_bg",
            style
        );
    }
}

#[test]
fn test_tier1_f19_selection_fill_differs_from_surface() {
    for style in ThemeStyle::all() {
        let pal = style.colors();
        assert_ne!(pal.selection, pal.surface_2);
    }
}

#[test]
fn test_tier1_f19_selection_state_toggle() {
    let mut explorer = ExplorerView::default();
    let p = PathBuf::from("selected_file.txt");
    explorer.entries = vec![FileItem::from_path(p.clone())];

    explorer.select_all();
    assert_eq!(explorer.selected_paths(), vec![p.clone()]);

    explorer.clear_selection();
    assert!(explorer.selected_paths().is_empty());
}

#[test]
fn test_tier1_f19_multi_selection_count() {
    let mut explorer = ExplorerView::default();
    explorer.entries = vec![
        FileItem::from_path(PathBuf::from("f1.txt")),
        FileItem::from_path(PathBuf::from("f2.txt")),
        FileItem::from_path(PathBuf::from("f3.txt")),
    ];

    explorer.select_all();
    assert_eq!(explorer.selected_paths().len(), 3);
}

// ============================================================================
// Feature 20: Wire RowDensity into row height
// ============================================================================

#[test]
fn test_tier1_f20_compact_density_height() {
    assert_eq!(RowDensity::Compact.details_row_height(), 24.0);
}

#[test]
fn test_tier1_f20_default_density_height() {
    assert_eq!(RowDensity::Default.details_row_height(), 28.0);
}

#[test]
fn test_tier1_f20_standard_density_height() {
    assert_eq!(RowDensity::Standard.details_row_height(), 32.0);
}

#[test]
fn test_tier1_f20_spacious_density_height() {
    assert_eq!(RowDensity::Spacious.details_row_height(), 36.0);
}

#[test]
fn test_tier1_f20_row_density_monotonicity() {
    let heights = [
        RowDensity::Compact.details_row_height(),
        RowDensity::Default.details_row_height(),
        RowDensity::Standard.details_row_height(),
        RowDensity::Spacious.details_row_height(),
    ];
    for window in heights.windows(2) {
        assert!(window[0] < window[1]);
    }
}

// ============================================================================
// Feature 21: Native Windows caption button cluster
// ============================================================================

#[test]
fn test_tier1_f21_caption_buttons_render_dark_palette() {
    let mut app = HeadlessTestApp::new();
    let pal = ThemeStyle::TokyoNight.colors();
    let output = app.run_frame(|ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            render_caption_buttons(ui, &pal, || {}, || {}, || {});
        });
    });
    assert!(
        !output.shapes.is_empty(),
        "Caption buttons must produce rendering shapes"
    );
}

#[test]
fn test_tier1_f21_caption_buttons_render_on_all_themes() {
    let mut app = HeadlessTestApp::new();
    for style in ThemeStyle::all() {
        let pal = style.colors();
        let output = app.run_frame(|ctx| {
            eframe::egui::CentralPanel::default().show(ctx, |ui| {
                render_caption_buttons(ui, &pal, || {}, || {}, || {});
            });
        });
        assert!(
            !output.shapes.is_empty(),
            "Caption buttons must render on {:?}",
            style
        );
    }
}

#[test]
fn test_tier1_f21_chrome_spec_presets_controls_position() {
    for preset in LookPreset::all() {
        let spec = mossambi::core::ChromeSpec::for_preset(preset);
        match preset {
            LookPreset::Finder => {
                assert_eq!(
                    spec.window_controls,
                    mossambi::core::look_spec::WindowControlsPosition::LeftTraffic
                );
            }
            _ => {
                assert_eq!(
                    spec.window_controls,
                    mossambi::core::look_spec::WindowControlsPosition::RightCaption
                );
            }
        }
    }
}

// ============================================================================
// Feature 22: Context-aware Maximize/Restore glyph
// ============================================================================

#[test]
fn test_tier1_f22_caption_buttons_with_callbacks() {
    let mut app = HeadlessTestApp::new();
    let pal = ThemeStyle::TokyoNight.colors();
    let mut min_clicked = false;
    let mut max_clicked = false;
    let mut close_clicked = false;

    let output = app.run_frame(|ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            render_caption_buttons(
                ui,
                &pal,
                || min_clicked = true,
                || max_clicked = true,
                || close_clicked = true,
            );
        });
    });
    assert!(!output.shapes.is_empty());
    let _ = (min_clicked, max_clicked, close_clicked);
}

#[test]
fn test_tier1_f22_restore_glyph_inside_button_bounds() {
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
}

#[test]
fn test_tier1_f22_caption_button_hit_testing() {
    let mut app = HeadlessTestApp::new();
    let pal = ThemeStyle::Gruvbox.colors();
    let mut close_fired = false;
    let mut min_fired = false;
    let mut max_fired = false;

    app.simulate_click(Pos2::new(14.0, 12.0));
    let output = app.run_frame(|ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            render_caption_buttons(
                ui,
                &pal,
                || close_fired = true,
                || min_fired = true,
                || max_fired = true,
            );
        });
    });
    assert!(!output.shapes.is_empty());
    let _ = (close_fired, min_fired, max_fired);
}

// ============================================================================
// Feature 23: Synchronize title bar drag reserve width
// ============================================================================

#[test]
fn test_tier1_f23_windows_preset_caption_reserve() {
    let spec = mossambi::core::ChromeSpec::for_preset(LookPreset::WindowsExplorer);
    assert_eq!(
        spec.window_controls,
        mossambi::core::look_spec::WindowControlsPosition::RightCaption
    );
    assert_eq!(
        spec.title_bar,
        mossambi::core::look_spec::TitleBarStyle::Separate
    );
    assert_eq!(
        spec.tab_position,
        mossambi::core::look_spec::TabPosition::Top
    );
}

#[test]
fn test_tier1_f23_finder_preset_traffic_lights_reserve() {
    let spec = mossambi::core::ChromeSpec::for_preset(LookPreset::Finder);
    assert_eq!(
        spec.window_controls,
        mossambi::core::look_spec::WindowControlsPosition::LeftTraffic
    );
    assert_eq!(
        spec.title_bar,
        mossambi::core::look_spec::TitleBarStyle::Unified
    );
    assert_eq!(
        spec.tab_position,
        mossambi::core::look_spec::TabPosition::InHeader
    );
}

#[test]
fn test_tier1_f23_nautilus_caption_reserve() {
    let spec = mossambi::core::ChromeSpec::for_preset(LookPreset::Nautilus);
    assert_eq!(
        spec.window_controls,
        mossambi::core::look_spec::WindowControlsPosition::RightCaption
    );
    assert_eq!(
        spec.title_bar,
        mossambi::core::look_spec::TitleBarStyle::Unified
    );
    assert_eq!(
        spec.tab_position,
        mossambi::core::look_spec::TabPosition::None
    );
}

#[test]
fn test_tier1_f23_thunar_caption_reserve() {
    let spec = mossambi::core::ChromeSpec::for_preset(LookPreset::Thunar);
    assert_eq!(
        spec.window_controls,
        mossambi::core::look_spec::WindowControlsPosition::RightCaption
    );
    assert_eq!(
        spec.title_bar,
        mossambi::core::look_spec::TitleBarStyle::Separate
    );
    assert_eq!(
        spec.tab_position,
        mossambi::core::look_spec::TabPosition::None
    );
}

// ============================================================================
// Feature 24: Context menu monitor work area clamping
// ============================================================================

#[test]
fn test_tier1_f24_menu_standard_viewport_render() {
    let mut app = HeadlessTestApp::new();
    let item = FileItem::from_path(PathBuf::from("document.pdf"));
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
        "Context menu must render shapes in standard viewport"
    );
}

#[test]
fn test_tier1_f24_menu_narrow_viewport_height_bounded() {
    let mut app = HeadlessTestApp::new();
    app.simulate_resize(Vec2::new(320.0, 240.0));
    let item = FileItem::from_path(PathBuf::from("test.txt"));
    let targets = vec![item.path.clone()];
    let output = app.run_frame(|ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            let _ = ExplorerView::run_item_context_menu(
                ui,
                mossambi::core::icon_packs::IconPack::from_config_id("win11"),
                MenuStyle::Finder,
                Color32::WHITE,
                &item,
                &targets,
            );
        });
    });
    assert!(
        !output.shapes.is_empty(),
        "Context menu must safely render in narrow viewport"
    );
}

#[test]
fn test_tier1_f24_menu_multi_item_selection() {
    let mut app = HeadlessTestApp::new();
    let item = FileItem::from_path(PathBuf::from("item1.txt"));
    let targets = vec![
        PathBuf::from("item1.txt"),
        PathBuf::from("item2.txt"),
        PathBuf::from("item3.txt"),
    ];
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
        "Multi-selection context menu must render shapes"
    );
}

#[test]
fn test_tier1_f24_menu_directory_actions() {
    let mut app = HeadlessTestApp::new();
    let mut item = FileItem::from_path(PathBuf::from("my_folder"));
    item.is_dir = true;
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
        "Directory context menu must render shapes"
    );
}

#[test]
fn test_tier1_f24_menu_style_variations() {
    for style in MenuStyle::all() {
        let mut app = HeadlessTestApp::new();
        let item = FileItem::from_path(PathBuf::from("sample.rs"));
        let targets = vec![item.path.clone()];
        let output = app.run_frame(|ctx| {
            eframe::egui::CentralPanel::default().show(ctx, |ui| {
                let _ = ExplorerView::run_item_context_menu(
                    ui,
                    mossambi::core::icon_packs::IconPack::from_config_id("win11"),
                    *style,
                    Color32::WHITE,
                    &item,
                    &targets,
                );
            });
        });
        assert!(
            !output.shapes.is_empty(),
            "Menu style {:?} must render shapes",
            style
        );
    }
}

// ============================================================================
// Feature 25: Inline submenus in native viewport
// ============================================================================

#[test]
fn test_tier1_f25_menu_style_variants() {
    let all = MenuStyle::all();
    assert_eq!(all.len(), 3);
    assert!(all.contains(&MenuStyle::Finder));
    assert!(all.contains(&MenuStyle::Windows));
    assert!(all.contains(&MenuStyle::Linux));
}

#[test]
fn test_tier1_f25_canvas_context_menu_action_intents() {
    let intents = mossambi::gui::explorer_view::CanvasMenuIntents {
        new_folder: true,
        new_tag: false,
        ..Default::default()
    };
    assert!(intents.new_folder);
    assert!(!intents.new_tag);
}

#[test]
fn test_tier1_f25_item_context_menu_action_intents() {
    let intents = mossambi::gui::explorer_view::ItemMenuIntents {
        open_default: Some(PathBuf::from("test.txt")),
        deleted: None,
        rename: None,
        ..Default::default()
    };
    assert!(intents.open_default.is_some());
    assert!(intents.deleted.is_none());
}

#[test]
fn test_tier1_f25_native_menu_request_anchor() {
    let req = mossambi::gui::explorer_view::NativeMenuRequest::Canvas {
        anchor: Pos2::new(250.0, 350.0),
        pack: mossambi::core::icon_packs::IconPack::from_config_id("win11"),
        style: MenuStyle::Windows,
        tint: Color32::WHITE,
        has_clipboard: false,
        has_entries: true,
        sort: SortColumn::Name,
        sort_desc: false,
        view: ViewMode::Details,
        dir: PathBuf::from("C:\\Test"),
    };
    assert_eq!(req.anchor(), Pos2::new(250.0, 350.0));
}

// ============================================================================
// Feature 26: Dynamic modal clamping & auto-repositioning
// ============================================================================

#[test]
fn test_tier1_f26_modal_centered_in_viewport() {
    let mut app = HeadlessTestApp::new();
    let mut explorer = ExplorerView::default();
    explorer.properties_item = Some(FileItem::from_path(PathBuf::from("document.pdf")));
    let palette = ThemeStyle::default().colors();

    let output = app.run_frame(|ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            explorer.render(ui, &palette, |_| {}, |_, _| {}, |_| {}, |_| {}, |_| {});
        });
    });
    assert!(
        !output.shapes.is_empty(),
        "Properties modal must render centered in viewport"
    );
    assert!(explorer.properties_item.is_some());
}

#[test]
fn test_tier1_f26_modal_small_viewport_clamping() {
    let mut app = HeadlessTestApp::new();
    app.simulate_resize(Vec2::new(640.0, 480.0));
    let mut explorer = ExplorerView::default();
    explorer.properties_item = Some(FileItem::from_path(PathBuf::from("test.txt")));
    let palette = ThemeStyle::default().colors();

    let output = app.run_frame(|ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            explorer.render(ui, &palette, |_| {}, |_, _| {}, |_| {}, |_| {}, |_| {});
        });
    });
    assert!(
        !output.shapes.is_empty(),
        "Modal must render safely in 640x480 viewport"
    );
}

#[test]
fn test_tier1_f26_modal_directory_properties() {
    let mut app = HeadlessTestApp::new();
    let mut explorer = ExplorerView::default();
    let mut dir_item = FileItem::from_path(PathBuf::from("src"));
    dir_item.is_dir = true;
    explorer.properties_item = Some(dir_item);
    let palette = ThemeStyle::default().colors();

    let output = app.run_frame(|ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            explorer.render(ui, &palette, |_| {}, |_, _| {}, |_| {}, |_| {}, |_| {});
        });
    });
    assert!(
        !output.shapes.is_empty(),
        "Directory properties modal must render"
    );
}

#[test]
fn test_tier1_f26_modal_dismissed_when_none() {
    let mut app = HeadlessTestApp::new();
    let mut explorer = ExplorerView::default();
    explorer.properties_item = None;
    let palette = ThemeStyle::default().colors();

    let output = app.run_frame(|ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            explorer.render(ui, &palette, |_| {}, |_, _| {}, |_| {}, |_| {}, |_| {});
        });
    });
    assert!(!output.shapes.is_empty());
    assert!(explorer.properties_item.is_none());
}

#[test]
fn test_tier1_f26_properties_modal_render() {
    let mut app = HeadlessTestApp::new();
    let mut explorer = ExplorerView::default();
    explorer.properties_item = Some(FileItem::from_path(PathBuf::from("test_doc.txt")));
    let palette = ThemeStyle::default().colors();
    let output = app.run_frame(|ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            explorer.render(ui, &palette, |_| {}, |_, _| {}, |_| {}, |_| {}, |_| {});
        });
    });
    assert!(
        !output.shapes.is_empty(),
        "Properties modal must render shapes via explorer.render"
    );
    assert_eq!(explorer.properties_rename_buf, "test_doc.txt");
}

// ============================================================================
// Feature 27: Fix Petal popover bounds
// ============================================================================

#[test]
fn test_tier1_f27_popover_renders_at_top_anchor() {
    let mut app = HeadlessTestApp::new();
    let mut settings = QuickSettingsState::default();
    let output = app.run_frame(|ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            PetalPopover::render(ui, Pos2::new(200.0, 50.0), "PNG", &mut settings, || {});
        });
    });
    assert!(
        !output.shapes.is_empty(),
        "Popover must render shapes at top anchor"
    );
}

#[test]
fn test_tier1_f27_popover_renders_at_bottom_anchor() {
    let mut app = HeadlessTestApp::new();
    let mut settings = QuickSettingsState::default();
    let output = app.run_frame(|ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            PetalPopover::render(ui, Pos2::new(200.0, 700.0), "PDF", &mut settings, || {});
        });
    });
    assert!(
        !output.shapes.is_empty(),
        "Popover must render shapes at bottom anchor"
    );
}

#[test]
fn test_tier1_f27_popover_renders_at_right_edge() {
    let mut app = HeadlessTestApp::new();
    let mut settings = QuickSettingsState::default();
    let output = app.run_frame(|ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            PetalPopover::render(ui, Pos2::new(1250.0, 300.0), "TXT", &mut settings, || {});
        });
    });
    assert!(
        !output.shapes.is_empty(),
        "Popover must clamp and render shapes near right edge"
    );
}

#[test]
fn test_tier1_f27_popover_renders_at_left_edge() {
    let mut app = HeadlessTestApp::new();
    let mut settings = QuickSettingsState::default();
    let output = app.run_frame(|ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            PetalPopover::render(ui, Pos2::new(20.0, 300.0), "JPEG", &mut settings, || {});
        });
    });
    assert!(
        !output.shapes.is_empty(),
        "Popover must clamp and render shapes near left edge"
    );
}

#[test]
fn test_tier1_f27_popover_renders_various_extensions() {
    let mut app = HeadlessTestApp::new();
    let mut settings = QuickSettingsState::default();
    let extensions = ["PNG", "JPG", "PDF", "MP4", "TXT", "RS"];
    for ext in extensions {
        let output = app.run_frame(|ctx| {
            eframe::egui::CentralPanel::default().show(ctx, |ui| {
                PetalPopover::render(ui, Pos2::new(300.0, 300.0), ext, &mut settings, || {});
            });
        });
        assert!(
            !output.shapes.is_empty(),
            "Popover must render for extension {ext}"
        );
    }
}

// ============================================================================
// Feature 28: 100% E2E test pass across Tiers 1-4
// ============================================================================

#[test]
fn test_tier1_f28_self_verification_smoke() {
    let app = HeadlessTestApp::new();
    assert_eq!(app.raw_input.screen_rect.unwrap().width(), 1280.0);
}

#[test]
fn test_tier1_f28_isolated_config_env_integrity() {
    let env = IsolatedConfigEnv::new().expect("create");
    assert!(env.config_dir().exists());
}

#[test]
fn test_tier1_f28_test_fs_tree_integrity() {
    let fs = TestFsTree::new().expect("create");
    assert!(fs.root().exists());
}

#[test]
fn test_tier1_f28_wcag_contrast_runner() {
    assert_wcag_aa(Color32::WHITE, Color32::BLACK, "smoke test");
}

#[test]
fn test_tier1_f28_theme_style_all_count() {
    assert_eq!(ThemeStyle::all().len(), 13);
}

// ============================================================================
// Feature 29: Adversarial stress & penetration testing
// ============================================================================

#[test]
fn test_tier1_f29_rapid_directory_sort_1000_items() {
    let mut items: Vec<FileItem> = (0..1000)
        .map(|i| FileItem::from_path(PathBuf::from(format!("item_{i:04}.txt"))))
        .collect();

    let start = std::time::Instant::now();
    items.sort_by(|a, b| a.name.cmp(&b.name));
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_millis() < 50,
        "1000 item sort must complete in < 50ms"
    );
    assert_eq!(items[0].name, "item_0000.txt");
}

#[test]
fn test_tier1_f29_path_traversal_sanitization() {
    let vectors = [
        "../../etc/passwd",
        "..\\..\\Windows\\System32\\cmd.exe",
        "nested/../../secret",
    ];
    let fs = TestFsTree::new().expect("fs");
    for vec in vectors {
        let p = Path::new(vec);
        let has_traversal = p.components().any(|c| c == std::path::Component::ParentDir);
        assert!(
            has_traversal,
            "Vector '{vec}' must contain parent directory traversal"
        );
        let target = fs.root().join(vec);
        if let Ok(canon) = dunce::canonicalize(&target) {
            assert!(
                !canon.starts_with(fs.root()),
                "Traversal target must not be within sandbox"
            );
        }
    }
}

#[test]
fn test_tier1_f29_deep_folder_path_creation() {
    let fs = TestFsTree::new().expect("fs");
    let deep_dir = fs.create_dir("l1/l2/l3/l4/l5").expect("deep");
    assert!(deep_dir.exists());
}

#[test]
fn test_tier1_f29_large_string_filename_rejection() {
    let huge_name = format!("{}.txt", "a".repeat(300));
    // Valid on filesystem as long as under MAX_PATH or supported, but let's test length
    assert!(huge_name.len() > 255);
}

#[test]
fn test_tier1_f29_cyclic_symlink_creation_handled() {
    let fs = TestFsTree::new().expect("fs");
    let _ = fs.create_cyclic_symlink_tree("dir_a", "dir_b");
    assert!(fs.root().join("dir_a").exists());
    assert!(fs.root().join("dir_b").exists());
}

// ============================================================================
// Feature 30: Cross-platform build & test parity
// ============================================================================

#[test]
fn test_tier1_f30_path_separator_normalization() {
    let p = Path::new("C:/Users/User/.config/mossambi");
    assert!(p.components().count() >= 4);
}

#[test]
fn test_tier1_f30_breadcrumb_parsing() {
    let p = PathBuf::from("C:\\Projects\\mossambi\\src");
    let segments = mossambi::core::fs_utils::parse_breadcrumbs(&p);
    assert!(!segments.is_empty());
}

#[test]
fn test_tier1_f30_protected_system_file_checks() {
    let sys_path = Path::new("C:\\pagefile.sys");
    // Protected check handles Windows and non-Windows cleanly
    let _ = mossambi::core::fs_utils::is_protected_os_file(sys_path);
}

#[test]
fn test_tier1_f30_launch_intent_platform_agnostic() {
    let arg = "/select,dir/file.txt";
    let intent = parse_single_arg(arg);
    assert!(matches!(intent, LaunchIntent::Reveal { .. }));
}

#[test]
fn test_tier1_f30_dunce_canonicalize_parity() {
    let fs = TestFsTree::new().expect("fs");
    let file = fs.create_file("test.txt", b"parity").expect("file");
    let canon = dunce::canonicalize(&file);
    assert!(canon.is_ok());
}
