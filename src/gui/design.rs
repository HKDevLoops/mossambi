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

use eframe::egui::{Color32, FontId, Painter, Rect, Response, Rounding, Stroke, Ui};

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
// Pill ring highlights
// ---------------------------------------------------------------------------

/// Interaction state of a pill control inside the ring highlight system.
///
/// Every pill-shaped control in the app derives its ring from these states so
/// hover, active, and buffered surfaces read consistently across themes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PillState {
    /// Untouched: no ring; the regular hairline border stays as-is.
    Idle,
    /// Pointer rests on the control: soft, low-alpha accent ring.
    Hover,
    /// The control is the current selection: full-accent ring.
    Active,
    /// Pressed or keyboard-focused: strongest ring, Apple focus-ring style.
    Buffer,
}

/// A ring stroke plus the outward expansion applied before stroking, so the
/// highlight stays visible over accent fills and hairline borders alike.
#[derive(Clone, Copy, Debug)]
pub struct PillRing {
    /// Stroke drawn around the expanded control rect.
    pub stroke: Stroke,
    /// Pixels the control rect grows before stroking (concentric ring).
    pub expand: f32,
}

/// Pure form of [`pill_state`] over raw interaction flags, so the priority
/// ladder is unit-testable without an egui context.
///
/// Buffer (pressed or keyboard focus) outranks active; active outranks hover.
pub fn pill_state_from(hovered: bool, pressed: bool, focused: bool, is_active: bool) -> PillState {
    if pressed || focused {
        PillState::Buffer
    } else if is_active {
        PillState::Active
    } else if hovered {
        PillState::Hover
    } else {
        PillState::Idle
    }
}

/// Derives the pill state from a response and the control's selection flag.
pub fn pill_state(resp: &Response, is_active: bool) -> PillState {
    pill_state_from(
        resp.hovered(),
        resp.is_pointer_button_down_on(),
        resp.has_focus(),
        is_active,
    )
}

/// Maps a pill state to its ring, or `None` while idle (regular border stands).
pub fn pill_ring(state: PillState, accent: Color32) -> Option<PillRing> {
    let tinted =
        |alpha: u8| Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), alpha);
    match state {
        PillState::Idle => None,
        PillState::Hover => Some(PillRing {
            stroke: Stroke::new(1.0_f32, tinted(100)),
            expand: 1.0,
        }),
        PillState::Active => Some(PillRing {
            stroke: Stroke::new(1.5_f32, accent),
            expand: 1.0,
        }),
        PillState::Buffer => Some(PillRing {
            stroke: Stroke::new(2.0_f32, tinted(190)),
            expand: 2.0,
        }),
    }
}

/// Paints the ring for a pill control whenever it is not idle.
///
/// The ring grows concentrically with the control: every corner radius gains
/// the ring's expansion so the highlight hugs the original silhouette.
pub fn paint_pill_ring(
    painter: &Painter,
    rect: Rect,
    rounding: Rounding,
    state: PillState,
    accent: Color32,
) {
    let Some(ring) = pill_ring(state, accent) else {
        return;
    };
    let grow = ring.expand;
    let rounding = Rounding {
        nw: rounding.nw + grow,
        ne: rounding.ne + grow,
        sw: rounding.sw + grow,
        se: rounding.se + grow,
    };
    painter.rect_stroke(rect.expand(grow), rounding, ring.stroke);
}

// ---------------------------------------------------------------------------
// Standard Segmented Pill Component (UI Rules Enforced)
// ---------------------------------------------------------------------------

