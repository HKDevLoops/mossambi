//! Test Preset Editor Model and Persistence Parity
//!
//! Asserts PresetDefinition and PresetOptions serialization,
//! ICC profile retention toggling, and fine-tuning controls.

use mossambi::core::presets::{
    get_builtin_presets, list_all_presets, PresetDefinition, PresetOptions,
};

#[test]
fn test_builtin_presets_coverage() {
    let builtins = get_builtin_presets();
    assert!(!builtins.is_empty(), "Builtin presets must not be empty");

    let ids: Vec<&str> = builtins.iter().map(|p| p.id.as_str()).collect();
    assert!(ids.contains(&"archive-lossless-png"));
    assert!(ids.contains(&"web-ultra-webp"));
    assert!(ids.contains(&"doc-archive-pdf"));
}

#[test]
fn test_preset_options_icc_and_color_grading_defaults() {
    let opts = PresetOptions::default();
    // Default must guarantee lossless color parity
    assert_eq!(opts.keep_icc_profile, Some(true));
    assert_eq!(opts.keep_metadata_exif, Some(true));
    assert_eq!(opts.grayscale, Some(false));
    assert_eq!(opts.remove_background, Some(false));
    assert_eq!(opts.bg_color_hex.as_deref(), Some("#FFFFFF"));
    assert_eq!(opts.debayer_fits, Some(true));
    assert_eq!(opts.save_video_frames, Some(false));
    assert_eq!(opts.pdf_stack_tall_image, Some(false));
}

#[test]
fn test_preset_json_roundtrip_with_icc_preservation() {
    let mut preset = PresetDefinition::new(
        "custom-p3-png",
        "Display P3 PNG",
        "Lossless PNG retaining wide-gamut Display P3 profile",
        "Image",
        &["PNG", "TIFF", "JPG"],
        "PNG",
    );

    preset.options.keep_icc_profile = Some(true);
    preset.options.dpi = Some(600);
    preset.options.remove_background = Some(true);
    preset.options.bg_color_hex = Some("#000000".to_string());
    preset.options.show_on_slice = Some("Custom".to_string());
    preset.options.text_direction = Some("Horizontal".to_string());
    preset.options.short_name = Some("P3 PNG".to_string());

    let json = preset.to_json().expect("Serialization to JSON failed");
    assert!(json.contains("\"keep_icc_profile\": true"));
    assert!(json.contains("\"dpi\": 600"));
    assert!(json.contains("\"bg_color_hex\": \"#000000\""));

    let restored = PresetDefinition::from_json(&json).expect("Deserialization failed");
    assert_eq!(preset, restored);
    assert_eq!(restored.options.keep_icc_profile, Some(true));
    assert_eq!(restored.options.dpi, Some(600));
    assert_eq!(restored.options.bg_color_hex.as_deref(), Some("#000000"));
}

#[test]
fn test_preset_list_and_filter() {
    let all = list_all_presets(None);
    assert!(!all.is_empty());

    let images = list_all_presets(Some("Image"));
    assert!(!images.is_empty());
    for p in images {
        assert_eq!(p.category.to_lowercase(), "image");
    }
}
