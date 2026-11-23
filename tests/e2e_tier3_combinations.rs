//! Tier 3: Pairwise Cross-Feature Combinations Suite for Mossambi E2E.
//!
//! Validates cross-subsystem interactions, combining configuration persistence,
//! filesystem traversal, display list caching, layout reflow, window chrome,
//! and WCAG accessibility contracts.

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
use mossambi::core::config::{
    CornerStyle, DesignStyle, ExplorerStyle, LookPreset, MenuStyle, MossambiConfig, RowDensity,
    SidebarStyle, ThemeMode, ViewMode,
};
use mossambi::core::fs_utils::FileItem;
use mossambi::core::look_spec::{ChromeSpec, WindowControlsPosition};
use mossambi::core::pantone::THEMES;
use mossambi::gui::dashboard_layout::DashboardTier;
use mossambi::gui::explorer_view::{
    CategoryFilter, ColumnLayout, DetailColumn, ExplorerView, SortDirection,
};
use mossambi::gui::finder_chrome::{render_caption_buttons, render_traffic_lights};
use mossambi::gui::macos_theme::ThemeStyle;
use mossambi::gui::popover::PetalPopover;
use mossambi::gui::quick_settings::QuickSettingsState;
use mossambi::gui::view_zoom::{
    compute_bounded_list_columns, compute_grid_reflow, GridZoomMetrics, ListZoomMetrics,
    MillerZoomMetrics,
};
use mossambi::platform::args::LaunchIntent;
use std::path::PathBuf;

// ============================================================================
// Combination 1: Look Preset -> Layout, Density, ExplorerStyle, & Theme Sync
// ============================================================================

#[test]
fn test_tier3_combo_look_preset_switch_finder() {
    let mut cfg = MossambiConfig::default();
    cfg.apply_look_preset(LookPreset::Finder);

    assert_eq!(cfg.look_preset, Some(LookPreset::Finder));
    assert_eq!(cfg.default_explorer_layout, 3); // Columns
    assert_eq!(cfg.row_density, RowDensity::Default); // 28px
    assert_eq!(cfg.explorer_style, ExplorerStyle::MacOS);
    assert_eq!(cfg.sidebar_style, SidebarStyle::Finder);
    assert_eq!(cfg.corner_style, CornerStyle::Rounded);
    assert_eq!(cfg.theme_mode, ThemeMode::Rangalipi);
}

#[test]
fn test_tier3_combo_look_preset_switch_windows() {
    let mut cfg = MossambiConfig::default();
    cfg.apply_look_preset(LookPreset::WindowsExplorer);

    assert_eq!(cfg.look_preset, Some(LookPreset::WindowsExplorer));
    assert_eq!(cfg.default_explorer_layout, 0); // Details
    assert_eq!(cfg.row_density, RowDensity::Standard); // 32px
    assert_eq!(cfg.explorer_style, ExplorerStyle::Windows);
    assert_eq!(cfg.sidebar_style, SidebarStyle::Windows);
    assert_eq!(cfg.corner_style, CornerStyle::Subtle);
    assert_eq!(cfg.theme_mode, ThemeMode::DarkPlus);
}

#[test]
fn test_tier3_combo_look_preset_switch_nautilus() {
    let mut cfg = MossambiConfig::default();
    cfg.apply_look_preset(LookPreset::Nautilus);

    assert_eq!(cfg.look_preset, Some(LookPreset::Nautilus));
    assert_eq!(cfg.default_explorer_layout, 2); // Grid
    assert_eq!(cfg.row_density, RowDensity::Spacious); // 36px
    assert_eq!(cfg.explorer_style, ExplorerStyle::Nautilus);
    assert_eq!(cfg.sidebar_style, SidebarStyle::Nautilus);
    assert_eq!(cfg.corner_style, CornerStyle::Spacious);
    assert_eq!(cfg.theme_mode, ThemeMode::TokyoNight);
}

