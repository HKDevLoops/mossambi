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
