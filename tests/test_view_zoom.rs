//! Tests for Phase D3: View Modes Parity, Independent Per-Mode Zoom, Reflow, and Bounded Columns.
//!
//! Strictly pure ASCII content.

use mossambi::core::config::{DesignStyle, MossambiConfig, ViewMode};
use mossambi::gui::dashboard_layout::DashboardTier;
use mossambi::gui::view_zoom::{
    clamp_zoom, compute_bounded_list_columns, compute_grid_reflow, step_zoom, GridZoomMetrics,
    ListZoomMetrics, MillerZoomMetrics, DEFAULT_ZOOM_STEP,
};

#[test]
fn test_zoom_clamping_and_stepping() {
    assert_eq!(clamp_zoom(-5), 0);
    assert_eq!(clamp_zoom(0), 0);
    assert_eq!(clamp_zoom(2), 2);
    assert_eq!(clamp_zoom(4), 4);
    assert_eq!(clamp_zoom(10), 4);

    assert_eq!(step_zoom(0, -1), 0);
    assert_eq!(step_zoom(0, 1), 1);
    assert_eq!(step_zoom(2, -1), 1);
    assert_eq!(step_zoom(2, 1), 3);
    assert_eq!(step_zoom(4, 1), 4);
    assert_eq!(step_zoom(4, -1), 3);

    assert_eq!(DEFAULT_ZOOM_STEP, 2);
}

#[test]
fn test_per_mode_zoom_metrics_monotonicity() {
    // List zoom metrics (row height, font size, icon size)
    let mut prev_row_h = 0.0_f32;
    let mut prev_font_sz = 0.0_f32;
    let mut prev_icon_sz = 0.0_f32;
    for step in 0..=4 {
        let m = ListZoomMetrics::for_step(step);
        assert_eq!(m.step, step);
        assert!(m.row_height > prev_row_h);
        assert!(m.font_size > prev_font_sz);
        assert!(m.icon_size > prev_icon_sz);
        prev_row_h = m.row_height;
        prev_font_sz = m.font_size;
        prev_icon_sz = m.icon_size;
    }

    // Grid zoom metrics (icon size, min tile width, font size)
    let mut prev_grid_icon = 0.0_f32;
    let mut prev_tile_w = 0.0_f32;
    let mut prev_grid_font = 0.0_f32;
    for step in 0..=4 {
        let m = GridZoomMetrics::for_step(step);
        assert_eq!(m.step, step);
        assert!(m.icon_size > prev_grid_icon);
        assert!(m.min_tile_width > prev_tile_w);
        assert!(m.font_size > prev_grid_font);
        prev_grid_icon = m.icon_size;
        prev_tile_w = m.min_tile_width;
        prev_grid_font = m.font_size;
    }

    // Miller zoom metrics (column width, font size)
    let mut prev_col_w = 0.0_f32;
    let mut prev_miller_font = 0.0_f32;
    for step in 0..=4 {
        let m = MillerZoomMetrics::for_step(step);
        assert_eq!(m.step, step);
        assert!(m.column_width > prev_col_w);
        assert!(m.font_size >= prev_miller_font);
        prev_col_w = m.column_width;
        prev_miller_font = m.font_size;
    }
}

#[test]
fn test_per_mode_zoom_independence() {
    let mut cfg = MossambiConfig::default();

    // Defaults should all be 2
    assert_eq!(cfg.zoom_for_mode(ViewMode::Details), 2);
    assert_eq!(cfg.zoom_for_mode(ViewMode::Grid), 2);
    assert_eq!(cfg.zoom_for_mode(ViewMode::Columns), 2);

    // Zoom List only
    cfg.set_zoom_for_mode(ViewMode::Details, 0);
    assert_eq!(cfg.zoom_for_mode(ViewMode::Details), 0);
    assert_eq!(cfg.zoom_for_mode(ViewMode::Grid), 2);
    assert_eq!(cfg.zoom_for_mode(ViewMode::Columns), 2);

    // Zoom Grid only
    cfg.set_zoom_for_mode(ViewMode::Grid, 4);
    assert_eq!(cfg.zoom_for_mode(ViewMode::Details), 0);
    assert_eq!(cfg.zoom_for_mode(ViewMode::Grid), 4);
    assert_eq!(cfg.zoom_for_mode(ViewMode::Columns), 2);

    // Zoom Miller only
    cfg.set_zoom_for_mode(ViewMode::Columns, 1);
    assert_eq!(cfg.zoom_for_mode(ViewMode::Details), 0);
    assert_eq!(cfg.zoom_for_mode(ViewMode::Grid), 4);
    assert_eq!(cfg.zoom_for_mode(ViewMode::Columns), 1);

    // Verify clamping
    cfg.set_zoom_for_mode(ViewMode::Details, 10);
    assert_eq!(cfg.zoom_for_mode(ViewMode::Details), 4);
}

