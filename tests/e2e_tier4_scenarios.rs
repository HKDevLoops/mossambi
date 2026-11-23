//! Tier 4: Real-World Application Workloads & Adversarial Stress Scenarios for Mossambi E2E.
//!
//! Enforces end-to-end integration across all subsystems through realistic user workflows
//! and adversarial penetration testing:
//! 1. Cold boot & session restoration workflow
//! 2. Batch folder navigation workflow with display list cache reuse
//! 3. Adversarial path traversal and directory escape penetration tests
//! 4. Adversarial symlink bomb & cyclic directory copy defense
//! 5. Process & shortcut command injection defense
//! 6. 10,000 item directory rapid sort & search performance stress test

#![allow(
    clippy::field_reassign_with_default,
    clippy::manual_range_contains,
    clippy::unnecessary_sort_by,
    clippy::useless_vec
)]

mod common;

use common::{IsolatedConfigEnv, TestFsTree};
use mossambi::core::config::{LookPreset, MossambiConfig, RowDensity, ThemeMode, ViewMode};
use mossambi::core::fs_utils::{list_directory_sync, FileItem};
use mossambi::gui::explorer_view::{ExplorerView, SortColumn, SortDirection};
use mossambi::platform::args::{parse_launch_intent, parse_single_arg, LaunchIntent};
use std::path::{Path, PathBuf};
use std::time::Instant;

// ============================================================================
// Scenario 1: Cold Boot & Complete Session Restoration Workflow
// ============================================================================

#[test]
fn test_tier4_scenario1_cold_boot_and_session_restore() {
    let env = IsolatedConfigEnv::new().expect("create isolated config env");

    // Phase 1: Fresh Installation Cold Boot
    // App boots, user applies custom look preset (Thunar), overrides theme (TokyoNight),
    // configures density (Compact), pins custom folders, and sets custom tags.
    {
        let mut initial_config = MossambiConfig::default();
        initial_config.apply_look_preset(LookPreset::Thunar);
        initial_config.theme_mode = ThemeMode::TokyoNight;
        initial_config.row_density = RowDensity::Compact;
        initial_config.show_custom_pinned = true;
        initial_config.pinned_custom = vec![
            "C:\\Development\\Mossambi".to_string(),
            "D:\\Media\\Audio".to_string(),
        ];
        initial_config.show_tags_section = true;
        initial_config.custom_tags = vec!["Active".to_string(), "Review".to_string()];
        initial_config.set_zoom_for_mode(ViewMode::Details, 0);
        initial_config.set_zoom_for_mode(ViewMode::Grid, 4);

        env.save_config(&initial_config)
            .expect("save initial config on clean shutdown");
    }

    // Phase 2: Application Restart in New Instance
    // Reboots from disk; verify 100% exact state restoration without state drift.
    {
        let restored_config = env
            .load_config()
            .expect("load persisted config on cold boot");

        assert_eq!(restored_config.look_preset, Some(LookPreset::Thunar));
        assert_eq!(restored_config.theme_mode, ThemeMode::TokyoNight);
        assert_eq!(restored_config.row_density, RowDensity::Compact);
        assert_eq!(restored_config.default_explorer_layout, 0); // Details view
        assert!(restored_config.show_custom_pinned);
        assert_eq!(
            restored_config.pinned_custom,
            vec![
                "C:\\Development\\Mossambi".to_string(),
                "D:\\Media\\Audio".to_string(),
            ]
        );
        assert!(restored_config.show_tags_section);
        assert_eq!(
            restored_config.custom_tags,
            vec!["Active".to_string(), "Review".to_string()]
        );
        assert_eq!(restored_config.zoom_for_mode(ViewMode::Details), 0);
        assert_eq!(restored_config.zoom_for_mode(ViewMode::Grid), 4);

        // Verify ChromeSpec derived from restored config matches Thunar
        let spec = restored_config.chrome_spec();
        assert_eq!(
            spec.window_controls,
            mossambi::core::look_spec::WindowControlsPosition::RightCaption
        );
        assert!(spec.menubar, "Thunar preset enables menubar");
    }
}

// ============================================================================
// Scenario 2: Batch Folder Navigation Workflow with Display List Cache Reuse
// ============================================================================

