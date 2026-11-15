//! Pure layout and zoom engine for Explorer view modes (List, Grid, Miller Columns).
//!
//! Provides step metrics, responsive breakpoint reflow, bounded list column width calculation,
//! and zoom step clamping with zero immediate-mode rendering side effects.
//!
//! All content is strictly ASCII.

use crate::core::config::DesignStyle;
use crate::gui::dashboard_layout::DashboardTier;

/// Clamps zoom level to valid range [0, 4].
pub fn clamp_zoom(level: i32) -> u8 {
    level.clamp(0, 4) as u8
}

/// Steps zoom level by delta and clamps to [0, 4].
pub fn step_zoom(current: u8, delta: i32) -> u8 {
    (current as i32 + delta).clamp(0, 4) as u8
}

/// Default zoom step (Medium, step 2).
pub const DEFAULT_ZOOM_STEP: u8 = 2;

/// List / Details view density metrics for a zoom step [0..=4].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ListZoomMetrics {
    pub step: u8,
    pub row_height: f32,
    pub font_size: f32,
    pub icon_size: f32,
}

impl ListZoomMetrics {
    pub fn for_step(step: u8) -> Self {
        let s = step.min(4);
        match s {
            0 => Self {
                step: 0,
                row_height: 22.0,
                font_size: 11.0,
                icon_size: 14.0,
            },
            1 => Self {
                step: 1,
                row_height: 26.0,
                font_size: 12.0,
                icon_size: 16.0,
            },
            2 => Self {
                step: 2,
                row_height: 30.0,
                font_size: 13.0,
                icon_size: 18.0,
            },
            3 => Self {
                step: 3,
                row_height: 36.0,
                font_size: 14.5,
                icon_size: 22.0,
            },
            _ => Self {
                step: 4,
                row_height: 42.0,
                font_size: 16.0,
                icon_size: 26.0,
            },
        }
    }
}

/// Grid / Icons view size metrics for a zoom step [0..=4].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridZoomMetrics {
    pub step: u8,
    pub icon_size: f32,
    pub min_tile_width: f32,
    pub font_size: f32,
}

impl GridZoomMetrics {
    pub fn for_step(step: u8) -> Self {
        let s = step.min(4);
        match s {
            0 => Self {
                step: 0,
                icon_size: 64.0,
                min_tile_width: 84.0,
                font_size: 10.5,
            },
            1 => Self {
                step: 1,
                icon_size: 96.0,
                min_tile_width: 116.0,
                font_size: 11.5,
            },
            2 => Self {
                step: 2,
                icon_size: 128.0,
                min_tile_width: 152.0,
                font_size: 13.0,
            },
            3 => Self {
                step: 3,
                icon_size: 180.0,
                min_tile_width: 208.0,
                font_size: 14.5,
            },
            _ => Self {
                step: 4,
                icon_size: 240.0,
                min_tile_width: 272.0,
                font_size: 16.0,
            },
        }
    }
}

/// Miller Columns view size metrics for a zoom step [0..=4].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MillerZoomMetrics {
    pub step: u8,
    pub column_width: f32,
    pub font_size: f32,
}

impl MillerZoomMetrics {
    pub fn for_step(step: u8) -> Self {
        let s = step.min(4);
        match s {
            0 => Self {
                step: 0,
                column_width: 180.0,
                font_size: 11.5,
            },
            1 => Self {
                step: 1,
                column_width: 220.0,
                font_size: 12.0,
            },
            2 => Self {
                step: 2,
                column_width: 260.0,
                font_size: 13.0,
            },
            3 => Self {
                step: 3,
                column_width: 320.0,
                font_size: 14.0,
            },
            _ => Self {
                step: 4,
                column_width: 380.0,
                font_size: 15.0,
            },
        }
    }
}

/// Calculated grid reflow result.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridReflowLayout {
    pub columns: usize,
    pub tile_width: f32,
    pub gap: f32,
}

/// Computes grid reflow column count and exact tile width to prevent horizontal dead voids.
pub fn compute_grid_reflow(
    available_width: f32,
    min_tile_width: f32,
    gap: f32,
) -> GridReflowLayout {
    let avail = available_width.max(min_tile_width);
    let cols = ((avail + gap) / (min_tile_width + gap)).floor().max(1.0) as usize;
    let total_gaps = (cols.saturating_sub(1)) as f32 * gap;
    let actual_tile_w = ((avail - total_gaps) / cols as f32).max(min_tile_width);

    GridReflowLayout {
        columns: cols,
        tile_width: actual_tile_w,
        gap,
    }
}

/// Bounded list layout for Details view.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundedListLayout {
    pub max_table_width: f32,
    pub name_min_width: f32,
    pub name_max_width: f32,
    pub date_width: f32,
    pub type_width: f32,
    pub size_width: f32,
    pub show_date: bool,
    pub show_type: bool,
    pub show_size: bool,
    pub show_path: bool,
    pub tabular_figures: bool,
}

/// Computes bounded list columns according to D2 breakpoints and DesignStyle.
pub fn compute_bounded_list_columns(
    available_width: f32,
    tier: DashboardTier,
    style: DesignStyle,
) -> BoundedListLayout {
    let tabular_figures = style == DesignStyle::ZenithPro;

    match tier {
        DashboardTier::Compact => BoundedListLayout {
            max_table_width: available_width,
            name_min_width: 140.0,
            name_max_width: 320.0,
            date_width: 0.0,
            type_width: 0.0,
            size_width: 75.0,
            show_date: false,
            show_type: false,
            show_size: true,
            show_path: false,
            tabular_figures,
        },
        DashboardTier::Medium => BoundedListLayout {
            max_table_width: available_width,
            name_min_width: 200.0,
            name_max_width: 500.0,
            date_width: 140.0,
            type_width: 0.0,
            size_width: 85.0,
            show_date: true,
            show_type: false,
            show_size: true,
            show_path: false,
            tabular_figures,
        },
        DashboardTier::Wide => BoundedListLayout {
            max_table_width: available_width.min(1200.0),
            name_min_width: 240.0,
            name_max_width: 600.0,
            date_width: 160.0,
            type_width: 100.0,
            size_width: 95.0,
            show_date: true,
            show_type: true,
            show_size: true,
            show_path: false,
            tabular_figures,
        },
        DashboardTier::Ultrawide => BoundedListLayout {
            max_table_width: available_width.min(1440.0),
            name_min_width: 280.0,
            name_max_width: 700.0,
            date_width: 180.0,
            type_width: 120.0,
            size_width: 105.0,
            show_date: true,
            show_type: true,
            show_size: true,
            show_path: true,
            tabular_figures,
        },
    }
}
