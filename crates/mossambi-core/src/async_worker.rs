//! Background Task Execution Engine with Watchdog Supervisors and Cancellation
//!
//! Prevents UI lockups, freezes, or crashes from heavy transcoding, corrupted files,
//! or stalled external processes.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum TaskUpdate {
    Started {
        id: u64,
        name: String,
    },
    Progress {
        id: u64,
        fraction: f32,
        status: String,
    },
    Completed {
        id: u64,
        output: Option<PathBuf>,
        message: String,
    },
    Failed {
        id: u64,
        error: String,
    },
}

static TASK_COUNTER: AtomicU64 = AtomicU64::new(100);

pub struct AsyncWorkerPool {
    tx: Sender<TaskUpdate>,
    rx: Receiver<TaskUpdate>,
}

impl AsyncWorkerPool {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        Self { tx, rx }
    }

    pub fn sender(&self) -> Sender<TaskUpdate> {
        self.tx.clone()
    }

    /// Try to receive any pending task updates without blocking
    pub fn try_recv(&self) -> Option<TaskUpdate> {
        self.rx.try_recv().ok()
    }

    /// Spawns a background worker protected by a 30-second watchdog timer.
    /// If the worker hangs or exceeds the timeout, cancellation is signaled.
    pub fn spawn_task<F>(&self, name: &'static str, f: F) -> (u64, Arc<AtomicBool>)
    where
        F: FnOnce(Arc<AtomicBool>, Sender<TaskUpdate>, u64) + Send + 'static,
    {
        let task_id = TASK_COUNTER.fetch_add(1, Ordering::SeqCst);
        let cancel_token = Arc::new(AtomicBool::new(false));
        let cancel_token_clone = Arc::clone(&cancel_token);
        let tx = self.tx.clone();

        let _ = tx.send(TaskUpdate::Started {
            id: task_id,
            name: name.to_string(),
        });

        // 1. Worker thread
        let tx_worker = tx.clone();
        let cancel_worker = Arc::clone(&cancel_token);
        thread::Builder::new()
            .name(format!("mossambi-worker-{}", name))
            .spawn(move || {
                f(cancel_worker, tx_worker, task_id);
            })
            .ok();

        // 2. Watchdog thread (30 second timeout)
        let cancel_watchdog = Arc::clone(&cancel_token);
        thread::Builder::new()
            .name(format!("mossambi-watchdog-{}", name))
            .spawn(move || {
                thread::sleep(Duration::from_secs(30));
                if !cancel_watchdog.load(Ordering::Relaxed) {
                    cancel_watchdog.store(true, Ordering::SeqCst);
                    let _ = tx.send(TaskUpdate::Failed {
                        id: task_id,
                        error: format!("Task '{}' timed out after 30 seconds (watchdog triggered).", name),
                    });
                }
            })
            .ok();

        (task_id, cancel_token_clone)
    }
}

impl Default for AsyncWorkerPool {
    fn default() -> Self {
        Self::new()
    }
}
