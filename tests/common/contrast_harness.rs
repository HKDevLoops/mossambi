//! WCAG 2.1 Contrast & Accessibility Test Harness.
//!
//! Provides mathematically verified luminance calculation and assertion helpers
//! for WCAG 2.1 AA (>= 4.5:1 normal, >= 3.0:1 large) and AAA (>= 7.0:1).

use eframe::egui::Color32;

/// Converts an 8-bit sRGB channel into linear luminance.
#[inline]
pub fn srgb_to_linear(c: u8) -> f64 {
    let v = c as f64 / 255.0;
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

/// Computes the WCAG 2.1 relative luminance of a Color32 (range 0.0 to 1.0).
pub fn relative_luminance(c: Color32) -> f64 {
    let r = srgb_to_linear(c.r());
    let g = srgb_to_linear(c.g());
    let b = srgb_to_linear(c.b());
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// Computes the WCAG 2.1 contrast ratio between two colors (range 1.0:1 to 21.0:1).
pub fn contrast_ratio(fg: Color32, bg: Color32) -> f64 {
    let l1 = relative_luminance(fg);
    let l2 = relative_luminance(bg);
    let lighter = l1.max(l2);
    let darker = l1.min(l2);
    (lighter + 0.05) / (darker + 0.05)
}

/// Asserts that normal text contrast meets or exceeds WCAG 2.1 Level AA (>= 4.5:1).
pub fn assert_wcag_aa(fg: Color32, bg: Color32, context: &str) {
    let cr = contrast_ratio(fg, bg);
    assert!(
        cr >= 4.5,
        "WCAG 2.1 AA Normal Text violation ({:.2}:1 < 4.5:1) in context: {}",
        cr,
        context
    );
}

/// Asserts that large text or graphic elements meet or exceed WCAG 2.1 Level AA (>= 3.0:1).
pub fn assert_wcag_aa_large(fg: Color32, bg: Color32, context: &str) {
    let cr = contrast_ratio(fg, bg);
    assert!(
        cr >= 3.0,
        "WCAG 2.1 AA Large Text violation ({:.2}:1 < 3.0:1) in context: {}",
        cr,
        context
    );
}

/// Asserts that high-contrast elements meet or exceed WCAG 2.1 Level AAA (>= 7.0:1).
pub fn assert_wcag_aaa(fg: Color32, bg: Color32, context: &str) {
    let cr = contrast_ratio(fg, bg);
    assert!(
        cr >= 7.0,
        "WCAG 2.1 AAA violation ({:.2}:1 < 7.0:1) in context: {}",
        cr,
        context
    );
}