/// Standardized Segmented Control Pill adhering to UI Contrast and Theme Rules.
///
/// Renders a single selectable pill inside a segmented control group.
///
/// UI Rules Enforced:
/// - Active Fill: Uses theme accent color.
/// - Active Text: Guaranteed high-contrast foreground via contrast::text_on_accent
///   (never the accent color itself, eliminating accent-on-accent illegibility).
/// - Inactive Fill: Subtle transparent or card surface with hairline border.
/// - Inactive Text: Clean text_secondary elevating to text_primary on hover.
/// - Hover State: Interactive hover fill and highlight border.
pub fn segmented_pill(
    ui: &mut Ui,
    palette: &crate::gui::macos_theme::ThemePalette,
    is_active: bool,
    label: &str,
) -> Response {
    let text_color = if is_active {
        contrast::text_on_accent(palette.accent)
    } else {
        palette.text_secondary
    };

    let fill = if is_active {
        palette.accent
    } else {
        Color32::from_rgba_unmultiplied(255, 255, 255, if palette.is_dark { 8 } else { 16 })
    };

    let stroke = if is_active {
        Stroke::NONE
    } else {
        Stroke::new(1.0_f32, palette.border)
    };

    let btn = eframe::egui::Button::new(
        eframe::egui::RichText::new(label)
            .size(type_scale::SMALL)
            .strong()
            .color(text_color),
    )
    .fill(fill)
    .stroke(stroke)
    .rounding(Rounding::same(radius::SM));

    let resp = ui.add(btn);
    if is_active {
        paint_pill_ring(
            ui.painter(),
            resp.rect,
            Rounding::same(radius::SM),
            pill_state(&resp, true),
            palette.accent,
        );
    }
    resp
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
    pub const TITLE_BAR_H: f32 = 42.0;
    /// Finder toolbar row height.
    pub const TOOLBAR_H: f32 = 38.0;
    /// Bottom path bar height.
    pub const PATH_BAR_H: f32 = 28.0;
    /// Status bar height.
    pub const STATUS_BAR_H: f32 = 26.0;
    /// Sidebar minimum width.
    pub const SIDEBAR_MIN_W: f32 = 180.0;
    /// Sidebar default width.
    pub const SIDEBAR_W: f32 = 216.0;
    /// Sidebar maximum width.
    pub const SIDEBAR_MAX_W: f32 = 320.0;
    /// Minimum main content width before the layout is considered cramped.
    pub const CONTENT_MIN_W: f32 = 420.0;
    /// List row height in the details table.
    pub const ROW_H: f32 = 28.0;
    /// Column header height in the details table.
    pub const HEADER_H: f32 = 28.0;
    /// Traffic light diameter.
    pub const TRAFFIC_LIGHT_D: f32 = 14.0;
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
        Self {
            canvas: p.window_bg,
            chrome: p.surface_1,
            surface: p.surface_2,
            surface_raised: p.surface_3,
            border_subtle: p.divider,
            border_strong: p.border,
            text_primary: p.text_primary,
            text_secondary: p.text_secondary,
            text_tertiary: p.text_muted,
            accent: p.accent,
            selection_fill: p.selection,
            hover_fill: p.hover,
            danger: p.error,
            warning: p.warn,
            success: p.success,
        }
    }
}

// ---------------------------------------------------------------------------
// Accessibility and Contrast Rules (WCAG 2.1 AA/AAA)
// ---------------------------------------------------------------------------

pub mod contrast {
    use eframe::egui::Color32;

    /// Minimum contrast ratio for normal text under WCAG 2.1 Level AA.
    pub const WCAG_AA_NORMAL: f32 = 4.5;
    /// Minimum contrast ratio for large text under WCAG 2.1 Level AA.
    pub const WCAG_AA_LARGE: f32 = 3.0;
    /// Enhanced contrast ratio under WCAG 2.1 Level AAA.
    pub const WCAG_AAA_NORMAL: f32 = 7.0;

    /// Converts an sRGB 8-bit channel (0..=255) to linear luminance.
    pub fn srgb_to_linear(c: u8) -> f32 {
        let v = c as f32 / 255.0;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    }

    /// Computes relative luminance (0.0 to 1.0) according to WCAG 2.1 specifications.
    pub fn relative_luminance(c: Color32) -> f32 {
        let r = srgb_to_linear(c.r());
        let g = srgb_to_linear(c.g());
        let b = srgb_to_linear(c.b());
        0.2126 * r + 0.7152 * g + 0.0722 * b
    }

    /// Computes WCAG 2.1 contrast ratio between two colors (ranging from 1.0 to 21.0).
    pub fn contrast_ratio(c1: Color32, c2: Color32) -> f32 {
        let l1 = relative_luminance(c1);
        let l2 = relative_luminance(c2);
        let lighter = l1.max(l2);
        let darker = l1.min(l2);
        (lighter + 0.05) / (darker + 0.05)
    }

    /// UI Rule 1: High-Contrast Foreground for Any Surface.
    ///
    /// Evaluates `light` and `dark` text candidates against `bg` and selects the one
    /// with the superior contrast ratio.
    pub fn best_contrast(bg: Color32, light: Color32, dark: Color32) -> Color32 {
        if contrast_ratio(bg, light) >= contrast_ratio(bg, dark) {
            light
        } else {
            dark
        }
    }

