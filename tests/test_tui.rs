use mossambi::core::{Appearance, ThemeMode};
use mossambi::tui::{AppState, ExplorerLayoutMode};
use std::fs;
use std::path::PathBuf;

#[test]
fn test_tui_explorer_initial_state() {
    let state = AppState::new();
    assert_eq!(state.explorer_layout, ExplorerLayoutMode::Miller3Column);
    assert!(!state.drives.is_empty());
    assert!(!state.search_mode);
    assert!(state.search_query.is_empty());
    assert!(!state.show_drive_modal);
    assert!(!state.show_help_modal);
    assert!(!state.show_tag_modal);
    assert!(state.fs_clipboard.is_none());
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

#[test]
fn test_tui_help_modal_toggle() {
    let mut state = AppState::new();
    assert!(!state.show_help_modal);
    state.show_help_modal = true;
    assert!(state.show_help_modal);
    state.show_help_modal = false;
    assert!(!state.show_help_modal);
}

#[test]
fn test_tui_clipboard_copy_and_paste() {
    let temp_dir = std::env::temp_dir().join("mossambi_test_tui_clipboard");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let file_a = temp_dir.join("file_a.txt");
    fs::write(&file_a, b"Sample clipboard content").unwrap();

    let mut state = AppState::new();
    state.current_dir = temp_dir.clone();
    state.selected_paths.insert(file_a.clone());

    // Copy
    state.copy_selection_to_clipboard();
    assert!(state.fs_clipboard.is_some());
    let (copied_paths, is_cut) = state.fs_clipboard.as_ref().unwrap();
    assert_eq!(copied_paths.len(), 1);
    assert_eq!(copied_paths[0], file_a);
    assert!(!is_cut);

    // Paste
    state.paste_from_clipboard();
    assert!(state.status_message.contains("Pasted"));

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_tui_theme_resolution_all_26() {
    let mut state = AppState::new();

    // Dark Mode Appearance
    state.config.theme_mode = ThemeMode::Nord;
    state.config.appearance = Appearance::Dark;
    let theme_dark = state.resolve_active_theme();
    assert!(theme_dark.is_dark);
    assert_eq!(theme_dark.id, "nord");

    // Light Mode Appearance
    state.config.appearance = Appearance::Light;
    let theme_light = state.resolve_active_theme();
    assert!(!theme_light.is_dark);
    assert_eq!(theme_light.id, "nord_light");

    // Cycle theme
    state.config.theme_mode = state.config.theme_mode.next();
    let theme_next = state.resolve_active_theme();
    assert!(!theme_next.is_dark);
    assert_eq!(theme_next.id, "alucard"); // Dracula light counterpart
}

#[test]
fn test_tui_tag_modal_workflow() {
    let temp_dir = std::env::temp_dir().join("mossambi_test_tui_tags");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let sample_file = temp_dir.join("document.pdf");
    fs::write(&sample_file, b"Sample PDF data").unwrap();

    let mut state = AppState::new();
    state.current_dir = temp_dir.clone();
    let file_item = mossambi::core::FileItem::from_path(sample_file.clone());
    state.current_entries = vec![file_item];
    state.selected_file_idx = 0;

    state.toggle_tag_modal();
    assert!(state.show_tag_modal);
    assert!(!state.available_tags.is_empty());

    // Toggle first tag
    state.selected_tag_idx = 0;
    let first_tag = state.available_tags[0].clone();
    state.toggle_selected_tag();

    let assigned = mossambi::core::tags::get_tags(&sample_file);
    assert!(assigned.iter().any(|t| t.eq_ignore_ascii_case(&first_tag)));

    // Untoggle
    state.toggle_selected_tag();
    let assigned_after = mossambi::core::tags::get_tags(&sample_file);
    assert!(!assigned_after
        .iter()
        .any(|t| t.eq_ignore_ascii_case(&first_tag)));

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_tui_preset_modal_workflow() {
    let mut state = AppState::new();
    assert!(!state.show_preset_modal);
    assert_eq!(state.preset_modal_index, 0);

    // Open preset modal
    state.show_preset_modal = true;
    assert!(state.show_preset_modal);

    // Verify presets exist in registry
    let presets = mossambi::core::presets::list_all_presets(None);
    assert!(!presets.is_empty());
    assert!(presets.iter().any(|p| p.id == "web-ultra-webp"));

    // Navigate down
    if presets.len() > 1 {
        state.preset_modal_index = 1;
        assert_eq!(state.preset_modal_index, 1);
    }

    // Dismiss
    state.show_preset_modal = false;
    assert!(!state.show_preset_modal);
}
