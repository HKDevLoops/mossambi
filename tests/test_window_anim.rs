use mossambi::core::window_anim::{
    genie_mesh_transform, window_anim_geometry, AltTabState, WindowAnimType,
};

#[test]
fn test_genie_effect_polynomial_convergence() {
    let window_rect = (0.0_f32, 0.0_f32, 1000.0_f32, 700.0_f32);
    let dock_target = (500.0_f32, 1050.0_f32); // dock at center bottom

    // At t = 0.0: points are untransformed
    let p0 = genie_mesh_transform(100.0, 150.0, window_rect, dock_target, 0.0);
    assert!((p0.x - 100.0).abs() < 0.1, "At t=0, x must match: {}", p0.x);
    assert!((p0.y - 150.0).abs() < 0.1, "At t=0, y must match: {}", p0.y);

    // At t = 1.0: points converge to the dock target
    let p1 = genie_mesh_transform(100.0, 150.0, window_rect, dock_target, 1.0);
    assert!(
        (p1.x - dock_target.0).abs() < 15.0,
        "At t=1, x must converge near dock target: {}",
        p1.x
    );
    assert!(
        (p1.y - dock_target.1).abs() < 15.0,
        "At t=1, y must converge near dock target: {}",
        p1.y
    );
}

#[test]
fn test_all_five_window_anim_types_supported() {
    for anim in [
        WindowAnimType::Genie,
        WindowAnimType::Scale,
        WindowAnimType::Slide,
        WindowAnimType::Morph,
        WindowAnimType::Fade,
    ] {
        let (scale, alpha, offset_y) = window_anim_geometry(anim, 0.5);
        assert!(
            scale > 0.0 && scale <= 1.0,
            "Scale must be between 0 and 1, got {scale} for {anim:?}"
        );
        assert!(
            (0.0..=1.0).contains(&alpha),
            "Alpha must be between 0 and 1, got {alpha} for {anim:?}"
        );
        assert!(
            offset_y >= 0.0,
            "Y offset must be non-negative for {anim:?}"
        );
    }
}

#[test]
fn test_alt_tab_state_cycling() {
    let mut state = AltTabState::new(vec![
        "Files".to_string(),
        "Transcoder".to_string(),
        "Exam Studio".to_string(),
    ]);
    assert_eq!(state.active_index(), 0);

    state.cycle_next();
    assert_eq!(state.active_index(), 1);

    state.cycle_next();
    assert_eq!(state.active_index(), 2);

    // Wraps around
    state.cycle_next();
    assert_eq!(state.active_index(), 0);

    // Cycles backward
    state.cycle_prev();
    assert_eq!(state.active_index(), 2);
}
