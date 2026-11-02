//! Comprehensive Unit Tests for Mossambi's 20 Certified Pantone Themes

use mossambi::core::pantone::{all_pantone_themes, get_pantone_theme, DARK_THEMES, LIGHT_THEMES};
use mossambi::core::ThemeMode;

#[test]
fn test_theme_counts() {
    assert_eq!(DARK_THEMES.len(), 10, "Must have exactly 10 Certified Dark Pantone Themes");
    assert_eq!(LIGHT_THEMES.len(), 10, "Must have exactly 10 Certified Light Pantone Themes");
    let all = all_pantone_themes();
    assert_eq!(all.len(), 20, "Total certified Pantone themes must be 20");
}

#[test]
fn test_dark_themes_integrity() {
    for theme in &DARK_THEMES {
        assert!(theme.is_dark, "Theme {} must have is_dark = true", theme.name);
        assert!(!theme.id.is_empty());
        assert!(!theme.name.is_empty());

        for color in [
            &theme.bg,
            &theme.surface,
            &theme.border,
            &theme.text_primary,
            &theme.text_secondary,
            &theme.text_muted,
            &theme.accent,
            &theme.highlight,
        ] {
            assert!(color.code.starts_with("Pantone"), "Color {} code must be official Pantone: {}", color.name, color.code);
            assert!(color.hex.starts_with('#') && color.hex.len() == 7, "Color {} hex must be 7 chars: {}", color.name, color.hex);
            let (r, g, b) = color.to_rgb();
            let reconstructed = format!("#{:02X}{:02X}{:02X}", r, g, b);
            assert_eq!(color.hex.to_uppercase(), reconstructed, "Color {} RGB must match hex", color.name);
        }
    }
}

#[test]
fn test_light_themes_integrity() {
    for theme in &LIGHT_THEMES {
        assert!(!theme.is_dark, "Theme {} must have is_dark = false", theme.name);
        assert!(!theme.id.is_empty());
        assert!(!theme.name.is_empty());

        for color in [
            &theme.bg,
            &theme.surface,
            &theme.border,
            &theme.text_primary,
            &theme.text_secondary,
            &theme.text_muted,
            &theme.accent,
            &theme.highlight,
        ] {
            assert!(color.code.starts_with("Pantone"), "Color {} code must be official Pantone: {}", color.name, color.code);
            assert!(color.hex.starts_with('#') && color.hex.len() == 7, "Color {} hex must be 7 chars: {}", color.name, color.hex);
            let (r, g, b) = color.to_rgb();
            let reconstructed = format!("#{:02X}{:02X}{:02X}", r, g, b);
            assert_eq!(color.hex.to_uppercase(), reconstructed, "Color {} RGB must match hex", color.name);
        }
    }
}

#[test]
fn test_theme_mode_enum_and_cycles() {
    let modes = ThemeMode::all();
    assert_eq!(modes.len(), 20);
    assert_eq!(ThemeMode::dark_themes().len(), 10);
    assert_eq!(ThemeMode::light_themes().len(), 10);

    // Test cycling through all 20 themes
    let mut current = modes[0];
    for mode in modes.iter().skip(1) {
        current = current.next();
        assert_eq!(current, *mode);
    }
    // Cycling after last wraps to first
    current = current.next();
    assert_eq!(current, modes[0]);

    // Test dark/light toggle
    assert!(ThemeMode::DarkAnthracite.is_dark());
    let toggled = ThemeMode::DarkAnthracite.toggle_dark_light();
    assert!(!toggled.is_dark());
    assert_eq!(toggled, ThemeMode::LightBrilliant);

    let toggled_back = toggled.toggle_dark_light();
    assert!(toggled_back.is_dark());
    assert_eq!(toggled_back, ThemeMode::DarkAnthracite);
}

#[test]
fn test_get_pantone_theme_lookup() {
    // Exact ID
    let t1 = get_pantone_theme("dark_forest");
    assert_eq!(t1.name, "Deep Forest Emerald");

    // Human readable
    let t2 = get_pantone_theme("Anthracite Obsidian");
    assert_eq!(t2.id, "dark_anthracite");

    // Case-insensitive & punctuation-insensitive
    let t3 = get_pantone_theme("light-glacier");
    assert_eq!(t3.name, "Glacier Aqua");

    let t4 = get_pantone_theme("Sweet Zest Sunlight");
    assert_eq!(t4.id, "light_zest");

    // Fallbacks
    let fallback_dark = get_pantone_theme("nonexistent_dark");
    assert!(fallback_dark.is_dark);

    let fallback_light = get_pantone_theme("some_light_style");
    assert!(!fallback_light.is_dark);
}

#[test]
fn test_serde_theme_mode() {
    for mode in ThemeMode::all() {
        let json = serde_json::to_string(mode).unwrap();
        let deserialized: ThemeMode = serde_json::from_str(&json).unwrap();
        assert_eq!(*mode, deserialized);
    }

    // Test legacy aliases resolve to the identical Pantone themes
    let legacy_dark: ThemeMode = serde_json::from_str("\"liquid_glass\"").unwrap();
    assert_eq!(legacy_dark.to_pantone_theme(), ThemeMode::DarkAnthracite.to_pantone_theme());

    let legacy_frosted: ThemeMode = serde_json::from_str("\"frosted_glass\"").unwrap();
    assert_eq!(legacy_frosted.to_pantone_theme(), ThemeMode::DarkPewter.to_pantone_theme());

    let legacy_light: ThemeMode = serde_json::from_str("\"light_aqua\"").unwrap();
    assert_eq!(legacy_light.to_pantone_theme(), ThemeMode::LightGlacier.to_pantone_theme());
}