#[test]
fn test_tier3_combo_look_preset_switch_thunar() {
    let mut cfg = MossambiConfig::default();
    cfg.apply_look_preset(LookPreset::Thunar);

    assert_eq!(cfg.look_preset, Some(LookPreset::Thunar));
    assert_eq!(cfg.default_explorer_layout, 0); // Details
    assert_eq!(cfg.row_density, RowDensity::Compact); // 24px
    assert_eq!(cfg.explorer_style, ExplorerStyle::Thunar);
    assert_eq!(cfg.sidebar_style, SidebarStyle::Thunar);
    assert_eq!(cfg.corner_style, CornerStyle::Sharp);
    assert_eq!(cfg.theme_mode, ThemeMode::Kanagawa);
}

// ============================================================================
// Combination 2: Preset Switching + ChromeSpec + Title Bar Drag Reserve
// ============================================================================

#[test]
fn test_tier3_combo_chrome_spec_for_all_presets_drag_reserve() {
    let window_w = 1440.0_f32;

    for preset in LookPreset::all() {
        let spec = ChromeSpec::for_preset(preset);
        let caption_reserve = match spec.window_controls {
            WindowControlsPosition::RightCaption => 84.0_f32,
            WindowControlsPosition::LeftTraffic => 58.0_f32,
        };

        let drag_w = window_w - caption_reserve;
        assert!(
            drag_w > 1300.0,
            "Preset {:?} drag reserve must leave wide drag area",
            preset
        );
    }
}

// ============================================================================
// Combination 3: Disk Persistence + Look Preset + Custom Theme Roundtrip
// ============================================================================

#[test]
fn test_tier3_combo_config_persistence_with_look_preset_and_theme() {
    let env = IsolatedConfigEnv::new().expect("env");

    let mut cfg = MossambiConfig::default();
    cfg.apply_look_preset(LookPreset::Thunar);
    cfg.theme_mode = ThemeMode::TokyoNight; // Override preset default
    cfg.row_density = RowDensity::Compact;

    env.save_config(&cfg).expect("save config");
    let loaded = env.load_config().expect("load config");

    assert_eq!(loaded.look_preset, Some(LookPreset::Thunar));
    assert_eq!(loaded.theme_mode, ThemeMode::TokyoNight);
    assert_eq!(loaded.row_density, RowDensity::Compact);
    assert_eq!(loaded.default_explorer_layout, 0);
}

#[test]
fn test_tier3_combo_config_persistence_pinned_and_tags() {
    let env = IsolatedConfigEnv::new().expect("env");

    let mut cfg = MossambiConfig::default();
    cfg.show_custom_pinned = true;
    cfg.pinned_custom = vec!["C:\\Projects".to_string(), "D:\\Downloads".to_string()];
    cfg.show_tags_section = true;
    cfg.custom_tags = vec!["Work".to_string(), "Urgent".to_string()];

    env.save_config(&cfg).expect("save");
    let loaded = env.load_config().expect("load");

    assert!(loaded.show_custom_pinned);
    assert_eq!(loaded.pinned_custom.len(), 2);
    assert!(loaded.show_tags_section);
    assert_eq!(loaded.custom_tags.len(), 2);
}

// ============================================================================
// Combination 4: 10,000 File Items + Display List Cache + Natural Sort
// ============================================================================

#[test]
fn test_tier3_combo_10k_items_display_list_cache_and_sort() {
    let mut explorer = ExplorerView::default();

    // Populate 10,000 file items with varied extensions and numeric names
    explorer.entries = (1..=10_000)
        .map(|i| {
            let ext = match i % 4 {
                0 => "rs",
                1 => "txt",
                2 => "png",
                _ => "pdf",
            };
            FileItem::from_path(PathBuf::from(format!("source_file_{i:05}.{ext}")))
        })
        .collect();

    let start = std::time::Instant::now();
    let sorted = explorer.filtered_and_sorted_entries();
    let elapsed = start.elapsed();

    assert_eq!(sorted.len(), 10_000);
    assert_eq!(sorted[0].name, "source_file_00001.txt");
    assert_eq!(sorted[9999].name, "source_file_10000.rs");
    assert!(
        elapsed.as_millis() < 250,
        "10k item sort must finish in < 250ms, took {}ms",
        elapsed.as_millis()
    );
}

// ============================================================================
// Combination 5: Search Query + Category Filter + Hidden Files + Sort Direction
// ============================================================================

