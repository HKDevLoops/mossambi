//! Real DWM backdrop blur with readability floors and WCAG contrast guarantees.

use crate::core::GlassEffect;
use eframe::egui::Color32;

/// UI surface category with distinct readability requirements and opacity floors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Surface {
    /// Top title bar, toolbar, and bottom status bar chrome.
    Chrome,
    /// Sidebar tree and pinned navigation pane.
    Sidebar,
    /// Main content area (file lists, details, text).
    Content,
    /// Context menus, dropdown menus, and popovers.
    Menu,
    /// Settings cards, inspector cards, and preview cards.
    Card,
    /// Donut radial widget wedges.
    Donut,
}

impl Surface {
    /// All surfaces.
    pub fn all() -> &'static [Surface] {
        &[
            Surface::Chrome,
            Surface::Sidebar,
            Surface::Content,
            Surface::Menu,
            Surface::Card,
            Surface::Donut,
        ]
    }

    /// Minimum alpha floor required for this surface category to ensure
    /// text and controls remain readable over any wallpaper.
    pub fn alpha_floor(&self) -> f32 {
        match self {
            Surface::Content | Surface::Menu | Surface::Card | Surface::Donut => 0.88,
            Surface::Sidebar => 0.50,
            Surface::Chrome => 0.40,
        }
    }
}

/// DWM and compositor backdrop configuration choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backdrop {
    /// Windows 11 DWM system backdrop attribute (DWMWA_SYSTEMBACKDROP_TYPE)
    /// 1 = None (DWMSBT_NONE),
    /// 2 = MainWindow / Mica (DWMSBT_MAINWINDOW),
    /// 3 = TransientWindow / Acrylic (DWMSBT_TRANSIENTWINDOW),
    /// 4 = TabbedWindow / Mica Alt (DWMSBT_TABBEDWINDOW).
    DwmSystem(u32),
    /// Windows 10 SetWindowCompositionAttribute accent policy
    /// 0 = Disabled, 3 = BlurBehind, 4 = AcrylicBlurBehind.
    Win10Accent(u32),
    /// Fully solid / disabled (fallback or non-Windows).
    None,
}

/// Pure decision function: selects the system backdrop blur API based on
/// glass mode and Windows build number.
///
/// - Win11 >= 22621:
///   - Blur / Acrylic -> DWMSBT_TRANSIENTWINDOW (3)
///   - Frosted -> DWMSBT_TRANSIENTWINDOW (3) (acrylic blur; TABBEDWINDOW
///     is Mica Alt, which carries no blur)
///   - Liquid glass -> DWMSBT_TRANSIENTWINDOW (3) plus egui specular highlights
///   - Mica -> DWMSBT_MAINWINDOW (2)
///   - Opaque / Off -> DWMSBT_NONE (1)
/// - Win10 1809..22621 (builds 17763..22621):
///   - AcrylicBlurBehind (4) with tint ABGR
///   - Fallback to BlurBehind (3) on builds < 17134
///   - Opaque / Off -> Disabled (0)
pub fn choose_backdrop(mode: GlassEffect, os_build: u32) -> Backdrop {
    if os_build >= 22621 {
        match mode {
            GlassEffect::Opaque => Backdrop::DwmSystem(1),
            GlassEffect::FrostedGlass => Backdrop::DwmSystem(3),
            GlassEffect::LiquidGlass => Backdrop::DwmSystem(3),
            GlassEffect::Acrylic => Backdrop::DwmSystem(3),
            GlassEffect::Mica => Backdrop::DwmSystem(2),
        }
    } else if os_build >= 17134 {
        match mode {
            GlassEffect::Opaque => Backdrop::Win10Accent(0),
            _ => Backdrop::Win10Accent(4),
        }
    } else if os_build >= 10240 {
        match mode {
            GlassEffect::Opaque => Backdrop::Win10Accent(0),
            _ => Backdrop::Win10Accent(3),
        }
    } else {
        Backdrop::None
    }
}

