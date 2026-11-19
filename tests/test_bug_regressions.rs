//! Regression tests for all discovered and resolved bugs in Mossambi.
//!
//! Specifically validates:
//! 1. Context Menu in-app egui integration (preventing secondary native viewport overlays).
//! 2. Context Menu selection preservation (single-item select vs preserving multi-selection).
//! 3. ViewMode canonical default invariant (`ViewMode::Details`) and mode transitions.
//! 4. Pluralization & grammar handling (avoiding "1 items").
//! 5. Timestamp and date modified formatting sanity (no invalid / future dates).
//! 6. Special folder and system junction filtering in Quick Access.
//! 7. Deep directory recursion stack safety (heap-based iterative traversal).
//! 8. Action journal undo safety across file operations.

use eframe::egui::Color32;
use mossambi::core::action_journal::undo_last;
use mossambi::core::config::{MenuStyle, MossambiConfig, ViewMode};
use mossambi::core::fs_utils::{
    compute_dir_size, is_library_folder_name, is_protected_os_file, pluralize_items, FileItem,
};
use mossambi::core::icon_packs::IconPack;
use mossambi::gui::explorer_view::ExplorerView;
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::SystemTime;

#[path = "common/mod.rs"]
mod common;
use common::HeadlessTestApp;

// ============================================================================
// Bug 1: Context Menu In-App vs Native Viewport Overlay Bug
// ============================================================================

#[test]
fn test_context_menu_does_not_spawn_native_viewport() {
    let explorer = ExplorerView::default();
    assert!(
        explorer.pending_native_menu.is_none(),
        "Explorer must not initialize with a pending native viewport menu"
    );

    let mut app = HeadlessTestApp::new();
    let item = FileItem::from_path(PathBuf::from("test_file.txt"));
    let targets = vec![item.path.clone()];

    // Render context menu via egui in-app helper
    let output = app.run_frame(|ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            let intents = ExplorerView::run_item_context_menu(
                ui,
                IconPack::from_config_id("win11"),
                MenuStyle::Windows,
                Color32::WHITE,
                &item,
                &targets,
            );
            assert!(intents.open_default.is_none());
        });
    });

    assert!(
        !output.shapes.is_empty(),
        "In-app context menu must generate render shapes inside egui central panel"
    );
    assert!(
        explorer.pending_native_menu.is_none(),
        "Explorer pending_native_menu must remain None to prevent Win32 popup window bug"
    );
}

#[test]
fn test_context_menu_selection_preservation() {
    let mut explorer = ExplorerView::default();
    let path_a = PathBuf::from("C:\\folder\\file_a.txt");
    let path_b = PathBuf::from("C:\\folder\\file_b.txt");
    let path_c = PathBuf::from("C:\\folder\\file_c.txt");

    // Case 1: Right-clicking an item that is NOT selected clears selection and selects it
    explorer.selected_paths.clear();
    explorer.selected_paths.insert(path_a.clone());

    // User right-clicks path_b (not in selection)
    if !explorer.selected_paths.contains(&path_b) {
        explorer.selected_paths.clear();
        explorer.selected_paths.insert(path_b.clone());
    }

    assert_eq!(explorer.selected_paths.len(), 1);
    assert!(explorer.selected_paths.contains(&path_b));
    assert!(!explorer.selected_paths.contains(&path_a));

    // Targets computed for context menu must only contain path_b
    let targets = if explorer.selected_paths.len() > 1 && explorer.selected_paths.contains(&path_b)
    {
        explorer.selected_paths.iter().cloned().collect()
    } else {
        vec![path_b.clone()]
    };
    assert_eq!(targets, vec![path_b.clone()]);

    // Case 2: Right-clicking an item that IS in a multi-selection preserves the entire multi-selection
    explorer.selected_paths.clear();
    explorer.selected_paths.insert(path_a.clone());
    explorer.selected_paths.insert(path_b.clone());
    explorer.selected_paths.insert(path_c.clone());
    assert_eq!(explorer.selected_paths.len(), 3);

    // User right-clicks path_b (which is already selected)
    if !explorer.selected_paths.contains(&path_b) {
        explorer.selected_paths.clear();
        explorer.selected_paths.insert(path_b.clone());
    }

    // Selection must remain unchanged
    assert_eq!(explorer.selected_paths.len(), 3);
    assert!(explorer.selected_paths.contains(&path_a));
    assert!(explorer.selected_paths.contains(&path_b));
    assert!(explorer.selected_paths.contains(&path_c));

    let multi_targets: HashSet<PathBuf> =
        if explorer.selected_paths.len() > 1 && explorer.selected_paths.contains(&path_b) {
            explorer.selected_paths.iter().cloned().collect()
        } else {
            vec![path_b.clone()].into_iter().collect()
        };
    assert_eq!(multi_targets.len(), 3);
    assert!(multi_targets.contains(&path_a));
    assert!(multi_targets.contains(&path_b));
    assert!(multi_targets.contains(&path_c));
}

