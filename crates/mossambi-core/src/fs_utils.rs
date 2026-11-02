//! File System Utilities: Fast Drive Enumerator, Async Directory Scanner & Breadcrumb Parser
//!
//! Inspired by File Pilot (zero-allocation fast paths) and Yazi (generation-based async scanning).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread;
use std::time::SystemTime;

/// System Drive or Mount Point Information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriveInfo {
    pub letter: char,
    pub path: PathBuf,
    pub display_name: String,
    pub is_system: bool,
    pub drive_type: String,
}

/// Enumerates all active system drives (Windows) or root / mount points (Linux/macOS)
pub fn get_system_drives() -> Vec<DriveInfo> {
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::Storage::FileSystem::GetLogicalDrives;
        let mut drives = Vec::new();
        let mask = unsafe { GetLogicalDrives() };
        for i in 0..26 {
            if (mask & (1 << i)) != 0 {
                let letter = (b'A' + i) as char;
                let root_str = format!("{}:\\", letter);
                let is_system = letter == 'C';
                let drive_type = if is_system {
                    "System Disk"
                } else {
                    "Local Volume"
                };
                drives.push(DriveInfo {
                    letter,
                    path: PathBuf::from(&root_str),
                    display_name: format!("{} ({}:)", drive_type, letter),
                    is_system,
                    drive_type: drive_type.to_string(),
                });
            }
        }
        if drives.is_empty() {
            drives.push(DriveInfo {
                letter: 'C',
                path: PathBuf::from("C:\\"),
                display_name: "Local Disk (C:)".into(),
                is_system: true,
                drive_type: "Fixed".into(),
            });
        }
        drives
    }

    #[cfg(not(target_os = "windows"))]
    {
        crate::linux_dna::discover_linux_mounts()
    }
}

/// Metadata-rich Directory Entry
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileItem {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub size_bytes: u64,
    pub modified: Option<SystemTime>,
    pub extension: String,
}

impl FileItem {
    pub fn from_path(path: PathBuf) -> Self {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());
        let is_dir = path.is_dir();
        let metadata = std::fs::metadata(&path).ok();
        let size_bytes = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
        let modified = metadata.and_then(|m| m.modified().ok());
        let extension = if is_dir {
            String::new()
        } else {
            path.extension()
                .map(|e| e.to_string_lossy().to_string().to_uppercase())
                .unwrap_or_default()
        };

        Self {
            path,
            name,
            is_dir,
            size_bytes,
            modified,
            extension,
        }
    }

    pub fn formatted_size(&self) -> String {
        if self.is_dir {
            return "Folder".to_string();
        }
        let bytes = self.size_bytes;
        if bytes < 1024 {
            format!("{} B", bytes)
        } else if bytes < 1024 * 1024 {
            format!("{:.1} KB", bytes as f64 / 1024.0)
        } else if bytes < 1024 * 1024 * 1024 {
            format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
        } else {
            format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
        }
    }
}

/// Batch of scanned directory entries tagged with generation counter
#[derive(Debug, Clone)]
pub struct ScanBatch {
    pub gen: u64,
    pub dir: PathBuf,
    pub entries: Vec<FileItem>,
    pub is_done: bool,
}

/// Spawns an asynchronous directory scan that streams entries in non-blocking batches.
/// Cancels immediately if `current_gen` is incremented.
pub fn spawn_async_dir_scan(
    dir: PathBuf,
    gen: u64,
    current_gen: Arc<AtomicU64>,
    tx: Sender<ScanBatch>,
) {
    thread::Builder::new()
        .name("mossambi-dir-scan".into())
        .spawn(move || {
            let read_dir = match std::fs::read_dir(&dir) {
                Ok(rd) => rd,
                Err(_) => {
                    let _ = tx.send(ScanBatch {
                        gen,
                        dir,
                        entries: Vec::new(),
                        is_done: true,
                    });
                    return;
                }
            };

            let mut all_items = Vec::new();

            for entry in read_dir.flatten() {
                // Stale scan check: cancel immediately if user changed directory
                if current_gen.load(Ordering::Relaxed) != gen {
                    return;
                }
                all_items.push(FileItem::from_path(entry.path()));
            }

            // Sort: folders first, then files alphabetically (case-insensitive)
            all_items.sort_by(|a, b| {
                if a.is_dir && !b.is_dir {
                    std::cmp::Ordering::Less
                } else if !a.is_dir && b.is_dir {
                    std::cmp::Ordering::Greater
                } else {
                    a.name.to_lowercase().cmp(&b.name.to_lowercase())
                }
            });

            // Send completed batch if still current
            if current_gen.load(Ordering::Relaxed) == gen {
                let _ = tx.send(ScanBatch {
                    gen,
                    dir,
                    entries: all_items,
                    is_done: true,
                });
            }
        })
        .ok();
}

/// Clickable Breadcrumb Segment (File Pilot style)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BreadcrumbSegment {
    pub label: String,
    pub target_path: PathBuf,
}

/// Parses any Path into clean clickable breadcrumb segments
pub fn parse_breadcrumbs(path: &Path) -> Vec<BreadcrumbSegment> {
    let mut segments = Vec::new();
    let mut accum = PathBuf::new();

    for comp in path.components() {
        accum.push(comp.as_os_str());
        let raw = comp.as_os_str().to_string_lossy();
        let clean = raw.trim_end_matches(['\\', '/']);
        if !clean.is_empty() {
            segments.push(BreadcrumbSegment {
                label: clean.to_string(),
                target_path: accum.clone(),
            });
        }
    }

    if segments.is_empty() {
        segments.push(BreadcrumbSegment {
            label: "/".into(),
            target_path: path.to_path_buf(),
        });
    }

    segments
}
