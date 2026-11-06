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
    pub total_bytes: u64,
    pub free_bytes: u64,
}

impl DriveInfo {
    pub fn formatted_space(&self) -> String {
        if self.total_bytes == 0 {
            return String::new();
        }
        let free_gb = self.free_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        let total_gb = self.total_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        format!("{:.0} GB free of {:.0} GB", free_gb, total_gb)
    }

    pub fn usage_ratio(&self) -> f32 {
        if self.total_bytes == 0 {
            0.0
        } else {
            let used = self.total_bytes.saturating_sub(self.free_bytes);
            (used as f64 / self.total_bytes as f64) as f32
        }
    }
}

/// Enumerates all active system drives (Windows) or root / mount points (Linux/macOS)
pub fn get_system_drives() -> Vec<DriveInfo> {
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::Storage::FileSystem::{GetDiskFreeSpaceExW, GetLogicalDrives};
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

                let mut free_avail = 0u64;
                let mut total = 0u64;
                let mut total_free = 0u64;
                let wide_root: Vec<u16> =
                    root_str.encode_utf16().chain(std::iter::once(0)).collect();
                unsafe {
                    GetDiskFreeSpaceExW(
                        wide_root.as_ptr(),
                        &mut free_avail,
                        &mut total,
                        &mut total_free,
                    );
                }

                drives.push(DriveInfo {
                    letter,
                    path: PathBuf::from(&root_str),
                    display_name: format!("{} ({}:)", drive_type, letter),
                    is_system,
                    drive_type: drive_type.to_string(),
                    total_bytes: total,
                    free_bytes: free_avail,
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
                total_bytes: 0,
                free_bytes: 0,
            });
        }
        drives
    }

    #[cfg(not(target_os = "windows"))]
    {
        crate::core::linux_dna::discover_linux_mounts()
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
    pub is_hidden: bool,
    pub is_symlink: bool,
    pub symlink_target: Option<PathBuf>,
    pub permissions_mode: u32,
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
        let modified = metadata.as_ref().and_then(|m| m.modified().ok());
        let extension = if is_dir {
            String::new()
        } else {
            path.extension()
                .map(|e| e.to_string_lossy().to_string().to_uppercase())
                .unwrap_or_default()
        };

        let sym_meta = std::fs::symlink_metadata(&path).ok();
        let is_symlink = sym_meta.as_ref().map(|m| m.is_symlink()).unwrap_or(false);
        let symlink_target = if is_symlink {
            std::fs::read_link(&path).ok()
        } else {
            None
        };

        #[cfg(target_os = "windows")]
        let is_hidden = name.starts_with('.') || {
            use std::os::windows::fs::MetadataExt;
            metadata
                .as_ref()
                .map(|m| m.file_attributes() & 0x2 != 0)
                .unwrap_or(false)
        };
        #[cfg(not(target_os = "windows"))]
        let is_hidden = name.starts_with('.');

        #[cfg(unix)]
        let permissions_mode = {
            use std::os::unix::fs::PermissionsExt;
            metadata
                .as_ref()
                .map(|m| m.permissions().mode())
                .unwrap_or(0o644)
        };
        #[cfg(not(unix))]
        let permissions_mode = {
            let readonly = metadata
                .as_ref()
                .map(|m| m.permissions().readonly())
                .unwrap_or(false);
            if is_dir {
                if readonly {
                    0o555
                } else {
                    0o755
                }
            } else if readonly {
                0o444
            } else {
                0o644
            }
        };

        Self {
            path,
            name,
            is_dir,
            size_bytes,
            modified,
            extension,
            is_hidden,
            is_symlink,
            symlink_target,
            permissions_mode,
        }
    }

    pub fn formatted_permissions(&self) -> String {
        let mode = self.permissions_mode;
        let file_type = if self.is_symlink {
            'l'
        } else if self.is_dir {
            'd'
        } else {
            '-'
        };

        let r = |bit: u32| if mode & bit != 0 { 'r' } else { '-' };
        let w = |bit: u32| if mode & bit != 0 { 'w' } else { '-' };
        let x = |bit: u32| if mode & bit != 0 { 'x' } else { '-' };

        format!(
            "{}{}{}{}{}{}{}{}{}{}",
            file_type,
            r(0o400),
            w(0o200),
            x(0o100),
            r(0o040),
            w(0o020),
            x(0o010),
            r(0o004),
            w(0o002),
            x(0o001),
        )
    }

    pub fn formatted_octal_permissions(&self) -> String {
        format!("{:04o}", self.permissions_mode & 0o777)
    }

    pub fn formatted_size(&self) -> String {
        if self.is_dir {
            return "Folder".to_string();
        }
        format_file_size(self.size_bytes)
    }

    /// Formats the modification timestamp in the machine's local timezone.
    ///
    /// The previous implementation approximated the calendar with 30-day
    /// months and ignored leap years, which drifted the rendered date by
    /// weeks (2026-11-06 was displayed as 2026-11-24). We now use a real
    /// civil-calendar conversion and apply the local UTC offset so the
    /// rendered wall-clock time matches what the user sees in Explorer.
    pub fn formatted_date(&self) -> String {
        let Some(time) = self.modified else {
            return "--".to_string();
        };
        let Ok(duration) = time.duration_since(SystemTime::UNIX_EPOCH) else {
            return "--".to_string();
        };
        let utc_secs = duration.as_secs() as i64;
        let local = utc_secs + local_utc_offset_seconds(utc_secs);
        let (year, month, day, hour, minute) = civil_from_unix_seconds(local);
        format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}")
    }
}

