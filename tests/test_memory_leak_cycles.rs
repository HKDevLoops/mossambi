//! Milestone 2 Memory Leak Elimination & Bounded Cache Verification Test Suite.

use mossambi::core::config::AnimStyle;
use mossambi::core::fs_utils::{
    load_dir_size_cache_from, save_dir_size_cache_to, DIR_SIZE_CACHE_CAPACITY,
};
use mossambi::core::shell_icons::{
    clear_shell_icon_cache, compute_shell_icon_key, shell_icon_cache_capacity, ShellIconKey,
};
use mossambi::gui::anim::AnimRegistry;
use mossambi::gui::explorer_view::{CachedThumb, ExplorerView, ThumbState};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[test]
fn test_column_cache_lru_bounding() {
    let mut explorer = ExplorerView::default();

    for i in 0..50 {
        let dummy_path = PathBuf::from(format!("C:/mossambi/mock_test_dir_{i}"));
        let _ = explorer.get_column_entries(&dummy_path);
    }

    assert!(
        explorer.column_cache.len() <= 32,
        "Column cache exceeded 32 entries: {}",
        explorer.column_cache.len()
    );
    assert_eq!(explorer.column_cache.len(), 32);

    assert!(!explorer
        .column_cache
        .contains_key(&PathBuf::from("C:/mossambi/mock_test_dir_0")));
    assert!(!explorer
        .column_cache
        .contains_key(&PathBuf::from("C:/mossambi/mock_test_dir_10")));

    let p20 = PathBuf::from("C:/mossambi/mock_test_dir_20");
    let _ = explorer.get_column_entries(&p20);

    let p_new = PathBuf::from("C:/mossambi/mock_test_dir_999");
    let _ = explorer.get_column_entries(&p_new);

    assert!(explorer.column_cache.contains_key(&p20));
    assert!(explorer.column_cache.contains_key(&p_new));
    assert_eq!(explorer.column_cache.len(), 32);
}

#[test]
fn test_thumb_cache_lru_bounding() {
    let mut explorer = ExplorerView::default();

    for i in 0..300 {
        let p = PathBuf::from(format!("C:/images/thumb_{i:04}.jpg"));
        explorer.thumb_cache.insert(
            p,
            CachedThumb {
                mtime: Some(SystemTime::UNIX_EPOCH),
                max_px: 64,
                state: ThumbState::Ready(None),
            },
        );
    }

    assert!(
        explorer.thumb_cache.len() <= 256,
        "thumb_cache exceeded 256 entries: {}",
        explorer.thumb_cache.len()
    );
    assert_eq!(explorer.thumb_cache.len(), 256);

    assert!(!explorer
        .thumb_cache
        .contains_key(&PathBuf::from("C:/images/thumb_0000.jpg")));
    assert!(!explorer
        .thumb_cache
        .contains_key(&PathBuf::from("C:/images/thumb_0043.jpg")));
    assert!(explorer
        .thumb_cache
        .contains_key(&PathBuf::from("C:/images/thumb_0299.jpg")));
}

#[test]
fn test_dir_sizes_cache_lru_bounding() {
    let mut explorer = ExplorerView::default();

    for i in 0..15_000 {
        let p = PathBuf::from(format!("C:/data/folder_{i:06}"));
        explorer
            .dir_sizes
            .insert(p, (Some(SystemTime::UNIX_EPOCH), (i as u64) * 1024));
    }

    assert!(
        explorer.dir_sizes.len() <= 10_000,
        "dir_sizes exceeded 10,000 entries: {}",
        explorer.dir_sizes.len()
    );
    assert_eq!(explorer.dir_sizes.len(), 10_000);

    assert!(!explorer
        .dir_sizes
        .contains_key(&PathBuf::from("C:/data/folder_000000")));
    assert!(!explorer
        .dir_sizes
        .contains_key(&PathBuf::from("C:/data/folder_002000")));
    assert!(explorer
        .dir_sizes
        .contains_key(&PathBuf::from("C:/data/folder_014999")));
}

#[test]
fn test_dir_size_disk_persistence_bounding() {
    let temp_file = std::env::temp_dir().join(format!(
        "mossambi_test_dir_sizes_{}.json",
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));

    let mut map = HashMap::new();
    for i in 0..12_000 {
        map.insert(
            PathBuf::from(format!("C:/project/sub_{i:05}")),
            (Some(SystemTime::UNIX_EPOCH), 4096),
        );
    }

    save_dir_size_cache_to(&temp_file, &map).expect("Must save cache");

    let loaded = load_dir_size_cache_from(&temp_file);
    let _ = std::fs::remove_file(&temp_file);

    assert!(
        loaded.len() <= DIR_SIZE_CACHE_CAPACITY,
        "Loaded cache exceeded 10,000 entries: {}",
        loaded.len()
    );
}

#[test]
fn test_hover_anims_spring_prune() {
    let mut registry = AnimRegistry::new();

    for id in 0..500 {
        registry.drive(id, 1.0, AnimStyle::Spring, 0.016);
    }
    registry.prune();
    assert!(!registry.is_empty(), "Active springs must not be pruned");

    // Advance 100 frames to reach rest at target 1.0
    for _ in 0..100 {
        for id in 0..500 {
            registry.drive(id, 1.0, AnimStyle::Spring, 0.016);
        }
    }

    registry.prune();
    assert_eq!(
        registry.len(),
        500,
        "Springs settled at target 1.0 must NOT be pruned (prevents infinite hover flicker)"
    );

    // Release all springs to target 0.0 (pointer leaves items)
    for _ in 0..100 {
        for id in 0..500 {
            registry.drive(id, 0.0, AnimStyle::Spring, 0.016);
        }
    }

    registry.prune();
    assert_eq!(
        registry.len(),
        0,
        "Springs released to target 0.0 and settled must be pruned to 0"
    );
}

