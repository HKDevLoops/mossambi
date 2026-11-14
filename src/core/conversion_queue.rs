//! Conversion Queue: pure state + reducer for the Studio's per-file job list.
//!
//! The GUI owns the queue on `StudioView`; the worker thread only reports
//! through `TaskUpdate`, and the frame drain applies updates through this
//! reducer. Everything here is egui-free so `tests/test_queue.rs` can lock
//! the state transitions.

use std::path::PathBuf;

/// Lifecycle of one queued conversion job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueItemStatus {
    /// Waiting for the worker to reach it.
    Queued,
    /// Converting right now.
    Running,
    /// User pressed Cancel; waiting for the encode to confirm exit.
    Stopping,
    /// Finished successfully.
    Done,
    /// Finished with an error (diagnostics in `error_detail`).
    Failed,
    /// Stopped by the user or the watchdog before completing.
    Cancelled,
}

impl QueueItemStatus {
    /// Chip label shown in the queue row (ASCII only).
    pub fn label(self) -> &'static str {
        match self {
            Self::Queued => "Waiting",
            Self::Running => "Converting",
            Self::Stopping => "Stopping",
            Self::Done => "Done",
            Self::Failed => "Failed",
            Self::Cancelled => "Cancelled",
        }
    }

    /// Terminal states are safe to clear or retry from.
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Done | Self::Failed | Self::Cancelled)
    }
}

/// One job in the conversion queue.
#[derive(Debug, Clone)]
pub struct QueueItem {
    pub id: usize,
    pub source: PathBuf,
    pub target: String,
    pub status: QueueItemStatus,
    /// 0.0..=1.0 intra-item progress (0.0 when unknown).
    pub progress: f32,
    /// Last human-readable progress line (percent, speed, stage).
    pub status_text: String,
    /// Successful outputs (filled on Done).
    pub outputs: Vec<PathBuf>,
    /// Error diagnostics (ffmpeg stderr tail, multi-hop step, ...).
    pub error_detail: Option<String>,
    /// Planned multi-hop route chips ("HWP -> HWPX -> PDF"), set at enqueue.
    pub planned_route: Option<String>,
}

/// Counts by state for the summary strip.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct QueueSummary {
    pub queued: usize,
    pub running: usize,
    pub stopping: usize,
    pub done: usize,
    pub failed: usize,
    pub cancelled: usize,
}

impl QueueSummary {
    /// Items the worker must still finish (or abandon).
    pub fn active(&self) -> usize {
        self.queued + self.running + self.stopping
    }

    pub fn total(&self) -> usize {
        self.active() + self.done + self.failed + self.cancelled
    }
}

/// Actions the queue panel defers to the app layer each frame.
#[derive(Debug, Clone, PartialEq)]
pub enum QueueAction {
    /// Signal the cancel token for the active run.
    CancelActive,
    /// Re-queue every failed item as a new run.
    RetryFailed,
    /// Drop all terminal items from the list.
    ClearFinished,
    /// Open the folder containing this output (Reveal).
    Reveal(PathBuf),
}

/// Pure queue state. Mutated only on the UI thread via the frame drain.
#[derive(Debug)]
pub struct ConversionQueue {
    items: Vec<QueueItem>,
    next_id: usize,
}

