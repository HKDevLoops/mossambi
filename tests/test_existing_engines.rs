//! Test Existing Engine Discovery and Quality-Gated Fallback Shift
//!
//! Asserts zero-download discovery of system-installed tools
//! (ArtCraft, LibreOffice, FFmpeg, Inkscape, Ghostscript)
//! and routing priority favoring pure native Rust first.

use mossambi::core::existing_engines::{EngineStatus, ExistingEngineRegistry};

#[test]
fn test_existing_engines_detection_is_offline_and_non_blocking() {
    let start = std::time::Instant::now();
    let registry = ExistingEngineRegistry::detect();
    let elapsed = start.elapsed();

    // Detection must complete purely locally via PATH/filesystem check in < 500ms
    assert!(
        elapsed.as_millis() < 500,
        "Engine detection took too long: {:?}",
        elapsed
    );

    // Native Rust engine is always present and active
    let native = registry.get_engine("native-rust");
    assert!(
        native.is_some(),
        "Native Rust engine must always be present"
    );
    let native = native.unwrap();
    assert_eq!(native.status, EngineStatus::Active);
    assert_eq!(native.download_required_mb, 0);
}

#[test]
fn test_known_engine_identifiers() {
    let registry = ExistingEngineRegistry::detect();
    let ids: Vec<&str> = registry.engines.iter().map(|e| e.id.as_str()).collect();

    assert!(ids.contains(&"native-rust"));
    assert!(ids.contains(&"artcraft"));
    assert!(ids.contains(&"libreoffice"));
    assert!(ids.contains(&"ffmpeg"));
    assert!(ids.contains(&"pandoc"));
    assert!(ids.contains(&"inkscape"));
    assert!(ids.contains(&"ghostscript"));
}

#[test]
fn test_route_prioritizes_native_rust_for_standard_codecs() {
    let registry = ExistingEngineRegistry::detect();

    // Standard raster conversion must always use native rust
    let engine = registry.select_engine_for_conversion("PNG", "WEBP");
    assert_eq!(engine.id, "native-rust");

    let engine = registry.select_engine_for_conversion("JPEG", "PNG");
    assert_eq!(engine.id, "native-rust");

    let engine = registry.select_engine_for_conversion("PDF", "PNG");
    assert_eq!(engine.id, "native-rust");
}