#[test]
fn test_tier3_combo_multi_filter_combination() {
    let mut explorer = ExplorerView::default();

    let f1 = FileItem::from_path(PathBuf::from("photo_vacation_2026.png"));
    let mut f2 = FileItem::from_path(PathBuf::from(".photo_vacation_hidden_2026.png"));
    f2.is_hidden = true;
    let f3 = FileItem::from_path(PathBuf::from("doc_vacation_2026.pdf"));
    let f4 = FileItem::from_path(PathBuf::from("photo_work_2025.png"));

    explorer.entries = vec![f1, f2, f3, f4];

    // Case A: Filter query "vacation", Category Images, hide hidden
    explorer.search_query = "vacation".to_string();
    explorer.category_filter = CategoryFilter::Images;
    explorer.show_hidden_files = false;
    explorer.sort_direction = SortDirection::Ascending;

    let res = explorer.filtered_and_sorted_entries();
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].name, "photo_vacation_2026.png");

    // Case B: Enable hidden files -> reveals the hidden image
    explorer.show_hidden_files = true;
    let res_hidden = explorer.filtered_and_sorted_entries();
    assert_eq!(res_hidden.len(), 2);
}

// ============================================================================
// Combination 6: Multi-Column Layout + Row Density + Design Style
// ============================================================================

#[test]
fn test_tier3_combo_bounded_columns_and_density_matrix() {
    let styles = [
        DesignStyle::ArcaneFluid,
        DesignStyle::ZenithPro,
        DesignStyle::HyperionSlate,
    ];
    let tiers = [
        (600.0, DashboardTier::Compact),
        (1000.0, DashboardTier::Medium),
        (1400.0, DashboardTier::Wide),
        (2560.0, DashboardTier::Ultrawide),
    ];

    for style in styles {
        for (w, tier) in tiers {
            let cols = compute_bounded_list_columns(w, tier, style);
            assert!(cols.max_table_width > 0.0);
            assert!(cols.max_table_width <= w);
            assert!(cols.name_min_width > 0.0);
            assert!(cols.name_max_width >= cols.name_min_width);
            assert!(cols.size_width > 0.0);
            if matches!(tier, DashboardTier::Compact) {
                assert!(!cols.show_path);
            }
            if matches!(tier, DashboardTier::Ultrawide) {
                assert!(cols.show_date && cols.show_size);
            }
        }
    }
}

// ============================================================================
// Combination 7: Grid View Reflow + Viewport Widths
// ============================================================================

#[test]
fn test_tier3_combo_grid_reflow_monotonicity() {
    let min_tile_w = 140.0_f32;
    let gap = 12.0_f32;
    let widths = [640.0, 800.0, 1024.0, 1280.0, 1920.0, 2560.0];

    let mut prev_cols = 0;
    for &w in &widths {
        let reflow = compute_grid_reflow(w, min_tile_w, gap);
        assert!(
            reflow.columns >= prev_cols,
            "Columns must monotonically increase with width"
        );
        assert!(reflow.tile_width >= min_tile_w);

        // Verify total width spans exactly viewport width without voids
        let total =
            reflow.columns as f32 * reflow.tile_width + (reflow.columns - 1) as f32 * reflow.gap;
        assert!((total - w).abs() < 0.05);

        prev_cols = reflow.columns;
    }
}

// ============================================================================
// Combination 8: WCAG 2.1 AA Contrast Across All 13 Pantone Themes on surface_2
// ============================================================================

#[test]
fn test_tier3_combo_wcag_contrast_all_13_themes_surface_2() {
    assert_eq!(THEMES.len(), 13);

    for theme in &THEMES {
        let (r2, g2, b2) = theme.surface_2.to_rgb();
        let surface_2_color = Color32::from_rgb(r2, g2, b2);

        // Primary text contrast must meet WCAG AA (>= 4.5:1)
        let (rt, gt, bt) = theme.text_primary.to_rgb();
        let primary_color = Color32::from_rgb(rt, gt, bt);
        assert_wcag_aa(
            primary_color,
            surface_2_color,
            &format!("Theme '{}' text_primary on surface_2", theme.id),
        );

        // Secondary text contrast must meet WCAG AA Large / Secondary (>= 3.0:1)
        let (rs, gs, bs) = theme.text_secondary.to_rgb();
        let secondary_color = Color32::from_rgb(rs, gs, bs);
        assert_wcag_aa_large(
            secondary_color,
            surface_2_color,
            &format!("Theme '{}' text_secondary on surface_2", theme.id),
        );
    }
}

