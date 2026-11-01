use eframe::egui::Pos2;
use mossambi_gui::citrus_geometry::CitrusGeometry;
use mossambi_gui::gesture::CircleGestureDetector;
use mossambi_gui::quick_settings::QuickSettingsState;
use std::f32::consts::PI;

#[test]
fn test_citrus_geometry_hit_testing() {
    let center = Pos2::new(160.0, 160.0);
    let geom = CitrusGeometry::new(center, 320.0);

    // 1. Hub center hit testing
    assert!(geom.hit_test_hub(center), "Center point must be in hub");
    assert!(geom.hit_test_hub(Pos2::new(160.0, 180.0)), "Point at r=20 must be in hub");
    assert!(!geom.hit_test_hub(Pos2::new(160.0, 240.0)), "Point at r=80 must not be in hub");

    // 2. Carpel hit testing across sectors (at radius r=90)
    let r = 90.0f32;
    let sector_span = (2.0 * PI) / 8.0;

    for i in 0..8 {
        // Sample at the midpoint angle of each sector
        let angle = -PI / 2.0 + (i as f32 + 0.5) * sector_span;
        let test_pt = Pos2::new(center.x + r * angle.cos(), center.y + r * angle.sin());
        let hit = geom.hit_test_carpel(test_pt);
        assert_eq!(hit, Some(i), "Point in sector {} must hit sector {}", i, i);
    }

    // Points outside outer radius
    assert_eq!(geom.hit_test_carpel(Pos2::new(500.0, 500.0)), None);
}

#[test]
fn test_circle_gesture_detector() {
    let mut detector = CircleGestureDetector::new();
    let cx = 300.0f32;
    let cy = 300.0f32;
    let radius = 70.0f32;

    // A. Linear motion should NOT trigger circle gesture
    let mut triggered_linear = false;
    for i in 0..30 {
        let x = cx + (i as f32) * 5.0;
        let y = cy;
        if detector.update(x, y, true) {
            triggered_linear = true;
        }
    }
    assert!(!triggered_linear, "Linear mouse motion must not trigger circle gesture");

    detector.reset();

    // B. Circular clockwise motion tracing 340 degrees (>= 300 threshold)
    let mut triggered_circle = false;
    let steps = 40;
    for s in 0..=steps {
        let theta = (s as f32 / steps as f32) * ((340.0 / 180.0) * PI);
        let x = cx + radius * theta.cos();
        let y = cy + radius * theta.sin();
        if detector.update(x, y, true) {
            triggered_circle = true;
            break;
        }
    }
    assert!(triggered_circle, "Full circular drag motion must trigger circle gesture");
}

#[test]
fn test_quick_settings_defaults_and_presets() {
    let mut qs = QuickSettingsState::default();

    // Verify PDF defaults
    assert_eq!(qs.pdf_page_size, "A4");
    assert_eq!(qs.pdf_font_family, "Helvetica");
    assert_eq!(qs.pdf_font_size, 11.0);
    assert_eq!(qs.pdf_margin, 54.0);
    assert!(qs.pdf_header_footer);

    // Switch format
    qs.set_active_format("MP3");
    assert_eq!(qs.active_format, "MP3");

    qs.set_active_format("UPSC_EXAM");
    assert!(qs.exam_mode_enabled);
}