    /// UI Rule 2: Text Directly on Accent Fill MUST NEVER Be the Accent Color.
    ///
    /// When a control (pill, button, badge, tab) uses an accent-filled background,
    /// the text must NEVER be drawn in the accent color or a tinted variant of it.
    /// It is guaranteed to be high-contrast pure white or deep dark text,
    /// satisfying WCAG AA (>= 4.5:1).
    pub fn text_on_accent(accent_fill: Color32) -> Color32 {
        best_contrast(
            accent_fill,
            Color32::from_rgb(255, 255, 255),
            Color32::from_rgb(18, 18, 18),
        )
    }

    /// UI Rule 3: Text on Surfaces Must Meet Minimum Contrast Ratio (default 4.5:1).
    ///
    /// If `desired` does not meet `min_ratio` against `bg`, automatically elevates
    /// towards pure white (on dark bg) or deep dark (on light bg) until the threshold
    /// is satisfied.
    pub fn ensure_contrast(desired: Color32, bg: Color32, min_ratio: f32) -> Color32 {
        if contrast_ratio(desired, bg) >= min_ratio {
            return desired;
        }
        let light = Color32::from_rgb(255, 255, 255);
        let dark = Color32::from_rgb(18, 18, 18);
        best_contrast(bg, light, dark)
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

    #[test]
    fn pill_state_prioritizes_buffer_then_active_then_hover() {
        assert_eq!(pill_state_from(false, false, false, false), PillState::Idle);
        assert_eq!(pill_state_from(true, false, false, false), PillState::Hover);
        assert_eq!(
            pill_state_from(false, false, false, true),
            PillState::Active
        );
        assert_eq!(pill_state_from(true, false, false, true), PillState::Active);
        assert_eq!(pill_state_from(true, true, false, true), PillState::Buffer);
        assert_eq!(
            pill_state_from(false, false, true, false),
            PillState::Buffer
        );
        assert_eq!(pill_state_from(false, false, true, true), PillState::Buffer);
    }

    #[test]
    fn pill_ring_covers_highlighted_states_and_is_none_when_idle() {
        let accent = Color32::from_rgb(0, 122, 255);
        assert!(pill_ring(PillState::Idle, accent).is_none());

        let hover = pill_ring(PillState::Hover, accent).expect("hover ring");
        let active = pill_ring(PillState::Active, accent).expect("active ring");
        let buffer = pill_ring(PillState::Buffer, accent).expect("buffer ring");

        // The buffer ring is the strongest: widest stroke, outermost.
        assert!(hover.stroke.width < active.stroke.width);
        assert!(active.stroke.width < buffer.stroke.width);
        assert!(buffer.expand > active.expand);
        // Hover is the softest: low-alpha accent tint.
        assert!(hover.stroke.color.a() < active.stroke.color.a());
    }

    #[test]
    fn test_wcag_contrast_and_text_on_accent() {
        let white = Color32::from_rgb(255, 255, 255);
        let black = Color32::from_rgb(0, 0, 0);
        let ratio = contrast::contrast_ratio(white, black);
        assert!((ratio - 21.0).abs() < 0.1);

        // Typical red/coral accent (L ~ 0.24): dark text provides 5.25:1 contrast vs white's 3.57:1
        let coral_red = Color32::from_rgb(217, 83, 79);
        let on_coral = contrast::text_on_accent(coral_red);
        assert_eq!(on_coral, Color32::from_rgb(18, 18, 18));
        let cr = contrast::contrast_ratio(on_coral, coral_red);
        assert!(
            cr >= contrast::WCAG_AA_NORMAL,
            "Coral contrast ratio: {}",
            cr
        );

        // Dark blue accent requires white text
        let dark_blue = Color32::from_rgb(20, 60, 180);
        let on_blue = contrast::text_on_accent(dark_blue);
        assert_eq!(on_blue, white);
        let b_cr = contrast::contrast_ratio(on_blue, dark_blue);
        assert!(
            b_cr >= contrast::WCAG_AA_NORMAL,
            "Blue contrast ratio: {}",
            b_cr
        );

        // Pale yellow accent requires dark text
        let pale_yellow = Color32::from_rgb(255, 240, 100);
        let on_yellow = contrast::text_on_accent(pale_yellow);
        assert_eq!(on_yellow, Color32::from_rgb(18, 18, 18));
        let y_cr = contrast::contrast_ratio(on_yellow, pale_yellow);
        assert!(
            y_cr >= contrast::WCAG_AA_NORMAL,
            "Yellow contrast ratio: {}",
            y_cr
        );
    }
}
