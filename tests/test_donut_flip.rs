// tests/test_donut_flip.rs
//
// Regression tests for Donut two-face toggle state, 180ms 3D flip animation,
// and Tab/wheel face transitions.
//
// Pure 7-bit ASCII.

use mossambi::widget::{
    face_flip_3d_scale, handle_donut_key, DonutFace, DonutKeyAction, MossambiWidgetApp,
};
use std::path::PathBuf;

#[test]
fn test_donut_face_toggle_state() {
    let face = DonutFace::PrimaryFormats;
    assert!(face.is_front());
    assert!(!face.is_back());

    let toggled = face.toggle();
    assert!(toggled.is_back());
    assert!(!toggled.is_front());

    let toggled_again = toggled.toggle();
    assert!(toggled_again.is_front());
    assert_eq!(toggled_again, DonutFace::PrimaryFormats);
}

#[test]
fn test_face_flip_3d_scale_animation() {
    // 0ms: fully open front face
    let (scale_0, alpha_0) = face_flip_3d_scale(0.0);
    assert!((scale_0 - 1.0).abs() < 1e-4, "scale at 0s must be 1.0");
    assert!((alpha_0 - 1.0).abs() < 1e-4);

    // 90ms (midpoint of 180ms): edge-on perspective (near 0.0)
    let (scale_mid, _) = face_flip_3d_scale(0.090);
    assert!(
        scale_mid < 0.15,
        "scale at mid-flip (0.090s) must be near 0, got {}",
        scale_mid
    );

    // 180ms: fully open back face
    let (scale_end, alpha_end) = face_flip_3d_scale(0.180);
    assert!(
        (scale_end - 1.0).abs() < 1e-4,
        "scale at end of flip (0.180s) must be 1.0, got {}",
        scale_end
    );
    assert!((alpha_end - 1.0).abs() < 1e-4);
}

#[test]
fn test_tab_key_triggers_toggle_face() {
    let action = handle_donut_key(eframe::egui::Key::Tab, 0, 8, false);
    assert_eq!(action, DonutKeyAction::ToggleFace);
}

#[test]
fn test_widget_app_face_switching_and_active_sectors() {
    let mut app = MossambiWidgetApp::new();
    assert!(app.face.is_front());

    // Select an image file to trigger profile loading
    let fake_image = PathBuf::from("test_image.png");
    app.select_file(fake_image);

    let front_sectors: Vec<String> = app
        .active_sectors()
        .into_iter()
        .map(|s| s.to_string())
        .collect();
    assert!(
        !front_sectors.is_empty(),
        "Front sectors should not be empty for image"
    );
    assert!(
        front_sectors.iter().any(|s| s == "PNG" || s == "to-png"),
        "Front face should include PNG format"
    );

    // Flip to Back face
    app.toggle_face();
    assert!(app.face.is_back());
    assert!(app.flip_start.is_some());

    let back_sectors: Vec<String> = app
        .active_sectors()
        .into_iter()
        .map(|s| s.to_string())
        .collect();
    assert!(
        !back_sectors.is_empty(),
        "Back sectors should not be empty"
    );
    // Back face for image should have tools (e.g. Crop, Rotate, Compress)
    assert_ne!(
        front_sectors, back_sectors,
        "Front and Back faces must display different sectors"
    );

    // Flip back to Front
    app.toggle_face();
    assert!(app.face.is_front());
    let current_sectors: Vec<String> = app
        .active_sectors()
        .into_iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(current_sectors, front_sectors);
}
