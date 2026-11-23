//! Mossambi Standalone Radial Fruit Widget Library
//! 100% frameless, topbarless, transparent circular desktop widget matching `Image reference.png`.

pub mod widget_app;
pub mod widget_geometry;

pub use widget_app::{
    run_widget, run_widget_with_args, DonutFace, MossambiWidgetApp, STUDIO_TOOL_SECTORS,
};
pub use widget_geometry::{
    advance_hover_factor, clamp_donut_diameter, face_flip_3d_scale, face_switch_factors,
    generate_capsule_polygon, gesture_direction_to_wedge, handle_donut_key, hover_growth, hub_hit,
    hub_hit_point, mid_angle, open_animation_factors, outer_submenu_radii, scale_for_dpi,
    sub_wedge_at, wedge_at, wedge_at_point, wedge_at_with_hover, wedge_label_bounds, wedge_span,
    DonutKeyAction, RadialWidgetGeometry, WidgetState, WidgetTheme, DEFAULT_DONUT_DIAMETER,
    DEFAULT_WEDGE_GAP_PX, HOVER_GROW_PX, MARKING_MENU_MIN_DISTANCE, MAX_DONUT_DIAMETER,
    MAX_INNER_RING_WEDGES, MIN_DONUT_DIAMETER,
};
