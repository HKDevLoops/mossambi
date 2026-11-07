//! Centralized SQLite State Database for Mossambi.
//!
//! Stores tags, persistent action journal, keystroke logs, custom conformer presets,
//! and custom user scripting commands in a single high-performance SQLite database
//! operating with Write-Ahead Logging (WAL) and busy timeout.
//!
//! Database location:
//! - Windows: %LOCALAPPDATA%/mossambi/mossambi_state.db
//! - macOS: ~/Library/Application Support/mossambi/mossambi_state.db
//! - Linux: $XDG_DATA_HOME/mossambi/mossambi_state.db

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CustomPresetRecord {
    pub name: String,
    pub category: String,
    pub target_format: String,
    pub width_px: u32,
    pub height_px: u32,
    pub dpi: u32,
    pub min_bytes: u64,
    pub max_bytes: u64,
    pub normalize_white_balance: bool,
    pub background_rgb: [u8; 3],
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CustomCommandRecord {
    pub id: String,
    pub title: String,
    pub script_lang: String,
    pub script_body: String,
    pub shortcut: Option<String>,
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Returns the platform-native database path.
pub fn db_path() -> PathBuf {
    crate::core::platform_paths::app_data_dir().join("mossambi_state.db")
}

static DB_LOCK: Mutex<()> = Mutex::new(());

/// Open a connection to the SQLite state database, ensuring tables exist.
pub fn open_connection() -> Result<rusqlite::Connection, String> {
    let _guard = DB_LOCK.lock().map_err(|e| e.to_string())?;
    let path = db_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let conn = rusqlite::Connection::open(&path).map_err(|e| format!("Failed to open DB: {e}"))?;

    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA busy_timeout = 5000;
         PRAGMA synchronous = NORMAL;

         CREATE TABLE IF NOT EXISTS file_tags (
             path TEXT PRIMARY KEY,
             tags TEXT NOT NULL,
             updated_at INTEGER NOT NULL
         );

         CREATE TABLE IF NOT EXISTS action_journal (
             id INTEGER PRIMARY KEY AUTOINCREMENT,
             action_kind TEXT NOT NULL,
             payload_json TEXT NOT NULL,
             created_at INTEGER NOT NULL
         );

         CREATE TABLE IF NOT EXISTS keystroke_log (
             id INTEGER PRIMARY KEY AUTOINCREMENT,
             context TEXT NOT NULL,
             key TEXT NOT NULL,
             modifiers TEXT NOT NULL,
             timestamp INTEGER NOT NULL
         );

         CREATE TABLE IF NOT EXISTS custom_presets (
             name TEXT PRIMARY KEY,
             category TEXT NOT NULL,
             target_format TEXT NOT NULL,
             width_px INTEGER NOT NULL,
             height_px INTEGER NOT NULL,
             dpi INTEGER NOT NULL,
             min_bytes INTEGER NOT NULL,
             max_bytes INTEGER NOT NULL,
             normalize_white_balance INTEGER NOT NULL,
             background_r INTEGER NOT NULL,
             background_g INTEGER NOT NULL,
             background_b INTEGER NOT NULL,
             created_at INTEGER NOT NULL
         );

         CREATE TABLE IF NOT EXISTS custom_commands (
             id TEXT PRIMARY KEY,
             title TEXT NOT NULL,
             script_lang TEXT NOT NULL,
             script_body TEXT NOT NULL,
             shortcut TEXT,
             created_at INTEGER NOT NULL
         );",
    )
    .map_err(|e| format!("DB schema init failed: {e}"))?;

    Ok(conn)
}

// ============================================================================
// File Tags Dual-Store API
// ============================================================================

/// Query tags for a given path from the SQLite database.
pub fn get_file_tags(path: &Path) -> Vec<String> {
    let conn = match open_connection() {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let p_str = path.to_string_lossy().to_string();
    let mut stmt = match conn.prepare("SELECT tags FROM file_tags WHERE path = ?1") {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let raw: Result<String, _> = stmt.query_row(rusqlite::params![p_str], |row| row.get(0));
    match raw {
        Ok(s) => s
            .split(',')
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .collect(),
        Err(_) => Vec::new(),
    }
}

/// Set tags for a given path in the SQLite database. If empty, removes the row.
pub fn set_file_tags(path: &Path, tags: &[String]) -> Result<(), String> {
    let conn = open_connection()?;
    let p_str = path.to_string_lossy().to_string();

    if tags.is_empty() {
        conn.execute(
            "DELETE FROM file_tags WHERE path = ?1",
            rusqlite::params![p_str],
        )
        .map_err(|e| e.to_string())?;
    } else {
        let joined = tags.join(",");
        let now = now_secs();
        conn.execute(
            "INSERT INTO file_tags (path, tags, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(path) DO UPDATE SET tags = ?2, updated_at = ?3",
            rusqlite::params![p_str, joined, now],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Query all files carrying a given tag.
pub fn list_files_by_tag(tag: &str) -> Vec<PathBuf> {
    let conn = match open_connection() {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut stmt = match conn.prepare("SELECT path, tags FROM file_tags") {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let mut results = Vec::new();
    let rows = stmt.query_map([], |row| {
        let path_str: String = row.get(0)?;
        let tags_str: String = row.get(1)?;
        Ok((path_str, tags_str))
    });

    if let Ok(iter) = rows {
        for item in iter.flatten() {
            let tags: Vec<&str> = item.1.split(',').map(|s| s.trim()).collect();
            if tags.contains(&tag) {
                results.push(PathBuf::from(item.0));
            }
        }
    }
    results
}

// ============================================================================
// Action Journal Persistent Store
// ============================================================================

/// Append an action to the persistent SQLite action journal.
pub fn record_action(action_kind: &str, payload_json: &str) -> Result<(), String> {
    let conn = open_connection()?;
    let now = now_secs();
    conn.execute(
        "INSERT INTO action_journal (action_kind, payload_json, created_at) VALUES (?1, ?2, ?3)",
        rusqlite::params![action_kind, payload_json, now],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Retrieve the most recent journal actions.
pub fn load_recent_actions(limit: usize) -> Vec<(String, String)> {
    let conn = match open_connection() {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut stmt = match conn
        .prepare("SELECT action_kind, payload_json FROM action_journal ORDER BY id DESC LIMIT ?1")
    {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let mut list = Vec::new();
    let rows = stmt.query_map(rusqlite::params![limit as i64], |row| {
        Ok((row.get(0)?, row.get(1)?))
    });

    if let Ok(iter) = rows {
        for item in iter.flatten() {
            list.push(item);
        }
    }
    list
}

// ============================================================================
// Keystroke Persistent Store
// ============================================================================

/// Record a keystroke into SQLite.
pub fn record_keystroke(context: &str, key: &str, modifiers: &str) -> Result<(), String> {
    let conn = open_connection()?;
    let now = now_secs();
    conn.execute(
        "INSERT INTO keystroke_log (context, key, modifiers, timestamp) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![context, key, modifiers, now],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

// ============================================================================
// Custom Presets Store
// ============================================================================

/// Save or update a custom conformer preset.
pub fn save_preset(preset: &CustomPresetRecord) -> Result<(), String> {
    let conn = open_connection()?;
    let now = now_secs();
    conn.execute(
        "INSERT INTO custom_presets (
            name, category, target_format, width_px, height_px, dpi,
            min_bytes, max_bytes, normalize_white_balance,
            background_r, background_g, background_b, created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
        ON CONFLICT(name) DO UPDATE SET
            category = ?2, target_format = ?3, width_px = ?4, height_px = ?5, dpi = ?6,
            min_bytes = ?7, max_bytes = ?8, normalize_white_balance = ?9,
            background_r = ?10, background_g = ?11, background_b = ?12",
        rusqlite::params![
            preset.name,
            preset.category,
            preset.target_format,
            preset.width_px as i64,
            preset.height_px as i64,
            preset.dpi as i64,
            preset.min_bytes as i64,
            preset.max_bytes as i64,
            if preset.normalize_white_balance { 1 } else { 0 },
            preset.background_rgb[0] as i64,
            preset.background_rgb[1] as i64,
            preset.background_rgb[2] as i64,
            now,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Load all custom presets from the database.
pub fn load_presets() -> Vec<CustomPresetRecord> {
    let conn = match open_connection() {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut stmt = match conn.prepare(
        "SELECT name, category, target_format, width_px, height_px, dpi,
                min_bytes, max_bytes, normalize_white_balance,
                background_r, background_g, background_b
         FROM custom_presets ORDER BY name ASC",
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let mut list = Vec::new();
    let rows = stmt.query_map([], |row| {
        let name: String = row.get(0)?;
        let category: String = row.get(1)?;
        let target_format: String = row.get(2)?;
        let width_px: u32 = row.get(3)?;
        let height_px: u32 = row.get(4)?;
        let dpi: u32 = row.get(5)?;
        let min_bytes: u64 = row.get(6)?;
        let max_bytes: u64 = row.get(7)?;
        let norm_int: i32 = row.get(8)?;
        let br: u8 = row.get(9)?;
        let bg: u8 = row.get(10)?;
        let bb: u8 = row.get(11)?;

        Ok(CustomPresetRecord {
            name,
            category,
            target_format,
            width_px,
            height_px,
            dpi,
            min_bytes,
            max_bytes,
            normalize_white_balance: norm_int != 0,
            background_rgb: [br, bg, bb],
        })
    });

    if let Ok(iter) = rows {
        for item in iter.flatten() {
            list.push(item);
        }
    }
    list
}

/// Delete a custom preset by name.
pub fn delete_preset(name: &str) -> Result<(), String> {
    let conn = open_connection()?;
    conn.execute(
        "DELETE FROM custom_presets WHERE name = ?1",
        rusqlite::params![name],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

// ============================================================================
// Custom Script Commands Store
// ============================================================================

/// Save or update a custom script command.
pub fn save_command(cmd: &CustomCommandRecord) -> Result<(), String> {
    let conn = open_connection()?;
    let now = now_secs();
    conn.execute(
        "INSERT INTO custom_commands (id, title, script_lang, script_body, shortcut, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(id) DO UPDATE SET
            title = ?2, script_lang = ?3, script_body = ?4, shortcut = ?5",
        rusqlite::params![
            cmd.id,
            cmd.title,
            cmd.script_lang,
            cmd.script_body,
            cmd.shortcut,
            now
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Load all custom script commands.
pub fn load_commands() -> Vec<CustomCommandRecord> {
    let conn = match open_connection() {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut stmt = match conn.prepare(
        "SELECT id, title, script_lang, script_body, shortcut FROM custom_commands ORDER BY title ASC",
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let mut list = Vec::new();
    let rows = stmt.query_map([], |row| {
        Ok(CustomCommandRecord {
            id: row.get(0)?,
            title: row.get(1)?,
            script_lang: row.get(2)?,
            script_body: row.get(3)?,
            shortcut: row.get(4)?,
        })
    });

    if let Ok(iter) = rows {
        for item in iter.flatten() {
            list.push(item);
        }
    }
    list
}

/// Delete a custom script command by ID.
pub fn delete_command(id: &str) -> Result<(), String> {
    let conn = open_connection()?;
    conn.execute(
        "DELETE FROM custom_commands WHERE id = ?1",
        rusqlite::params![id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_db_roundtrip() {
        let temp_dir = std::env::temp_dir().join(format!("mossambi_db_test_{}", now_secs()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let test_path = temp_dir.join("test_file.txt");
        let _ = std::fs::write(&test_path, b"hello state db");

        let tags = vec!["Important".to_string(), "Work".to_string()];
        let res = set_file_tags(&test_path, &tags);
        assert!(res.is_ok());

        let retrieved = get_file_tags(&test_path);
        assert_eq!(retrieved, tags);

        let files = list_files_by_tag("Important");
        assert!(files.contains(&test_path));

        let preset = CustomPresetRecord {
            name: "Test Photo".into(),
            category: "Passport".into(),
            target_format: "JPG".into(),
            width_px: 600,
            height_px: 600,
            dpi: 300,
            min_bytes: 10240,
            max_bytes: 51200,
            normalize_white_balance: true,
            background_rgb: [255, 255, 255],
        };
        assert!(save_preset(&preset).is_ok());
        let presets = load_presets();
        assert!(presets.iter().any(|p| p.name == "Test Photo"));

        assert!(delete_preset("Test Photo").is_ok());
        let presets_after = load_presets();
        assert!(!presets_after.iter().any(|p| p.name == "Test Photo"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