impl Default for ConversionQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl ConversionQueue {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            next_id: 1,
        }
    }
    pub fn items(&self) -> &[QueueItem] {
        &self.items
    }

    pub fn summary(&self) -> QueueSummary {
        let mut summary = QueueSummary::default();
        for item in &self.items {
            match item.status {
                QueueItemStatus::Queued => summary.queued += 1,
                QueueItemStatus::Running => summary.running += 1,
                QueueItemStatus::Stopping => summary.stopping += 1,
                QueueItemStatus::Done => summary.done += 1,
                QueueItemStatus::Failed => summary.failed += 1,
                QueueItemStatus::Cancelled => summary.cancelled += 1,
            }
        }
        summary
    }

    pub fn has_active(&self) -> bool {
        self.summary().active() > 0
    }

    /// Registers a batch; returns (item_id, source) pairs in submission order.
    pub fn enqueue_batch(&mut self, files: Vec<PathBuf>, target: &str) -> Vec<(usize, PathBuf)> {
        let mut staged = Vec::with_capacity(files.len());
        for file in files {
            let id = self.next_id;
            self.next_id += 1;
            self.items.push(QueueItem {
                id,
                source: file.clone(),
                target: target.to_string(),
                status: QueueItemStatus::Queued,
                progress: 0.0,
                status_text: String::new(),
                outputs: Vec::new(),
                error_detail: None,
                planned_route: None,
            });
            staged.push((id, file));
        }
        staged
    }

    /// Records a planned route on an item before it runs (W-7 route chips).
    pub fn set_planned_route(&mut self, item_id: usize, route: String) {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == item_id) {
            item.planned_route = Some(route);
        }
    }

    /// Worker reached the item: Queued -> Running.
    pub fn mark_running(&mut self, item_id: usize) {
        if let Some(item) = self.item_mut(item_id) {
            if item.status == QueueItemStatus::Queued {
                item.status = QueueItemStatus::Running;
                item.status_text.clear();
            }
        }
    }

    /// Intra-item progress. Only moves Running/Stopping items; also promotes
    /// Queued items that just started (the worker reports before running).
    pub fn on_progress(&mut self, item_id: usize, fraction: f32, status_text: &str) {
        let Some(item) = self.item_mut(item_id) else {
            return;
        };
        if item.status == QueueItemStatus::Queued {
            item.status = QueueItemStatus::Running;
        }
        if !matches!(
            item.status,
            QueueItemStatus::Running | QueueItemStatus::Stopping
        ) {
            return;
        }
        item.progress = fraction.clamp(0.0, 1.0);
        item.status_text = status_text.to_string();
    }

    /// Item finished successfully (from any non-terminal state).
    pub fn on_item_done(&mut self, item_id: usize, outputs: Vec<PathBuf>, status_text: &str) {
        let Some(item) = self.item_mut(item_id) else {
            return;
        };
        if item.status.is_terminal() {
            return;
        }
        item.status = QueueItemStatus::Done;
        item.progress = 1.0;
        item.status_text = status_text.to_string();
        item.outputs = outputs;
        item.error_detail = None;
    }

    /// Item failed (from any non-terminal state).
    pub fn on_item_failed(&mut self, item_id: usize, error: &str) {
        let Some(item) = self.item_mut(item_id) else {
            return;
        };
        if item.status.is_terminal() {
            return;
        }
        item.status = QueueItemStatus::Failed;
        item.progress = 0.0;
        item.status_text.clear();
        item.error_detail = Some(error.to_string());
    }

    /// Item was cancelled before completing (from any non-terminal state).
    pub fn on_item_cancelled(&mut self, item_id: usize) {
        let Some(item) = self.item_mut(item_id) else {
            return;
        };
        if item.status.is_terminal() {
            return;
        }
        item.status = QueueItemStatus::Cancelled;
        item.progress = 0.0;
        item.status_text.clear();
    }

    /// User pressed Cancel: the running item shows Stopping while the encode
    /// exits; queued items are cancelled outright.
    pub fn request_cancel(&mut self) {
        for item in &mut self.items {
            match item.status {
                QueueItemStatus::Queued => {
                    item.status = QueueItemStatus::Cancelled;
                    item.status_text.clear();
                }
                QueueItemStatus::Running => item.status = QueueItemStatus::Stopping,
                _ => {}
            }
        }
    }

    /// Watchdog stalled the task: per-item results will never arrive.
    /// Running items fail with the watchdog error; queued items are cancelled.
    pub fn on_task_stalled(&mut self, error: &str) {
        for item in &mut self.items {
            match item.status {
                QueueItemStatus::Running => {
                    item.status = QueueItemStatus::Failed;
                    item.progress = 0.0;
                    item.status_text.clear();
                    item.error_detail = Some(error.to_string());
                }
                QueueItemStatus::Stopping | QueueItemStatus::Queued => {
                    item.status = QueueItemStatus::Cancelled;
                    item.progress = 0.0;
                    item.status_text.clear();
                }
                _ => {}
            }
        }
    }

    /// Re-queues every failed item; returns (item_id, source, target) triples
    /// for the retry run. Failed rows reset to Queued with cleared diagnostics.
    pub fn take_failed_for_retry(&mut self) -> Vec<(usize, PathBuf, String)> {
        let mut retry = Vec::new();
        for item in &mut self.items {
            if item.status == QueueItemStatus::Failed {
                item.status = QueueItemStatus::Queued;
                item.progress = 0.0;
                item.status_text.clear();
                item.error_detail = None;
                retry.push((item.id, item.source.clone(), item.target.clone()));
            }
        }
        retry
    }

    /// Drops every terminal item from the list.
    pub fn clear_finished(&mut self) {
        self.items.retain(|item| !item.status.is_terminal());
    }

    /// All successful outputs across the queue.
    pub fn all_outputs(&self) -> Vec<PathBuf> {
        self.items
            .iter()
            .flat_map(|item| item.outputs.iter().cloned())
            .collect()
    }

    fn item_mut(&mut self, item_id: usize) -> Option<&mut QueueItem> {
        self.items.iter_mut().find(|item| item.id == item_id)
    }
}
