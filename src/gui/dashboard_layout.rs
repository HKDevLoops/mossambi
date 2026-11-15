//! Responsive Home / Dashboard Layout Engine
//!
//! Provides pure layout calculations for the home dashboard:
//! - 4 responsive breakpoint tiers (<800, 800-1199, 1200-1599, >=1600)
//! - Centered max-width content container (max 1440px on wide/ultrawide displays)
//! - Bento grid columns per tier and DesignStyle preset
//! - 60/40 cockpit split calculations for Zenith Pro
//! - Card grid sizing (Quick Access and Drives) preventing empty voids
//! - Bounded multi-column Recent Files table layout avoiding 1600px line stretching
//! - Storage usage gauge semantic colors (success, warn, error)
//!
//! All content is pure ASCII.

use crate::core::config::DesignStyle;
use crate::gui::macos_theme::ThemePalette;
use eframe::egui::Color32;

/// Responsive breakpoint tiers for the Mossambi home dashboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DashboardTier {
    /// Width < 800px: single column stacked, compact table.
    Compact,
    /// Width 800px..=1199px: 2 bento columns, 3-column recent table.
    Medium,
    /// Width 1200px..=1599px: 3 bento columns, 4-column recent table, 1200-1440px container.
    Wide,
    /// Width >= 1600px: 4 bento columns (or 60/40 cockpit), 5-column table, 1440px centered container.
    Ultrawide,
}

impl DashboardTier {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Compact => "Compact (<800px)",
            Self::Medium => "Medium (800-1199px)",
            Self::Wide => "Wide (1200-1599px)",
            Self::Ultrawide => "Ultrawide (>=1600px)",
        }
    }
}

/// Resolves the responsive dashboard tier from available viewport width.
pub fn tier_for_width(width: f32) -> DashboardTier {
    if width < 800.0 {
        DashboardTier::Compact
    } else if width < 1200.0 {
        DashboardTier::Medium
    } else if width < 1600.0 {
        DashboardTier::Wide
    } else {
        DashboardTier::Ultrawide
    }
}

/// Content container dimensions for centered dashboard display.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContainerMetrics {
    pub viewport_width: f32,
    pub content_width: f32,
    pub margin_x: f32,
    pub tier: DashboardTier,
}

/// Computes the centered container width and horizontal margin.
/// Bounds content within max_content_width (e.g. 1440px) to prevent void expansion,
/// and enforces a symmetrical outer gutter floor (24px) so content never touches the window edges.
pub fn compute_container_metrics(viewport_width: f32, max_content_width: f32) -> ContainerMetrics {
    let tier = tier_for_width(viewport_width);
    let cap = max_content_width.max(640.0);
    let gutter = 24.0_f32;
    let available_w = (viewport_width - gutter * 2.0).max(280.0);
    let content_width = available_w.min(cap);
    let margin_x = ((viewport_width - content_width) * 0.5).max(gutter);
    ContainerMetrics {
        viewport_width,
        content_width,
        margin_x,
        tier,
    }
}

/// Computes number of bento grid columns for a given tier and design style.
pub fn bento_column_count(tier: DashboardTier, style: DesignStyle) -> usize {
    match style {
        DesignStyle::ArcaneFluid => match tier {
            DashboardTier::Compact => 1,
            DashboardTier::Medium => 2,
            DashboardTier::Wide => 3,
            DashboardTier::Ultrawide => 4,
        },
        DesignStyle::ZenithPro => match tier {
            DashboardTier::Compact => 1,
            DashboardTier::Medium => 2,
            DashboardTier::Wide | DashboardTier::Ultrawide => 2, // 60/40 cockpit split
        },
        DesignStyle::HyperionSlate => match tier {
            DashboardTier::Compact => 1,
            DashboardTier::Medium => 2,
            DashboardTier::Wide | DashboardTier::Ultrawide => 3,
        },
    }
}

/// 60/40 cockpit split metrics for Zenith Pro preset.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CockpitSplit {
    pub left_width: f32,
    pub right_width: f32,
    pub is_split: bool,
}

/// Computes 60/40 split for Zenith Pro cockpit layout when space allows.
pub fn compute_zenith_cockpit(available_width: f32, tier: DashboardTier) -> CockpitSplit {
    if matches!(tier, DashboardTier::Wide | DashboardTier::Ultrawide) && available_width >= 860.0 {
        let gap = 16.0;
        let left_width = ((available_width - gap) * 0.60).floor();
        let right_width = (available_width - gap - left_width).max(220.0);
        CockpitSplit {
            left_width,
            right_width,
            is_split: true,
        }
    } else {
        CockpitSplit {
            left_width: available_width,
            right_width: available_width,
            is_split: false,
        }
    }
}

