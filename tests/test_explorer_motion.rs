use mossambi::core::config::AnimationSettings;
use mossambi::gui::anim::{advance, AnimStyle, Spring};

#[test]
fn test_animation_settings_explorer_motion_fields() {
    let anims = AnimationSettings::default();
    assert!(
        anims.folder_nav_motion,
        "Folder nav motion should be enabled by default"
    );
    assert!(
        anims.click_feedback,
        "Click feedback should be enabled by default"
    );
}

#[test]
fn test_folder_navigation_drift_math() {
    // When entering folder: starts at +14px offset and steps to 0.0
    let mut offset_spring = Spring::new(14.0);
    offset_spring.target = 0.0;
    let step1 = advance(&mut offset_spring, 0.0, AnimStyle::Natural, 0.05);
    assert!(
        (0.0..14.0).contains(&step1),
        "Offset should step toward 0.0, got {step1}"
    );

    // When backing out: starts at -14px offset and steps to 0.0
    let mut back_spring = Spring::new(-14.0);
    back_spring.target = 0.0;
    let step2 = advance(&mut back_spring, 0.0, AnimStyle::Natural, 0.05);
    assert!(
        (-14.0..=0.0).contains(&step2),
        "Offset should step toward 0.0, got {step2}"
    );
}

#[test]
fn test_click_depth_scale_spring() {
    // Tactile press: dips to 0.985 and springs back to 1.000
    let mut scale_spring = Spring::new(0.985);
    scale_spring.target = 1.0;
    scale_spring.stiffness = 320.0;
    let step = advance(&mut scale_spring, 1.0, AnimStyle::Snappy, 0.05);
    assert!(
        step > 0.985 && step <= 1.005,
        "Scale should recover towards 1.0, got {step}"
    );
}