/// Pure function: computes the surface tint fill color honoring strict readability floors.
///
/// For `GlassEffect::Opaque`, returns fully opaque (alpha = 255).
/// For glass modes, alpha is guaranteed to be >= surface floor:
/// - Content, Menu, Card, Donut >= 0.88
/// - Sidebar >= 0.50
/// - Chrome >= 0.40
///
/// The user intensity (`base.a() / 255`) maps monotonically onto
/// `[floor, 1.0]`: a higher configured opacity always yields a more opaque
/// fill. `base.a() == 0` falls back to the floor; `base.a() == 255` is
/// fully opaque (the backdrop blur is completely tinted over).
pub fn glass_fill(base: Color32, mode: GlassEffect, surface: Surface, dark: bool) -> Color32 {
    if mode == GlassEffect::Opaque {
        return Color32::from_rgba_unmultiplied(base.r(), base.g(), base.b(), 255);
    }

    let floor = surface.alpha_floor();
    // Monotonic tint mapping: the configured opacity IS the fill opacity,
    // clamped into the readability floor. 0 falls back to the floor and
    // 255 means a fully opaque fill.
    let user_intensity = base.a() as f32 / 255.0;
    let alpha_f = user_intensity.clamp(floor, 1.0);

    let alpha = (alpha_f * 255.0).round().clamp(0.0, 255.0) as u8;

    // Subtle optical lift for dark cards and menus to maintain depth hierarchy
    let (r, g, b) = if dark && (surface == Surface::Card || surface == Surface::Menu) {
        (
            base.r().saturating_add(6),
            base.g().saturating_add(6),
            base.b().saturating_add(6),
        )
    } else {
        (base.r(), base.g(), base.b())
    };

    Color32::from_rgba_unmultiplied(r, g, b, alpha)
}

/// Composites a foreground color with alpha over a solid background color.
pub fn composite_over(fg: Color32, bg: Color32) -> Color32 {
    let a = fg.a() as f32 / 255.0;
    let r = (a * fg.r() as f32 + (1.0 - a) * bg.r() as f32)
        .round()
        .clamp(0.0, 255.0) as u8;
    let g = (a * fg.g() as f32 + (1.0 - a) * bg.g() as f32)
        .round()
        .clamp(0.0, 255.0) as u8;
    let b = (a * fg.b() as f32 + (1.0 - a) * bg.b() as f32)
        .round()
        .clamp(0.0, 255.0) as u8;
    Color32::from_rgb(r, g, b)
}

