use eframe::egui::Pos2;
use mossambi_core::{FileCategory, UniversalRegistry};
use mossambi_widget::{RadialWidgetGeometry, WidgetState, WidgetTheme};

#[test]
fn test_widget_geometry_dimensions() {
    let geo = RadialWidgetGeometry::default();
    assert_eq!(geo.center, Pos2::new(180.0, 180.0));
    assert!(geo.hub_radius < geo.inner_radius);
    assert!(geo.inner_radius < geo.outer_radius);
    assert_eq!(geo.theme, WidgetTheme::Light);

    let scaled = RadialWidgetGeometry::new(Pos2::new(200.0, 200.0), 400.0);
    assert_eq!(scaled.center, Pos2::new(200.0, 200.0));
    assert!(scaled.outer_radius > geo.outer_radius);
}

#[test]
fn test_widget_hub_hit_testing() {
    let geo = RadialWidgetGeometry::default();

    // Center point is definitely inside hub
    assert!(geo.hit_test_hub(Pos2::new(180.0, 180.0)));

    // Point close to center is inside hub
    assert!(geo.hit_test_hub(Pos2::new(180.0 + 30.0, 180.0)));
    assert!(geo.hit_test_hub(Pos2::new(180.0, 180.0 - 40.0)));

    // Point at outer edge of hub (hub_radius is 60.0)
    assert!(geo.hit_test_hub(Pos2::new(180.0 + 55.0, 180.0)));

    // Point outside hub radius (> 60.0)
    assert!(!geo.hit_test_hub(Pos2::new(180.0 + 75.0, 180.0)));
    assert!(!geo.hit_test_hub(Pos2::new(180.0, 180.0 + 100.0)));
}

#[test]
fn test_widget_petal_hit_testing() {
    let geo = RadialWidgetGeometry::default();
    let r_mid = (geo.inner_radius + geo.outer_radius) / 2.0;

    // Petal 0 is North (-PI/2) -> y = center.y - r_mid, x = center.x
    let p0 = Pos2::new(geo.center.x, geo.center.y - r_mid);
    assert_eq!(geo.hit_test_petal(p0), Some(0));

    // Petal 2 is East (0) -> x = center.x + r_mid, y = center.y
    let p2 = Pos2::new(geo.center.x + r_mid, geo.center.y);
    assert_eq!(geo.hit_test_petal(p2), Some(2));

    // Petal 4 is South (+PI/2) -> y = center.y + r_mid, x = center.x
    let p4 = Pos2::new(geo.center.x, geo.center.y + r_mid);
    assert_eq!(geo.hit_test_petal(p4), Some(4));

    // Petal 6 is West (PI) -> x = center.x - r_mid, y = center.y
    let p6 = Pos2::new(geo.center.x - r_mid, geo.center.y);
    assert_eq!(geo.hit_test_petal(p6), Some(6));

    // Inside hub -> None
    assert_eq!(geo.hit_test_petal(geo.center), None);

    // Far away outside widget -> None
    assert_eq!(geo.hit_test_petal(Pos2::new(500.0, 500.0)), None);
}

#[test]
fn test_petal_center_calculation() {
    let geo = RadialWidgetGeometry::default();
    let r_mid = (geo.inner_radius + geo.outer_radius) / 2.0;

    // Check petal 0 center coordinates
    let c0 = geo.petal_center(0);
    assert!((c0.x - geo.center.x).abs() < 0.01);
    assert!((c0.y - (geo.center.y - r_mid)).abs() < 0.01);

    // Check petal 2 center coordinates (East)
    let c2 = geo.petal_center(2);
    assert!((c2.x - (geo.center.x + r_mid)).abs() < 0.01);
    assert!((c2.y - geo.center.y).abs() < 0.01);
}

#[test]
fn test_widget_states() {
    let idle = WidgetState::Idle;
    let hovered = WidgetState::Hovered(3);
    let converting = WidgetState::Converting { progress: 0.65 };
    let success = WidgetState::Success { file_count: 2 };

    assert_eq!(idle, WidgetState::Idle);
    assert_ne!(idle, hovered);
    assert_ne!(hovered, converting);
    assert_ne!(converting, success);

    if let WidgetState::Converting { progress } = converting {
        assert!((progress - 0.65).abs() < 0.001);
    } else {
        panic!("Expected converting state");
    }

    if let WidgetState::Success { file_count } = success {
        assert_eq!(file_count, 2);
    } else {
        panic!("Expected success state");
    }
}

#[test]
fn test_widget_theme_toggle() {
    let mut geo = RadialWidgetGeometry::default();
    assert_eq!(geo.theme, WidgetTheme::Light);

    geo.theme = WidgetTheme::Dark;
    assert_eq!(geo.theme, WidgetTheme::Dark);

    geo.theme = WidgetTheme::Light;
    assert_eq!(geo.theme, WidgetTheme::Light);
}

#[test]
fn test_widget_dynamic_category_sectors() {
    // 8 Petals for each media category
    let img_sectors = UniversalRegistry::get_widget_sectors_for_category(FileCategory::Image);
    assert_eq!(img_sectors.len(), 8);
    assert!(img_sectors.contains(&"JPG"));
    assert!(img_sectors.contains(&"WEBP"));
    assert!(img_sectors.contains(&"PNG"));

    let doc_sectors = UniversalRegistry::get_widget_sectors_for_category(FileCategory::Document);
    assert_eq!(doc_sectors.len(), 8);
    assert!(doc_sectors.contains(&"PDF"));
    assert!(doc_sectors.contains(&"DOCX"));

    let audio_sectors = UniversalRegistry::get_widget_sectors_for_category(FileCategory::Audio);
    assert_eq!(audio_sectors.len(), 8);
    assert!(audio_sectors.contains(&"FLAC"));
    assert!(audio_sectors.contains(&"MP3"));
    assert!(audio_sectors.contains(&"WAV"));

    let video_sectors = UniversalRegistry::get_widget_sectors_for_category(FileCategory::Video);
    assert_eq!(video_sectors.len(), 8);
    assert!(video_sectors.contains(&"MP4"));
    assert!(video_sectors.contains(&"MKV"));

    let arch_sectors = UniversalRegistry::get_widget_sectors_for_category(FileCategory::Archive);
    assert_eq!(arch_sectors.len(), 8);
    assert!(arch_sectors.contains(&"ZIP"));
    assert!(arch_sectors.contains(&"7Z"));
    assert!(arch_sectors.contains(&"TAR.GZ"));
}
