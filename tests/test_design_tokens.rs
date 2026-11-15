//! Unit Tests for DesignStyle Presets, DesignTokens, and Semantic Palette Parity

use mossambi::core::{DesignStyle, MossambiConfig};
use mossambi::gui::macos_theme::ThemeStyle;

#[test]
fn test_design_style_defaults_and_labels() {
    let default_style = DesignStyle::default();
    assert_eq!(default_style, DesignStyle::ArcaneFluid);
    assert_eq!(default_style.label(), "Arcane Fluid");

    for style in DesignStyle::all() {
        assert!(!style.label().is_empty());
        assert!(!style.description().is_empty());
    }
}

#[test]
fn test_design_style_serde_roundtrip() {
    for style in DesignStyle::all() {
        let json = serde_json::to_string(&style).expect("serialize design style");
        let parsed: DesignStyle = serde_json::from_str(&json).expect("deserialize design style");
        assert_eq!(style, parsed);
    }

    // Verify snake_case serde keys
    assert_eq!(
        serde_json::to_string(&DesignStyle::ArcaneFluid).unwrap(),
        "\"arcane_fluid\""
    );
    assert_eq!(
        serde_json::to_string(&DesignStyle::ZenithPro).unwrap(),
        "\"zenith_pro\""
    );
    assert_eq!(
        serde_json::to_string(&DesignStyle::HyperionSlate).unwrap(),
        "\"hyperion_slate\""
    );
}

#[test]
fn test_config_design_style_persistence() {
    let mut cfg = MossambiConfig::default();
    assert_eq!(cfg.design_style, DesignStyle::ArcaneFluid);
    assert_eq!(cfg.tokens().radius_card, 10.0);

    cfg.design_style = DesignStyle::ZenithPro;
    let serialized = serde_json::to_string(&cfg).expect("serialize config");
    let restored: MossambiConfig = serde_json::from_str(&serialized).expect("deserialize config");
    assert_eq!(restored.design_style, DesignStyle::ZenithPro);
    assert_eq!(restored.tokens().radius_card, 4.0);

    // Default fallback when design_style is omitted from JSON
    let mut val: serde_json::Value = serde_json::to_value(MossambiConfig::default()).unwrap();
    if let serde_json::Value::Object(ref mut map) = val {
        map.remove("design_style");
    }
    let restored_default: MossambiConfig =
        serde_json::from_value(val).expect("deserialize config without design_style");
    assert_eq!(restored_default.design_style, DesignStyle::ArcaneFluid);
}

#[test]
fn test_design_tokens_for_all_presets() {
    let arcane = DesignStyle::ArcaneFluid.tokens();
    let zenith = DesignStyle::ZenithPro.tokens();
    let hyperion = DesignStyle::HyperionSlate.tokens();

    // 8pt grid consistency across all styles
    for tokens in [arcane, zenith, hyperion] {
        assert_eq!(tokens.space_xxs, 4.0);
        assert_eq!(tokens.space_xs, 8.0);
        assert_eq!(tokens.space_sm, 12.0);
        assert_eq!(tokens.space_md, 16.0);
        assert_eq!(tokens.space_lg, 24.0);
        assert_eq!(tokens.space_xl, 32.0);
        assert!(tokens.max_content_width >= 1200.0);
        assert!(tokens.min_glass_opacity >= 0.70);
    }

    // Distinct style characteristics
    assert_eq!(arcane.radius_card, 10.0);
    assert_eq!(arcane.radius_pill, 6.0);
    assert_eq!(arcane.max_content_width, 1440.0);

    assert_eq!(zenith.radius_card, 4.0);
    assert_eq!(zenith.radius_pill, 3.0);
    assert_eq!(zenith.min_glass_opacity, 0.95);
    assert_eq!(zenith.max_content_width, 1600.0);

    assert_eq!(hyperion.radius_card, 12.0);
    assert_eq!(hyperion.radius_pill, 8.0);
    assert_eq!(hyperion.border_width, 1.5);
    assert_eq!(hyperion.max_content_width, 1500.0);
}

#[test]
fn test_theme_palette_semantic_roles_and_contrast() {
    for style in ThemeStyle::all() {
        let pal = style.colors_with_glass(mossambi::core::GlassEffect::Opaque, 1.0);

        // Verify all 16 semantic roles are populated with full opacity in opaque mode
        assert_eq!(pal.surface_1.a(), 255);
        assert_eq!(pal.surface_2.a(), 255);
        assert_eq!(pal.surface_3.a(), 255);
        assert_eq!(pal.selection.a(), 255);
        assert_eq!(pal.hover.a(), 255);
        assert_eq!(pal.accent_alt.a(), 255);
        assert_eq!(pal.success.a(), 255);
        assert_eq!(pal.warn.a(), 255);
        assert_eq!(pal.error.a(), 255);
        assert_eq!(pal.tag_code.a(), 255);
        assert_eq!(pal.tag_media.a(), 255);
        assert_eq!(pal.tag_archive.a(), 255);
        assert_eq!(pal.tag_document.a(), 255);
        assert_eq!(pal.tag_image.a(), 255);

        // Verify contrast between primary text and card background
        let pt = style.to_pantone();
        let cr = pt.text_primary.contrast_ratio(&pt.surface_2);
        assert!(
            cr >= 4.5,
            "Style '{:?}' text_primary vs surface_2 contrast {:.2} < 4.5",
            style,
            cr
        );
    }
}