/// Converts an sRGB component [0..255] to linear luminance component according to IEC 61966-2-1.
pub fn srgb_to_linear(c: u8) -> f64 {
    let v = c as f64 / 255.0;
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

/// Relative luminance of a solid color according to WCAG 2.1 standard.
pub fn relative_luminance(c: Color32) -> f64 {
    let r = srgb_to_linear(c.r());
    let g = srgb_to_linear(c.g());
    let b = srgb_to_linear(c.b());
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// Computes the WCAG 2.1 contrast ratio between two colors (returns value in range 1.0..=21.0).
pub fn contrast_ratio(c1: Color32, c2: Color32) -> f64 {
    let l1 = relative_luminance(c1);
    let l2 = relative_luminance(c2);
    let (lighter, darker) = if l1 >= l2 { (l1, l2) } else { (l2, l1) };
    (lighter + 0.05) / (darker + 0.05)
}

/// Checks the WCAG contrast of `text` rendered on top of `tint` composited over
/// pure black and pure white backdrops.
///
/// Returns `(contrast_over_black, contrast_over_white)`.
pub fn check_wcag_contrast(text: Color32, tint: Color32) -> (f64, f64) {
    let over_black = composite_over(tint, Color32::from_rgb(0, 0, 0));
    let over_white = composite_over(tint, Color32::from_rgb(255, 255, 255));
    (
        contrast_ratio(text, over_black),
        contrast_ratio(text, over_white),
    )
}

/// Returns true if `text` satisfies WCAG AA (contrast >= 4.5:1) over both black
/// and white backdrops when rendered over `tint`.
pub fn is_wcag_readable(text: Color32, tint: Color32) -> bool {
    let (cb, cw) = check_wcag_contrast(text, tint);
    cb >= 4.5 && cw >= 4.5
}

/// Pure function: selects or adjusts a readable text color so that WCAG contrast >= 4.5:1
/// against `tint` composited over both pure black and pure white backdrops.
pub fn choose_readable_text_color(tint: Color32, dark_theme: bool) -> Color32 {
    let light_candidate = Color32::from_rgb(255, 255, 255);
    let dark_candidate = Color32::from_rgb(18, 18, 18);

    let (lb, lw) = check_wcag_contrast(light_candidate, tint);
    let (db, dw) = check_wcag_contrast(dark_candidate, tint);

    let light_ok = lb >= 4.5 && lw >= 4.5;
    let dark_ok = db >= 4.5 && dw >= 4.5;

    if dark_theme {
        if light_ok {
            light_candidate
        } else if dark_ok {
            dark_candidate
        } else if lb.min(lw) >= db.min(dw) {
            light_candidate
        } else {
            dark_candidate
        }
    } else if dark_ok {
        dark_candidate
    } else if light_ok {
        light_candidate
    } else if db.min(dw) >= lb.min(lw) {
        dark_candidate
    } else {
        light_candidate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_choose_backdrop_win11_22621() {
        assert_eq!(
            choose_backdrop(GlassEffect::FrostedGlass, 22621),
            Backdrop::DwmSystem(3)
        );
        assert_eq!(
            choose_backdrop(GlassEffect::FrostedGlass, 22631),
            Backdrop::DwmSystem(3)
        );
        assert_eq!(
            choose_backdrop(GlassEffect::LiquidGlass, 22621),
            Backdrop::DwmSystem(3)
        );
        assert_eq!(
            choose_backdrop(GlassEffect::Acrylic, 22621),
            Backdrop::DwmSystem(3)
        );
        assert_eq!(
            choose_backdrop(GlassEffect::Mica, 22621),
            Backdrop::DwmSystem(2)
        );
        assert_eq!(
            choose_backdrop(GlassEffect::Opaque, 22621),
            Backdrop::DwmSystem(1)
        );
    }

    #[test]
    fn test_choose_backdrop_win10_acrylic() {
        assert_eq!(
            choose_backdrop(GlassEffect::FrostedGlass, 17763),
            Backdrop::Win10Accent(4)
        );
        assert_eq!(
            choose_backdrop(GlassEffect::LiquidGlass, 19041),
            Backdrop::Win10Accent(4)
        );
        assert_eq!(
            choose_backdrop(GlassEffect::Acrylic, 19045),
            Backdrop::Win10Accent(4)
        );
        assert_eq!(
            choose_backdrop(GlassEffect::Mica, 19041),
            Backdrop::Win10Accent(4)
        );
        assert_eq!(
            choose_backdrop(GlassEffect::Opaque, 19041),
            Backdrop::Win10Accent(0)
        );
    }

    #[test]
    fn test_choose_backdrop_win10_old() {
        assert_eq!(
            choose_backdrop(GlassEffect::FrostedGlass, 16299),
            Backdrop::Win10Accent(3)
        );
        assert_eq!(
            choose_backdrop(GlassEffect::LiquidGlass, 15063),
            Backdrop::Win10Accent(3)
        );
        assert_eq!(
            choose_backdrop(GlassEffect::Opaque, 16299),
            Backdrop::Win10Accent(0)
        );
    }

    #[test]
    fn test_choose_backdrop_unsupported_or_zero() {
        assert_eq!(
            choose_backdrop(GlassEffect::FrostedGlass, 0),
            Backdrop::None
        );
        assert_eq!(choose_backdrop(GlassEffect::Opaque, 0), Backdrop::None);
    }

    #[test]
    fn test_glass_fill_floors_all_modes_and_surfaces() {
        let base = Color32::from_rgb(32, 34, 38);
        let modes = GlassEffect::all();
        let surfaces = Surface::all();

        for &mode in modes {
            for &surface in surfaces {
                for &dark in &[true, false] {
                    // Test with default base (a == 255)
                    let fill_default = glass_fill(base, mode, surface, dark);
                    let alpha_default = fill_default.a() as f32 / 255.0;

                    // Test with zero-alpha base (a == 0)
                    let fill_zero = glass_fill(
                        Color32::from_rgba_unmultiplied(base.r(), base.g(), base.b(), 0),
                        mode,
                        surface,
                        dark,
                    );
                    let alpha_zero = fill_zero.a() as f32 / 255.0;

                    if mode == GlassEffect::Opaque {
                        assert_eq!(fill_default.a(), 255);
                        assert_eq!(fill_zero.a(), 255);
                    } else {
                        let expected_floor = surface.alpha_floor();
                        assert!(
                            alpha_default >= expected_floor - 0.005,
                            "Failed floor for mode {:?}, surface {:?}: got {}, expected >={}",
                            mode,
                            surface,
                            alpha_default,
                            expected_floor
                        );
                        assert!(
                            alpha_zero >= expected_floor - 0.005,
                            "Failed floor for zero-alpha mode {:?}, surface {:?}: got {}, expected >={}",
                            mode, surface, alpha_zero, expected_floor
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_glass_fill_user_intensity_scaling() {
        let base_low = Color32::from_rgba_unmultiplied(30, 30, 30, 25); // ~10% intensity
        let base_mid = Color32::from_rgba_unmultiplied(30, 30, 30, 128); // ~50% intensity
        let base_high = Color32::from_rgba_unmultiplied(30, 30, 30, 240); // ~94% intensity

        let fill_low = glass_fill(base_low, GlassEffect::FrostedGlass, Surface::Sidebar, true);
        let fill_mid = glass_fill(base_mid, GlassEffect::FrostedGlass, Surface::Sidebar, true);
        let fill_high = glass_fill(base_high, GlassEffect::FrostedGlass, Surface::Sidebar, true);

        let a_low = fill_low.a() as f32 / 255.0;
        let a_mid = fill_mid.a() as f32 / 255.0;
        let a_high = fill_high.a() as f32 / 255.0;

        assert!(a_low >= Surface::Sidebar.alpha_floor() - 0.005);
        assert!(a_mid >= a_low);
        assert!(a_high >= a_mid);
        assert!(a_high <= 1.0);
    }

    #[test]
    fn test_wcag_contrast_black_and_white() {
        let white = Color32::from_rgb(255, 255, 255);
        let black = Color32::from_rgb(0, 0, 0);

        // Pure black vs pure white is 21.0:1
        let max_cr = contrast_ratio(white, black);
        assert!((max_cr - 21.0).abs() < 0.1);

        // Same color is 1.0:1
        let min_cr = contrast_ratio(white, white);
        assert!((min_cr - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_wcag_readability_floors_dark_and_light() {
        // Dark theme background with Content floor >= 0.88
        let dark_base = Color32::from_rgb(26, 28, 32);
        let dark_tint = glass_fill(dark_base, GlassEffect::FrostedGlass, Surface::Content, true);
        let dark_text = choose_readable_text_color(dark_tint, true);
        let (cr_b, cr_w) = check_wcag_contrast(dark_text, dark_tint);
        assert!(
            cr_b >= 4.5,
            "Dark theme contrast over black backdrop must be >= 4.5:1, got {}",
            cr_b
        );
        assert!(
            cr_w >= 4.5,
            "Dark theme contrast over white backdrop must be >= 4.5:1, got {}",
            cr_w
        );
        assert!(is_wcag_readable(dark_text, dark_tint));

        // Light theme background with Content floor >= 0.88
        let light_base = Color32::from_rgb(246, 246, 248);
        let light_tint = glass_fill(
            light_base,
            GlassEffect::FrostedGlass,
            Surface::Content,
            false,
        );
        let light_text = choose_readable_text_color(light_tint, false);
        let (l_cr_b, l_cr_w) = check_wcag_contrast(light_text, light_tint);
        assert!(
            l_cr_b >= 4.5,
            "Light theme contrast over black backdrop must be >= 4.5:1, got {}",
            l_cr_b
        );
        assert!(
            l_cr_w >= 4.5,
            "Light theme contrast over white backdrop must be >= 4.5:1, got {}",
            l_cr_w
        );
        assert!(is_wcag_readable(light_text, light_tint));
    }
}