/// Dynamic card grid layout (Quick Access and Drives).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CardGridLayout {
    pub card_width: f32,
    pub columns: usize,
    pub gap: f32,
}

/// Calculates dynamic card width and column count fitting within available width.
pub fn compute_card_grid(available_width: f32, min_w: f32, max_w: f32, gap: f32) -> CardGridLayout {
    let safe_w = available_width.max(min_w);
    let cols = (((safe_w + gap) / (min_w + gap)).floor() as usize).max(1);
    let raw_w = (safe_w - (cols - 1) as f32 * gap) / cols as f32;
    let card_width = raw_w.clamp(min_w, max_w);
    CardGridLayout {
        card_width,
        columns: cols,
        gap,
    }
}

/// Drive card sizing for responsive tiers.
pub fn compute_drive_card_width(
    available_width: f32,
    tier: DashboardTier,
    gap: f32,
) -> (f32, usize) {
    match tier {
        DashboardTier::Compact => (available_width.max(180.0), 1),
        DashboardTier::Medium => {
            let cols = 2;
            let w = ((available_width - gap) * 0.5).max(180.0);
            (w, cols)
        }
        DashboardTier::Wide | DashboardTier::Ultrawide => {
            let target_w = 260.0;
            let cols = (((available_width + gap) / (target_w + gap)).floor() as usize).clamp(2, 4);
            let w = ((available_width - (cols - 1) as f32 * gap) / cols as f32).max(200.0);
            (w, cols)
        }
    }
}

/// Recent Files table column layout with bounded column lengths.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RecentTableLayout {
    pub name_width: f32,
    pub date_width: f32,
    pub kind_width: f32,
    pub size_width: f32,
    pub path_width: f32,
    pub show_date: bool,
    pub show_kind: bool,
    pub show_path: bool,
}

/// Computes bounded column widths for the Recent Files table to prevent 1600px voids.
pub fn compute_recent_table(available_width: f32, tier: DashboardTier) -> RecentTableLayout {
    match tier {
        DashboardTier::Compact => {
            let size_w = 70.0;
            let name_w = (available_width - size_w - 24.0).max(120.0);
            RecentTableLayout {
                name_width: name_w,
                date_width: 0.0,
                kind_width: 0.0,
                size_width: size_w,
                path_width: 0.0,
                show_date: false,
                show_kind: false,
                show_path: false,
            }
        }
        DashboardTier::Medium => {
            let size_w = 80.0;
            let date_w = 140.0;
            let name_w = (available_width - size_w - date_w - 32.0).max(160.0);
            RecentTableLayout {
                name_width: name_w,
                date_width: date_w,
                kind_width: 0.0,
                size_width: size_w,
                path_width: 0.0,
                show_date: true,
                show_kind: false,
                show_path: false,
            }
        }
        DashboardTier::Wide => {
            let size_w = 85.0;
            let kind_w = 110.0;
            let date_w = 150.0;
            let name_w = (available_width - size_w - kind_w - date_w - 40.0).max(200.0);
            RecentTableLayout {
                name_width: name_w,
                date_width: date_w,
                kind_width: kind_w,
                size_width: size_w,
                path_width: 0.0,
                show_date: true,
                show_kind: true,
                show_path: false,
            }
        }
        DashboardTier::Ultrawide => {
            let size_w = 90.0;
            let kind_w = 120.0;
            let date_w = 160.0;
            let path_w = 180.0;
            let name_w = (available_width - size_w - kind_w - date_w - path_w - 48.0).max(240.0);
            RecentTableLayout {
                name_width: name_w,
                date_width: date_w,
                kind_width: kind_w,
                size_width: size_w,
                path_width: path_w,
                show_date: true,
                show_kind: true,
                show_path: true,
            }
        }
    }
}

/// Returns the semantic color for storage usage gauges based on utilization ratio.
pub fn storage_usage_color(usage_ratio: f32, palette: &ThemePalette) -> Color32 {
    let r = usage_ratio.clamp(0.0, 1.0);
    if r >= 0.90 {
        palette.error
    } else if r >= 0.75 {
        palette.warn
    } else {
        palette.success
    }
}