#[test]
fn test_tier4_scenario2_batch_folder_navigation_workflow() {
    let fs = TestFsTree::new().expect("fs");
    let mut explorer = ExplorerView::default();

    // Create 20 distinct directories with varied contents (0 to 200 items each)
    let mut folder_paths = Vec::new();
    for dir_idx in 0..20 {
        let dir_name = format!("workspace_dir_{dir_idx:02}");
        let dir_path = fs.create_dir(&dir_name).expect("dir");
        let item_count = dir_idx * 10; // 0, 10, 20, ..., 190 items
        for item_idx in 0..item_count {
            fs.create_file(
                &format!("{dir_name}/item_{item_idx:03}.txt"),
                b"sample payload",
            )
            .expect("file");
        }
        folder_paths.push(dir_path);
    }

    let start_nav = Instant::now();

    // Simulate rapid navigation through all 20 directories
    for path in &folder_paths {
        explorer.navigate_to(path.clone());
        let entries = list_directory_sync(path);
        explorer.entries = entries;

        // Perform display list sorting and filtering
        let sorted = explorer.filtered_and_sorted_entries();
        assert_eq!(sorted.len(), explorer.entries.len());

        // Verify breadcrumbs correctly reflect the current directory
        let crumbs = mossambi::core::fs_utils::parse_breadcrumbs(path);
        assert!(!crumbs.is_empty());
    }

    // Forward and back navigation history
    assert!(explorer.can_go_back());
    explorer.go_back();
    assert!(explorer.can_go_forward());
    explorer.go_forward();

    let total_nav_time = start_nav.elapsed();
    assert!(
        total_nav_time.as_millis() < 500,
        "20 directory navigation cycle must complete in < 500ms, took {}ms",
        total_nav_time.as_millis()
    );
}

// ============================================================================
// Scenario 3: Adversarial Path Traversal & Directory Escape Attack Simulation
// ============================================================================

#[test]
fn test_tier4_scenario3_adversarial_path_traversal_attack() {
    let fs = TestFsTree::new().expect("fs");

    // 1. Verify OS-protected paths and traversal targets are identified by Mossambi
    let protected_paths = [
        "C:\\pagefile.sys",
        "C:\\hiberfil.sys",
        "C:\\swapfile.sys",
        "C:\\dumpstack.log",
        "C:\\$Recycle.Bin\\S-1-5-18",
        "D:\\System Volume Information",
    ];
    for p in protected_paths {
        assert!(
            mossambi::core::fs_utils::is_protected_os_file(Path::new(p)),
            "System protected file '{}' must be flagged by is_protected_os_file",
            p
        );
        // Mutating operations on protected files must be blocked
        let dest = fs.root().join("escaped.sys");
        assert!(
            mossambi::core::fs_utils::move_entry(Path::new(p), &dest).is_err(),
            "move_entry must refuse to touch protected system file '{}'",
            p
        );
        assert!(
            mossambi::core::fs_utils::remove_entry_recursive(Path::new(p)).is_err(),
            "remove_entry_recursive must refuse to delete protected system file '{}'",
            p
        );
    }

    // 2. Adversarial filename creation in sandbox rejected by Mossambi file operations
    let source_file = fs
        .create_file("source_payload.txt", b"secret")
        .expect("source");
    // Windows forbids < > " | ? *; POSIX forbids '/' and NUL. A NUL byte is
    // illegal on every OS, so use a Windows-specific set there and a NUL-bearing
    // name on POSIX to keep the assertion valid across the whole CI matrix.
    #[cfg(windows)]
    let illegal_names = [
        "bad<less>",
        "bad>greater<",
        "bad\"quote\"",
        "bad|pipe",
        "bad?question",
        "bad*wildcard",
    ];
    #[cfg(not(windows))]
    let illegal_names = ["bad\0nul"];
    for illegal in illegal_names {
        let target = fs.root().join(illegal);
        assert!(
            mossambi::core::fs_utils::move_entry(&source_file, &target).is_err(),
            "move_entry must reject moving file to destination with illegal character: '{}'",
            illegal
        );
        let unique = mossambi::core::fs_utils::resolve_unique_paste_path(&target);
        assert!(!unique.as_os_str().is_empty());
    }

    // 3. Path traversal escape containment check
    let sandbox_dest = fs.root().join("containment.txt");

    // Traversal targets pointing at protected system files must be refused
    let protected_traversals = [
        "../../../../pagefile.sys",
        "../../../../hiberfil.sys",
        "../../../../swapfile.sys",
        "../../../../System Volume Information",
        "../../../../$Recycle.Bin/S-1-5-18",
    ];
    for vector in protected_traversals {
        let trav_path = PathBuf::from(vector);
        assert!(
            mossambi::core::fs_utils::is_protected_os_file(&trav_path),
            "Protected traversal vector '{}' must be flagged by is_protected_os_file",
            vector
        );
        assert!(
            mossambi::core::fs_utils::move_entry(&trav_path, &sandbox_dest).is_err(),
            "move_entry must refuse to move protected traversal target '{}'",
            vector
        );
        assert!(
            mossambi::core::fs_utils::remove_entry_recursive(&trav_path).is_err(),
            "remove_entry_recursive must refuse to delete protected traversal target '{}'",
            vector
        );
    }

    // Non-existent traversal sources must fail cleanly without escapes
    let missing_traversal = fs.root().join("../../../../missing_secret.key");
    assert!(
        mossambi::core::fs_utils::move_entry(&missing_traversal, &sandbox_dest).is_err(),
        "move_entry must fail for missing traversal source"
    );

    // Unique paste path resolution must handle existing items and duplicates safely
    let paste1 = mossambi::core::fs_utils::resolve_unique_paste_path(&source_file);
    assert_eq!(paste1, fs.root().join("source_payload - Copy.txt"));
    let _copy = fs
        .create_file("source_payload - Copy.txt", b"copy")
        .expect("copy");
    let paste2 = mossambi::core::fs_utils::resolve_unique_paste_path(&source_file);
    assert_eq!(paste2, fs.root().join("source_payload - Copy (2).txt"));

    // Breadcrumb parsing must remain safe and non-panicking for traversal vectors
    let test_vectors = [
        "nested/../../secret.key",
        "nested\\..\\..\\config.json",
        "../../../../../../../../Windows/System32/cmd.exe",
    ];
    for vector in test_vectors {
        let crumbs = mossambi::core::fs_utils::parse_breadcrumbs(Path::new(vector));
        assert!(
            !crumbs.is_empty(),
            "Breadcrumbs must safely parse vector '{}'",
            vector
        );
        let item = FileItem::from_path(PathBuf::from(vector));
        assert!(
            !item.name.is_empty(),
            "FileItem must parse traversal vector '{}'",
            vector
        );
    }
}

