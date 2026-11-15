//! Window-level motion mathematics and state machines.
//!
//! Provides the mathematical models for 5 OS window transition animations:
//! - [`WindowAnimType::Genie`]: macOS non-linear polynomial funnel mesh suction.
//! - [`WindowAnimType::Scale`]: macOS / modern Windows 11 critically-damped spring scale.
//! - [`WindowAnimType::Slide`]: iOS / GNOME fluid edge glide.
//! - [`WindowAnimType::Morph`]: Windows Fluent bounds-to-icon interpolation.
//! - [`WindowAnimType::Fade`]: KDE / Deepin soft blur-fade dissolve.
//!
//! Also provides the in-app Alt+Tab switcher HUD state machine.

use serde::{Deserialize, Serialize};

/// The 5 supported window-level transition animation styles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum WindowAnimType {
    /// macOS classic Genie effect: non-linear polynomial funnel wave.
    #[default]
    Genie,
    /// Apple / Windows 11 scale down to taskbar/dock icon.
    Scale,
    /// iOS / GNOME slide down to bottom edge.
    Slide,
    /// Windows Fluent rectangular morph into taskbar bounds.
    Morph,
    /// KDE / Deepin instantaneous soft dissolve.
    Fade,
}

impl WindowAnimType {
    pub const ALL: [Self; 5] = [
        Self::Genie,
        Self::Scale,
        Self::Slide,
        Self::Morph,
        Self::Fade,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::Genie => "Genie (macOS)",
            Self::Scale => "Scale (Apple)",
            Self::Slide => "Slide (iOS/GNOME)",
            Self::Morph => "Morph (Fluent)",
            Self::Fade => "Fade (Deepin)",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Genie => "Polynomial funnel suction curving gracefully into the taskbar.",
            Self::Scale => "Critically-damped spring shrink centered on the taskbar icon.",
            Self::Slide => "Smooth directional slide down off the bottom display edge.",
            Self::Morph => "Restrained rectangle bounds interpolation into the taskbar pill.",
            Self::Fade => "Soft, elegant blur-fade dissolve with minimal displacement.",
        }
    }
}

/// A 2D point resulting from mesh deformation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeniePoint {
    pub x: f32,
    pub y: f32,
}

/// Calculates the transformed position of a point `(x, y)` inside `window_rect`
/// during a Genie effect minimize animation toward `dock_target`.
///
/// * `window_rect`: `(x, y, width, height)` of the unminimized window.
/// * `dock_target`: `(x, y)` coordinates of the dock or taskbar icon.
/// * `t`: normalized animation progress in `[0.0, 1.0]`.
pub fn genie_mesh_transform(
    x: f32,
    y: f32,
    window_rect: (f32, f32, f32, f32),
    dock_target: (f32, f32),
    t: f32,
) -> GeniePoint {
    let t = t.clamp(0.0, 1.0);
    if t <= 0.0001 {
        return GeniePoint { x, y };
    }

    let (wx, wy, ww, wh) = window_rect;
    let (dx, dy) = dock_target;

    let u = ((x - wx) / ww.max(1.0)).clamp(0.0, 1.0);
    let v = ((y - wy) / wh.max(1.0)).clamp(0.0, 1.0);

    // Non-linear suction wave: bottom of the window (v = 1.0) leads, top follows.
    let slice_lead = 0.35_f32;
    let s = ((t - (1.0 - v) * slice_lead) / (1.0 - slice_lead).max(0.01)).clamp(0.0, 1.0);

    // Ease-in polynomial curve for the suction acceleration
    let suction = s * s;

    // Width funnels from original window width down to a narrow 20px pinch at dock
    let slice_w = ww * (1.0 - suction) + 20.0 * suction;

    // Horizontal center drifts from window center to dock target x
    let window_cx = wx + ww * 0.5;
    let slice_cx = window_cx * (1.0 - suction) + dx * suction;

    let px = slice_cx + (u - 0.5) * slice_w;
    let py = y * (1.0 - suction) + dy * suction;

    GeniePoint { x: px, y: py }
}

/// Returns the `(scale, alpha, offset_y)` geometry tuple for standard window
/// animation styles at progress `t` in `[0.0, 1.0]`.
pub fn window_anim_geometry(anim: WindowAnimType, t: f32) -> (f32, f32, f32) {
    let t = t.clamp(0.0, 1.0);
    match anim {
        WindowAnimType::Genie => {
            let scale = (1.0 - 0.75 * t).clamp(0.05, 1.0);
            let alpha = (1.0 - 0.85 * t * t).clamp(0.0, 1.0);
            let offset_y = 250.0 * (t * t);
            (scale, alpha, offset_y)
        }
        WindowAnimType::Scale => {
            let scale = (1.0 - 0.90 * t).clamp(0.05, 1.0);
            let alpha = (1.0 - t).clamp(0.0, 1.0);
            (scale, alpha, 0.0)
        }
        WindowAnimType::Slide => {
            let scale = 1.0;
            let alpha = (1.0 - t).clamp(0.0, 1.0);
            let offset_y = 350.0 * (1.0 - (-6.0 * t).exp());
            (scale, alpha, offset_y)
        }
        WindowAnimType::Morph => {
            let scale = (1.0 - 0.85 * t).clamp(0.05, 1.0);
            let alpha = (1.0 - 0.90 * t).clamp(0.0, 1.0);
            let offset_y = 200.0 * t;
            (scale, alpha, offset_y)
        }
        WindowAnimType::Fade => {
            let scale = 1.0;
            let alpha = (1.0 - t).clamp(0.0, 1.0);
            (scale, alpha, 0.0)
        }
    }
}

/// In-app Alt+Tab / Quick Window Switcher state machine.
#[derive(Debug, Clone, Default)]
pub struct AltTabState {
    pub is_open: bool,
    pub items: Vec<String>,
    pub active_idx: usize,
}

impl AltTabState {
    pub fn new(items: Vec<String>) -> Self {
        Self {
            is_open: true,
            items,
            active_idx: 0,
        }
    }

    pub fn active_index(&self) -> usize {
        self.active_idx
    }

    pub fn cycle_next(&mut self) {
        if !self.items.is_empty() {
            self.active_idx = (self.active_idx + 1) % self.items.len();
        }
    }

    pub fn cycle_prev(&mut self) {
        if !self.items.is_empty() {
            if self.active_idx == 0 {
                self.active_idx = self.items.len() - 1;
            } else {
                self.active_idx -= 1;
            }
        }
    }

    pub fn close(&mut self) {
        self.is_open = false;
    }
}