/// The machine's UTC offset in seconds at the given instant (DST-aware).
///
/// Windows/Linux/macOS all expose a local-time API; we use `chrono`'s
/// `Local` when it is available and fall back to UTC (offset 0) otherwise.
fn local_utc_offset_seconds(utc_secs: i64) -> i64 {
    use chrono::{DateTime, Local, Utc};
    let Some(utc) = DateTime::<Utc>::from_timestamp(utc_secs, 0) else {
        return 0;
    };
    utc.with_timezone(&Local).offset().local_minus_utc() as i64
}

/// Converts Unix seconds into a civil (year, month, day, hour, minute) tuple.
///
/// This is Howard Hinnant's `civil_from_days` algorithm, which is exact for
/// the proleptic Gregorian calendar (leap years included) and handles dates
/// before the Unix epoch via floor division.
fn civil_from_unix_seconds(total_secs: i64) -> (i64, u32, u32, u32, u32) {
    let days = total_secs.div_euclid(86_400);
    let secs_of_day = total_secs.rem_euclid(86_400);
    let hour = (secs_of_day / 3600) as u32;
    let minute = ((secs_of_day % 3600) / 60) as u32;

    // Shift the epoch to 0000-03-01 so leap days land at the end of the
    // 400-year era and the month/day arithmetic stays uniform.
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    let year = if m <= 2 { y + 1 } else { y };

    (year, m as u32, d as u32, hour, minute)
}

/// Pluralizes an item count for status-bar / menu headers.
///
/// `"1 item"` / `"N items"` - the previous code rendered "1 items", which
/// the user flagged as a grammar bug.
pub fn pluralize_items(count: usize) -> String {
    if count == 1 {
        "1 item".to_string()
    } else {
        format!("{count} items")
    }
}

/// True when a home-directory entry is a genuine user library folder.
///
/// Windows profiles contain many system-managed folders and NT junctions
/// (NetHood, PrintHood, SendTo, Start Menu, Recent, Templates, Local
/// Settings, ...) that must never appear in Quick access. Only the real
/// library folders qualify.
pub fn is_library_folder_name(name: &str) -> bool {
    const LIBRARY_FOLDERS: [&str; 7] = [
        "desktop",
        "downloads",
        "documents",
        "pictures",
        "music",
        "videos",
        "saved games",
    ];
    LIBRARY_FOLDERS.contains(&name.to_lowercase().as_str())
}

