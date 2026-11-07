//! Sandboxed Scripting Engine for Mossambi
//! Embeds a safe, sandboxed Rhai runtime with CPU instruction limits, memory caps,
//! and native Mossambi automation primitives (transcode, tagging, sha256 checksums,
//! and batch operations).
//! All script invocations can be dispatched asynchronously to ensure zero UI thread blocking.

use crate::core::platform_paths::app_data_dir;
use crate::core::state_db;
use crate::core::tags;
use rhai::{Array, Dynamic, Engine, Scope};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Structured outcome of a script execution.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScriptExecutionOutcome {
    pub success: bool,
    pub return_value: String,
    pub logs: Vec<String>,
    pub error: Option<String>,
}

/// Source location where a script originates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptSource {
    Disk(PathBuf),
    Database,
}

/// Runnable script descriptor registered in the Command Palette and menus.
#[derive(Debug, Clone)]
pub struct ScriptCommand {
    pub id: String,
    pub title: String,
    pub source: ScriptSource,
    pub body: String,
    pub shortcut: Option<String>,
}

/// Returns the user scripts directory (%LOCALAPPDATA%/mossambi/scripts/).
pub fn get_user_scripts_dir() -> PathBuf {
    let dir = app_data_dir().join("mossambi").join("scripts");
    if !dir.exists() {
        let _ = fs::create_dir_all(&dir);
        seed_starter_scripts(&dir);
    }
    dir
}

/// Populates starter template scripts if the directory was newly created.
fn seed_starter_scripts(dir: &Path) {
    let stamp_script = dir.join("stamp_reviewed.rhai");
    if !stamp_script.exists() {
        let content = r#"// Tags the current file as "Reviewed" and computes its SHA-256
if current_file != "" {
    mossambi_set_tag(current_file, "Reviewed");
    let hash = mossambi_sha256(current_file);
    print("Stamped " + current_file + " with [Reviewed]. Hash: " + hash);
} else {
    print("No file selected.");
}
"#;
        let _ = fs::write(stamp_script, content);
    }

    let inspect_script = dir.join("inspect_metadata.rhai");
    if !inspect_script.exists() {
        let content = r#"// Inspects current file size and existing tags
if current_file != "" {
    let sz = mossambi_file_size(current_file);
    let tags = mossambi_get_tags(current_file);
    print("File: " + current_file + " | Size: " + sz + " bytes | Tags: " + tags);
} else {
    print("No file selected.");
}
"#;
        let _ = fs::write(inspect_script, content);
    }
}

/// Loads all runnable scripts from both disk (.rhai) and SQLite database.
pub fn load_all_user_scripts() -> Vec<ScriptCommand> {
    let mut commands = Vec::new();

    // 1. Scan disk scripts
    let scripts_dir = get_user_scripts_dir();
    if let Ok(entries) = fs::read_dir(&scripts_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("rhai") {
                if let Ok(body) = fs::read_to_string(&path) {
                    let stem = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("script")
                        .to_string();
                    let title = stem.replace('_', " ");
                    commands.push(ScriptCommand {
                        id: format!("disk:{}", stem),
                        title,
                        source: ScriptSource::Disk(path),
                        body,
                        shortcut: None,
                    });
                }
            }
        }
    }

    // 2. Scan SQLite custom commands
    for record in state_db::load_commands() {
        commands.push(ScriptCommand {
            id: format!("db:{}", record.id),
            title: record.title,
            source: ScriptSource::Database,
            body: record.script_body,
            shortcut: record.shortcut,
        });
    }

    commands
}

/// Builds a sandboxed Rhai engine with strict security limits and mossambi primitives.
fn build_engine(logs: Arc<Mutex<Vec<String>>>) -> Engine {
    let mut engine = Engine::new();

    // Sandboxing limits to prevent infinite loops, denial of service, and excessive memory allocations
    engine.set_max_operations(100_000);
    engine.set_max_call_levels(64);
    engine.set_max_string_size(10_000_000);
    engine.set_max_array_size(10_000);
    engine.set_max_map_size(10_000);

    // Capture print(...) output into logs buffer
    let logs_for_print = Arc::clone(&logs);
    engine.on_print(move |msg| {
        if let Ok(mut l) = logs_for_print.lock() {
            l.push(msg.to_string());
        }
    });

    // mossambi_log(msg)
    let logs_for_fn = Arc::clone(&logs);
    engine.register_fn("mossambi_log", move |msg: &str| {
        if let Ok(mut l) = logs_for_fn.lock() {
            l.push(msg.to_string());
        }
    });

    // mossambi_sha256(path) -> String
    engine.register_fn("mossambi_sha256", |path_str: &str| -> String {
        let path = Path::new(path_str);
        if !path.exists() {
            return String::new();
        }
        let mut file = match fs::File::open(path) {
            Ok(f) => f,
            Err(_) => return String::new(),
        };
        let mut hasher = Sha256::new();
        let mut buf = [0u8; 8192];
        while let Ok(n) = file.read(&mut buf) {
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
        }
        format!("{:x}", hasher.finalize())
    });

    // mossambi_get_tags(path) -> Array
    engine.register_fn("mossambi_get_tags", |path_str: &str| -> Array {
        let path = Path::new(path_str);
        let list = tags::get_tags(path);
        list.into_iter().map(Dynamic::from).collect()
    });

    // mossambi_set_tag(path, tag) -> bool
    engine.register_fn("mossambi_set_tag", |path_str: &str, tag: &str| -> bool {
        let path = Path::new(path_str);
        tags::set_tag(path, tag).is_ok()
    });

    // mossambi_clear_tag(path, tag) -> bool
    engine.register_fn("mossambi_clear_tag", |path_str: &str, tag: &str| -> bool {
        let path = Path::new(path_str);
        tags::clear_tag(path, tag).is_ok()
    });

    // mossambi_file_exists(path) -> bool
    engine.register_fn("mossambi_file_exists", |path_str: &str| -> bool {
        Path::new(path_str).exists()
    });

    // mossambi_file_size(path) -> i64
    engine.register_fn("mossambi_file_size", |path_str: &str| -> i64 {
        fs::metadata(path_str).map(|m| m.len() as i64).unwrap_or(-1)
    });

    // mossambi_transcode(input, target_format) -> bool
    engine.register_fn(
        "mossambi_transcode",
        |input_str: &str, target_format: &str| -> bool {
            let input = Path::new(input_str);
            crate::codec::transcode_file(
                input,
                target_format,
                crate::codec::LosslessMode::StrictBitExact,
                None,
            )
            .is_ok()
        },
    );

    engine
}

