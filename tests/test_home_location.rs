//! Tests for virtual dashboard location vs real filesystem user profile home directory.

use mossambi::core::parse_breadcrumbs;
use mossambi::core::platform_paths::{
    config_dir, dashboard_location, home_dir, is_dashboard_location, DASHBOARD_URI,
};
use mossambi::gui::explorer_view::ExplorerView;
use std::path::{Path, PathBuf};

#[test]
fn test_dashboard_location_distinct_from_filesystem_paths() {
    let dash = dashboard_location();
    assert_eq!(dash.to_string_lossy(), DASHBOARD_URI);

    let home = home_dir();
    assert_ne!(dash, home, "Dashboard location must never equal home_dir");

    let current = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    assert_ne!(
        dash, current,
        "Dashboard location must never equal current_dir"
    );

    let arbitrary_win = PathBuf::from("C:\\Users\\haris");
    assert_ne!(dash, arbitrary_win);

    let arbitrary_unix = PathBuf::from("/home/haris");
    assert_ne!(dash, arbitrary_unix);
}

#[test]
fn test_is_dashboard_location_predicate() {
    let dash = dashboard_location();
    assert!(is_dashboard_location(&dash));
    assert!(is_dashboard_location(Path::new("home:")));

    assert!(!is_dashboard_location(&home_dir()));
    assert!(!is_dashboard_location(Path::new(".")));
    assert!(!is_dashboard_location(Path::new("C:\\Users\\test")));
    assert!(!is_dashboard_location(Path::new("/home/test")));
    assert!(!is_dashboard_location(Path::new("home")));
}

#[test]
fn test_parse_breadcrumbs_for_dashboard_location() {
    let dash = dashboard_location();
    let segments = parse_breadcrumbs(&dash);
    assert_eq!(segments.len(), 1);
    assert_eq!(segments[0].label, "Home");
    assert_eq!(segments[0].target_path, dash);
}

#[test]
fn test_explorer_navigates_between_dashboard_and_real_home() {
    let mut explorer = ExplorerView::default();

    // Default initialization must open the virtual dashboard location
    assert!(
        explorer.is_home_view,
        "Explorer must start in dashboard view"
    );
    assert_eq!(
        explorer.current_dir,
        dashboard_location(),
        "Explorer current_dir must start at dashboard_location"
    );

    // Navigating to the real filesystem user home folder MUST list files, not dashboard
    let real_home = home_dir();
    explorer.navigate_to(real_home.clone());

    assert!(
        !explorer.is_home_view,
        "Navigating to real home folder must NOT set is_home_view"
    );
    assert_eq!(explorer.current_dir, real_home);
    assert_ne!(explorer.current_dir, dashboard_location());

    // Navigating to home dashboard explicitly via navigate_to_home
    explorer.navigate_to_home();
    assert!(
        explorer.is_home_view,
        "navigate_to_home must set is_home_view to true"
    );
    assert_eq!(explorer.current_dir, dashboard_location());

    // Back / forward history should preserve views
    assert!(explorer.can_go_back());
    explorer.go_back();
    assert_eq!(explorer.current_dir, real_home);
    assert!(!explorer.is_home_view);

    assert!(explorer.can_go_forward());
    explorer.go_forward();
    assert_eq!(explorer.current_dir, dashboard_location());
    assert!(explorer.is_home_view);
}

#[test]
fn test_multi_user_profile_isolation() {
    // Verified that config directory is rooted strictly inside home_dir
    let home = home_dir();
    let cfg = config_dir();
    assert!(
        cfg.starts_with(&home),
        "Config directory must be scoped to current user profile"
    );

    // Confirm home directory is valid and not a system-wide root
    assert_ne!(home.to_string_lossy(), "C:\\Users");
    assert_ne!(home.to_string_lossy(), "/home");
}
