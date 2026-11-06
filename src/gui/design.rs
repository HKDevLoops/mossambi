//! Mossambi Design Token System
//!
//! A single source of truth for spacing, typography, radii, motion, semantic color
//! roles, and text fitting. Every surface in the app (Studio explorer, Preferences,
//! radial widget, TUI-adjacent panels) draws from these tokens so the UI reads as one
//! system instead of a pile of hand-picked pixel values.
//!
//! Design intent: macOS Finder proportions (tight, dense, information-first) expressed
//! on a 4px base grid, with a Tailwind-like discipline: every gap, padding, and type
//! size is named, never inline.
//!
//! All content is pure ASCII.

use eframe::egui::{Color32, FontId, Ui};

// ---------------------------------------------------------------------------
// Spacing: 4px base grid
// ---------------------------------------------------------------------------

/// Named spacing steps. Every gap and padding in the app comes from this table.
pub mod space {
    /// 2px - hairline separation inside a single control.
    pub const XXS: f32 = 2.0;
    /// 4px - tight inner gap, icon-to-label.
    pub const XS: f32 = 4.0;
    /// 6px - sub-control gap.
    pub const SM: f32 = 6.0;
    /// 8px - default control inner padding.
    pub const MD: f32 = 8.0;
    /// 12px - default gap between related controls.
    pub const LG: f32 = 12.0;
    /// 16px - default panel padding.
    pub const XL: f32 = 16.0;
    /// 20px - card padding.
    pub const XXL: f32 = 20.0;
    /// 24px - section gap.
    pub const XXXL: f32 = 24.0;
    /// 32px - major section separation.
    pub const HUGE: f32 = 32.0;
    /// 40px - page-level separation.
    pub const GIANT: f32 = 40.0;
}

// ---------------------------------------------------------------------------
// Typography
// ---------------------------------------------------------------------------

/// Named type scale. Finder uses a compact scale; these map to UI roles.
pub mod type_scale {
    /// 10px - badge / counter text.
    pub const MICRO: f32 = 10.0;
    /// 11px - secondary caption, column header.
    pub const CAPTION: f32 = 11.0;
    /// 12px - helper text, sidebar sub-label.
    pub const SMALL: f32 = 12.0;
    /// 13px - dense table cell body (Finder list rows).
    pub const BODY_SM: f32 = 13.0;
    /// 14px - default body text.
    pub const BODY: f32 = 14.0;
    /// 15px - emphasized body, list item primary.
    pub const BODY_LG: f32 = 15.0;
    /// 17px - card title.
    pub const TITLE_SM: f32 = 17.0;
    /// 20px - pane title.
    pub const TITLE: f32 = 20.0;
    /// 24px - page title.
    pub const TITLE_LG: f32 = 24.0;
    /// 30px - display title.
    pub const DISPLAY: f32 = 30.0;
}

/// Returns the proportional `FontId` for a named type step.
pub fn font(size: f32) -> FontId {
    FontId::proportional(size)
}

// ---------------------------------------------------------------------------
// Radii
// ---------------------------------------------------------------------------

/// Named corner radii.
pub mod radius {
    /// 3px - checkbox, tiny badge.
    pub const XS: f32 = 3.0;
    /// 5px - inline button.
    pub const SM: f32 = 5.0;
    /// 7px - control (segmented switch, input).
    pub const MD: f32 = 7.0;
    /// 10px - card.
    pub const LG: f32 = 10.0;
    /// 13px - popover, sheet.
    pub const XL: f32 = 13.0;
    /// 18px - modal.
    pub const XXL: f32 = 18.0;
    /// 999px - pill (traffic lights, segmented indicator).
    pub const PILL: f32 = 999.0;
}

// ---------------------------------------------------------------------------
// Motion
// ---------------------------------------------------------------------------

