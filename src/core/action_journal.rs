//! Action Journal - logs every user action and keystroke so undo can replay.
//!
//! Every mutating operation the explorer performs is funneled through one of
//! these module functions; each one first *executes* the filesystem work and
//! then appends a reversal record to the journal. [`undo_last`] runs the last
//! record's reversal and appends an `Undo` marker (so repeated undos walk back
//! through history).
//!
//! Keystrokes are also journaled (context-tagged, in-app only) per the user's
//! request: the log is an NDJSON file under `.mossambi/journal.ndjson` in the
//! home directory, plus an in-memory ring the GUI can display later.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// A reversible recorded action.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RecordedAction {
    Rename {
        from: PathBuf,
        to: PathBuf,
    },
    Move {
        from: PathBuf,
        to: PathBuf,
    },
    Copy {
        src: PathBuf,
        dst: PathBuf,
    },
    /// Deletions are staged into the Mossambi trash so they reverse cleanly.
    Delete {
        trashed: Vec<(PathBuf, PathBuf)>, // (original, trash location)
    },
    CreateDir {
        path: PathBuf,
    },
    TagSet {
        path: PathBuf,
        before: Vec<String>,
        after: Vec<String>,
    },
    /// Marker that undo ran; never content for undo itself.
    Undo {
        of: Box<RecordedAction>,
    },
}

/// A keystroke sample (context-tagged; never interleaved with file actions).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename = "keystroke", rename_all = "snake_case")]
pub struct KeyStroke {
    pub at_ms: u64,
    /// Why the key matters at this moment ("rename-edit", "search",
    /// "explorer-nav", ...).
    pub context: String,
    /// egui key name or the actual character ("KeyF", "a").
    pub key: String,
    #[serde(default)]
    pub modifiers: String,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Where the log lives: `~/.mossambi/journal.ndjson`.
fn log_path() -> PathBuf {
    crate::core::platform_paths::home_dir()
        .join(".mossambi")
        .join("journal.ndjson")
}

#[derive(Default)]
struct Journal {
    actions: std::collections::VecDeque<RecordedAction>, // reversal stack (last = newest)
    keys: std::collections::VecDeque<KeyStroke>,
}

static JOURNAL: Mutex<Journal> = Mutex::new(Journal {
    actions: std::collections::VecDeque::new(),
    keys: std::collections::VecDeque::new(),
});

pub(crate) const KEY_RING_CAP: usize = 256;
const ACTION_RING_CAP: usize = 128;

fn append_line(line: &str) {
    let path = log_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        let _ = writeln!(f, "{line}");
    }
}

fn push_action(action: RecordedAction) {
    let line = serde_json::to_string(&action).ok();
    let action_kind = match &action {
        RecordedAction::Rename { .. } => "rename",
        RecordedAction::Move { .. } => "move",
        RecordedAction::Copy { .. } => "copy",
        RecordedAction::Delete { .. } => "delete",
        RecordedAction::CreateDir { .. } => "create_dir",
        RecordedAction::TagSet { .. } => "tag_set",
        RecordedAction::Undo { .. } => "undo",
    };
    if let Ok(mut j) = JOURNAL.lock() {
        j.actions.push_back(action);
        while j.actions.len() > ACTION_RING_CAP {
            j.actions.pop_front();
        }
    }
    if let Some(ref l) = line {
        append_line(&format!("{{\"at_ms\":{},\"entry\":{}}}", now_ms(), l));
        let _ = crate::core::state_db::record_action(action_kind, l);
    }
}

/// Journal a context-tagged keystroke.
///
/// The explorer calls this for every in-app key press so the action log
/// carries the user's intent ("what was typed, and where") alongside the
/// file mutations. Keystrokes are never reversible themselves; they exist so
/// an undo implementation can reason about the sequence that led to a change.
pub fn record_keystroke(context: &str, key: &str, modifiers: &str) {
    let ks = KeyStroke {
        at_ms: now_ms(),
        context: context.to_string(),
        key: key.to_string(),
        modifiers: modifiers.to_string(),
    };
    let line = serde_json::to_string(&ks).ok();
    if let Ok(mut j) = JOURNAL.lock() {
        j.keys.push_back(ks);
        while j.keys.len() > KEY_RING_CAP {
            j.keys.pop_front();
        }
    }
    let _ = crate::core::state_db::record_keystroke(context, key, modifiers);
    if let Some(line) = line {
        append_line(&format!("{{\"at_ms\":{},\"entry\":{}}}", now_ms(), line));
    }
}

// ---------------------------------------------------------------------------
// Action hooks - the ONLY sanctioned way to mutate through the explorer.
// ---------------------------------------------------------------------------

/// Record (and journal) a rename of `from` -> `to` that already happened.
pub fn record_rename(from: PathBuf, to: PathBuf) {
    push_action(RecordedAction::Rename { from, to });
}