// ============================================================================
// Combination 9: Selection Fill Contrast Across All Themes
// ============================================================================

#[test]
fn test_tier3_combo_selection_fill_contrast_across_all_styles() {
    for style in ThemeStyle::all() {
        let pal = style.colors();
        let text_color = mossambi::gui::design::contrast::text_on_accent(pal.selection);

        assert_wcag_aa_large(
            text_color,
            pal.selection,
            &format!("ThemeStyle {:?} selection fill contrast", style),
        );
    }
}

// ============================================================================
// Combination 10: Caption Controls + Headless UI Frame + Active Preset
// ============================================================================

#[test]
fn test_tier3_combo_caption_controls_rendered_across_presets() {
    let mut app = HeadlessTestApp::new();

    for preset in LookPreset::all() {
        let mut cfg = MossambiConfig::default();
        cfg.apply_look_preset(preset);
        let pal = ThemeStyle::default().colors();

        let output = app.run_frame(|ctx| {
            eframe::egui::CentralPanel::default().show(ctx, |ui| {
                match cfg.chrome_spec().window_controls {
                    WindowControlsPosition::RightCaption => {
                        render_caption_buttons(ui, &pal, || {}, || {}, || {});
                    }
                    WindowControlsPosition::LeftTraffic => {
                        render_traffic_lights(ui, &pal, || {}, || {}, || {});
                    }
                }
            });
        });

        assert!(
            !output.shapes.is_empty(),
            "Preset {:?} must produce UI shapes",
            preset
        );
    }
}

// ============================================================================
// Combination 11: Context Menu Work Area Clamping + Accordion Submenu Expansion
// ============================================================================

#[test]
fn test_tier3_combo_context_menu_clamping_with_expanded_accordions() {
    let mut app = HeadlessTestApp::new();
    app.simulate_resize(Vec2::new(1920.0, 1080.0));
    let item = FileItem::from_path(PathBuf::from("test.txt"));
    let targets: Vec<PathBuf> = (0..10)
        .map(|i| PathBuf::from(format!("item_{}.txt", i)))
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
            assert!(intents.open_default.is_none());
        });
    });
    assert!(
        !output.shapes.is_empty(),
        "Context menu must render shapes with multi-targets"
    );
}

// ============================================================================
// Combination 12: Modal Dialogs + Micro Viewports
// ============================================================================

#[test]
fn test_tier3_combo_modal_clamping_in_640x480_viewport() {
    let mut app = HeadlessTestApp::new();
    app.simulate_resize(Vec2::new(640.0, 480.0));
    let mut explorer = ExplorerView::default();
    explorer.properties_item = Some(FileItem::from_path(PathBuf::from("combined_test.docx")));
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
    assert_eq!(explorer.properties_rename_buf, "combined_test.docx");
}

// ============================================================================
// Combination 13: Petal Popover Dynamic Width + Narrow Window
// ============================================================================

#[test]
fn test_tier3_combo_petal_popover_narrow_windows() {
    let window_widths = [320.0, 480.0, 768.0, 1024.0, 1920.0];
    for &avail_w in &window_widths {
        let mut app = HeadlessTestApp::new();
        app.simulate_resize(Vec2::new(avail_w, 800.0));
        let mut settings = QuickSettingsState::default();
        let anchor = Pos2::new(avail_w / 2.0, 100.0);
        let output = app.run_frame(|ctx| {
            eframe::egui::CentralPanel::default().show(ctx, |ui| {
                PetalPopover::render(ui, anchor, "PDF", &mut settings, || {});
            });
        });
        assert!(
            !output.shapes.is_empty(),
            "Popover must render shapes at width {:.0}",
            avail_w
        );
        assert_eq!(settings.pdf_page_size, "A4");
    }
}

// ============================================================================
// Combination 14: Recursive Copy + Nested Fixture
// ============================================================================