// ============================================================================
// Scenario 4: Adversarial Symlink Bomb & Cyclic Copy Defense
// ============================================================================

#[test]
fn test_tier4_scenario4_adversarial_symlink_bomb_defense() {
    let fs = TestFsTree::new().expect("fs");

    // Construct a multi-hop circular graph: A -> B -> C -> A
    let dir_a = fs.create_dir("cyclic_graph/node_a").expect("node_a");
    let dir_b = fs.create_dir("cyclic_graph/node_b").expect("node_b");
    let dir_c = fs.create_dir("cyclic_graph/node_c").expect("node_c");

    fs.create_file("cyclic_graph/node_a/data_a.txt", b"A")
        .expect("file");
    fs.create_file("cyclic_graph/node_b/data_b.txt", b"B")
        .expect("file");
    fs.create_file("cyclic_graph/node_c/data_c.txt", b"C")
        .expect("file");

    // Link A -> B, B -> C, C -> A
    let _ = fs.create_symlink_or_junction("cyclic_graph/node_a/link_to_b", &dir_b);
    let _ = fs.create_symlink_or_junction("cyclic_graph/node_b/link_to_c", &dir_c);
    let _ = fs.create_symlink_or_junction("cyclic_graph/node_c/link_to_a", &dir_a);

    let dest = fs.root().join("dest_cyclic_defense");

    let start = Instant::now();
    let result = mossambi::core::fs_utils::copy_entry_recursive(&dir_a, &dest);
    let elapsed = start.elapsed();

    assert!(
        result.is_err(),
        "Cyclic copy on unbounded circular graph must terminate with an error without hanging"
    );
    assert!(
        elapsed.as_secs() < 3,
        "Cyclic copy traversal must terminate in < 3s, took {:?}",
        elapsed
    );

    // Verify clean multi-level tree copy succeeds with production copy_entry_recursive
    let clean_src = fs.create_dir("clean_tree").expect("dir");
    fs.create_file("clean_tree/file1.txt", b"hello world")
        .expect("file");
    fs.create_file("clean_tree/sub/file2.txt", b"nested data")
        .expect("file");
    let clean_dest = fs.root().join("dest_clean");
    let copy_bytes = mossambi::core::fs_utils::copy_entry_recursive(&clean_src, &clean_dest)
        .expect("clean copy");
    assert_eq!(copy_bytes, 22);
    assert!(clean_dest.join("file1.txt").exists());
    assert!(clean_dest.join("sub/file2.txt").exists());
}

