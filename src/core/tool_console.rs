//! Tool console: egui-free bridge for background tool operations
//! (file inspection, speed benchmarking, hardware telemetry).
//!
//! Modeled on `core::scripting::ScriptConsole`: the studio owns the channel
//! pair, work runs on plain `std::thread::spawn` threads (hashing and
//! probing can exceed the 30s worker-pool watchdog), and the UI drains
//! pending events once per frame.

use crate::codec::{BenchmarkReport, FileQualityReport};
use crate::hwaccel::{
    detect_hardware_capabilities, detect_system_specs, HwCapabilities, SystemSpecs,
};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};

/// One finished background tool operation, drained by the studio UI once
/// per frame. Report payloads are boxed to keep the enum small.
pub enum ToolEvent {
    /// `FileQualityChecker::inspect_file` finished.
    Inspected {
        path: PathBuf,
        result: Result<Box<FileQualityReport>, String>,
    },
    /// `BenchmarkEngine::benchmark_file` finished.
    Benchmarked {
        path: PathBuf,
        result: Result<Box<BenchmarkReport>, String>,
    },
    /// Hardware capabilities + system specs probed (one-shot cache fill).
    Telemetry {
        hw: Box<HwCapabilities>,
        specs: Box<SystemSpecs>,
    },
}

/// Per-app tool console. The studio owns the channel pair; operations are
/// dispatched on plain threads and events arrive via the sender. The UI
/// drains pending events once per frame.
pub struct ToolConsole {
    tx: Sender<ToolEvent>,
    rx: Receiver<ToolEvent>,
}

impl Default for ToolConsole {
    fn default() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        Self { tx, rx }
    }
}

impl ToolConsole {
    /// Cloneable sender handed to background threads.
    pub fn sender(&self) -> Sender<ToolEvent> {
        self.tx.clone()
    }

    /// Take every event queued since the last drain.
    pub fn drain(&mut self) -> Vec<ToolEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.rx.try_recv() {
            events.push(event);
        }
        events
    }
}

/// Runs `FileQualityChecker::inspect_file` on a plain thread.
pub fn inspect_file_async(tx: Sender<ToolEvent>, path: PathBuf) {
    std::thread::spawn(move || {
        let result =
            crate::codec::FileQualityChecker::inspect_file(&path).map_err(|e| e.to_string());
        let _ = tx.send(ToolEvent::Inspected {
            path,
            result: result.map(Box::new),
        });
    });
}

/// Runs `BenchmarkEngine::benchmark_file` on a plain thread.
pub fn benchmark_file_async(tx: Sender<ToolEvent>, path: PathBuf) {
    std::thread::spawn(move || {
        let result =
            crate::codec::BenchmarkEngine::benchmark_file(&path, None).map_err(|e| e.to_string());
        let _ = tx.send(ToolEvent::Benchmarked {
            path,
            result: result.map(Box::new),
        });
    });
}

/// Detects hardware capabilities + system specs on a plain thread.
pub fn detect_telemetry_async(tx: Sender<ToolEvent>) {
    std::thread::spawn(move || {
        let hw = detect_hardware_capabilities();
        let specs = detect_system_specs();
        let _ = tx.send(ToolEvent::Telemetry {
            hw: Box::new(hw),
            specs: Box::new(specs),
        });
    });
}