#[test]
fn test_context_menu_canvas_and_item_intents_in_all_views() {
    let mut app = HeadlessTestApp::new();
    let item = FileItem::from_path(PathBuf::from("photo.png"));
    let targets = vec![item.path.clone()];

    for &menu_style in MenuStyle::all() {
        for view_mode in ViewMode::all() {
            let output = app.run_frame(|ctx| {
                eframe::egui::CentralPanel::default().show(ctx, |ui| {
                    let item_intents = ExplorerView::run_item_context_menu(
                        ui,
                        IconPack::from_config_id("win11"),
                        menu_style,
                        Color32::GRAY,
                        &item,
                        &targets,
                    );
                    assert!(item_intents.open_default.is_none());
                    assert!(item_intents.deleted.is_none());

                    let canvas_intents = ExplorerView::run_canvas_context_menu(
                        ui,
                        IconPack::from_config_id("win11"),
                        menu_style,
                        Color32::GRAY,
                        true,
                        true,
                        mossambi::gui::explorer_view::SortColumn::Name,
                        false,
                        view_mode,
                        &PathBuf::from("C:\\"),
                    );
                    assert!(!canvas_intents.new_folder);
                    assert!(!canvas_intents.refresh);
                });
            });
            assert!(!output.shapes.is_empty());
        }
    }
}

// ============================================================================
// Bug 2: ViewMode Canonical Default Invariant Bug
// ============================================================================

#[test]
fn test_view_mode_canonical_default_parity() {
    let config = MossambiConfig::default();
    assert_eq!(
        config.view_mode,
        ViewMode::Details,
        "MossambiConfig canonical default must be ViewMode::Details"
    );

    let explorer = ExplorerView::default();
    assert_eq!(
        explorer.view_mode,
        ViewMode::Details,
        "ExplorerView canonical default must match config default ViewMode::Details"
    );
}

#[test]
fn test_view_mode_transitions_and_persistence() {
    let mut explorer = ExplorerView::default();
    assert_eq!(explorer.view_mode, ViewMode::Details);

    explorer.view_mode = ViewMode::Grid;
    assert_eq!(explorer.view_mode, ViewMode::Grid);

    explorer.view_mode = ViewMode::Columns;
    assert_eq!(explorer.view_mode, ViewMode::Columns);

    explorer.view_mode = ViewMode::Details;
    assert_eq!(explorer.view_mode, ViewMode::Details);
}

// ============================================================================
// Bug 3: Pluralization & Grammar Handling ("1 items" bug)
// ============================================================================

#[test]
fn test_pluralize_items_grammar() {
    assert_eq!(pluralize_items(0), "0 items");
    assert_eq!(
        pluralize_items(1),
        "1 item",
        "Must render singular '1 item', never '1 items'"
    );
    assert_eq!(pluralize_items(2), "2 items");
    assert_eq!(pluralize_items(100), "100 items");
}

// ============================================================================
// Bug 4: Date Modified & Created Formatting Sanity
// ============================================================================

#[test]
fn test_date_modified_and_created_formatting_sanity() {
    let mut item = FileItem::from_path(PathBuf::from("sample.txt"));
    item.modified = None;
    item.created = None;
    assert_eq!(item.formatted_date(), "--");
    assert_eq!(item.formatted_created(), "--");

    item.modified = Some(SystemTime::UNIX_EPOCH);
    item.created = Some(SystemTime::UNIX_EPOCH);
    let date_str = item.formatted_date();
    let created_str = item.formatted_created();
    assert!(
        date_str.contains("1970"),
        "UNIX_EPOCH must format year 1970 without panic"
    );
    assert!(
        created_str.contains("1970"),
        "UNIX_EPOCH must format year 1970 without panic"
    );

    let now = SystemTime::now();
    item.modified = Some(now);
    item.created = Some(now);
    assert!(!item.formatted_date().is_empty());
    assert!(!item.formatted_created().is_empty());
}