/// Named durations in milliseconds. The app animates every state change.
pub mod motion {
    /// 60ms - micro feedback (button press, checkbox tick).
    pub const MICRO: f32 = 60.0;
    /// 110ms - hover tint, chevron rotate.
    pub const FAST: f32 = 110.0;
    /// 180ms - popover open, sheet slide.
    pub const NORMAL: f32 = 180.0;
    /// 280ms - panel transition, column expansion.
    pub const SLOW: f32 = 280.0;
    /// 420ms - large surface reveal, window chrome.
    pub const SLOWER: f32 = 420.0;

    /// Converts milliseconds to seconds for `egui::Context::animate_bool_with_time`.
    pub fn secs(ms: f32) -> f32 {
        ms / 1000.0
    }

    /// Standard deceleration curve: fast start, gentle settle.
    pub fn ease_out(t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        1.0 - (1.0 - t).powi(3)
    }

    /// Symmetric ease for reversible transitions.
    pub fn ease_in_out(t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        if t < 0.5 {
            4.0 * t * t * t
        } else {
            1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
        }
    }
}

// ---------------------------------------------------------------------------
// Layout metrics
// ---------------------------------------------------------------------------

/// Fixed layout metrics that must agree across surfaces.
pub mod layout {
    /// Finder title bar / chrome height.
    pub const TITLE_BAR_H: f32 = 40.0;
    /// Finder toolbar row height.
    pub const TOOLBAR_H: f32 = 36.0;
    /// Bottom path bar height.
    pub const PATH_BAR_H: f32 = 26.0;
    /// Status bar height.
    pub const STATUS_BAR_H: f32 = 24.0;
    /// Sidebar minimum width.
    pub const SIDEBAR_MIN_W: f32 = 176.0;
    /// Sidebar default width.
    pub const SIDEBAR_W: f32 = 208.0;
    /// Sidebar maximum width.
    pub const SIDEBAR_MAX_W: f32 = 320.0;
    /// Minimum main content width before the layout is considered cramped.
    pub const CONTENT_MIN_W: f32 = 420.0;
    /// List row height in the details table.
    pub const ROW_H: f32 = 24.0;
    /// Column header height in the details table.
    pub const HEADER_H: f32 = 26.0;
    /// Traffic light diameter.
    pub const TRAFFIC_LIGHT_D: f32 = 12.0;
    /// Traffic light horizontal gap.
    pub const TRAFFIC_LIGHT_GAP: f32 = 8.0;
}

/// Responsive breakpoints. Below `COMPACT_MAX` the sidebar collapses.
pub mod breakpoint {
    /// Below this width the layout switches to compact rules.
    pub const COMPACT_MAX: f32 = 900.0;
    /// Below this width only a narrow sidebar is allowed.
    pub const MEDIUM_MAX: f32 = 1280.0;
}

/// Layout density derived from the available width.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Density {
    /// Narrow: sidebar collapses to icons or hides, columns collapse.
    Compact,
    /// Standard: full Finder proportions.
    Medium,
    /// Wide: room for the inspector / metadata column.
    Wide,
}

/// Maps an available width to a layout density.
pub fn density_for(width: f32) -> Density {
    if width < breakpoint::COMPACT_MAX {
        Density::Compact
    } else if width < breakpoint::MEDIUM_MAX {
        Density::Medium
    } else {
        Density::Wide
    }
}

// ---------------------------------------------------------------------------
// Semantic colors
// ---------------------------------------------------------------------------