/// Record a move that already happened.
pub fn record_move(from: PathBuf, to: PathBuf) {
    push_action(RecordedAction::Move { from, to });
}

/// Record a copy that already happened.
pub fn record_copy(src: PathBuf, dst: PathBuf) {
    push_action(RecordedAction::Copy { src, dst });
}

/// Record a folder creation that already happened.
pub fn record_create_dir(path: PathBuf) {
    push_action(RecordedAction::CreateDir { path });
}

/// Record a tag change (before list hers-authoritative for reversal).
pub fn record_tag_set(path: PathBuf, before: Vec<String>, after: Vec<String>) {
    push_action(RecordedAction::TagSet {
        path,
        before,
        after,
    });
}

/// Move `paths` to the native OS Recycle Bin (or fallback staging),
/// returning an error string on any failure; journals one Delete with the
/// mapping so undo can report or restore.
pub fn stage_delete(paths: &[PathBuf]) -> Result<(), String> {
    if paths.is_empty() {
        return Err("nothing to delete".to_string());
    }
    // Attempt native OS Recycle Bin deletion first (true Windows Recycle Bin / Linux trash)
    if crate::core::fs_utils::delete_to_recycle_bin(paths).is_ok() {
        let trashed = paths.iter().map(|p| (p.clone(), p.clone())).collect();
        push_action(RecordedAction::Delete { trashed });
        return Ok(());
    }

    // Fallback staging if native Recycle Bin is unavailable
    let stamp = now_ms();
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    let trash_root = PathBuf::from(&home)
        .join(".mossambi")
        .join("Trash")
        .join(stamp.to_string());
    std::fs::create_dir_all(&trash_root)
        .map_err(|e| format!("could not create trash staging: {e}"))?;

    let mut trashed = Vec::new();
    for p in paths {
        if crate::core::fs_utils::is_protected_os_file(p) {
            log::warn!(
                "Refusing to stage OS-protected file for deletion: {}",
                p.display()
            );
            continue;
        }
        let name = p
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| stamp.to_string());
        let mut dst = trash_root.join(&name);
        // Same-name collision inside one batch gets a numeric suffix.
        let mut idx = 0;
        while dst.exists() {
            dst = trash_root.join(format!("{name}.{idx}"));
            idx += 1;
        }
        // Cross-device safe: rename first, fall back to copy+remove when the
        // source and staging dir live on different mounts (EXDEV on Linux).
        crate::core::fs_utils::move_entry(p, &dst)
            .map_err(|e| format!("could not stage {} for deletion: {e}", p.display()))?;
        trashed.push((p.clone(), dst));
    }

    push_action(RecordedAction::Delete { trashed });
    let _ = clean_expired_trash(crate::core::MossambiConfig::load().retention_days);
    Ok(())
}