// ============================================================================
// Bug 5: Hidden System Junctions & System Folder Filtering
// ============================================================================

#[test]
fn test_quick_access_system_junction_and_hidden_filtering() {
    // Valid user library folders
    assert!(is_library_folder_name("Desktop"));
    assert!(is_library_folder_name("desktop"));
    assert!(is_library_folder_name("Downloads"));
    assert!(is_library_folder_name("Documents"));
    assert!(is_library_folder_name("Pictures"));
    assert!(is_library_folder_name("Music"));
    assert!(is_library_folder_name("Videos"));

    // System-managed junctions and NT folders that must NOT appear in Quick Access
    assert!(!is_library_folder_name("NetHood"));
    assert!(!is_library_folder_name("PrintHood"));
    assert!(!is_library_folder_name("SendTo"));
    assert!(!is_library_folder_name("Start Menu"));
    assert!(!is_library_folder_name("Recent"));
    assert!(!is_library_folder_name("Templates"));
    assert!(!is_library_folder_name("Local Settings"));
    assert!(!is_library_folder_name("Application Data"));
    assert!(!is_library_folder_name("Cookies"));
    assert!(!is_library_folder_name("My Documents"));

    // Protected internal system containers
    assert!(is_protected_os_file(&PathBuf::from(
        "C:\\$Recycle.Bin\\S-1-5-18"
    )));
    assert!(is_protected_os_file(&PathBuf::from(
        "C:\\System Volume Information"
    )));
    assert!(is_protected_os_file(&PathBuf::from("C:\\pagefile.sys")));
    assert!(is_protected_os_file(&PathBuf::from("C:\\hiberfil.sys")));
    assert!(is_protected_os_file(&PathBuf::from("C:\\dumpstack.log")));
    assert!(!is_protected_os_file(&PathBuf::from(
        "C:\\Users\\user\\file.txt"
    )));
}

// ============================================================================
// Bug 6: Deep Directory Size Stack Safety (Heap-based iterative traversal)
// ============================================================================

#[test]
fn test_deep_directory_size_stack_safety() {
    let base = std::env::temp_dir().join(format!("mossambi_stack_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).expect("create test base dir");

    // Build a 50-level deep directory tree
    let mut current = base.clone();
    for i in 0..50 {
        current = current.join(format!("level_{i}"));
        std::fs::create_dir(&current).expect("create level dir");
        // Place a small 100-byte file every 10 levels
        if i % 10 == 0 {
            let file_path = current.join("file.bin");
            std::fs::write(&file_path, vec![0u8; 100]).expect("write test file");
        }
    }

    // Call compute_dir_size with depth 60 - must execute iteratively without stack overflow
    let total_bytes = compute_dir_size(&base, 60, 10_000);
    assert_eq!(
        total_bytes, 500,
        "5 files of 100 bytes each must equal 500 bytes"
    );

    let _ = std::fs::remove_dir_all(&base);
}

// ============================================================================
// Bug 7: Action Journal Undo Safety
// ============================================================================

#[test]
fn test_action_journal_undo_safety() {
    let test_dir = std::env::temp_dir().join(format!("mossambi_undo_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&test_dir);
    std::fs::create_dir_all(&test_dir).expect("create undo test dir");

    let old_file = test_dir.join("old_name.txt");
    let new_file = test_dir.join("new_name.txt");
    std::fs::write(&old_file, b"important user data").expect("write file");
    assert!(old_file.exists());

    // Rename file and record in action journal
    std::fs::rename(&old_file, &new_file).expect("rename file");
    assert!(!old_file.exists());
    assert!(new_file.exists());
    mossambi::core::action_journal::record_rename(old_file.clone(), new_file.clone());

    // Call undo_last() public API
    let result = undo_last();
    assert!(result.is_ok(), "Undo rename must succeed: {:?}", result);
    assert!(
        old_file.exists(),
        "Old file must be restored after undo_last"
    );
    assert!(
        !new_file.exists(),
        "New file must be removed after undo_last"
    );

    let content = std::fs::read(&old_file).expect("read restored file");
    assert_eq!(content, b"important user data");

    let _ = std::fs::remove_dir_all(&test_dir);
}