/// Semantic color roles derived from the active theme palette.
///
/// Raw theme colors describe a theme; these roles describe *intent*, so surfaces
/// stay consistent when the palette changes.
#[derive(Debug, Clone, Copy)]
pub struct SemanticColors {
    /// App canvas behind everything.
    pub canvas: Color32,
    /// Sidebar / chrome surface.
    pub chrome: Color32,
    /// Raised card surface.
    pub surface: Color32,
    /// Surface one step above a card.
    pub surface_raised: Color32,
    /// Hairline separator.
    pub border_subtle: Color32,
    /// Visible border on interactive surfaces.
    pub border_strong: Color32,
    /// Primary text.
    pub text_primary: Color32,
    /// Secondary text.
    pub text_secondary: Color32,
    /// Tertiary / disabled text.
    pub text_tertiary: Color32,
    /// Accent (selection, focus, primary action).
    pub accent: Color32,
    /// Fill behind a selected row.
    pub selection_fill: Color32,
    /// Fill behind a hovered row.
    pub hover_fill: Color32,
    /// Destructive action.
    pub danger: Color32,
    /// Cautionary state.
    pub warning: Color32,
    /// Positive state.
    pub success: Color32,
}

impl SemanticColors {
    /// Derives semantic roles from a theme palette.
    pub fn from_palette(p: &crate::gui::macos_theme::ThemePalette) -> Self {
        let (accent, selection_fill) = if p.is_dark {
            (
                p.accent,
                Color32::from_rgba_premultiplied(p.accent.r(), p.accent.g(), p.accent.b(), 58),
            )
        } else {
            (
                p.accent,
                Color32::from_rgba_premultiplied(p.accent.r(), p.accent.g(), p.accent.b(), 44),
            )
        };
        Self {
            canvas: p.window_bg,
            chrome: p.sidebar_bg,
            surface: p.card_bg,
            surface_raised: p.card_hover,
            border_subtle: p.divider,
            border_strong: p.border,
            text_primary: p.text_primary,
            text_secondary: p.text_secondary,
            text_tertiary: p.text_muted,
            accent,
            selection_fill,
            hover_fill: p.card_hover,
            danger: Color32::from_rgb(255, 95, 86),
            warning: Color32::from_rgb(255, 189, 46),
            success: Color32::from_rgb(52, 199, 89),
        }
    }
}

// ---------------------------------------------------------------------------
// Text fitting
// ---------------------------------------------------------------------------

/// Measures the rendered width of `text` at `font_id`.
pub fn measure_text(ui: &Ui, text: &str, font_id: &FontId) -> f32 {
    if text.is_empty() {
        return 0.0;
    }
    ui.painter()
        .layout_no_wrap(text.to_owned(), font_id.clone(), Color32::WHITE)
        .size()
        .x
}

/// Truncates `text` with a trailing ellipsis so it fits inside `max_width`.
///
/// Uses a binary search over character count, so the cost is O(log n) layout
/// measurements rather than a linear scan.
pub fn elide_end(ui: &Ui, text: &str, font_id: &FontId, max_width: f32) -> String {
    if text.is_empty() || max_width <= 0.0 {
        return String::new();
    }
    if measure_text(ui, text, font_id) <= max_width {
        return text.to_owned();
    }
    let ellipsis = "...";
    let chars: Vec<char> = text.chars().collect();
    let mut lo = 0usize;
    let mut hi = chars.len();
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        let mut candidate: String = chars[..mid].iter().collect();
        candidate.push_str(ellipsis);
        if measure_text(ui, &candidate, font_id) <= max_width {
            lo = mid;
        } else {
            hi = mid.saturating_sub(1);
        }
    }
    if lo == 0 {
        return ellipsis.to_owned();
    }
    let mut out: String = chars[..lo].iter().collect();
    out.push_str(ellipsis);
    out
}

/// Truncates the middle of `text` with an ellipsis so it fits inside `max_width`.
///
/// Finder keeps the extension visible on long file names, which is why the tail
/// matters more than the head.
pub fn elide_middle(ui: &Ui, text: &str, font_id: &FontId, max_width: f32) -> String {
    if text.is_empty() || max_width <= 0.0 {
        return String::new();
    }
    if measure_text(ui, text, font_id) <= max_width {
        return text.to_owned();
    }
    let ellipsis = "...";
    let chars: Vec<char> = text.chars().collect();
    let mut lo = 0usize;
    let mut hi = chars.len();
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        let head = mid / 2;
        let tail = mid - head;
        let mut candidate: String = chars[..head].iter().collect();
        candidate.push_str(ellipsis);
        candidate.extend(chars[chars.len() - tail..].iter());
        if measure_text(ui, &candidate, font_id) <= max_width {
            lo = mid;
        } else {
            hi = mid.saturating_sub(1);
        }
    }
    if lo == 0 {
        return ellipsis.to_owned();
    }
    let head = lo / 2;
    let tail = lo - head;
    let mut out: String = chars[..head].iter().collect();
    out.push_str(ellipsis);
    out.extend(chars[chars.len() - tail..].iter());
    out
}

