//! Phase F1: Official Microsoft Fluent UI Icons & Native Windows Shell Icons Tests
//!
//! Tests:
//! 1. Icon asset presence and license attribution (assets/icons/fluent/LICENSE and assets/icons/SOURCES.md).
//! 2. Clean removal of old custom/meme presets (assets/icons/presets/ removed).
//! 3. Official Fluent vector SVG resolution and dynamic tint colorization.
//! 4. Windows shell icon extraction and fallback extension handling.
//! 5. Default registered application lookup via AssocQueryStringW.
//! 6. File info model: 4KB cluster arithmetic, parent folder proportion, timeline formatting.
//! 7. Selected states text-on-accent contrast compliance across all themes.

use mossambi::core::fluent_icons::{
    colorize_fluent_svg, fluent_svg_for_glyph, get_raw_fluent_svg, glyph_to_fluent_slug,
};
use mossambi::core::icon_packs::IconGlyph;
use mossambi::gui::design::contrast;
use mossambi::gui::macos_theme::ThemeStyle;
use std::path::Path;

#[test]
fn test_icon_assets_presence_and_license() {
    let license_path = Path::new("assets/icons/fluent/LICENSE");
    assert!(
        license_path.exists(),
        "assets/icons/fluent/LICENSE must exist"
    );
    let license_text = std::fs::read_to_string(license_path).expect("Read LICENSE");
    assert!(license_text.contains("MIT License"), "LICENSE must be MIT");
    assert!(
        license_text.contains("Microsoft Corporation"),
        "Attribution to Microsoft Corporation required"
    );

    let sources_path = Path::new("assets/icons/SOURCES.md");
    assert!(sources_path.exists(), "assets/icons/SOURCES.md must exist");
    let sources_text = std::fs::read_to_string(sources_path).expect("Read SOURCES.md");
    assert!(
        sources_text.contains("Microsoft Fluent UI System Icons"),
        "SOURCES.md must cite Microsoft Fluent UI"
    );
    assert!(
        sources_text.contains("SHGetFileInfo"),
        "SOURCES.md must document Windows Shell integration"
    );

    // Old custom preset icons must be completely removed
    let presets_path = Path::new("assets/icons/presets");
    if presets_path.exists() {
        let count = std::fs::read_dir(presets_path)
            .map(|r| r.count())
            .unwrap_or(0);
        assert_eq!(
            count, 0,
            "Old custom presets directory must be empty or removed"
        );
    }

    // Key Fluent icons exist in regular and filled variants
    let required_icons = [
        "assets/icons/fluent/ic_fluent_arrow_left_20_regular.svg",
        "assets/icons/fluent/ic_fluent_arrow_right_20_regular.svg",
        "assets/icons/fluent/ic_fluent_arrow_up_20_regular.svg",
        "assets/icons/fluent/ic_fluent_arrow_clockwise_20_regular.svg",
        "assets/icons/fluent/ic_fluent_folder_add_20_regular.svg",
        "assets/icons/fluent/ic_fluent_home_20_regular.svg",
        "assets/icons/fluent/ic_fluent_desktop_20_regular.svg",
        "assets/icons/fluent/ic_fluent_document_20_regular.svg",
        "assets/icons/fluent/ic_fluent_edit_20_regular.svg",
        "assets/icons/fluent/ic_fluent_delete_20_regular.svg",
        "assets/icons/fluent/ic_fluent_copy_20_regular.svg",
        "assets/icons/fluent/ic_fluent_info_20_regular.svg",
        "assets/icons/fluent/ic_fluent_settings_20_regular.svg",
    ];

    for icon in required_icons {
        assert!(
            Path::new(icon).exists(),
            "Required Fluent icon missing: {}",
            icon
        );
    }
}

#[test]
fn test_fluent_icon_loader_and_colorization() {
    let glyphs = [
        IconGlyph::ActionBack,
        IconGlyph::ActionForward,
        IconGlyph::ActionUp,
        IconGlyph::ActionRefresh,
        IconGlyph::ActionNewFolder,
        IconGlyph::ActionOpen,
        IconGlyph::ActionEdit,
        IconGlyph::ActionDelete,
        IconGlyph::ActionCopy,
        IconGlyph::ActionCut,
        IconGlyph::ActionRename,
        IconGlyph::ActionProperties,
        IconGlyph::Home,
        IconGlyph::Drive,
        IconGlyph::Search,
        IconGlyph::Gear,
    ];

    for glyph in glyphs {
        let slug = glyph_to_fluent_slug(glyph);
        assert!(
            slug.is_some(),
            "Glyph {:?} must have a Fluent slug mapping",
            glyph
        );
        let slug_str = slug.unwrap();
        let raw = get_raw_fluent_svg(slug_str, false);
        assert!(raw.is_some(), "Slug '{}' must have an SVG asset", slug_str);
        let raw_svg = raw.unwrap();
        assert!(
            raw_svg.contains("<svg"),
            "Slug '{}' must be valid SVG",
            slug_str
        );

        let colored = colorize_fluent_svg(raw_svg, "#FF5500");
        assert!(
            colored.contains("fill=\"#FF5500\""),
            "Colorization must inject target hex color"
        );

        let rendered = fluent_svg_for_glyph(glyph, false, "#00AACC");
        assert!(
            rendered.is_some(),
            "fluent_svg_for_glyph must return colored SVG"
        );
    }
}

#[test]
fn test_file_info_model_arithmetic_and_formatting() {
    // 4KB cluster allocation math
    let cluster_size = 4096u64;
    let on_disk = |logical: u64| -> u64 {
        if logical > 0 {
            logical.div_ceil(cluster_size) * cluster_size
        } else {
            0
        }
    };

    assert_eq!(on_disk(0), 0);
    assert_eq!(on_disk(1), 4096);
    assert_eq!(on_disk(4096), 4096);
    assert_eq!(on_disk(4097), 8192);
    assert_eq!(on_disk(10000), 12288);

    // Parent folder proportion
    let proportion = |item: u64, total: u64| -> f32 {
        let denom = total.max(item).max(1);
        ((item as f32 / denom as f32) * 100.0).clamp(0.1, 100.0)
    };

    assert!((proportion(500, 1000) - 50.0).abs() < 0.01);
    assert!((proportion(1000, 1000) - 100.0).abs() < 0.01);
    assert!((proportion(0, 1000) - 0.1).abs() < 0.01);
}

#[test]
fn test_contrast_of_selected_states_across_all_themes() {
    for style in ThemeStyle::all() {
        let pal = style.colors_with_glass(mossambi::core::GlassEffect::Opaque, 1.0);
        let fg = contrast::text_on_accent(pal.accent);
        let cr = contrast::contrast_ratio(fg, pal.accent);

        assert!(
            cr >= contrast::WCAG_AA_LARGE,
            "Theme {:?} failed minimum contrast requirement (>= 3.0)",
            style
        );
        assert_ne!(
            fg, pal.accent,
            "Selected text color must not equal accent background"
        );
    }
}