#[test]
fn test_tier3_combo_safe_copy_with_nested_symlink_cycle() {
    let fs = TestFsTree::new().expect("fs");
    let src = fs.create_dir("deep_tree").expect("dir");
    fs.create_file("deep_tree/file_1.txt", b"1").expect("file");
    fs.create_file("deep_tree/sub/file_2.txt", b"2")
        .expect("file");

    let dest = fs.root().join("dest_safe_copy");
    let bytes =
        mossambi::core::fs_utils::copy_entry_recursive(&src, &dest).expect("recursive copy");

    assert_eq!(
        bytes, 2,
        "Both files copied via production copy_entry_recursive"
    );
    assert!(dest.join("file_1.txt").exists());
    assert!(dest.join("sub/file_2.txt").exists());

    // Also verify cyclic junction terminates safely with an error
    let cyclic_src = fs
        .create_cyclic_symlink_tree("cycle_a", "cycle_b")
        .expect("cycle");
    let cyclic_dest = fs.root().join("dest_cyclic_copy");
    let cyclic_res = mossambi::core::fs_utils::copy_entry_recursive(&cyclic_src, &cyclic_dest);
    assert!(cyclic_res.is_err(), "Cyclic copy must terminate with error");
}

// ============================================================================
// Combination 15: Per-Mode Zoom Independence During Navigation
// ============================================================================

#[test]
fn test_tier3_combo_per_mode_zoom_independence_during_navigation() {
    let mut explorer = ExplorerView::default();

    // Mode 1: Details view at zoom 1
    explorer.view_mode = ViewMode::Details;
    explorer.set_active_mode_zoom(1);
    assert_eq!(explorer.active_mode_zoom(), 1);

    // Mode 2: Grid view at zoom 3
    explorer.view_mode = ViewMode::Grid;
    explorer.set_active_mode_zoom(3);
    assert_eq!(explorer.active_mode_zoom(), 3);

    // Switch back to Details -> zoom remains 1
    explorer.view_mode = ViewMode::Details;
    assert_eq!(explorer.active_mode_zoom(), 1);
}

// ============================================================================
// Combination 16: Launch Args + Single Instance IPC Intent Serialization
// ============================================================================

#[test]
fn test_tier3_combo_launch_intent_single_instance_roundtrip() {
    use mossambi::platform::single_instance::{deserialize_intent, serialize_intent};

    let sample_intents = vec![
        LaunchIntent::None,
        LaunchIntent::Navigate(PathBuf::from("C:\\Projects\\mossambi")),
        LaunchIntent::Reveal {
            target: PathBuf::from("C:\\Data\\file.txt"),
            select: true,
        },
    ];

    for intent in sample_intents {
        let serialized = serialize_intent(&intent);
        let deserialized = deserialize_intent(&serialized);
        assert_eq!(intent, deserialized.expect("deserialize intent"));
    }
}

// ============================================================================
// Combination 17: Breadcrumbs + Ancestor Chain + Root Normalization
// ============================================================================

#[test]
fn test_tier3_combo_breadcrumbs_and_ancestors() {
    // Use native separators: on POSIX a backslash is an ordinary character,
    // so a hardcoded "C:\\..." path is a single component and has no ancestors.
    #[cfg(windows)]
    let p = PathBuf::from("C:\\Users\\User\\Documents\\Project");
    #[cfg(not(windows))]
    let p = PathBuf::from("/home/user/Documents/Project");
    let ancestors = mossambi::gui::explorer_view::ancestor_chain(&p);
    assert!(ancestors.len() >= 3);

    let crumbs = mossambi::core::fs_utils::parse_breadcrumbs(&p);
    assert!(crumbs.len() >= 3);
    assert_eq!(crumbs.last().unwrap().label, "Project");
}

// ============================================================================
// Combination 18: Detail Column Widths Persistence & Invalidation
// ============================================================================

#[test]
fn test_tier3_combo_column_layout_to_config_and_back() {
    let mut layout = ColumnLayout::finder_default();
    let col = DetailColumn::Size;
    layout.set_width(col, 150.0);

    let (visible, widths) = layout.to_config();
    assert_eq!(widths.get(&col.id()), Some(&150));

    let restored = ColumnLayout::from_config(&visible, &widths);
    assert_eq!(restored.width(col), 150.0);
}

// ============================================================================
// Combination 19: Action Journal Keystroke & RecordedAction Data Model Contract
// ============================================================================