/// Executes a Rhai script synchronously with provided context variables.
pub fn execute_script_sync(
    script_source: &str,
    current_file: Option<&Path>,
    selected_files: &[PathBuf],
) -> ScriptExecutionOutcome {
    let logs = Arc::new(Mutex::new(Vec::new()));
    let engine = build_engine(Arc::clone(&logs));

    let mut scope = Scope::new();
    let current_str = current_file
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    scope.push("current_file", current_str);

    let sel_array: Array = selected_files
        .iter()
        .map(|p| Dynamic::from(p.to_string_lossy().to_string()))
        .collect();
    scope.push("selected_files", sel_array);

    match engine.eval_with_scope::<Dynamic>(&mut scope, script_source) {
        Ok(result) => {
            let logs_vec = logs.lock().map(|l| l.clone()).unwrap_or_default();
            ScriptExecutionOutcome {
                success: true,
                return_value: format!("{}", result),
                logs: logs_vec,
                error: None,
            }
        }
        Err(e) => {
            let logs_vec = logs.lock().map(|l| l.clone()).unwrap_or_default();
            ScriptExecutionOutcome {
                success: false,
                return_value: String::new(),
                logs: logs_vec,
                error: Some(format!("{}", e)),
            }
        }
    }
}

/// Dispatches script execution asynchronously on a dedicated worker thread.
/// Guaranteed never to block the main egui UI thread.
pub fn execute_script_async(
    script_source: String,
    current_file: Option<PathBuf>,
    selected_files: Vec<PathBuf>,
    on_done: impl FnOnce(ScriptExecutionOutcome) + Send + 'static,
) {
    std::thread::spawn(move || {
        let outcome = execute_script_sync(
            &script_source,
            current_file.as_deref(),
            &selected_files,
        );
        on_done(outcome);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandboxed_math_and_print() {
        let script = r#"
            let a = 10;
            let b = 25;
            print("Sum is " + (a + b));
            a + b
        "#;
        let res = execute_script_sync(script, None, &[]);
        assert!(res.success);
        assert_eq!(res.return_value, "35");
        assert_eq!(res.logs.len(), 1);
        assert_eq!(res.logs[0], "Sum is 35");
    }

    #[test]
    fn test_infinite_loop_is_bounded() {
        let script = r#"
            let x = 0;
            while true {
                x += 1;
            }
        "#;
        let res = execute_script_sync(script, None, &[]);
        assert!(!res.success);
        assert!(res.error.is_some());
        let err = res.error.unwrap();
        assert!(err.contains("Too many operations"), "Error was: {}", err);
    }

    #[test]
    fn test_file_primitives() {
        let temp_dir = std::env::temp_dir().join(format!("mossambi_script_{}", std::time::SystemTime::now().elapsed().unwrap().as_nanos()));
        let _ = fs::create_dir_all(&temp_dir);
        let test_file = temp_dir.join("test.txt");
        let _ = fs::write(&test_file, b"rhai test content");

        let p_str = test_file.to_string_lossy().to_string();
        let script = format!(
            r#"
                let exists = mossambi_file_exists("{}");
                let sz = mossambi_file_size("{}");
                print("exists=" + exists + " sz=" + sz);
                sz
            "#,
            p_str.replace('\\', "\\\\"),
            p_str.replace('\\', "\\\\")
        );

        let res = execute_script_sync(&script, Some(&test_file), &[]);
        assert!(res.success);
        assert_eq!(res.return_value, "17");
        assert!(res.logs[0].contains("exists=true"));
        assert!(res.logs[0].contains("sz=17"));

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
