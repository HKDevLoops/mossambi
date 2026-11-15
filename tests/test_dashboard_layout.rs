//! Tests for Responsive Home / Dashboard Layout & Max-Width Container (Phase D2)
//!
//! Verifies:
//! - 4 responsive breakpoint tiers (<800, 800-1199, 1200-1599, >=1600)
//! - Centered max-width (1440px) content container calculation
//! - Bento grid columns per tier and DesignStyle preset
//! - Zenith Pro 60/40 cockpit split calculations
//! - Bounded Recent Files table column layout (no 1600px line stretch)
//! - Storage usage ring semantic color thresholds
//! - Dashboard widget slot ordering and config persistence
//!
//! All content is pure ASCII.

use mossambi::core::config::{DashboardWidgetSlot, DesignStyle, MossambiConfig};
use mossambi::gui::dashboard_layout::{
    bento_column_count, compute_card_grid, compute_container_metrics, compute_drive_card_width,
    compute_recent_table, compute_zenith_cockpit, format_bytes_ascii, storage_usage_color,
    tier_for_width, DashboardTier,
};
use mossambi::gui::macos_theme::ThemeStyle;

#[test]
fn test_tier_breakpoints_and_labels() {
    assert_eq!(tier_for_width(500.0), DashboardTier::Compact);
    assert_eq!(tier_for_width(799.0), DashboardTier::Compact);
    assert_eq!(tier_for_width(800.0), DashboardTier::Medium);
    assert_eq!(tier_for_width(1199.0), DashboardTier::Medium);
    assert_eq!(tier_for_width(1200.0), DashboardTier::Wide);
    assert_eq!(tier_for_width(1599.0), DashboardTier::Wide);
    assert_eq!(tier_for_width(1600.0), DashboardTier::Ultrawide);
    assert_eq!(tier_for_width(3840.0), DashboardTier::Ultrawide);

    assert_eq!(DashboardTier::Compact.label(), "Compact (<800px)");
    assert_eq!(DashboardTier::Medium.label(), "Medium (800-1199px)");
    assert_eq!(DashboardTier::Wide.label(), "Wide (1200-1599px)");
    assert_eq!(DashboardTier::Ultrawide.label(), "Ultrawide (>=1600px)");
}

#[test]
fn test_max_width_centered_container() {
    // Compact (680px): outer 24px gutter -> 632px content, 24px margin
    let c = compute_container_metrics(680.0, 1440.0);
    assert_eq!(c.tier, DashboardTier::Compact);
    assert_eq!(c.content_width, 632.0);
    assert_eq!(c.margin_x, 24.0);

    // Medium (1024px): outer 24px gutter -> 976px content, 24px margin
    let m = compute_container_metrics(1024.0, 1440.0);
    assert_eq!(m.tier, DashboardTier::Medium);
    assert_eq!(m.content_width, 976.0);
    assert_eq!(m.margin_x, 24.0);

    // Wide (1300px): outer 24px gutter -> 1252px content, 24px margin
    let w = compute_container_metrics(1300.0, 1440.0);
    assert_eq!(w.tier, DashboardTier::Wide);
    assert_eq!(w.content_width, 1252.0);
    assert_eq!(w.margin_x, 24.0);

    // Ultrawide 1080p maximized (1920px): capped at 1440px, centered with 240px margin on each side
    let u1080 = compute_container_metrics(1920.0, 1440.0);
    assert_eq!(u1080.tier, DashboardTier::Ultrawide);
    assert_eq!(u1080.content_width, 1440.0);
    assert_eq!(u1080.margin_x, 240.0);

    // 1440p / ultrawide display (2560px): capped at 1440px, centered with 560px margin on each side
    let u1440 = compute_container_metrics(2560.0, 1440.0);
    assert_eq!(u1440.tier, DashboardTier::Ultrawide);
    assert_eq!(u1440.content_width, 1440.0);
    assert_eq!(u1440.margin_x, 560.0);
}