/// Formats a raw byte count into human-readable B, KB, MB, or GB
pub fn format_file_size(bytes: u64) -> String {
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

/// Synchronous directory reader returning sorted FileItem vector (directories first, then case-insensitive names)
pub fn list_directory_sync(dir: &Path) -> Vec<FileItem> {
    let Ok(read_dir) = std::fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut items = Vec::new();
    for entry in read_dir.flatten() {
        items.push(FileItem::from_path(entry.path()));
    }

    items.sort_by(|a, b| {
        if a.is_dir && !b.is_dir {
            std::cmp::Ordering::Less
        } else if !a.is_dir && b.is_dir {
            std::cmp::Ordering::Greater
        } else {
            a.name.to_lowercase().cmp(&b.name.to_lowercase())
        }
    });

    items
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

/// Recursively copies a file or directory tree to destination, returning total bytes copied.
pub fn copy_entry_recursive(src: &Path, dest: &Path) -> std::io::Result<u64> {
    if src.is_dir() {
        std::fs::create_dir_all(dest)?;
        let mut total = 0u64;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let entry_path = entry.path();
            let dest_child = dest.join(entry.file_name());
            total += copy_entry_recursive(&entry_path, &dest_child)?;
        }
        Ok(total)
    } else {
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(src, dest)
    }
}

/// Moves a file or directory safely.
/// Attempts atomic rename first; if cross-device move fails, falls back to recursive copy + remove.
pub fn move_entry(src: &Path, dest: &Path) -> std::io::Result<()> {
    if let Err(_rename_err) = std::fs::rename(src, dest) {
        // Fallback for cross-filesystem / cross-drive boundaries
        copy_entry_recursive(src, dest)?;
        remove_entry_recursive(src)?;
    }
    Ok(())
}

/// Recursively removes a file, symlink, or directory tree safely.
pub fn remove_entry_recursive(path: &Path) -> std::io::Result<()> {
    if path.is_dir() {
        std::fs::remove_dir_all(path)
    } else if path.exists() {
        std::fs::remove_file(path)
    } else {
        Ok(())
    }
}

/// Resolves a non-colliding destination path for paste operations (e.g., "file - Copy.txt").
pub fn resolve_unique_paste_path(dest: &Path) -> PathBuf {
    if !dest.exists() {
        return dest.to_path_buf();
    }

    let parent = dest.parent().unwrap_or_else(|| Path::new("."));
    let stem = dest.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
    let ext_suffix = if let Some(ext) = dest.extension().and_then(|s| s.to_str()) {
        format!(".{}", ext)
    } else {
        String::new()
    };

    let first_try = parent.join(format!("{} - Copy{}", stem, ext_suffix));
    if !first_try.exists() {
        return first_try;
    }

    let mut counter = 2;
    loop {
        let candidate = parent.join(format!("{} - Copy ({}){}", stem, counter, ext_suffix));
        if !candidate.exists() {
            return candidate;
        }
        counter += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Epoch anchor: 1970-01-01T00:00:00Z.
    #[test]
    fn civil_conversion_epoch() {
        assert_eq!(civil_from_unix_seconds(0), (1970, 1, 1, 0, 0));
    }

    /// The exact regression that the user reported: a folder created on
    /// 2026-11-06 was rendered as 2026-11-24 by the old 30-day-month
    /// approximation. The civil conversion must return the true date.
    #[test]
    fn civil_conversion_2026_11_06() {
        // 2026-11-06T09:39:00Z
        let secs = 1_793_957_940i64;
        assert_eq!(civil_from_unix_seconds(secs), (2026, 11, 6, 9, 39));
    }

    /// Leap day must land on Feb 29, not drift into March.
    #[test]
    fn civil_conversion_leap_day() {
        // 2024-02-29T12:00:00Z
        let secs = 1_709_208_000i64;
        assert_eq!(civil_from_unix_seconds(secs), (2024, 2, 29, 12, 0));
    }

    /// Century non-leap year (1900 is not a leap year, 2000 is).
    #[test]
    fn civil_conversion_century_rules() {
        // 1900-03-01T00:00:00Z - day 60 of a non-leap year.
        let secs = -2_203_891_200i64;
        assert_eq!(civil_from_unix_seconds(secs), (1900, 3, 1, 0, 0));
    }

    /// Pre-epoch timestamps must floor correctly (no negative day rollover).
    #[test]
    fn civil_conversion_before_epoch() {
        // 1969-12-31T23:59:00Z
        assert_eq!(civil_from_unix_seconds(-60), (1969, 12, 31, 23, 59));
    }

    /// "1 item" vs "N items" - the grammar bug the user reported.
    #[test]
    fn pluralize_items_grammar() {
        assert_eq!(pluralize_items(0), "0 items");
        assert_eq!(pluralize_items(1), "1 item");
        assert_eq!(pluralize_items(2), "2 items");
        assert_eq!(pluralize_items(42), "42 items");
    }

    /// Only real library folders may appear in Quick access; NT junctions and
    /// system-managed profile folders must be rejected.
    #[test]
    fn library_folder_filter() {
        for ok in [
            "Desktop",
            "Downloads",
            "Documents",
            "Pictures",
            "Music",
            "Videos",
            "Saved Games",
        ] {
            assert!(
                is_library_folder_name(ok),
                "{ok} should be a library folder"
            );
        }
        for bad in [
            "NetHood",
            "PrintHood",
            "SendTo",
            "Start Menu",
            "Recent",
            "Templates",
            "Local Settings",
            "Application Data",
            "Cookies",
            "Links",
            "Searches",
            "My Documents",
            "My Music",
            "My Pictures",
            "My Video",
            "My Network Places",
            "Contacts",
            "Favorites",
        ] {
            assert!(
                !is_library_folder_name(bad),
                "{bad} must not be treated as a library folder"
            );
        }
    }
}