#[test]
fn test_config_zoom_persistence_serde_roundtrip() {
    let cfg = MossambiConfig {
        list_zoom: 1,
        grid_zoom: 3,
        miller_zoom: 4,
        explorer_zoom: 3,
        ..Default::default()
    };

    let json = serde_json::to_string(&cfg).expect("serialize config");
    let loaded: MossambiConfig = serde_json::from_str(&json).expect("deserialize config");

    assert_eq!(loaded.list_zoom, 1);
    assert_eq!(loaded.grid_zoom, 3);
    assert_eq!(loaded.miller_zoom, 4);
    assert_eq!(loaded.explorer_zoom, 3);

    assert_eq!(loaded.zoom_for_mode(ViewMode::Details), 1);
    assert_eq!(loaded.zoom_for_mode(ViewMode::Grid), 3);
    assert_eq!(loaded.zoom_for_mode(ViewMode::Columns), 4);
}

#[test]
fn test_grid_reflow_math_no_voids() {
    let min_tile_w = 152.0_f32;
    let gap = 14.0_f32;

    // Narrow viewport (< min_tile_w + gap)
    let narrow = compute_grid_reflow(100.0, min_tile_w, gap);
    assert_eq!(narrow.columns, 1);
    assert!(narrow.tile_width >= min_tile_w);

    // Viewport 800px: exactly 4 columns
    let r800 = compute_grid_reflow(800.0, min_tile_w, gap);
    assert_eq!(r800.columns, 4);
    let total_span_800 =
        r800.columns as f32 * r800.tile_width + (r800.columns - 1) as f32 * r800.gap;
    assert!((total_span_800 - 800.0).abs() < 0.01);

    // Viewport 1440px
    let r1440 = compute_grid_reflow(1440.0, min_tile_w, gap);
    assert!(r1440.columns >= 8);
    let total_span_1440 =
        r1440.columns as f32 * r1440.tile_width + (r1440.columns - 1) as f32 * r1440.gap;
    assert!((total_span_1440 - 1440.0).abs() < 0.01);

    // Viewport 2560px
    let r2560 = compute_grid_reflow(2560.0, min_tile_w, gap);
    assert!(r2560.columns >= 14);
    let total_span_2560 =
        r2560.columns as f32 * r2560.tile_width + (r2560.columns - 1) as f32 * r2560.gap;
    assert!((total_span_2560 - 2560.0).abs() < 0.01);
}

#[test]
fn test_bounded_list_columns_and_density() {
    // Compact (<800px)
    let compact =
        compute_bounded_list_columns(700.0, DashboardTier::Compact, DesignStyle::ArcaneFluid);
    assert!(!compact.show_date);
    assert!(!compact.show_type);
    assert!(compact.show_size);
    assert!(!compact.show_path);
    assert!(!compact.tabular_figures);

    // Medium (800-1199px)
    let medium =
        compute_bounded_list_columns(1000.0, DashboardTier::Medium, DesignStyle::ArcaneFluid);
    assert!(medium.show_date);
    assert!(!medium.show_type);
    assert!(medium.show_size);
    assert!(!medium.show_path);

    // Wide (1200-1599px)
    let wide = compute_bounded_list_columns(1400.0, DashboardTier::Wide, DesignStyle::ArcaneFluid);
    assert!(wide.show_date);
    assert!(wide.show_type);
    assert!(wide.show_size);
    assert!(!wide.show_path);
    assert_eq!(wide.max_table_width, 1200.0);

    // Ultrawide (>=1600px)
    let ultrawide =
        compute_bounded_list_columns(2560.0, DashboardTier::Ultrawide, DesignStyle::ZenithPro);
    assert!(ultrawide.show_date);
    assert!(ultrawide.show_type);
    assert!(ultrawide.show_size);
    assert!(ultrawide.show_path);
    assert_eq!(ultrawide.max_table_width, 1440.0);
    assert!(ultrawide.tabular_figures);

    // Zenith Pro tabular figures check
    let zenith =
        compute_bounded_list_columns(1000.0, DashboardTier::Medium, DesignStyle::ZenithPro);
    assert!(zenith.tabular_figures);

    let hyperion =
        compute_bounded_list_columns(1000.0, DashboardTier::Medium, DesignStyle::HyperionSlate);
    assert!(!hyperion.tabular_figures);
}

#[test]
fn test_view_mode_all_and_labels() {
    let all = ViewMode::all();
    assert_eq!(all.len(), 3);
    assert_eq!(all[0], ViewMode::Details);
    assert_eq!(all[1], ViewMode::Grid);
    assert_eq!(all[2], ViewMode::Columns);

    assert_eq!(ViewMode::default(), ViewMode::Details);
    assert_eq!(ViewMode::Details.label(), "Details / List");
    assert_eq!(ViewMode::Grid.label(), "Icons / Grid");
    assert_eq!(ViewMode::Columns.label(), "Miller Columns");

    // Serde roundtrip for ViewMode
    let serialized = serde_json::to_string(&ViewMode::Grid).expect("serialize ViewMode");
    assert_eq!(serialized, "\"grid\"");
    let deserialized: ViewMode = serde_json::from_str(&serialized).expect("deserialize ViewMode");
    assert_eq!(deserialized, ViewMode::Grid);
}