#[test]
fn test_bento_columns_for_all_tiers_and_styles() {
    // ArcaneFluid: 1 -> 2 -> 3 -> 4
    assert_eq!(
        bento_column_count(DashboardTier::Compact, DesignStyle::ArcaneFluid),
        1
    );
    assert_eq!(
        bento_column_count(DashboardTier::Medium, DesignStyle::ArcaneFluid),
        2
    );
    assert_eq!(
        bento_column_count(DashboardTier::Wide, DesignStyle::ArcaneFluid),
        3
    );
    assert_eq!(
        bento_column_count(DashboardTier::Ultrawide, DesignStyle::ArcaneFluid),
        4
    );

    // ZenithPro: 1 -> 2 -> 2 (cockpit) -> 2 (cockpit)
    assert_eq!(
        bento_column_count(DashboardTier::Compact, DesignStyle::ZenithPro),
        1
    );
    assert_eq!(
        bento_column_count(DashboardTier::Medium, DesignStyle::ZenithPro),
        2
    );
    assert_eq!(
        bento_column_count(DashboardTier::Wide, DesignStyle::ZenithPro),
        2
    );
    assert_eq!(
        bento_column_count(DashboardTier::Ultrawide, DesignStyle::ZenithPro),
        2
    );

    // HyperionSlate: 1 -> 2 -> 3 -> 3
    assert_eq!(
        bento_column_count(DashboardTier::Compact, DesignStyle::HyperionSlate),
        1
    );
    assert_eq!(
        bento_column_count(DashboardTier::Medium, DesignStyle::HyperionSlate),
        2
    );
    assert_eq!(
        bento_column_count(DashboardTier::Wide, DesignStyle::HyperionSlate),
        3
    );
    assert_eq!(
        bento_column_count(DashboardTier::Ultrawide, DesignStyle::HyperionSlate),
        3
    );
}

#[test]
fn test_zenith_cockpit_split_math() {
    // Narrow / medium screen: single pane stacked
    let narrow = compute_zenith_cockpit(750.0, DashboardTier::Compact);
    assert!(!narrow.is_split);
    assert_eq!(narrow.left_width, 750.0);

    let medium = compute_zenith_cockpit(1000.0, DashboardTier::Medium);
    assert!(!medium.is_split);
    assert_eq!(medium.left_width, 1000.0);

    // Wide screen: 60/40 split with 16px gap
    let wide = compute_zenith_cockpit(1200.0, DashboardTier::Wide);
    assert!(wide.is_split);
    assert_eq!(wide.left_width, 710.0); // (1200 - 16) * 0.60 = 710.4 -> floor 710
    assert_eq!(wide.right_width, 474.0); // 1200 - 16 - 710 = 474
    assert_eq!(wide.left_width + wide.right_width + 16.0, 1200.0);

    // Ultrawide capped container (1440px): 60/40 split
    let ultrawide = compute_zenith_cockpit(1440.0, DashboardTier::Ultrawide);
    assert!(ultrawide.is_split);
    assert_eq!(ultrawide.left_width, 854.0); // (1440 - 16) * 0.60 = 854.4 -> 854
    assert_eq!(ultrawide.right_width, 570.0); // 1440 - 16 - 854 = 570
    assert_eq!(ultrawide.left_width + ultrawide.right_width + 16.0, 1440.0);
}

#[test]
fn test_recent_table_bounded_widths_no_infinite_stretch() {
    // Compact: shows name + size only, no unbounded void
    let c = compute_recent_table(640.0, DashboardTier::Compact);
    assert!(!c.show_date);
    assert!(!c.show_kind);
    assert!(!c.show_path);
    assert_eq!(c.size_width, 70.0);
    assert_eq!(c.name_width, 640.0 - 70.0 - 24.0);

    // Medium (1000px): shows name + date + size
    let m = compute_recent_table(1000.0, DashboardTier::Medium);
    assert!(m.show_date);
    assert!(!m.show_kind);
    assert!(!m.show_path);
    assert_eq!(m.date_width, 140.0);
    assert_eq!(m.size_width, 80.0);
    assert!(m.name_width >= 200.0);

    // Wide (1440px): shows name + date + kind + size
    let w = compute_recent_table(1440.0, DashboardTier::Wide);
    assert!(w.show_date);
    assert!(w.show_kind);
    assert!(!w.show_path);
    assert_eq!(w.date_width, 150.0);
    assert_eq!(w.kind_width, 110.0);
    assert_eq!(w.size_width, 85.0);
    assert!(w.name_width >= 200.0);

    // Ultrawide (1920px): shows all 5 columns including parent path
    let u = compute_recent_table(1920.0, DashboardTier::Ultrawide);
    assert!(u.show_date);
    assert!(u.show_kind);
    assert!(u.show_path);
    assert_eq!(u.date_width, 160.0);
    assert_eq!(u.kind_width, 120.0);
    assert_eq!(u.path_width, 180.0);
    assert_eq!(u.size_width, 90.0);
    assert!(u.name_width >= 240.0);
}