/// Formats byte sizes into human-readable plain ASCII string.
pub fn format_bytes_ascii(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.1} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_breakpoints() {
        assert_eq!(tier_for_width(640.0), DashboardTier::Compact);
        assert_eq!(tier_for_width(799.0), DashboardTier::Compact);
        assert_eq!(tier_for_width(800.0), DashboardTier::Medium);
        assert_eq!(tier_for_width(1199.0), DashboardTier::Medium);
        assert_eq!(tier_for_width(1200.0), DashboardTier::Wide);
        assert_eq!(tier_for_width(1599.0), DashboardTier::Wide);
        assert_eq!(tier_for_width(1600.0), DashboardTier::Ultrawide);
        assert_eq!(tier_for_width(2560.0), DashboardTier::Ultrawide);
    }

    #[test]
    fn test_centered_container_metrics() {
        // Compact window has 24px outer margin floor
        let c = compute_container_metrics(750.0, 1440.0);
        assert_eq!(c.tier, DashboardTier::Compact);
        assert_eq!(c.content_width, 702.0);
        assert_eq!(c.margin_x, 24.0);

        // Medium window has 24px outer margin floor
        let m = compute_container_metrics(1000.0, 1440.0);
        assert_eq!(m.tier, DashboardTier::Medium);
        assert_eq!(m.content_width, 952.0);
        assert_eq!(m.margin_x, 24.0);

        // Ultrawide (1920px) is capped at 1440px and centered
        let u = compute_container_metrics(1920.0, 1440.0);
        assert_eq!(u.tier, DashboardTier::Ultrawide);
        assert_eq!(u.content_width, 1440.0);
        assert_eq!(u.margin_x, 240.0);

        // 2560px screen is centered with 560px margins
        let uw = compute_container_metrics(2560.0, 1440.0);
        assert_eq!(uw.content_width, 1440.0);
        assert_eq!(uw.margin_x, 560.0);
    }

    #[test]
    fn test_bento_columns_per_preset() {
        // Arcane Fluid scales 1 -> 2 -> 3 -> 4
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

        // Zenith Pro maintains 2-col cockpit on wide/ultrawide
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

        // Hyperion Slate maintains up to 3 modular slots
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
    fn test_zenith_cockpit_split() {
        // Narrow window falls back to stacked
        let stacked = compute_zenith_cockpit(700.0, DashboardTier::Compact);
        assert!(!stacked.is_split);
        assert_eq!(stacked.left_width, 700.0);

        // Wide window activates 60/40 split
        let split = compute_zenith_cockpit(1200.0, DashboardTier::Wide);
        assert!(split.is_split);
        // (1200 - 16) * 0.60 = 1184 * 0.6 = 710.4 -> floor 710
        assert_eq!(split.left_width, 710.0);
        assert_eq!(split.right_width, 474.0);
        assert_eq!(split.left_width + split.right_width + 16.0, 1200.0);
    }

    #[test]
    fn test_card_grid_dynamic_scaling() {
        let grid = compute_card_grid(1000.0, 110.0, 160.0, 12.0);
        assert!(grid.columns >= 6);
        assert!((110.0..=160.0).contains(&grid.card_width));
    }

    #[test]
    fn test_recent_table_bounded_widths() {
        // Compact hides extra columns
        let c = compute_recent_table(600.0, DashboardTier::Compact);
        assert!(!c.show_date);
        assert!(!c.show_kind);
        assert!(!c.show_path);
        assert!(c.name_width > 100.0);

        // Wide shows name, date, kind, size
        let w = compute_recent_table(1400.0, DashboardTier::Wide);
        assert!(w.show_date);
        assert!(w.show_kind);
        assert!(!w.show_path);
        assert!(w.name_width >= 200.0);

        // Ultrawide includes path column
        let u = compute_recent_table(1920.0, DashboardTier::Ultrawide);
        assert!(u.show_date);
        assert!(u.show_kind);
        assert!(u.show_path);
        assert!(u.path_width > 0.0);
    }

    #[test]
    fn test_storage_usage_color_tiers() {
        let pal = crate::gui::macos_theme::ThemeStyle::default().colors();
        let healthy = storage_usage_color(0.50, &pal);
        assert_eq!(healthy, pal.success);

        let warn = storage_usage_color(0.80, &pal);
        assert_eq!(warn, pal.warn);

        let err = storage_usage_color(0.95, &pal);
        assert_eq!(err, pal.error);
    }
}
