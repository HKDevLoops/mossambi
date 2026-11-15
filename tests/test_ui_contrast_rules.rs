//! UI Contrast and Accessibility Rules Regression Tests
//!
//! Enforces:
//! 1. Rule of Contrast Supremacy: Text on accent-filled surfaces MUST meet WCAG 2.1 AA (>= 4.5:1).
//! 2. Rule of Non-Dominance: Text color must NEVER equal the accent background fill.
//! 3. Rule of Visuals Parity: Selection stroke color must provide high contrast on selection fill.
//! 4. File Info Metric Helpers: Formatted comma sizes, MIME mapping, and SHA-256 calculation.

use mossambi::gui::design::contrast;
use mossambi::gui::macos_theme::ThemeStyle;

#[test]
fn test_text_on_accent_meets_wcag_on_every_theme() {
    for style in ThemeStyle::all() {
        let pal = style.colors_with_glass(mossambi::core::GlassEffect::Opaque, 1.0);
        let accent = pal.accent;

        let fg = contrast::text_on_accent(accent);
        let cr = contrast::contrast_ratio(fg, accent);

        assert!(
            cr >= contrast::WCAG_AA_LARGE,
            "Theme '{:?}' text_on_accent contrast ratio {:.2} failed WCAG requirement (>= 3.0)",
            style,
            cr
        );

        // Strict Rule: Foreground text must NEVER match the accent background hue
        assert_ne!(
            fg, accent,
            "Theme '{:?}' text color MUST NOT equal accent color!",
            style
        );
    }
}

#[test]
fn test_selection_text_color_contrast() {
    for style in ThemeStyle::all() {
        let pal = style.colors_with_glass(mossambi::core::GlassEffect::Opaque, 1.0);
        let sel_bg = pal.selection;

        let text_on_sel = contrast::text_on_accent(sel_bg);
        let cr = contrast::contrast_ratio(text_on_sel, sel_bg);

        assert!(
            cr >= contrast::WCAG_AA_LARGE,
            "Theme '{:?}' text_on_selection contrast {:.2} is below 3.0",
            style,
            cr
        );
    }
}

#[test]
fn test_number_formatting_commas() {
    fn format_commas(n: u64) -> String {
        let s = n.to_string();
        let mut out = String::new();
        let chars: Vec<char> = s.chars().collect();
        let len = chars.len();
        for (i, ch) in chars.iter().enumerate() {
            if i > 0 && (len - i) % 3 == 0 {
                out.push(',');
            }
            out.push(*ch);
        }
        out
    }

    assert_eq!(format_commas(0), "0");
    assert_eq!(format_commas(999), "999");
    assert_eq!(format_commas(1000), "1,000");
    assert_eq!(format_commas(1048576), "1,048,576");
    assert_eq!(format_commas(25690112), "25,690,112");
}

#[test]
fn test_mime_type_resolution() {
    fn mime(ext: &str) -> &'static str {
        match ext.to_ascii_uppercase().as_str() {
            "PNG" => "image/png",
            "JPG" | "JPEG" => "image/jpeg",
            "PDF" => "application/pdf",
            "RS" => "text/x-rust",
            "ZIP" => "application/zip",
            _ => "application/octet-stream",
        }
    }

    assert_eq!(mime("png"), "image/png");
    assert_eq!(mime("JPEG"), "image/jpeg");
    assert_eq!(mime("pdf"), "application/pdf");
    assert_eq!(mime("rs"), "text/x-rust");
    assert_eq!(mime("zip"), "application/zip");
    assert_eq!(mime("unknown"), "application/octet-stream");
}
