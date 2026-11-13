//! Folder-sizes wave unit tests: the budgeted BFS compute kernel
//! (exact sum, empty dir, depth cap, entry cap, unix symlink skip) and
//! the folder-size disk cache roundtrip.

use mossambi::core::fs_utils::{
    compute_dir_size, load_dir_size_cache_from, save_dir_size_cache_to,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Unique scratch root so parallel test binaries never collide.
fn temp_root(tag: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("mossambi_dir_sizes_{tag}_{stamp}"));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_file(dir: &Path, name: &str, len: usize) {
    std::fs::write(dir.join(name), vec![b'x'; len]).unwrap();
}

#[test]
fn sums_two_files_exactly() {
    let root = temp_root("exact");
    write_file(&root, "a.bin", 100);
    write_file(&root, "b.bin", 250);
    assert_eq!(compute_dir_size(&root, 8, 100_000), 350);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn empty_dir_is_zero() {
    let root = temp_root("empty");
    assert_eq!(compute_dir_size(&root, 8, 100_000), 0);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn depth_cap_limits_descent() {
    let root = temp_root("depth");
    write_file(&root, "f0.bin", 10);
    let mut cur = root.clone();
    for (level, size) in [20usize, 30, 40, 50, 60].iter().enumerate() {
        cur = cur.join(format!("d{}", level + 1));
        std::fs::create_dir_all(&cur).unwrap();
        write_file(&cur, &format!("f{}.bin", level + 1), *size);
    }
    // depth 0 = root (10), depth 1 = d1 (20), depth 2 = d2 (30); d3 sits
    // at depth 3, beyond max_depth 2, so it is never descended into.
    assert_eq!(compute_dir_size(&root, 2, 100_000), 60);
    assert_eq!(compute_dir_size(&root, 8, 100_000), 210);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn entry_cap_bounds_work() {
    let root = temp_root("cap");
    for i in 0..40 {
        write_file(&root, &format!("f{i:02}.bin"), 10);
    }
    // The cap stops after exactly 10 equal-size files are visited.
    assert_eq!(compute_dir_size(&root, 8, 10), 100);
    assert_eq!(compute_dir_size(&root, 8, 100_000), 400);
    let _ = std::fs::remove_dir_all(&root);
}

#[cfg(unix)]
#[test]
fn symlinks_are_skipped() {
    let root = temp_root("links");
    let outside = root.parent().unwrap().join("mossambi_symlink_target.bin");
    std::fs::write(&outside, vec![b'x'; 500]).unwrap();
    std::os::unix::fs::symlink(&outside, root.join("link.bin")).unwrap();
    // The link is neither descended nor length-accounted.
    assert_eq!(compute_dir_size(&root, 8, 100_000), 0);
    let _ = std::fs::remove_file(&outside);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn disk_cache_roundtrip() {
    let root = temp_root("cache");
    let path = root.join("dir_sizes.json");
    let mut map: HashMap<PathBuf, (Option<SystemTime>, u64)> = HashMap::new();
    map.insert(
        PathBuf::from("C:/data/with/mtime"),
        (
            Some(SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000)),
            4096,
        ),
    );
    map.insert(PathBuf::from("D:/no/mtime"), (None, 7));
    save_dir_size_cache_to(&path, &map).unwrap();
    let loaded = load_dir_size_cache_from(&path);
    assert_eq!(loaded.len(), 2);
    assert_eq!(
        loaded.get(Path::new("C:/data/with/mtime")),
        map.get(Path::new("C:/data/with/mtime"))
    );
    assert_eq!(loaded.get(Path::new("D:/no/mtime")), Some(&(None, 7)));
    let _ = std::fs::remove_dir_all(&root);
}
