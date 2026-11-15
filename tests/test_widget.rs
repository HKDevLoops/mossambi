use eframe::egui::Pos2;
use mossambi::core::{FileCategory, UniversalRegistry};
use mossambi::widget::{RadialWidgetGeometry, WidgetState, WidgetTheme};

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

#[test]
fn test_sub_ring_geometry_and_hit_testing() {
    let geo = RadialWidgetGeometry::default();
    let (sub_inner, sub_outer) = geo.sub_radii();
    assert!(sub_inner > geo.outer_radius);
    assert!(sub_outer > sub_inner);
    assert_eq!(sub_outer - sub_inner, 46.0);

    // Group 0 is at top North (-PI/2)
    // Center of sub-ring radial distance:
    let r_sub_mid = (sub_inner + sub_outer) / 2.0;
    let north_point = Pos2::new(geo.center.x, geo.center.y - r_sub_mid);

    // 4 sub-items: items 0, 1, 2, 3
    // Item 1 or 2 should be close to center angle
    let hit = geo.hit_test_sub_petal(north_point, 0, 4);
    assert!(hit.is_some());
    let idx = hit.unwrap();
    assert!(idx < 4);

    // Point outside sub-ring radius -> None
    let far_point = Pos2::new(geo.center.x, geo.center.y - (sub_outer + 30.0));
    assert_eq!(geo.hit_test_sub_petal(far_point, 0, 4), None);

    // Inside main donut -> None for sub-ring
    let inside_point = Pos2::new(geo.center.x, geo.center.y - geo.outer_radius * 0.5);
    assert_eq!(geo.hit_test_sub_petal(inside_point, 0, 4), None);
}

#[test]
fn test_pie_diameter_clamping() {
    use mossambi::widget::{
        clamp_donut_diameter, DEFAULT_DONUT_DIAMETER, MAX_DONUT_DIAMETER, MIN_DONUT_DIAMETER,
    };

    assert_eq!(clamp_donut_diameter(200.0), MIN_DONUT_DIAMETER);
    assert_eq!(clamp_donut_diameter(279.9), MIN_DONUT_DIAMETER);
    assert_eq!(clamp_donut_diameter(360.0), DEFAULT_DONUT_DIAMETER);
    assert_eq!(clamp_donut_diameter(450.0), 450.0);
    assert_eq!(clamp_donut_diameter(480.0), MAX_DONUT_DIAMETER);
    assert_eq!(clamp_donut_diameter(540.0), MAX_DONUT_DIAMETER);
    assert_eq!(clamp_donut_diameter(1000.0), MAX_DONUT_DIAMETER);
}

#[test]
fn test_hub_hit_functions() {
    use mossambi::widget::{hub_hit, hub_hit_point};

    let hub_r = 60.0;
    assert!(hub_hit(0.0, hub_r));
    assert!(hub_hit(30.0, hub_r));
    assert!(hub_hit(60.0, hub_r));
    assert!(!hub_hit(60.1, hub_r));
    assert!(!hub_hit(100.0, hub_r));

    let center = Pos2::new(180.0, 180.0);
    assert!(hub_hit_point(center, center, hub_r));
    assert!(hub_hit_point(Pos2::new(180.0 + 40.0, 180.0), center, hub_r));
    assert!(hub_hit_point(Pos2::new(180.0, 180.0 - 60.0), center, hub_r));
    assert!(!hub_hit_point(
        Pos2::new(180.0 + 61.0, 180.0),
        center,
        hub_r
    ));
}

