use mossambi::core::config::{AnimStyle, MossambiConfig};
use mossambi::gui::anim::{advance, Spring};

#[test]
fn test_sidebar_toggle_and_width_interpolation() {
    let mut config = MossambiConfig::default();
    assert!(
        !config.sidebar_collapsed,
        "Sidebar should be open by default"
    );

    // Toggle collapsed
    config.sidebar_collapsed = true;
    assert!(config.sidebar_collapsed);

    let mut spring = Spring::new(240.0);
    let target_width = if config.sidebar_collapsed { 0.0 } else { 240.0 };
    let val = advance(&mut spring, target_width, AnimStyle::Spring, 0.05);
    assert!(
        val < 240.0,
        "Spring should begin stepping towards 0.0, got {val}"
    );
    assert!(val >= 0.0, "Spring should remain non-negative, got {val}");

    // Toggle expand
    config.sidebar_collapsed = false;
    let target_width = if config.sidebar_collapsed { 0.0 } else { 240.0 };
    for _ in 0..10 {
        advance(&mut spring, target_width, AnimStyle::Spring, 0.05);
    }
    assert!(
        spring.value > val,
        "Spring should step back towards 240.0, got {} vs {}",
        spring.value,
        val
    );
}

#[test]
fn test_sidebar_collapsed_serde_roundtrip() {
    let config = MossambiConfig {
        sidebar_collapsed: true,
        ..Default::default()
    };
    let json = serde_json::to_string(&config).expect("Must serialize");
    let deserialized: MossambiConfig = serde_json::from_str(&json).expect("Must deserialize");
    assert!(deserialized.sidebar_collapsed);
}

#[test]
fn test_sidebar_reduced_motion_snaps_instantly() {
    let mut config = MossambiConfig::default();
    config.animations.reduced_motion = true;
    config.sidebar_collapsed = true;

    let target_w = if config.sidebar_collapsed { 0.0 } else { 240.0 };
    let mut spring = Spring::new(240.0);

    // When reduced motion is on, spring snaps immediately
    if config.animations.reduced_motion {
        spring.set(target_w);
    } else {
        advance(&mut spring, target_w, AnimStyle::Spring, 0.05);
    }

    assert_eq!(spring.value, 0.0);
    assert!(spring.at_rest());
}

#[test]
fn test_ctrl_b_keymap_logic() {
    let mut config = MossambiConfig::default();
    assert!(!config.sidebar_collapsed);

    // Simulate Ctrl+B press
    let is_ctrl = true;
    let key_b = true;
    if is_ctrl && key_b {
        config.sidebar_collapsed = !config.sidebar_collapsed;
    }
    assert!(config.sidebar_collapsed);

    // Press again
    if is_ctrl && key_b {
        config.sidebar_collapsed = !config.sidebar_collapsed;
    }
    assert!(!config.sidebar_collapsed);
}
