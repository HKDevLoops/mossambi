//! Milestone 2 Performance Test Suite: 10,000-item traversal and sorting latency budget (<250ms).

use mossambi::core::fs_utils::{
    compare_file_names, list_directory_sync, sort_file_items, spawn_async_dir_scan, FileItem,
    ScanBatch,
};
use std::fs::{create_dir_all, File};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicU64;
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

fn temp_perf_dir(tag: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("mossambi_perf_{tag}_{stamp}"));
    create_dir_all(&dir).expect("Must create test directory");
    dir
}

fn create_large_fixture(dir: &Path, dir_count: usize, file_count: usize) {
    for i in 0..dir_count {
        let name = if i % 2 == 0 {
            format!("FOLDER_{i:04}")
        } else {
            format!("folder_{i:04}")
        };
        create_dir_all(dir.join(name)).expect("Create subdir");
    }

    for i in 0..file_count {
        let name = match i % 4 {
            0 => format!("file_{i:05}.txt"),
            1 => format!("FILE_{i:05}.RS"),
            2 => format!("data_{i:05}.bin"),
            _ => format!("Image_{i:05}.png"),
        };
        let _ = File::create(dir.join(name)).expect("Create file");
    }
}

#[test]
fn test_sync_traversal_and_sort_10k_budget() {
    let dir = temp_perf_dir("sync_10k");
    const DIRS: usize = 500;
    const FILES: usize = 9500;
    const TOTAL: usize = DIRS + FILES;

    create_large_fixture(&dir, DIRS, FILES);

    let start = Instant::now();
    let items = list_directory_sync(&dir);
    let elapsed = start.elapsed();

    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(
        items.len(),
        TOTAL,
        "Expected {TOTAL} items, got {}",
        items.len()
    );

    let first_file_idx = items
        .iter()
        .position(|it| !it.is_dir)
        .unwrap_or(items.len());
    assert_eq!(
        first_file_idx, DIRS,
        "Expected all {DIRS} directories first"
    );

    for it in &items[..DIRS] {
        assert!(it.is_dir, "Item in directory slice must be directory");
    }
    for it in &items[DIRS..] {
        assert!(!it.is_dir, "Item in file slice must be file");
    }

    for window in items[..DIRS].windows(2) {
        let cmp = compare_file_names(&window[0].name, &window[1].name);
        assert!(
            cmp != std::cmp::Ordering::Greater,
            "Directory slice not sorted: {} vs {}",
            window[0].name,
            window[1].name
        );
    }
    for window in items[DIRS..].windows(2) {
        let cmp = compare_file_names(&window[0].name, &window[1].name);
        assert!(
            cmp != std::cmp::Ordering::Greater,
            "File slice not sorted: {} vs {}",
            window[0].name,
            window[1].name
        );
    }

    println!(
        "[PERF] 10,000 item sync traversal & sort elapsed: {:?}",
        elapsed
    );
    assert!(
        elapsed < Duration::from_millis(250),
        "Traversal and sort exceeded 250ms budget: {:?}",
        elapsed
    );
}

#[test]
fn test_async_dir_scan_10k_completion() {
    let dir = temp_perf_dir("async_10k");
    const DIRS: usize = 200;
    const FILES: usize = 9800;
    const TOTAL: usize = DIRS + FILES;

    create_large_fixture(&dir, DIRS, FILES);

    let (tx, rx) = channel::<ScanBatch>();
    let current_gen = Arc::new(AtomicU64::new(1));
    let start = Instant::now();

    spawn_async_dir_scan(dir.clone(), 1, Arc::clone(&current_gen), tx);

    let mut final_batch = None;
    while let Ok(batch) = rx.recv() {
        if batch.is_done {
            final_batch = Some(batch);
            break;
        }
    }
    let elapsed = start.elapsed();

    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        final_batch.is_some(),
        "Async scan must complete with is_done=true"
    );
    let batch = final_batch.unwrap();
    assert_eq!(batch.entries.len(), TOTAL);
    println!("[PERF] 10,000 item async scan elapsed: {:?}", elapsed);
}

#[test]
fn test_in_memory_sort_10k_microbenchmark() {
    let mut items: Vec<FileItem> = (0..10_000)
        .map(|i| {
            let is_dir = i < 1_000;
            let name = match i % 3 {
                0 => format!("Alpha_{i:05}"),
                1 => format!("beta_{i:05}"),
                _ => format!("GAMMA_{i:05}"),
            };
            FileItem {
                path: PathBuf::from(&name),
                name,
                is_dir,
                is_hidden: false,
                is_symlink: false,
                symlink_target: None,
                extension: if is_dir {
                    String::new()
                } else {
                    "TXT".to_string()
                },
                size_bytes: 1024,
                modified: None,
                created: None,
                permissions_mode: 0o644,
            }
        })
        .collect();

    let start = Instant::now();
    sort_file_items(&mut items);
    let elapsed = start.elapsed();

    println!("[PERF] 10,000 item in-memory sort elapsed: {:?}", elapsed);
    assert!(
        elapsed < Duration::from_millis(250),
        "In-memory sort exceeded 250ms: {:?}",
        elapsed
    );

    // Verify folders first
    assert!(items[0].is_dir);
    assert!(items[999].is_dir);
    assert!(!items[1000].is_dir);
}

#[test]
fn test_natural_comparator_numeric_and_case() {
    let mut names = vec![
        "file10.txt",
        "file1.txt",
        "file2.txt",
        "file02.txt",
        "FILE1.txt",
    ];
    names.sort_by(|a, b| compare_file_names(a, b));
    assert_eq!(
        names,
        vec![
            "FILE1.txt",
            "file1.txt",
            "file2.txt",
            "file02.txt",
            "file10.txt"
        ]
    );

    let a = "img_99999999999999999999.png";
    let b = "img_100000000000000000000.png";
    assert_eq!(compare_file_names(a, b), std::cmp::Ordering::Less);
}