// ============================================================================
// Scenario 5: Process & Shortcut Command Injection Defense
// ============================================================================

#[test]
fn test_tier4_scenario5_powershell_and_shortcut_injection_defense() {
    // Malicious filenames designed to trigger command execution if improperly quoted
    let injection_payloads = [
        "sample;calc.exe.txt",
        "file&notepad.exe.txt",
        "data|whoami.txt",
        "test$(Get-Process).png",
        "archive`whoami`.zip",
        "report\" & dir & \".pdf",
        "test'; Invoke-Expression 'calc.exe'; '.txt",
    ];

    for payload in injection_payloads {
        // Test 1: Real Mossambi parse_launch_intent treats malicious args as paths, never shell commands
        let args = ["mossambi.exe", payload];
        let intent = parse_launch_intent(&args);
        assert_eq!(
            intent,
            LaunchIntent::Reveal {
                target: PathBuf::from(payload),
                select: true,
            },
            "Payload '{}' must be parsed as a literal path reveal intent",
            payload
        );

        // Test 2: Real parse_single_arg preserves the verbatim argument as a target
        let single_intent = parse_single_arg(payload);
        assert_eq!(
            single_intent,
            LaunchIntent::Reveal {
                target: PathBuf::from(payload),
                select: true,
            },
            "Payload '{}' must be treated as a single reveal target",
            payload
        );

        // Test 3: Real parse_breadcrumbs handles paths with injection metacharacters safely
        let crumbs = mossambi::core::fs_utils::parse_breadcrumbs(Path::new(payload));
        assert!(
            !crumbs.is_empty(),
            "Breadcrumbs must parse without panicking on payload '{}'",
            payload
        );
    }

    // Verify LaunchIntent parser does not execute or split paths with semicolons or ampersands
    let complex_path = "C:\\Data;calc\\file&test.txt";
    let intent = parse_single_arg(complex_path);
    assert_eq!(
        intent,
        LaunchIntent::Reveal {
            target: PathBuf::from(complex_path),
            select: true,
        },
        "Argument parser must preserve literal path without shell execution"
    );
}

// ============================================================================
// Scenario 6: 10,000 Item Directory Rapid Sort & Search Performance Stress Test
// ============================================================================

#[test]
fn test_tier4_scenario6_stress_10k_directory_rapid_sort_and_search() {
    let mut explorer = ExplorerView::default();

    // 1. Rapidly populate 10,000 items with natural numbering and varied extensions
    explorer.entries = (1..=10_000)
        .map(|i| {
            let ext = match i % 5 {
                0 => "rs",
                1 => "txt",
                2 => "png",
                3 => "pdf",
                _ => "json",
            };
            FileItem::from_path(PathBuf::from(format!("bench_file_{i:05}.{ext}")))
        })
        .collect();

    // 2. Perform rapid sorting by Name (Ascending)
    let start_sort_asc = Instant::now();
    explorer.sort_column = SortColumn::Name;
    explorer.sort_direction = SortDirection::Ascending;
    let asc_results = explorer.filtered_and_sorted_entries();
    let sort_asc_time = start_sort_asc.elapsed();

    assert_eq!(asc_results.len(), 10_000);
    assert_eq!(asc_results[0].name, "bench_file_00001.txt");
    assert!(
        sort_asc_time.as_millis() < 150,
        "10k item sort must complete in < 150ms, took {}ms",
        sort_asc_time.as_millis()
    );

    // 3. Perform rapid sort direction reversal (Descending)
    let start_sort_desc = Instant::now();
    explorer.sort_direction = SortDirection::Descending;
    let desc_results = explorer.filtered_and_sorted_entries();
    let sort_desc_time = start_sort_desc.elapsed();

    assert_eq!(desc_results.len(), 10_000);
    assert_eq!(desc_results[0].name, "bench_file_10000.rs");
    assert!(
        sort_desc_time.as_millis() < 150,
        "10k reverse sort must complete in < 150ms, took {}ms",
        sort_desc_time.as_millis()
    );

    // 4. Perform rapid keystroke search query filtering across 10,000 items
    let search_queries = ["bench", "file_05", "file_0500", "bench_file_09999"];
    for query in search_queries {
        let start_search = Instant::now();
        explorer.search_query = query.to_string();
        let query_results = explorer.filtered_and_sorted_entries();
        let search_time = start_search.elapsed();

        assert!(!query_results.is_empty());
        assert!(
            search_time.as_millis() < 150,
            "10k search for '{}' must complete in < 150ms, took {}ms",
            query,
            search_time.as_millis()
        );
    }
}