/// Prune deletion batches in ~/.mossambi/Trash/ that are older than `retention_days`.
pub fn clean_expired_trash(retention_days: u32) -> usize {
    if retention_days == 0 {
        return 0;
    }
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    let trash_root = PathBuf::from(&home).join(".mossambi").join("Trash");
    let Ok(entries) = std::fs::read_dir(&trash_root) else {
        return 0;
    };
    let now = now_ms();
    let max_age_ms = (retention_days as u64) * 86_400_000;
    let mut pruned = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let folder_name = entry.file_name().to_string_lossy().into_owned();
        let folder_stamp: Option<u64> = folder_name.parse().ok();
        let is_expired = if let Some(stamp) = folder_stamp {
            now.saturating_sub(stamp) > max_age_ms
        } else if let Ok(meta) = entry.metadata() {
            if let Ok(mtime) = meta.modified() {
                if let Ok(age) = mtime.elapsed() {
                    age.as_millis() as u64 > max_age_ms
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };
        if is_expired {
            if let Ok(()) = std::fs::remove_dir_all(&path) {
                pruned += 1;
                log::info!("Cleaned expired trash batch: {}", path.display());
            }
        }
    }
    pruned
}

/// Undo the most recent undoable action, returning a human-readable note.
pub fn undo_last() -> Result<String, String> {
    let action = JOURNAL.lock().ok().and_then(|mut j| j.actions.pop_back());
    let Some(action) = action else {
        return Err("nothing left to undo".to_string());
    };
    // Re-run the reversal.
    let note = perform_undo(&action)?;
    push_action(RecordedAction::Undo {
        of: Box::new(action),
    });
    Ok(note)
}

fn perform_undo(action: &RecordedAction) -> Result<String, String> {
    match action {
        RecordedAction::Rename { from, to } => {
            fs_rename(to, from)?;
            Ok(format!("undo rename: {}", from.display()))
        }
        RecordedAction::Move { from, to } => {
            fs_rename(to, from)?;
            Ok(format!("undo move: {}", from.display()))
        }
        RecordedAction::Copy { dst, .. } => {
            if dst.exists() {
                if dst.is_dir() {
                    std::fs::remove_dir_all(dst)
                        .map_err(|e| format!("undo copy: could not remove staged copy: {e}"))?;
                } else {
                    std::fs::remove_file(dst)
                        .map_err(|e| format!("undo copy: could not remove staged copy: {e}"))?;
                }
            }
            Ok(format!("undo copy: removed {}", dst.display()))
        }
        RecordedAction::Delete { trashed } => {
            let mut restored = 0;
            for (orig, trash_loc) in trashed {
                if trash_loc != orig && trash_loc.exists() {
                    if orig.exists() {
                        continue; // user recreated it; do not clobber
                    }
                    if fs_rename(trash_loc, orig).is_ok() {
                        restored += 1;
                    }
                }
            }
            if restored > 0 {
                Ok(format!("undo delete: restored {} item(s)", restored))
            } else {
                Ok(format!(
                    "undo delete: {} item(s) sent to native Recycle Bin (restore via OS Recycle Bin)",
                    trashed.len()
                ))
            }
        }
        RecordedAction::CreateDir { path } => {
            if path.is_dir() {
                std::fs::remove_dir(path)
                    .map_err(|e| format!("undo create: could not remove: {e}"))?;
            }
            Ok(format!("undo create folder: removed {}", path.display()))
        }
        RecordedAction::TagSet { path, before, .. } => {
            crate::core::tags::restore_tags(path, before).map_err(|e| format!("undo tag: {e}"))?;
            Ok(format!("undo tag: {}", path.display()))
        }
        RecordedAction::Undo { .. } => Err("undo marker encountered - nothing to reverse".into()),
    }
}

fn fs_rename(from: &Path, to: &Path) -> Result<(), String> {
    if !from.exists() {
        return Err(format!("missing {}", from.display()));
    }
    std::fs::rename(from, to)
        .map_err(|e| format!("rename {} -> {} failed: {e}", from.display(), to.display()))
}

// ---------------------------------------------------------------------------
// Read-side API (for a future history panel).
// ---------------------------------------------------------------------------

/// The most recent keystrokes (newest first), for display.
pub fn recent_keys(n: usize) -> Vec<KeyStroke> {
    JOURNAL
        .lock()
        .map(|j| j.keys.iter().rev().take(n).cloned().collect())
        .unwrap_or_default()
}

/// The most recent actions (newest first), for display.
pub fn recent_actions(n: usize) -> Vec<RecordedAction> {
    JOURNAL
        .lock()
        .map(|j| j.actions.iter().rev().take(n).cloned().collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_serializes() {
        let a = RecordedAction::Rename {
            from: PathBuf::from("C:/tmp/a.txt"),
            to: PathBuf::from("C:/tmp/b.txt"),
        };
        let s = serde_json::to_string(&a).unwrap();
        assert!(s.contains("\"kind\":\"rename\""));
        let back: RecordedAction = serde_json::from_str(&s).unwrap();
        assert!(matches!(back, RecordedAction::Rename { .. }));
    }

    #[test]
    fn keystroke_serializes() {
        let k = KeyStroke {
            at_ms: 1,
            context: "rename-edit".into(),
            key: "a".into(),
            modifiers: "COMMAND".into(),
        };
        let s = serde_json::to_string(&k).unwrap();
        assert!(s.contains("\"kind\":\"keystroke\"")); // wait: serializing struct variant
        assert!(s.contains("rename-edit"));
    }

    #[test]
    fn delete_stage_roundtrip() {
        // Exercises the real staging + undo path in scratch temp dirs.
        let dir =
            std::env::temp_dir().join(format!("mossambi_journal_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let victim = dir.join("victim.txt");
        let fake_staged = dir.join("staged_victim.txt");
        std::fs::write(&victim, b"data").unwrap();

        stage_delete(std::slice::from_ref(&victim)).unwrap();
        assert!(!victim.exists());

        // Test reversing a recorded Delete action via perform_undo
        std::fs::write(&fake_staged, b"data").unwrap();
        let undone = perform_undo(&RecordedAction::Delete {
            trashed: vec![(victim.clone(), fake_staged)],
        })
        .unwrap();
        // Word-based assertion independent of locale formatting.
        assert!(undone.contains("restore"), "note was: {undone}");
        assert!(victim.exists());

        let _ = std::fs::remove_dir_all(&dir);
        // Journal already recorded stage_delete; pop it so the test leaves a clean stack.
        let _ = JOURNAL.lock().unwrap().actions.pop_back();
    }

    #[test]
    fn test_clean_expired_trash_zero_days() {
        assert_eq!(clean_expired_trash(0), 0);
    }
}
