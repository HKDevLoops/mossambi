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
        if let Some(handle) = self.worker.take() {
            let _ = handle.join();
        }
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
    use std::ffi::c_void;
    use std::ptr::null_mut;
    use windows_sys::Win32::Foundation::{CloseHandle, BOOL, HANDLE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, FILE_FLAG_BACKUP_SEMANTICS, FILE_LIST_DIRECTORY, FILE_NOTIFY_CHANGE_DIR_NAME,
        FILE_NOTIFY_CHANGE_FILE_NAME, FILE_NOTIFY_CHANGE_LAST_WRITE, FILE_NOTIFY_CHANGE_SIZE,
        FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };

    #[link(name = "kernel32")]
    extern "system" {
        fn ReadDirectoryChangesW(
            hDirectory: HANDLE,
            lpBuffer: *mut c_void,
            nBufferLength: u32,
            bWatchSubtree: BOOL,
            dwNotifyFilter: u32,
            lpBytesReturned: *mut u32,
            lpOverlapped: *mut c_void,
            lpCompletionRoutine: Option<unsafe extern "system" fn(u32, u32, *mut c_void)>,
        ) -> BOOL;
    }

    if !dir.is_dir() {
        return;
    }

    let mut wide_path: Vec<u16> = dir.as_os_str().encode_wide().collect();
    wide_path.push(0);

    let handle = unsafe {
        CreateFileW(
            wide_path.as_ptr(),
            FILE_LIST_DIRECTORY,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            null_mut(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS,
            null_mut(),
        )
    };

    if handle == INVALID_HANDLE_VALUE || handle.is_null() {
        // Fall back to robust polling if directory handle cannot be opened
        run_polling_fallback(dir, shutdown, tx);
        return;
    }

    let filter = FILE_NOTIFY_CHANGE_FILE_NAME
        | FILE_NOTIFY_CHANGE_DIR_NAME
        | FILE_NOTIFY_CHANGE_LAST_WRITE
        | FILE_NOTIFY_CHANGE_SIZE;

    let mut buffer = [0u8; 4096];
    let mut bytes_returned = 0u32;

    while !shutdown.load(Ordering::Relaxed) {
        let success = unsafe {
            ReadDirectoryChangesW(
                handle,
                buffer.as_mut_ptr() as *mut _,
                buffer.len() as u32,
                0, // watch subtree = false (immediate children only for max efficiency)
                filter,
                &mut bytes_returned,
                null_mut(),
                None,
            )
        };

        if success != 0 && bytes_returned > 0 {
            // Debounce rapid bursts (e.g. multi-file writes)
            std::thread::sleep(Duration::from_millis(80));
            let _ = tx.send(());
        } else {
            // Directory handle might have become invalid or closed
            std::thread::sleep(Duration::from_millis(200));
            if !dir.exists() {
                break;
            }
        }
    }

    unsafe {
        CloseHandle(handle);
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
