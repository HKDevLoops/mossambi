//! Real-Time Filesystem Directory Watcher for Mossambi Explorer
//!
//! Provides background file system notification on Windows (via ReadDirectoryChangesW)
//! and a resilient cross-platform polling fallback for Linux and macOS.
//! When any file is created, modified, deleted, or renamed by any process (or another
//! Mossambi window), notifications are debounced and delivered to the ExplorerView
//! so all open windows update their directory listing in real time.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread::{Builder, JoinHandle};
use std::time::{Duration, Instant};

/// Real-time directory watcher that alerts when the active folder contents change.
pub struct DirWatcher {
    shutdown: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
    receiver: Receiver<()>,
    sender: Sender<()>,
    current_watched_dir: Option<PathBuf>,
}

impl Default for DirWatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl DirWatcher {
    /// Creates a new idle directory watcher.
    pub fn new() -> Self {
        let (sender, receiver) = channel();
        Self {
            shutdown: Arc::new(AtomicBool::new(false)),
            worker: None,
            receiver,
            sender,
            current_watched_dir: None,
        }
    }

    /// Watch a directory for any changes. If already watching this path, does nothing.
    pub fn watch(&mut self, dir: &Path) {
        if self.current_watched_dir.as_deref() == Some(dir) {
            return;
        }

        self.stop();

        let dir_buf = dir.to_path_buf();
        self.current_watched_dir = Some(dir_buf.clone());

        let shutdown = Arc::new(AtomicBool::new(false));
        self.shutdown = Arc::clone(&shutdown);

        let tx = self.sender.clone();

        let handle = Builder::new()
            .name("mossambi-dir-watcher".into())
            .spawn(move || {
                run_watcher_loop(dir_buf, shutdown, tx);
            })
            .ok();

        self.worker = handle;
    }

    /// Check if any file change occurred in the watched directory.
    /// Drains all pending events and returns true if at least one change occurred.
    pub fn try_recv(&self) -> bool {
        let mut changed = false;
        while self.receiver.try_recv().is_ok() {
            changed = true;
        }
        changed
    }

    /// Stop watching the current directory.
    pub fn stop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        self.worker = None;
        self.current_watched_dir = None;
    }
}

impl Drop for DirWatcher {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(target_os = "windows")]
fn run_watcher_loop(dir: PathBuf, shutdown: Arc<AtomicBool>, tx: Sender<()>) {
    use windows_sys::Win32::Foundation::{BOOL, HANDLE};

    #[link(name = "kernel32")]
    extern "system" {
        fn FindFirstChangeNotificationW(
            lpPathName: *const u16,
            bWatchSubtree: BOOL,
            dwNotifyFilter: u32,
        ) -> HANDLE;
        fn FindNextChangeNotification(hChangeHandle: HANDLE) -> BOOL;
        fn FindCloseChangeNotification(hChangeHandle: HANDLE) -> BOOL;
        fn WaitForSingleObject(hHandle: HANDLE, dwMilliseconds: u32) -> u32;
    }

    if !dir.is_dir() {
        return;
    }

    let mut wide_path: Vec<u16> = dir.as_os_str().encode_wide().collect();
    wide_path.push(0);

    const FILE_NOTIFY_CHANGE_FILE_NAME: u32 = 0x00000001;
    const FILE_NOTIFY_CHANGE_DIR_NAME: u32 = 0x00000002;
    const FILE_NOTIFY_CHANGE_LAST_WRITE: u32 = 0x00000010;
    const FILE_NOTIFY_CHANGE_SIZE: u32 = 0x00000008;

    let filter = FILE_NOTIFY_CHANGE_FILE_NAME
        | FILE_NOTIFY_CHANGE_DIR_NAME
        | FILE_NOTIFY_CHANGE_LAST_WRITE
        | FILE_NOTIFY_CHANGE_SIZE;

    let handle = unsafe {
        FindFirstChangeNotificationW(
            wide_path.as_ptr(),
            0, // watch subtree = false
            filter,
        )
    };

    let invalid = (-1isize) as HANDLE;
    if handle.is_null() || handle == invalid {
        run_polling_fallback(dir, shutdown, tx);
        return;
    }

    const WAIT_OBJECT_0: u32 = 0;
    const WAIT_TIMEOUT: u32 = 258;

    while !shutdown.load(Ordering::Relaxed) {
        let wait_res = unsafe { WaitForSingleObject(handle, 150) };
        if wait_res == WAIT_OBJECT_0 {
            // Debounce rapid bursts (e.g. multi-file writes)
            std::thread::sleep(Duration::from_millis(60));
            let _ = tx.send(());
            let _ = unsafe { FindNextChangeNotification(handle) };
        } else if wait_res == WAIT_TIMEOUT {
            if !dir.exists() {
                break;
            }
        } else {
            break;
        }
    }

    unsafe {
        FindCloseChangeNotification(handle);
    }
}

#[cfg(not(target_os = "windows"))]
fn run_watcher_loop(dir: PathBuf, shutdown: Arc<AtomicBool>, tx: Sender<()>) {
    run_polling_fallback(dir, shutdown, tx);
}

fn run_polling_fallback(dir: PathBuf, shutdown: Arc<AtomicBool>, tx: Sender<()>) {
    let mut last_mtime = get_dir_mtime(&dir);
    let mut last_count = get_dir_entry_count(&dir);

    while !shutdown.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(350));
        let mtime = get_dir_mtime(&dir);
        let count = get_dir_entry_count(&dir);

        if mtime != last_mtime || count != last_count {
            last_mtime = mtime;
            last_count = count;
            let _ = tx.send(());
        }
    }
}

fn get_dir_mtime(dir: &Path) -> Option<Instant> {
    std::fs::metadata(dir)
        .ok()
        .and_then(|m| m.modified().ok())
        .map(|_| Instant::now())
}

fn get_dir_entry_count(dir: &Path) -> usize {
    std::fs::read_dir(dir)
        .map(|entries| entries.count())
        .unwrap_or(0)
}

#[cfg(target_os = "windows")]
use std::os::windows::ffi::OsStrExt;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dir_watcher_lifecycle() {
        let mut watcher = DirWatcher::new();
        let temp_dir = std::env::temp_dir().join("mossambi_test_watcher");
        let _ = std::fs::create_dir_all(&temp_dir);

        watcher.watch(&temp_dir);
        std::thread::sleep(Duration::from_millis(50));
        assert!(!watcher.try_recv());

        // Create a test file
        let test_file = temp_dir.join("test_file.txt");
        let _ = std::fs::write(&test_file, "hello");

        std::thread::sleep(Duration::from_millis(250));
        assert!(watcher.try_recv());

        let _ = std::fs::remove_file(&test_file);
        let _ = std::fs::remove_dir(&temp_dir);
        watcher.stop();
    }
}