#[test]
fn test_wedge_at_boundaries_and_gaps() {
    use mossambi::widget::{wedge_at, wedge_at_with_hover, DEFAULT_WEDGE_GAP_PX, HOVER_GROW_PX};
    use std::f32::consts::PI;

    let inner_r = 50.0;
    let outer_r = 150.0;
    let count = 8;

    // Wedge 0 is centered at North (-PI/2)
    let north = -PI / 2.0;

    // Boundary checks on radius
    assert_eq!(wedge_at(north, 40.0, count, inner_r, outer_r, 0.0), None);
    assert_eq!(wedge_at(north, 50.0, count, inner_r, outer_r, 0.0), Some(0));
    assert_eq!(
        wedge_at(north, 100.0, count, inner_r, outer_r, 0.0),
        Some(0)
    );
    assert_eq!(
        wedge_at(north, 150.0, count, inner_r, outer_r, 0.0),
        Some(0)
    );
    assert_eq!(wedge_at(north, 151.0, count, inner_r, outer_r, 0.0), None);

    // Hover grow extends outer_r by HOVER_GROW_PX
    assert_eq!(
        wedge_at_with_hover(
            north,
            154.0,
            count,
            inner_r,
            outer_r,
            Some(0),
            HOVER_GROW_PX,
            0.0
        ),
        Some(0)
    );
    assert_eq!(
        wedge_at_with_hover(
            north,
            157.0,
            count,
            inner_r,
            outer_r,
            Some(0),
            HOVER_GROW_PX,
            0.0
        ),
        None
    );
    // Non-hovered wedge does not get hover grow
    assert_eq!(
        wedge_at_with_hover(
            north,
            154.0,
            count,
            inner_r,
            outer_r,
            Some(1),
            HOVER_GROW_PX,
            0.0
        ),
        None
    );

    // Gaps: for count = 8, span is PI/4 (45 deg).
    // Wedge 0 spans [-PI/2 - PI/8, -PI/2 + PI/8] = [-5*PI/8, -3*PI/8].
    // Boundary between wedge 0 and wedge 1 is at -3*PI/8.
    let boundary_0_1 = -3.0 * PI / 8.0;
    let mid_r = 100.0;
    // Exactly on boundary with gap_px = 2.0 returns None (in gap)
    assert_eq!(
        wedge_at(
            boundary_0_1,
            mid_r,
            count,
            inner_r,
            outer_r,
            DEFAULT_WEDGE_GAP_PX
        ),
        None
    );

    // Very close to boundary within half_gap returns None
    let half_gap_angle = (DEFAULT_WEDGE_GAP_PX * 0.5) / mid_r;
    assert_eq!(
        wedge_at(
            boundary_0_1 - half_gap_angle * 0.5,
            mid_r,
            count,
            inner_r,
            outer_r,
            DEFAULT_WEDGE_GAP_PX
        ),
        None
    );
    assert_eq!(
        wedge_at(
            boundary_0_1 + half_gap_angle * 0.5,
            mid_r,
            count,
            inner_r,
            outer_r,
            DEFAULT_WEDGE_GAP_PX
        ),
        None
    );

    // Well inside wedge 0 returns Some(0)
    assert_eq!(
        wedge_at(
            boundary_0_1 - half_gap_angle * 3.0,
            mid_r,
            count,
            inner_r,
            outer_r,
            DEFAULT_WEDGE_GAP_PX
        ),
        Some(0)
    );
    // Well inside wedge 1 returns Some(1)
    assert_eq!(
        wedge_at(
            boundary_0_1 + half_gap_angle * 3.0,
            mid_r,
            count,
            inner_r,
            outer_r,
            DEFAULT_WEDGE_GAP_PX
        ),
        Some(1)
    );
}

#[test]
fn test_wedge_counts_1_to_8() {
    use mossambi::widget::{mid_angle, wedge_at, wedge_span};
    use std::f32::consts::PI;

    let inner_r = 50.0;
    let outer_r = 150.0;
    let r = 100.0;

    for count in 1..=8 {
        let span = wedge_span(count);
        assert!((span - (2.0 * PI / (count as f32))).abs() < 1e-4);

        for idx in 0..count {
            let angle = mid_angle(idx, count);
            let hit = wedge_at(angle, r, count, inner_r, outer_r, 0.0);
            assert_eq!(hit, Some(idx), "Failed for count {} idx {}", count, idx);
        }
    }
}

#[test]
fn test_outer_ring_submenu_geometry() {
    use mossambi::widget::{outer_submenu_radii, sub_wedge_at};
    use std::f32::consts::PI;

    let inner_ring_outer_r = 150.0;
    let (sub_inner, sub_outer) = outer_submenu_radii(inner_ring_outer_r);
    assert_eq!(sub_inner, 163.0);
    assert_eq!(sub_outer, 209.0);

    // Parent wedge 0 is at North (-PI/2) with 4 sub-items
    let parent_idx = 0;
    let parent_count = 8;
    let sub_count = 4;
    let sub_mid_r = (sub_inner + sub_outer) / 2.0;

    // Radius boundary checks
    assert_eq!(
        sub_wedge_at(
            -PI / 2.0,
            sub_inner - 10.0,
            parent_idx,
            parent_count,
            sub_count,
            sub_inner,
            sub_outer,
            0.0
        ),
        None
    );
    assert_eq!(
        sub_wedge_at(
            -PI / 2.0,
            sub_outer + 20.0,
            parent_idx,
            parent_count,
            sub_count,
            sub_inner,
            sub_outer,
            0.0
        ),
        None
    );

    // Center of sub-wedge 0 (leftmost) and sub-wedge 3 (rightmost)
    let hit_mid = sub_wedge_at(
        -PI / 2.0 + 0.02,
        sub_mid_r,
        parent_idx,
        parent_count,
        sub_count,
        sub_inner,
        sub_outer,
        0.0,
    );
    assert!(hit_mid.is_some());
    assert!(hit_mid.unwrap() < sub_count);

    // Far angle outside submenu fan returns None
    assert_eq!(
        sub_wedge_at(
            PI / 2.0,
            sub_mid_r,
            parent_idx,
            parent_count,
            sub_count,
            sub_inner,
            sub_outer,
            0.0
        ),
        None
    );
}

