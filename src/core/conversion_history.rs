//! Conversion History Persistence Engine
//!
//! Tracks recent conversion records in ~/.mossambi/conversion_history.json.
//! Respects `disable_conversion_history` and `conversion_history_limit`.
//!
//! All content is pure ASCII.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionRecord {
    pub source: String,
    pub target: String,
    pub format: String,
    pub size_bytes: u64,
    pub timestamp_ms: u64,
    pub success: bool,
}

static HISTORY_LOCK: Mutex<()> = Mutex::new(());

fn history_file_path() -> PathBuf {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".mossambi")
        .join("conversion_history.json")
}

pub fn record_conversion(source: &str, target: &str, format: &str, size_bytes: u64, success: bool) {
    let cfg = crate::core::MossambiConfig::load();
    if cfg.disable_conversion_history {
        return;
    }
    let _guard = HISTORY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut list = load_history_locked();
    let record = ConversionRecord {
        source: source.to_string(),
        target: target.to_string(),
        format: format.to_string(),
        size_bytes,
        timestamp_ms: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
        success,
    };
    list.insert(0, record);
    let limit = cfg.conversion_history_limit.max(10);
    if list.len() > limit {
        list.truncate(limit);
    }
    save_history_locked(&list);
}

pub fn conversion_history_count() -> usize {
    let _guard = HISTORY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    load_history_locked().len()
}

pub fn get_recent_conversions() -> Vec<ConversionRecord> {
    let _guard = HISTORY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    load_history_locked()
}

pub fn clear_conversion_history() {
    let _guard = HISTORY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let path = history_file_path();
    let _ = fs::remove_file(path);
}

fn load_history_locked() -> Vec<ConversionRecord> {
    let path = history_file_path();
    if !path.exists() {
        return Vec::new();
    }
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_history_locked(records: &[ConversionRecord]) {
    let path = history_file_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(records) {
        let _ = fs::write(path, json);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversion_record_roundtrip() {
        let record = ConversionRecord {
            source: "sample.png".to_string(),
            target: "sample.webp".to_string(),
            format: "webp".to_string(),
            size_bytes: 4096,
            timestamp_ms: 1700000000,
            success: true,
        };
        let serialized = serde_json::to_string(&record).expect("must serialize");
        let deserialized: ConversionRecord =
            serde_json::from_str(&serialized).expect("must deserialize");
        assert_eq!(deserialized.source, "sample.png");
        assert_eq!(deserialized.target, "sample.webp");
        assert_eq!(deserialized.format, "webp");
        assert_eq!(deserialized.size_bytes, 4096);
        assert_eq!(deserialized.timestamp_ms, 1700000000);
        assert!(deserialized.success);
    }
}