/// Greedy word-wrap of `text` into lines that each fit inside `max_width`.
///
/// A single word longer than `max_width` is hard-split rather than allowed to
/// overflow, which is what keeps long paths from colliding with the next column.
pub fn wrap_text(ui: &Ui, text: &str, font_id: &FontId, max_width: f32) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    if max_width <= 0.0 {
        return vec![text.to_owned()];
    }
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let candidate = if current.is_empty() {
            word.to_owned()
        } else {
            format!("{} {}", current, word)
        };
        if measure_text(ui, &candidate, font_id) <= max_width {
            current = candidate;
            continue;
        }
        if !current.is_empty() {
            lines.push(std::mem::take(&mut current));
        }
        if measure_text(ui, word, font_id) <= max_width {
            current = word.to_owned();
        } else {
            // Hard-split an over-long token.
            let mut chunk = String::new();
            for ch in word.chars() {
                let probe = format!("{}{}", chunk, ch);
                if !chunk.is_empty() && measure_text(ui, &probe, font_id) > max_width {
                    lines.push(std::mem::take(&mut chunk));
                }
                chunk.push(ch);
            }
            current = chunk;
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(text.to_owned());
    }
    lines
}

/// Greedy word-wrap that preserves explicit newlines.
pub fn wrap_text_preserve_newlines(
    ui: &Ui,
    text: &str,
    font_id: &FontId,
    max_width: f32,
) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for para in text.split('\n') {
        if para.trim().is_empty() {
            out.push(String::new());
        } else {
            out.extend(wrap_text(ui, para, font_id, max_width));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spacing_is_monotonic_on_the_base_grid() {
        let steps = [
            space::XXS,
            space::XS,
            space::SM,
            space::MD,
            space::LG,
            space::XL,
            space::XXL,
            space::XXXL,
            space::HUGE,
            space::GIANT,
        ];
        for w in steps.windows(2) {
            assert!(w[0] < w[1], "spacing steps must increase: {:?}", w);
        }
    }

    #[test]
    fn type_scale_is_monotonic() {
        let steps = [
            type_scale::MICRO,
            type_scale::CAPTION,
            type_scale::SMALL,
            type_scale::BODY_SM,
            type_scale::BODY,
            type_scale::BODY_LG,
            type_scale::TITLE_SM,
            type_scale::TITLE,
            type_scale::TITLE_LG,
            type_scale::DISPLAY,
        ];
        for w in steps.windows(2) {
            assert!(w[0] < w[1], "type steps must increase: {:?}", w);
        }
    }

    #[test]
    fn density_breakpoints_follow_the_spec() {
        assert_eq!(density_for(600.0), Density::Compact);
        assert_eq!(density_for(1000.0), Density::Medium);
        assert_eq!(density_for(1600.0), Density::Wide);
    }

    #[test]
    fn motion_seconds_conversion() {
        assert!((motion::secs(180.0) - 0.18).abs() < f32::EPSILON);
    }

    #[test]
    fn easing_endpoints_are_stable() {
        assert!(motion::ease_out(0.0).abs() < 1e-6);
        assert!((motion::ease_out(1.0) - 1.0).abs() < 1e-6);
        assert!(motion::ease_in_out(0.0).abs() < 1e-6);
        assert!((motion::ease_in_out(1.0) - 1.0).abs() < 1e-6);
    }
}