#[test]
fn test_marking_menu_gesture_direction_and_threshold() {
    use eframe::egui::Vec2;
    use mossambi::widget::{gesture_direction_to_wedge, MARKING_MENU_MIN_DISTANCE};

    let count = 8;

    // Below distance threshold (24.0px) -> None
    assert_eq!(
        gesture_direction_to_wedge(Vec2::new(0.0, 0.0), count, MARKING_MENU_MIN_DISTANCE),
        None
    );
    assert_eq!(
        gesture_direction_to_wedge(Vec2::new(0.0, -20.0), count, MARKING_MENU_MIN_DISTANCE),
        None
    );
    assert_eq!(
        gesture_direction_to_wedge(Vec2::new(15.0, 15.0), count, MARKING_MENU_MIN_DISTANCE),
        None
    );

    // Exactly at and above threshold:
    // North (0, -30) -> Wedge 0
    assert_eq!(
        gesture_direction_to_wedge(Vec2::new(0.0, -30.0), count, MARKING_MENU_MIN_DISTANCE),
        Some(0)
    );

    // North-East (30, -30) -> Wedge 1
    assert_eq!(
        gesture_direction_to_wedge(Vec2::new(30.0, -30.0), count, MARKING_MENU_MIN_DISTANCE),
        Some(1)
    );

    // East (30, 0) -> Wedge 2
    assert_eq!(
        gesture_direction_to_wedge(Vec2::new(30.0, 0.0), count, MARKING_MENU_MIN_DISTANCE),
        Some(2)
    );

    // South-East (30, 30) -> Wedge 3
    assert_eq!(
        gesture_direction_to_wedge(Vec2::new(30.0, 30.0), count, MARKING_MENU_MIN_DISTANCE),
        Some(3)
    );

    // South (0, 30) -> Wedge 4
    assert_eq!(
        gesture_direction_to_wedge(Vec2::new(0.0, 30.0), count, MARKING_MENU_MIN_DISTANCE),
        Some(4)
    );

    // South-West (-30, 30) -> Wedge 5
    assert_eq!(
        gesture_direction_to_wedge(Vec2::new(-30.0, 30.0), count, MARKING_MENU_MIN_DISTANCE),
        Some(5)
    );

    // West (-30, 0) -> Wedge 6
    assert_eq!(
        gesture_direction_to_wedge(Vec2::new(-30.0, 0.0), count, MARKING_MENU_MIN_DISTANCE),
        Some(6)
    );

    // North-West (-30, -30) -> Wedge 7
    assert_eq!(
        gesture_direction_to_wedge(Vec2::new(-30.0, -30.0), count, MARKING_MENU_MIN_DISTANCE),
        Some(7)
    );
}

#[test]
fn test_keyboard_navigation_mapping() {
    use eframe::egui::Key;
    use mossambi::widget::{handle_donut_key, DonutKeyAction};

    let count = 8;

    // Keys 1..8 select wedges directly
    assert_eq!(
        handle_donut_key(Key::Num1, 0, count, false),
        DonutKeyAction::SelectWedge(0)
    );
    assert_eq!(
        handle_donut_key(Key::Num2, 0, count, false),
        DonutKeyAction::SelectWedge(1)
    );
    assert_eq!(
        handle_donut_key(Key::Num3, 0, count, false),
        DonutKeyAction::SelectWedge(2)
    );
    assert_eq!(
        handle_donut_key(Key::Num4, 0, count, false),
        DonutKeyAction::SelectWedge(3)
    );
    assert_eq!(
        handle_donut_key(Key::Num5, 0, count, false),
        DonutKeyAction::SelectWedge(4)
    );
    assert_eq!(
        handle_donut_key(Key::Num6, 0, count, false),
        DonutKeyAction::SelectWedge(5)
    );
    assert_eq!(
        handle_donut_key(Key::Num7, 0, count, false),
        DonutKeyAction::SelectWedge(6)
    );
    assert_eq!(
        handle_donut_key(Key::Num8, 0, count, false),
        DonutKeyAction::SelectWedge(7)
    );

    // Arrow keys cycle forward/backward
    assert_eq!(
        handle_donut_key(Key::ArrowRight, 0, count, false),
        DonutKeyAction::HoverWedge(1)
    );
    assert_eq!(
        handle_donut_key(Key::ArrowDown, 7, count, false),
        DonutKeyAction::HoverWedge(0)
    );
    assert_eq!(
        handle_donut_key(Key::ArrowLeft, 0, count, false),
        DonutKeyAction::HoverWedge(7)
    );
    assert_eq!(
        handle_donut_key(Key::ArrowUp, 3, count, false),
        DonutKeyAction::HoverWedge(2)
    );

    // Enter confirms
    assert_eq!(
        handle_donut_key(Key::Enter, 2, count, false),
        DonutKeyAction::Confirm
    );

    // Esc closes submenu when in submenu, or closes donut when not
    assert_eq!(
        handle_donut_key(Key::Escape, 0, count, true),
        DonutKeyAction::CloseSubmenu
    );
    assert_eq!(
        handle_donut_key(Key::Escape, 0, count, false),
        DonutKeyAction::Close
    );

    // Tab toggles face
    assert_eq!(
        handle_donut_key(Key::Tab, 0, count, false),
        DonutKeyAction::ToggleFace
    );
}

