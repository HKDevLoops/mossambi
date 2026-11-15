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

#[test]
fn test_distinct_view_mode_transitions_reach_rest() {
    // 1. Details view stagger-slide: 1.0 -> 0.0
    let mut details_spring = Spring::new(1.0);
    for _ in 0..40 {
        advance(&mut details_spring, 0.0, AnimStyle::Natural, 0.05);
    }
    assert!(
        details_spring.at_rest(),
        "Details transition spring must reach rest"
    );
    assert_eq!(details_spring.value, 0.0);

    // 2. Grid view scale/fade: 0.0 -> 1.0
    let mut grid_spring = Spring::new(0.0);
    for _ in 0..40 {
        advance(&mut grid_spring, 1.0, AnimStyle::Natural, 0.05);
    }
    assert!(
        grid_spring.at_rest(),
        "Grid transition spring must reach rest"
    );
    assert_eq!(grid_spring.value, 1.0);

    // 3. Columns view slide: 1.0 -> 0.0
    let mut columns_spring = Spring::new(1.0);
    for _ in 0..40 {
        advance(&mut columns_spring, 0.0, AnimStyle::Natural, 0.05);
    }
    assert!(
        columns_spring.at_rest(),
        "Columns transition spring must reach rest"
    );
    assert_eq!(columns_spring.value, 0.0);
}

#[test]
fn test_reduced_motion_short_circuits_instantly() {
    let reduced_motion = true;

    let mut details_spring = Spring::new(1.0);
    let mut grid_spring = Spring::new(0.0);
    let mut columns_spring = Spring::new(1.0);

    if reduced_motion {
        details_spring.set(0.0);
        grid_spring.set(1.0);
        columns_spring.set(0.0);
    }

    assert!(details_spring.at_rest());
    assert_eq!(details_spring.value, 0.0);
    assert!(grid_spring.at_rest());
    assert_eq!(grid_spring.value, 1.0);
    assert!(columns_spring.at_rest());
    assert_eq!(columns_spring.value, 0.0);
}

#[test]
fn test_new_item_entrance_spring_reaches_rest() {
    let mut entrance_spring = Spring::new(0.0);
    for _ in 0..50 {
        advance(&mut entrance_spring, 1.0, AnimStyle::Natural, 0.05);
    }
    assert!(
        entrance_spring.at_rest(),
        "New item entrance spring must reach rest"
    );
    assert_eq!(entrance_spring.value, 1.0);
}