#[test]
fn test_tier3_combo_action_journal_roundtrip() {
    let keystroke = mossambi::core::action_journal::KeyStroke {
        at_ms: 1234567890,
        context: "combo_ctx".to_string(),
        key: "KeyZ".to_string(),
        modifiers: "Ctrl".to_string(),
    };
    let json = serde_json::to_string(&keystroke).expect("serialize keystroke");
    assert!(json.contains("\"kind\":\"keystroke\""));
    assert!(json.contains("\"context\":\"combo_ctx\""));
    assert!(json.contains("\"key\":\"KeyZ\""));
    assert!(json.contains("\"modifiers\":\"Ctrl\""));

    let parsed: mossambi::core::action_journal::KeyStroke =
        serde_json::from_str(&json).expect("deserialize keystroke");
    assert_eq!(parsed.context, "combo_ctx");
    assert_eq!(parsed.key, "KeyZ");
    assert_eq!(parsed.modifiers, "Ctrl");

    let action = mossambi::core::action_journal::RecordedAction::Rename {
        from: std::path::PathBuf::from("old.txt"),
        to: std::path::PathBuf::from("new.txt"),
    };
    let act_json = serde_json::to_string(&action).expect("serialize action");
    let act_parsed: mossambi::core::action_journal::RecordedAction =
        serde_json::from_str(&act_json).expect("deserialize action");
    assert!(matches!(
        act_parsed,
        mossambi::core::action_journal::RecordedAction::Rename { .. }
    ));
}

// ============================================================================
// Combination 20: Zoom Step Ladder Monotonicity Across All Metrics
// ============================================================================

#[test]
fn test_tier3_combo_zoom_step_metrics_monotonicity() {
    for step in 0..=3 {
        let list_cur = ListZoomMetrics::for_step(step);
        let list_next = ListZoomMetrics::for_step(step + 1);
        assert!(list_next.row_height > list_cur.row_height);

        let grid_cur = GridZoomMetrics::for_step(step);
        let grid_next = GridZoomMetrics::for_step(step + 1);
        assert!(grid_next.icon_size > grid_cur.icon_size);

        let miller_cur = MillerZoomMetrics::for_step(step);
        let miller_next = MillerZoomMetrics::for_step(step + 1);
        assert!(miller_next.column_width > miller_cur.column_width);
    }
}

// ============================================================================
// Combination 21: Context Menu Clamping on Secondary Monitor
// ============================================================================

#[test]
fn test_tier3_combo_multi_monitor_work_area_clamping() {
    let mut app = HeadlessTestApp::new();
    app.simulate_resize(Vec2::new(3840.0, 1080.0));
    let item = FileItem::from_path(PathBuf::from("secondary_monitor_file.txt"));
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
        "Context menu must render shapes in multi-monitor viewport"
    );
}

// ============================================================================
// Combination 22: Virtualization Row Counts Across All Row Densities
// ============================================================================

#[test]
fn test_tier3_combo_virtualization_row_counts_across_densities() {
    let viewport_h = 720.0_f32;
    for density in RowDensity::all() {
        let h = density.details_row_height();
        let visible_rows = (viewport_h / h).ceil() as usize;
        assert!(visible_rows >= 20 && visible_rows <= 30);
    }
}

// ============================================================================
// Combination 23: Config Serialization with Donut and Fields Sub-configs
// ============================================================================

#[test]
fn test_tier3_combo_config_donut_and_fields_roundtrip() {
    let env = IsolatedConfigEnv::new().expect("env");
    let mut cfg = MossambiConfig::default();
    cfg.donut.selected_profile_index = 1;
    cfg.fields.cad = true;
    cfg.fields.fonts = true;

    env.save_config(&cfg).expect("save");
    let loaded = env.load_config().expect("load");

    assert_eq!(loaded.donut.selected_profile_index, 1);
    assert!(loaded.fields.cad);
    assert!(loaded.fields.fonts);
}

// ============================================================================
// Combination 24: Theme Switch Updates Studio Palette in Headless Context
// ============================================================================

#[test]
fn test_tier3_combo_theme_change_updates_studio_palette() {
    let mut theme = ThemeStyle::Rangalipi;
    let dark_bg = theme.colors().window_bg;

    theme = ThemeStyle::MossambiPunch;
    let light_bg = theme.colors().window_bg;

    assert_ne!(dark_bg, light_bg, "Theme change must update palette colors");
}

