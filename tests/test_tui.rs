use mossambi::tui::{AppState, ExplorerLayoutMode};
use std::path::PathBuf;

#[test]
fn test_tui_explorer_initial_state() {
    let state = AppState::new();
    assert_eq!(state.explorer_layout, ExplorerLayoutMode::Miller3Column);
    assert!(!state.drives.is_empty());
    assert!(!state.search_mode);
    assert!(state.search_query.is_empty());
    assert!(!state.show_drive_modal);
}

#[test]
fn test_tui_layout_cycle() {
    let mut layout = ExplorerLayoutMode::Miller3Column;
    layout = layout.cycle();
    assert_eq!(layout, ExplorerLayoutMode::FullScreen);
    layout = layout.cycle();
    assert_eq!(layout, ExplorerLayoutMode::DualPane);
    layout = layout.cycle();
    assert_eq!(layout, ExplorerLayoutMode::Miller3Column);
}

#[test]
fn test_tui_selection_toggle() {
    let mut state = AppState::new();
    let dummy_path = PathBuf::from("test_file.png");
    state.selected_paths.insert(dummy_path.clone());
    assert!(state.selected_paths.contains(&dummy_path));

    state.selected_paths.remove(&dummy_path);
    assert!(!state.selected_paths.contains(&dummy_path));
}
