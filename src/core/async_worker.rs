//! Background Task Execution Engine with Watchdog Supervisors and Cancellation
//!
//! Prevents UI lockups, freezes, or crashes from heavy transcoding, corrupted files,
//! or stalled external processes.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub enum TaskUpdate {
    Started {
        id: usize,
        name: String,
    },
    Progress {
        id: usize,
        fraction: f32,
        status: String,
    },
    Completed {
        id: usize,
        output: Option<PathBuf>,
        message: String,
    },
    Failed {
        id: usize,
        error: String,
    },
}

// AtomicUsize: fetch_add on AtomicU64 panics at runtime on some 32-bit
// ARM/RISC-V targets, so the counter must be native-word sized.
static TASK_COUNTER: AtomicUsize = AtomicUsize::new(100);

/// A task is considered stalled only when it produced NO update (progress or
/// otherwise) for this long — not merely when it has been running 30s.
const WATCHDOG_TIMEOUT: Duration = Duration::from_secs(30);

/// How often the watchdog re-checks the progress clock.
const WATCHDOG_POLL: Duration = Duration::from_millis(500);

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

    /// Spawns a background worker protected by a progress-aware watchdog.
    /// The 30s timeout resets every time the task reports `Progress`, so long
    /// video transcodes that keep making headway are never falsely cancelled.
    /// If the worker truly stalls (no updates for 30s), the cancel token is
    /// signaled and a `Failed` update is emitted.
    pub fn spawn_task<F>(&self, name: &'static str, f: F) -> (usize, Arc<AtomicBool>)
    where
        F: FnOnce(Arc<AtomicBool>, Sender<TaskUpdate>, usize) + Send + 'static,
    {
        let task_id = TASK_COUNTER.fetch_add(1, Ordering::SeqCst);
        let cancel_token = Arc::new(AtomicBool::new(false));
        let cancel_token_clone = Arc::clone(&cancel_token);
        let tx = self.tx.clone();

        let _ = tx.send(TaskUpdate::Started {
            id: task_id,
            name: name.to_string(),
        });

        // Shared progress clock: (last-progress Instant, finished flag). The
        // pump thread bumps the Instant on every Progress update; the watchdog
        // reads it to decide whether the task is genuinely stalled.
        let progress_clock: Arc<(Mutex<Instant>, AtomicBool)> =
            Arc::new((Mutex::new(Instant::now()), AtomicBool::new(false)));

        // 1. Worker thread. The task's sends go through an intercept channel
        //    so every Progress update can push the watchdog deadline forward.
        let (pump_tx, pump_rx) = channel::<TaskUpdate>();
        let cancel_worker = Arc::clone(&cancel_token);
        let worker = thread::Builder::new()
            .name(format!("mossambi-worker-{}", name))
            .spawn(move || {
                f(cancel_worker, pump_tx, task_id);
            });

        if let Err(spawn_err) = worker {
            // Never swallow thread-spawn failure silently: the caller would
            // wait forever for a result that is never coming.
            log::error!(
                "Failed to spawn worker thread for task '{}' (id {}): {}",
                name,
                task_id,
                spawn_err
            );
            let _ = tx.send(TaskUpdate::Failed {
                id: task_id,
                error: format!(
                    "Could not spawn worker thread for task '{}': {}",
                    name, spawn_err
                ),
            });
            return (task_id, cancel_token_clone);
        }
        drop(worker); // the thread runs independently; no join needed

        // 2. Pump thread: forwards task updates to the pool while refreshing
        //    the watchdog's progress clock. Exits when the worker finishes
        //    (intercept sender dropped).
        let tx_pump = tx.clone();
        let clock_pump = Arc::clone(&progress_clock);
        let _ = thread::Builder::new()
            .name(format!("mossambi-pump-{}", name))
            .spawn(move || {
                while let Ok(update) = pump_rx.recv() {
                    if matches!(update, TaskUpdate::Progress { .. }) {
                        if let Ok(mut last) = clock_pump.0.lock() {
                            *last = Instant::now();
                        }
                    }
                    if matches!(
                        update,
                        TaskUpdate::Completed { .. } | TaskUpdate::Failed { .. }
                    ) {
                        // Terminal update seen: let the watchdog retire early.
                        clock_pump.1.store(true, Ordering::Relaxed);
                    }
                    let _ = tx_pump.send(update);
                }
            });

        // 3. Watchdog thread: fires only after 30s WITHOUT any progress.
        let cancel_watchdog = Arc::clone(&cancel_token);
        let clock_watchdog = Arc::clone(&progress_clock);
        let _ = thread::Builder::new()
            .name(format!("mossambi-watchdog-{}", name))
            .spawn(move || {
                loop {
                    thread::sleep(WATCHDOG_POLL);
                    if cancel_watchdog.load(Ordering::Relaxed) {
                        return; // user-cancelled
                    }
                    if clock_watchdog.1.load(Ordering::Relaxed) {
                        return; // task already completed or failed
                    }
                    let stalled = match clock_watchdog.0.lock() {
                        Ok(last) => last.elapsed() >= WATCHDOG_TIMEOUT,
                        Err(poisoned) => poisoned.into_inner().elapsed() >= WATCHDOG_TIMEOUT,
                    };
                    if stalled {
                        cancel_watchdog.store(true, Ordering::SeqCst);
                        log::warn!(
                            "task '{}' exceeded 30s without progress — cancel token set",
                            name
                        );
                        let _ = tx.send(TaskUpdate::Failed {
                            id: task_id,
                            error: format!(
                                "Task '{}' exceeded 30 seconds without progress (watchdog triggered).",
                                name
                            ),
                        });
                        return;
                    }
                }
            });

        (task_id, cancel_token_clone)
    }
}

impl Default for AsyncWorkerPool {
    fn default() -> Self {
        Self::new()
    }
}