#[test]
fn test_wedge_label_bounds_placement() {
    use eframe::egui::Vec2;
    use mossambi::widget::{mid_angle, wedge_label_bounds};

    let center = Pos2::new(180.0, 180.0);
    let inner_r = 60.0;
    let outer_r = 160.0;
    let angle0 = mid_angle(0, 8);
    let text_size = Vec2::new(50.0, 16.0);

    let bounds = wedge_label_bounds(center, angle0, inner_r, outer_r, text_size);
    // Petal 0 is North: bounds center should be at x = center.x, y < center.y
    assert!((bounds.center().x - center.x).abs() < 0.1);
    assert!(bounds.center().y < center.y);
    assert!(bounds.width() >= 20.0);
    assert!(bounds.height() >= 12.0);
}

#[test]
fn test_scale_for_dpi_helper() {
    use mossambi::widget::scale_for_dpi;

    assert_eq!(scale_for_dpi(360.0, 1.0), 360.0);
    assert_eq!(scale_for_dpi(360.0, 1.25), 360.0);
    assert_eq!(scale_for_dpi(360.0, 1.5), 360.0);
    assert_eq!(scale_for_dpi(360.0, 2.0), 360.0);
    assert!((scale_for_dpi(360.333, 1.0) - 360.0).abs() < 1e-4);
}

#[test]
fn test_open_animation_factors() {
    use mossambi::widget::open_animation_factors;

    let (s0, a0) = open_animation_factors(0.0);
    assert!((s0 - 0.85).abs() < 1e-4);
    assert!((a0 - 0.0).abs() < 1e-4);

    let (s_mid, a_mid) = open_animation_factors(0.060);
    assert!(s_mid > 0.85 && s_mid < 1.0);
    assert!(a_mid > 0.0 && a_mid < 1.0);

    let (s_end, a_end) = open_animation_factors(0.120);
    assert_eq!(s_end, 1.0);
    assert_eq!(a_end, 1.0);

    let (s_over, a_over) = open_animation_factors(0.500);
    assert_eq!(s_over, 1.0);
    assert_eq!(a_over, 1.0);
}

#[test]
fn test_hover_animation_transition() {
    use mossambi::widget::{advance_hover_factor, hover_growth, HOVER_GROW_PX};

    let f0 = advance_hover_factor(0.0, 1.0, 0.040);
    assert!((f0 - 0.50).abs() < 1e-3);
    assert_eq!(hover_growth(0.0), 0.0);
    assert_eq!(hover_growth(1.0), HOVER_GROW_PX);
    assert!((hover_growth(0.5) - 3.0).abs() < 1e-3);

    let f1 = advance_hover_factor(f0, 1.0, 0.040);
    assert!((f1 - 1.0).abs() < 1e-3);

    let f_down = advance_hover_factor(1.0, 0.0, 0.040);
    assert!((f_down - 0.50).abs() < 1e-3);
}

#[test]
fn test_face_switch_factors() {
    use mossambi::widget::face_switch_factors;
    use std::f32::consts::PI;

    let (rot0, a0) = face_switch_factors(0.0);
    assert!((rot0 - (-PI / 9.0)).abs() < 1e-4);
    assert!((a0 - 0.0).abs() < 1e-4);

    let (rot_mid, a_mid) = face_switch_factors(0.090);
    assert!(rot_mid > -PI / 9.0 && rot_mid < 0.0);
    assert!(a_mid > 0.0 && a_mid < 1.0);

    let (rot_end, a_end) = face_switch_factors(0.180);
    assert_eq!(rot_end, 0.0);
    assert_eq!(a_end, 1.0);
}

#[test]
fn test_generate_capsule_polygon() {
    use mossambi::widget::generate_capsule_polygon;
    use std::f32::consts::PI;

    let center = Pos2::new(180.0, 180.0);
    let pts = generate_capsule_polygon(center, 60.0, 150.0, -PI / 4.0, PI / 4.0, 10.0, 6.0);
    assert!(pts.len() >= 20);
    for p in &pts {
        let dist = (*p - center).length();
        assert!((50.0..=160.0).contains(&dist));
    }
}
