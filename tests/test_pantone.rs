//! Unit Tests for Mossambi's 13 Named Themes

use mossambi::core::pantone::{all_pantone_themes, get_pantone_theme, THEMES};
use mossambi::core::ThemeMode;
use mossambi::gui::macos_theme::ThemeStyle;

#[test]
fn test_theme_counts() {
    assert_eq!(
        THEMES.len(),
        13,
        "Catalog must hold exactly 13 named themes"
    );
    assert_eq!(
        all_pantone_themes().len(),
        13,
        "all_pantone_themes must expose all 13"
    );
    assert_eq!(
        THEMES[0..12].len(),
        12,
        "11 canonical palettes + Rangalipi dark"
    );
    assert_eq!(
        THEMES[12..13].len(),
        1,
        "Mossambi Punch is the one light theme"
    );
    assert!(THEMES[0..12].iter().all(|t| t.is_dark));
    assert!(!THEMES[12].is_dark);
    assert_eq!(THEMES[11].id, "rangalipi");
    assert_eq!(THEMES[12].id, "mossambi_punch");
}

#[test]
fn test_theme_integrity() {
    let mut ids: Vec<&'static str> = THEMES.iter().map(|t| t.id).collect();
    ids.sort_unstable();
    let before = ids.len();
    ids.dedup();
    assert_eq!(ids.len(), before, "Theme ids must be unique");
    for theme in &THEMES {
        for c in [
            &theme.bg,
            &theme.surface,
            &theme.border,
            &theme.text_primary,
            &theme.text_secondary,
            &theme.text_muted,
            &theme.accent,
            &theme.highlight,
        ] {
            assert_eq!(c.hex.len(), 7, "hex {} must be #RRGGBB", c.hex);
            assert!(c.hex.starts_with('#'), "hex {} must start with #", c.hex);
            assert!(c.hex[1..].chars().all(|ch| ch.is_ascii_hexdigit()));
            let (r, g, b) = c.to_rgb();
            assert_eq!(
                format!("#{:02X}{:02X}{:02X}", r, g, b).to_lowercase(),
                c.hex.to_lowercase(),
                "RGB roundtrip must match for {}",
                c.hex
            );
            assert!(!c.code.is_empty(), "provenance code must be non-empty");
        }
    }
}

#[test]
fn test_theme_lookup() {
    for theme in &THEMES {
        assert_eq!(
            get_pantone_theme(theme.id).id,
            theme.id,
            "lookup by id must hit {}",
            theme.id
        );
        assert_eq!(
            get_pantone_theme(theme.name).id,
            theme.id,
            "lookup by name must hit {}",
            theme.id
        );
    }
    assert_eq!(
        get_pantone_theme("lightaqua").id,
        "mossambi_punch",
        "legacy light strings fall to the light theme"
    );
    assert_eq!(
        get_pantone_theme("unknown-thing").id,
        "rangalipi",
        "unknown strings fall to the default dark theme"
    );
}

#[test]
fn test_theme_mode_cycles() {
    let all = ThemeMode::all();
    assert_eq!(all.len(), 13);
    assert_eq!(all[0], ThemeMode::Gruvbox);
    assert_eq!(all[11], ThemeMode::Rangalipi);
    assert_eq!(all[12], ThemeMode::MossambiPunch);
    assert_eq!(ThemeMode::dark_themes().len(), 12);
    assert_eq!(ThemeMode::light_themes().len(), 1);
    let mut m = ThemeMode::Gruvbox;
    for _ in 0..13 {
        m = m.next();
    }
    assert_eq!(m, ThemeMode::Gruvbox, "13 next() steps must cycle back");
    assert_eq!(
        ThemeMode::Rangalipi.toggle_dark_light(),
        ThemeMode::MossambiPunch
    );
    assert_eq!(
        ThemeMode::MossambiPunch.toggle_dark_light(),
        ThemeMode::Rangalipi
    );
}

#[test]
fn test_serde_aliases() {
    let liquid: ThemeMode = serde_json::from_str("\"liquid_glass\"").unwrap();
    assert_eq!(liquid, ThemeMode::Rangalipi);
    let dark: ThemeMode = serde_json::from_str("\"dark\"").unwrap();
    assert_eq!(dark, ThemeMode::Rangalipi);
    let old_dark: ThemeMode = serde_json::from_str("\"dark_anthracite\"").unwrap();
    assert_eq!(old_dark, ThemeMode::Rangalipi);
    let light: ThemeMode = serde_json::from_str("\"light\"").unwrap();
    assert_eq!(light, ThemeMode::MossambiPunch);
    let old_light: ThemeMode = serde_json::from_str("\"light_glacier\"").unwrap();
    assert_eq!(old_light, ThemeMode::MossambiPunch);
    assert_eq!(
        serde_json::to_string(&ThemeMode::Rangalipi).unwrap(),
        "\"rangalipi\""
    );
    assert_eq!(
        serde_json::to_string(&ThemeMode::MossambiPunch).unwrap(),
        "\"mossambi_punch\""
    );
}

#[test]
fn test_defaults() {
    assert_eq!(ThemeMode::default(), ThemeMode::Rangalipi);
    assert!(ThemeMode::default().is_dark());
    assert_eq!(ThemeStyle::default(), ThemeStyle::Rangalipi);
    assert_eq!(ThemeStyle::Rangalipi.to_pantone().id, "rangalipi");
}