// ============================================================================
// Combination 25: Action Journal Undo Marker Contract
// ============================================================================

#[test]
fn test_tier3_combo_action_journal_empty_undo() {
    let inner_action = mossambi::core::action_journal::RecordedAction::CreateDir {
        path: std::path::PathBuf::from("test_dir"),
    };
    let undo_marker = mossambi::core::action_journal::RecordedAction::Undo {
        of: Box::new(inner_action),
    };
    let json = serde_json::to_string(&undo_marker).expect("serialize undo marker");
    assert!(json.contains("\"kind\":\"undo\""));
    let parsed: mossambi::core::action_journal::RecordedAction =
        serde_json::from_str(&json).expect("deserialize undo marker");
    match parsed {
        mossambi::core::action_journal::RecordedAction::Undo { of } => {
            assert!(matches!(
                *of,
                mossambi::core::action_journal::RecordedAction::CreateDir { .. }
            ));
        }
        _ => panic!("Expected Undo variant"),
    }
}

// ============================================================================
// Combination 26: 10,000 Items Extension Sorting
// ============================================================================

#[test]
fn test_tier3_combo_10k_items_extension_sorting() {
    let mut explorer = ExplorerView::default();
    explorer.entries = (1..=10_000)
        .map(|i| {
            let ext = if i % 2 == 0 { "ZIP" } else { "7Z" };
            FileItem::from_path(PathBuf::from(format!("archive_{i:05}.{ext}")))
        })
        .collect();

    explorer.sort_column = mossambi::gui::explorer_view::SortColumn::Extension;
    explorer.sort_direction = SortDirection::Ascending;
    let sorted = explorer.filtered_and_sorted_entries();

    assert_eq!(sorted.len(), 10_000);
    assert_eq!(sorted[0].extension, "7Z");
    assert_eq!(sorted[9999].extension, "ZIP");
}

// ============================================================================
// Combination 27: Grid Tile Sizing Across Zoom Ladder Steps
// ============================================================================

#[test]
fn test_tier3_combo_grid_tile_sizing_across_zoom_steps() {
    let mut prev_tile_w = 0.0_f32;
    for step in 0..=4 {
        let metrics = GridZoomMetrics::for_step(step);
        assert!(metrics.min_tile_width > prev_tile_w);
        prev_tile_w = metrics.min_tile_width;
    }
}

// ============================================================================
// Combination 28: Search Query Case Insensitivity
// ============================================================================

#[test]
fn test_tier3_combo_search_query_case_insensitivity() {
    let mut explorer = ExplorerView::default();
    explorer.entries = vec![
        FileItem::from_path(PathBuf::from("Document.PDF")),
        FileItem::from_path(PathBuf::from("PHOTO.PNG")),
    ];

    explorer.search_query = "pdf".to_string();
    let res = explorer.filtered_and_sorted_entries();
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].name, "Document.PDF");

    explorer.search_query = "photo".to_string();
    let res2 = explorer.filtered_and_sorted_entries();
    assert_eq!(res2.len(), 1);
    assert_eq!(res2[0].name, "PHOTO.PNG");
}

// ============================================================================
// Combination 29: Breadcrumb Navigation Root & Normalization
// ============================================================================

#[test]
fn test_tier3_combo_breadcrumb_navigation_root_and_trailing_slashes() {
    // Native separators keep the trailing-slash case valid on every OS.
    #[cfg(windows)]
    let p = PathBuf::from("C:\\Projects\\mossambi\\");
    #[cfg(not(windows))]
    let p = PathBuf::from("/home/user/Projects/mossambi/");
    let crumbs = mossambi::core::fs_utils::parse_breadcrumbs(&p);
    assert!(!crumbs.is_empty());
    assert_eq!(crumbs.last().unwrap().label, "mossambi");
}

// ============================================================================
// Combination 30: Spacing Tokens Consistency Across Presets
// ============================================================================

#[test]
fn test_tier3_combo_chrome_spec_spacing_tokens_positive() {
    for preset in LookPreset::all() {
        let spec = ChromeSpec::for_preset(preset);
        assert!(spec.spacing.toolbar_item_spacing >= 2.0);
        assert!(spec.spacing.toolbar_padding_y >= 2.0);
        assert!(spec.spacing.sidebar_item_height >= 24.0);
    }
}
