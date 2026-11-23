//! Test Donut Toolbar View and Configuration Model Parity
//!
//! Asserts profile navigation, Front/Back slice assignment,
//! extension management, and profile ordering.

use mossambi::core::donut_config::{DonutConfig, DonutSlicePreset};

#[test]
fn test_donut_profiles_count_and_selection() {
    let cfg = DonutConfig::default();
    assert_eq!(cfg.profiles.len(), 20, "Must have exactly 20 profiles");

    // Profile names match standard Filee catalog
    let names: Vec<&str> = cfg.profiles.iter().map(|p| p.name.as_str()).collect();
    assert!(names.contains(&"Images"));
    assert!(names.contains(&"PDF"));
    assert!(names.contains(&"Documents"));
    assert!(names.contains(&"Spreadsheets"));
    assert!(names.contains(&"Presentations"));
    assert!(names.contains(&"HWP / HWPX"));
    assert!(names.contains(&"Text & Markdown"));
    assert!(names.contains(&"E-books"));
    assert!(names.contains(&"Video"));
    assert!(names.contains(&"Audio"));
    assert!(names.contains(&"Subtitles"));
    assert!(names.contains(&"Vector graphics"));
    assert!(names.contains(&"Archives"));
    assert!(names.contains(&"Folders"));
    assert!(names.contains(&"CAD drawings"));
    assert!(names.contains(&"Fonts"));
    assert!(names.contains(&"Data"));
    assert!(names.contains(&"3D models"));
    assert!(names.contains(&"Geospatial"));
    assert!(names.contains(&"Mixed files"));
}

#[test]
fn test_donut_front_and_back_slice_limits() {
    let mut cfg = DonutConfig::default();
    let img_prof = &mut cfg.profiles[0];
    assert_eq!(img_prof.id, "images");
    assert_eq!(img_prof.front_slices.len(), 7);
    assert_eq!(img_prof.back_slices.len(), 8);
    assert_eq!(img_prof.max_front_slices, 12);
    assert_eq!(img_prof.max_back_slices, 12);

    // Can add slices up to 12
    while img_prof.front_slices.len() < img_prof.max_front_slices {
        img_prof.front_slices.push(DonutSlicePreset::format("TIFF"));
    }
    assert_eq!(img_prof.front_slices.len(), 12);

    // Slices beyond 12 should be rejected by caller logic
    if img_prof.front_slices.len() < img_prof.max_front_slices {
        img_prof
            .front_slices
            .push(DonutSlicePreset::format("EXTRA"));
    }
    assert_eq!(img_prof.front_slices.len(), 12);
}

#[test]
fn test_donut_extension_tag_management() {
    let mut cfg = DonutConfig::default();
    let img_prof = &mut cfg.profiles[0];

    assert!(img_prof.extensions.contains(&".png".to_string()));
    assert!(img_prof.extensions.contains(&".jpg".to_string()));

    // Add new extension tag
    let new_ext = ".testext".to_string();
    if !img_prof.extensions.contains(&new_ext) {
        img_prof.extensions.push(new_ext.clone());
    }
    assert!(img_prof.extensions.contains(&new_ext));

    // Remove extension tag
    img_prof.extensions.retain(|e| e != &new_ext);
    assert!(!img_prof.extensions.contains(&new_ext));
}

#[test]
fn test_donut_profile_reordering() {
    let mut cfg = DonutConfig::default();
    let first_id = cfg.profiles[0].id.clone();
    let second_id = cfg.profiles[1].id.clone();

    // Move first down
    cfg.move_profile_down(0);
    assert_eq!(cfg.profiles[0].id, second_id);
    assert_eq!(cfg.profiles[1].id, first_id);

    // Move second back up
    cfg.move_profile_up(1);
    assert_eq!(cfg.profiles[0].id, first_id);
    assert_eq!(cfg.profiles[1].id, second_id);
}