#[test]
fn test_hover_spring_not_dropped_at_target_one() {
    let mut registry = AnimRegistry::new();
    let id = 42;

    // Simulate 60 frames hovering over item
    for _ in 0..60 {
        registry.drive(id, 1.0, AnimStyle::Spring, 0.016);
    }

    // Spring has reached rest at target 1.0
    registry.prune();
    assert_eq!(
        registry.len(),
        1,
        "Spring at rest at target 1.0 must be preserved"
    );

    // Frame 61: drive again, value must remain at 1.0 (no reset or visual flicker)
    let val = registry.drive(id, 1.0, AnimStyle::Spring, 0.016);
    assert!(
        (val - 1.0).abs() < 0.01,
        "Value must not reset to 0; was {val}"
    );
}

#[test]
fn test_unhover_spring_dropped_when_settled_at_zero() {
    let mut registry = AnimRegistry::new();
    let id = 42;

    // Hover
    for _ in 0..60 {
        registry.drive(id, 1.0, AnimStyle::Spring, 0.016);
    }
    assert_eq!(registry.len(), 1);

    // Unhover: target 0.0
    for _ in 0..60 {
        registry.drive(id, 0.0, AnimStyle::Spring, 0.016);
    }

    // Prune must now clean up the settled-to-zero spring
    registry.prune();
    assert_eq!(registry.len(), 0, "Spring settled to 0.0 must be pruned");
}

#[test]
fn test_virtualized_scrolling_capacity_bounded() {
    let mut registry = AnimRegistry::new();

    // Simulate scrolling 2,000 items past the cursor under virtualization.
    // Each row is briefly driven as it passes under the pointer, then scrolled off-screen.
    for item_id in 0..2000 {
        registry.drive(item_id, 1.0, AnimStyle::Spring, 0.016);
        registry.prune();
        assert!(
            registry.len() <= AnimRegistry::MAX_SPRINGS,
            "Registry len {} exceeded MAX_SPRINGS {}",
            registry.len(),
            AnimRegistry::MAX_SPRINGS
        );
    }

    assert_eq!(
        registry.len(),
        AnimRegistry::MAX_SPRINGS,
        "Registry should be strictly capped at MAX_SPRINGS (512)"
    );
    assert_eq!(registry.capacity(), 512);
}

#[test]
fn test_navigation_clears_hover_anims() {
    let mut explorer = ExplorerView::default();
    explorer
        .hover_anims
        .drive(101, 1.0, AnimStyle::Spring, 0.016);
    explorer
        .hover_anims
        .drive(102, 1.0, AnimStyle::Spring, 0.016);
    assert_eq!(explorer.hover_anims.len(), 2);

    // Navigating to a directory clears stale hover springs
    explorer.navigate_to(PathBuf::from("C:/"));
    assert_eq!(explorer.hover_anims.len(), 0);

    explorer
        .hover_anims
        .drive(201, 1.0, AnimStyle::Spring, 0.016);
    assert_eq!(explorer.hover_anims.len(), 1);
    explorer.open_directory(PathBuf::from("C:/"));
    assert_eq!(explorer.hover_anims.len(), 0);
}

#[test]
fn test_shell_icon_coalescing_and_bounding() {
    clear_shell_icon_cache();
    assert_eq!(shell_icon_cache_capacity(), 256);

    let txt1 = Path::new("C:/docs/alpha.txt");
    let txt2 = Path::new("D:/other/BETA.TXT");
    let txt3 = Path::new("sub/folder/notes.Txt");

    let k1 = compute_shell_icon_key(txt1, false, false);
    let k2 = compute_shell_icon_key(txt2, false, false);
    let k3 = compute_shell_icon_key(txt3, false, false);

    assert_eq!(k1, ShellIconKey::Extension("txt".into(), false));
    assert_eq!(k2, ShellIconKey::Extension("txt".into(), false));
    assert_eq!(k3, ShellIconKey::Extension("txt".into(), false));

    let exe1 = Path::new("C:/bin/app1.exe");
    let exe2 = Path::new("C:/bin/app2.exe");
    let dir1 = Path::new("C:/my_folder");

    let k_exe1 = compute_shell_icon_key(exe1, false, false);
    let k_exe2 = compute_shell_icon_key(exe2, false, false);
    let k_dir1 = compute_shell_icon_key(dir1, true, false);

    assert_eq!(k_exe1, ShellIconKey::Path(exe1.to_path_buf(), false));
    assert_eq!(k_exe2, ShellIconKey::Path(exe2.to_path_buf(), false));
    assert_ne!(k_exe1, k_exe2);
    assert_eq!(k_dir1, ShellIconKey::Path(dir1.to_path_buf(), false));

    clear_shell_icon_cache();
}

#[test]
fn test_navigation_cycles_memory_stability() {
    let mut explorer = ExplorerView::default();

    for cycle in 0..50 {
        let cycle_dir = PathBuf::from(format!("C:/mossambi/cycle_{cycle}"));
        explorer.get_column_entries(&cycle_dir);
        explorer
            .hover_anims
            .drive(cycle as u64, 1.0, AnimStyle::Spring, 0.016);
    }

    // When pointer unhovers and springs settle to 0.0, prune cleans them up
    for _ in 0..60 {
        for cycle in 0..50 {
            explorer
                .hover_anims
                .drive(cycle as u64, 0.0, AnimStyle::Spring, 0.016);
        }
    }
    explorer.hover_anims.prune();

    assert!(explorer.column_cache.len() <= 32);
    assert!(explorer.dir_sizes.len() <= 10_000);
    assert_eq!(explorer.hover_anims.len(), 0);
}