#[test]
fn test_storage_usage_color_floors() {
    let pal = ThemeStyle::default().colors();

    // 0-74%: normal/healthy -> success color
    assert_eq!(storage_usage_color(0.00, &pal), pal.success);
    assert_eq!(storage_usage_color(0.50, &pal), pal.success);
    assert_eq!(storage_usage_color(0.74, &pal), pal.success);

    // 75-89%: warning -> warn color
    assert_eq!(storage_usage_color(0.75, &pal), pal.warn);
    assert_eq!(storage_usage_color(0.85, &pal), pal.warn);
    assert_eq!(storage_usage_color(0.89, &pal), pal.warn);

    // 90-100%: critical space -> error color
    assert_eq!(storage_usage_color(0.90, &pal), pal.error);
    assert_eq!(storage_usage_color(0.98, &pal), pal.error);
    assert_eq!(storage_usage_color(1.00, &pal), pal.error);
}

#[test]
fn test_card_grid_dynamic_scaling() {
    let grid = compute_card_grid(1000.0, 110.0, 160.0, 12.0);
    assert!(grid.columns >= 6);
    assert!((110.0..=160.0).contains(&grid.card_width));
}

#[test]
fn test_drive_card_width_scaling() {
    // Compact: 1 column
    let (w_compact, cols_c) = compute_drive_card_width(600.0, DashboardTier::Compact, 12.0);
    assert_eq!(cols_c, 1);
    assert_eq!(w_compact, 600.0);

    // Medium: 2 columns
    let (w_med, cols_m) = compute_drive_card_width(1000.0, DashboardTier::Medium, 12.0);
    assert_eq!(cols_m, 2);
    assert_eq!(w_med, (1000.0 - 12.0) * 0.5);

    // Wide: multiple columns
    let (w_wide, cols_w) = compute_drive_card_width(1440.0, DashboardTier::Wide, 12.0);
    assert!((2..=4).contains(&cols_w));
    assert!(w_wide >= 200.0);
}

#[test]
fn test_dashboard_slots_persistence_roundtrip() {
    let mut cfg = MossambiConfig::default();
    assert_eq!(
        cfg.dashboard_slots,
        vec![
            DashboardWidgetSlot::QuickAccess,
            DashboardWidgetSlot::Drives,
            DashboardWidgetSlot::QuickActions,
            DashboardWidgetSlot::RecentFiles,
            DashboardWidgetSlot::Activity,
        ]
    );

    // Reorder slots (e.g. user moved Drives to top)
    cfg.dashboard_slots = vec![
        DashboardWidgetSlot::Drives,
        DashboardWidgetSlot::QuickAccess,
        DashboardWidgetSlot::Activity,
        DashboardWidgetSlot::RecentFiles,
        DashboardWidgetSlot::QuickActions,
    ];

    let json = serde_json::to_string(&cfg).expect("serialize config");
    let restored: MossambiConfig = serde_json::from_str(&json).expect("deserialize config");
    assert_eq!(restored.dashboard_slots, cfg.dashboard_slots);

    // Default fallback when dashboard_slots is omitted
    let mut val: serde_json::Value = serde_json::to_value(MossambiConfig::default()).unwrap();
    if let serde_json::Value::Object(ref mut map) = val {
        map.remove("dashboard_slots");
    }
    let restored_default: MossambiConfig =
        serde_json::from_value(val).expect("deserialize config without dashboard_slots");
    assert_eq!(
        restored_default.dashboard_slots,
        DashboardWidgetSlot::default_slots()
    );
}

#[test]
fn test_format_bytes_ascii_units() {
    assert_eq!(format_bytes_ascii(500), "500 B");
    assert_eq!(format_bytes_ascii(1024), "1.0 KB");
    assert_eq!(format_bytes_ascii(1024 * 1024 * 50), "50.0 MB");
    assert_eq!(format_bytes_ascii(1024 * 1024 * 1024 * 250), "250.0 GB");
    assert_eq!(format_bytes_ascii(1024 * 1024 * 1024 * 1024 * 2), "2.0 TB");
}
