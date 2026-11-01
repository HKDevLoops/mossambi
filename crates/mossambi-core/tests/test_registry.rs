use mossambi_core::{
    get_system_arch_info, resolve_sibling_output_path, CitrusShade, FileCategory,
    PantonePalette, UniversalRegistry,
};
use std::path::Path;

#[test]
fn test_universal_registry_classification() {
    assert_eq!(UniversalRegistry::classify_extension("png"), FileCategory::Image);
    assert_eq!(UniversalRegistry::classify_extension("FLAC"), FileCategory::Audio);
    assert_eq!(UniversalRegistry::classify_extension("mp4"), FileCategory::Video);
    assert_eq!(UniversalRegistry::classify_extension("docx"), FileCategory::Document);
    assert_eq!(UniversalRegistry::classify_extension("xlsx"), FileCategory::Spreadsheet);
    assert_eq!(UniversalRegistry::classify_extension("tar.gz"), FileCategory::Archive);
    assert_eq!(UniversalRegistry::classify_extension("srt"), FileCategory::Subtitle);
}

#[test]
fn test_eight_petal_widget_layout_has_all_sector() {
    for cat in [
        FileCategory::Image,
        FileCategory::Video,
        FileCategory::Audio,
        FileCategory::Document,
        FileCategory::Spreadsheet,
        FileCategory::Archive,
        FileCategory::Subtitle,
    ] {
        let sectors = UniversalRegistry::get_widget_sectors_for_category(cat);
        assert_eq!(sectors.len(), 8, "Category {:?} must have exactly 8 radial sectors", cat);
        assert_eq!(*sectors.last().unwrap(), "ALL", "Category {:?} sector 8 must be golden 'ALL'", cat);
    }
}

#[test]
fn test_non_destructive_sibling_output_path() {
    let input = Path::new("test_data").join("audio_recording.flac");
    let out = resolve_sibling_output_path(&input, None, "WAV", None);
    assert_eq!(out, Path::new("test_data").join("audio_recording_converted.wav"));
}

#[test]
fn test_pantone_certified_swatches() {
    let raw_peel = CitrusShade::RawPeel.to_pantone();
    assert_eq!(raw_peel.code, "Pantone 376 C");
    assert_eq!(raw_peel.hex, "#70C922");

    let gold = PantonePalette::CITRUS_GOLD;
    assert_eq!(gold.code, "Pantone 1235 C");
    assert_eq!(gold.hex, "#FFB81C");
}

#[test]
fn test_dynamic_wheel_layout_flac_example() {
    let sectors = UniversalRegistry::get_dynamic_wheel_sectors(FileCategory::Audio, "FLAC");
    assert!(sectors.len() >= 6 && sectors.len() <= 8, "Wheel must have 6-8 choices");
    
    // Check WAV uncompressed and ALAC lossless
    assert_eq!(sectors[0].format, "WAV");
    assert_eq!(sectors[0].nature, mossambi_core::FormatNature::Uncompressed);
    assert_eq!(sectors[1].format, "ALAC");
    assert_eq!(sectors[1].nature, mossambi_core::FormatNature::Lossless);

    // Check MP3 lossy warning
    assert_eq!(sectors[2].format, "MP3");
    assert_eq!(sectors[2].nature, mossambi_core::FormatNature::Lossy);
    assert!(sectors[2].quality_warning.is_some());

    // Check MORE and ALL
    let has_more = sectors.iter().any(|s| s.is_more && s.format == "MORE");
    let has_all = sectors.iter().any(|s| s.is_all && s.format == "ALL");
    assert!(has_more, "Must have MORE action to browse catalogue");
    assert!(has_all, "Must have ALL dynamic compatible action");
}

#[test]
fn test_search_catalogue() {
    let flac_results = UniversalRegistry::search_catalogue("FLAC");
    assert!(!flac_results.is_empty());
    assert_eq!(flac_results[0].extension, "FLAC");
    assert!(flac_results[0].supports_zero_loss);

    let all_catalog = UniversalRegistry::search_catalogue("");
    assert!(all_catalog.len() >= 30, "Catalog must index all major formats");
}

#[test]
fn test_platform_arch_info() {
    let (os, arch) = get_system_arch_info();
    assert!(!os.is_empty());
    assert!(!arch.is_empty());
}
