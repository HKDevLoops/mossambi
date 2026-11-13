//! Sandboxed Scripting Engine for Mossambi
//! Embeds safe, sandboxed Rhai and Lua (5.4) runtimes with CPU instruction limits, memory caps,
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
use std::sync::atomic::{AtomicU64, Ordering};
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

/// Scripting languages supported by the engine dispatch. The database
/// stores the lowercase id in the `script_lang` column; legacy rows
/// (and unknown ids) parse as Rhai.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ScriptLang {
    Rhai,
    Lua,
}

impl ScriptLang {
    /// Stable lowercase id stored in the database (`script_lang` column).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Rhai => "rhai",
            Self::Lua => "lua",
        }
    }

    /// Human-readable label for the UI.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Rhai => "Rhai",
            Self::Lua => "Lua",
        }
    }

    /// Parses a stored id; unknown or blank ids fall back to Rhai.
    pub fn from_str_opt(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "lua" => Self::Lua,
            _ => Self::Rhai,
        }
    }
}

/// Runnable script descriptor registered in the Command Palette and menus.
#[derive(Debug, Clone)]
pub struct ScriptCommand {
    pub id: String,
    pub title: String,
    pub source: ScriptSource,
    pub body: String,
    pub shortcut: Option<String>,
    pub lang: ScriptLang,
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

    // 1. Scan disk scripts (.rhai and .lua side by side)
    let scripts_dir = get_user_scripts_dir();
    if let Ok(entries) = fs::read_dir(&scripts_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_ascii_lowercase());
            let lang = match ext.as_deref() {
                Some("rhai") => Some(ScriptLang::Rhai),
                Some("lua") => Some(ScriptLang::Lua),
                _ => None,
            };
            if let Some(lang) = lang {
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
                        lang,
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
            lang: ScriptLang::from_str_opt(&record.script_lang),
        });
    }

    commands
}

/// Computes the SHA-256 digest of a file as lowercase hex. Missing or
/// unreadable files yield an empty string. Shared by the Rhai and Lua
/// engine registrations.
fn sha256_of_path(path_str: &str) -> String {
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
        sha256_of_path(path_str)
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

/// Renders a Lua value the way the console should show it.
fn lua_value_to_string(v: &mlua::Value) -> String {
    match v {
        mlua::Value::Nil => "nil".to_string(),
        mlua::Value::Boolean(b) => b.to_string(),
        mlua::Value::Integer(i) => i.to_string(),
        mlua::Value::Number(n) => n.to_string(),
        mlua::Value::String(s) => s.to_string_lossy().to_string(),
        other => format!("{:?}", other),
    }
}

/// Builds a sandboxed Lua (5.4) engine mirroring the Rhai sandbox: an
/// explicit safe std-library subset (coroutine, table, string, utf8, math;
/// mlua's ALL_SAFE is rejected because it also loads os/io), a memory cap,
/// an instruction-count hook budget, captured `print`, and the same
/// mossambi primitives.
fn build_lua_engine(logs: Arc<Mutex<Vec<String>>>) -> Result<mlua::Lua, mlua::Error> {
    let safe_libs = mlua::StdLib::COROUTINE
        | mlua::StdLib::TABLE
        | mlua::StdLib::STRING
        | mlua::StdLib::UTF8
        | mlua::StdLib::MATH;
    let lua = mlua::Lua::new_with(safe_libs, mlua::LuaOptions::new())?;
    lua.set_memory_limit(16 * 1024 * 1024)?;

    // Instruction budget: the hook fires every BUDGET instructions; abort
    // after MAX_HOOK_FIRES fires (mirrors the Rhai op cap in spirit).
    const BUDGET: u32 = 100_000;
    const MAX_HOOK_FIRES: u64 = 20;
    let fires = Arc::new(AtomicU64::new(0));
    let fires_hook = Arc::clone(&fires);
    lua.set_hook(
        mlua::HookTriggers::new().every_nth_instruction(BUDGET),
        move |_, _| {
            let n = fires_hook.fetch_add(1, Ordering::Relaxed);
            if n >= MAX_HOOK_FIRES {
                Err(mlua::Error::RuntimeError("Too many operations".to_string()))
            } else {
                Ok(mlua::VmState::Continue)
            }
        },
    );

    let globals = lua.globals();

    // print(...) captured into the logs buffer (Lua prints tab-joined).
    let logs_for_print = Arc::clone(&logs);
    let print_fn = lua.create_function(move |_, args: mlua::MultiValue| {
        let parts: Vec<String> = args.into_iter().map(|v| lua_value_to_string(&v)).collect();
        if let Ok(mut l) = logs_for_print.lock() {
            l.push(parts.join("\t"));
        }
        Ok(())
    })?;
    globals.set("print", print_fn)?;

    // mossambi_log(msg)
    let logs_for_log = Arc::clone(&logs);
    let log_fn = lua.create_function(move |_, msg: String| {
        if let Ok(mut l) = logs_for_log.lock() {
            l.push(msg);
        }
        Ok(())
    })?;
    globals.set("mossambi_log", log_fn)?;

    // mossambi_sha256(path) -> String
    let sha_fn = lua.create_function(|_, path: String| Ok(sha256_of_path(&path)))?;
    globals.set("mossambi_sha256", sha_fn)?;

    // mossambi_get_tags(path) -> table
    let get_tags_fn = lua.create_function(|_, path: String| {
        let list = tags::get_tags(Path::new(&path));
        Ok(list)
    })?;
    globals.set("mossambi_get_tags", get_tags_fn)?;

    // mossambi_set_tag(path, tag) -> bool
    let set_tag_fn = lua.create_function(|_, (path, tag): (String, String)| {
        Ok(tags::set_tag(Path::new(&path), &tag).is_ok())
    })?;
    globals.set("mossambi_set_tag", set_tag_fn)?;

    // mossambi_clear_tag(path, tag) -> bool
    let clear_tag_fn = lua.create_function(|_, (path, tag): (String, String)| {
        Ok(tags::clear_tag(Path::new(&path), &tag).is_ok())
    })?;
    globals.set("mossambi_clear_tag", clear_tag_fn)?;

    // mossambi_file_exists(path) -> bool
    let exists_fn = lua.create_function(|_, path: String| Ok(Path::new(&path).exists()))?;
    globals.set("mossambi_file_exists", exists_fn)?;

    // mossambi_file_size(path) -> integer
    let size_fn = lua.create_function(|_, path: String| {
        Ok(fs::metadata(&path).map(|m| m.len() as i64).unwrap_or(-1))
    })?;
    globals.set("mossambi_file_size", size_fn)?;

    // mossambi_transcode(input, target_format) -> bool
    let transcode_fn = lua.create_function(|_, (input, format): (String, String)| {
        Ok(crate::codec::transcode_file(
            Path::new(&input),
            &format,
            crate::codec::LosslessMode::StrictBitExact,
            None,
        )
        .is_ok())
    })?;
    globals.set("mossambi_transcode", transcode_fn)?;

    Ok(lua)
}

/// Executes a script synchronously with provided context variables,
/// dispatching on the language to the Rhai or Lua sandbox.
pub fn execute_script_sync(
    script_source: &str,
    lang: ScriptLang,
    current_file: Option<&Path>,
    selected_files: &[PathBuf],
) -> ScriptExecutionOutcome {
    let logs = Arc::new(Mutex::new(Vec::new()));
    match lang {
        ScriptLang::Rhai => {
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
        ScriptLang::Lua => match build_lua_engine(Arc::clone(&logs)) {
            Ok(lua) => {
                let current_str = current_file
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();
                let globals = lua.globals();
                let _ = globals.set("current_file", current_str);
                if let Ok(table) = lua.create_table() {
                    for (i, p) in selected_files.iter().enumerate() {
                        let _ = table.set(i + 1, p.to_string_lossy().to_string());
                    }
                    let _ = globals.set("selected_files", table);
                }
                match lua.load(script_source).eval::<mlua::Value>() {
                    Ok(result) => {
                        let logs_vec = logs.lock().map(|l| l.clone()).unwrap_or_default();
                        ScriptExecutionOutcome {
                            success: true,
                            return_value: lua_value_to_string(&result),
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
            Err(e) => ScriptExecutionOutcome {
                success: false,
                return_value: String::new(),
                logs: logs.lock().map(|l| l.clone()).unwrap_or_default(),
                error: Some(format!("Lua engine init failed: {}", e)),
            },
        },
    }
}

/// Dispatches script execution asynchronously on a dedicated worker thread.
/// Guaranteed never to block the main egui UI thread.
pub fn execute_script_async(
    script_source: String,
    lang: ScriptLang,
    current_file: Option<PathBuf>,
    selected_files: Vec<PathBuf>,
    on_done: impl FnOnce(ScriptExecutionOutcome) + Send + 'static,
) {
    std::thread::spawn(move || {
        let outcome = execute_script_sync(
            &script_source,
            lang,
            current_file.as_deref(),
            &selected_files,
        );
        on_done(outcome);
    });
}

/// One rendered console line: which script ran, its outcome, and when.
#[derive(Debug, Clone)]
pub struct ScriptConsoleEntry {
    pub title: String,
    pub outcome: ScriptExecutionOutcome,
    pub time: String,
}

/// Per-app script execution console. The studio owns the channel pair;
/// runs are dispatched on worker threads and outcomes arrive via the
/// sender. The UI drains pending outcomes once per frame.
pub struct ScriptConsole {
    tx: std::sync::mpsc::Sender<(String, ScriptExecutionOutcome)>,
    rx: std::sync::mpsc::Receiver<(String, ScriptExecutionOutcome)>,
    pub entries: Vec<ScriptConsoleEntry>,
}

impl Default for ScriptConsole {
    fn default() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        Self {
            tx,
            rx,
            entries: Vec::new(),
        }
    }
}

impl ScriptConsole {
    /// Clones the sending half for a run about to be dispatched.
    pub fn sender(&self) -> std::sync::mpsc::Sender<(String, ScriptExecutionOutcome)> {
        self.tx.clone()
    }

    /// Dispatch-side convenience: record an outcome from anywhere.
    pub fn record(&self, title: &str, outcome: ScriptExecutionOutcome) {
        let _ = self.tx.send((title.to_string(), outcome));
    }

    /// Moves every outcome delivered since the last frame into entries,
    /// oldest first; keeps the buffer bounded at 50.
    pub fn drain(&mut self) {
        while let Ok((title, outcome)) = self.rx.try_recv() {
            self.entries.push(ScriptConsoleEntry {
                title,
                outcome,
                time: chrono::Local::now().format("%H:%M:%S").to_string(),
            });
        }
        while self.entries.len() > 50 {
            self.entries.remove(0);
        }
    }
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
        let res = execute_script_sync(script, ScriptLang::Rhai, None, &[]);
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
        let res = execute_script_sync(script, ScriptLang::Rhai, None, &[]);
        assert!(!res.success);
        assert!(res.error.is_some());
        let err = res.error.unwrap();
        assert!(err.contains("Too many operations"), "Error was: {}", err);
    }

    #[test]
    fn test_file_primitives() {
        let temp_dir = std::env::temp_dir().join(format!(
            "mossambi_script_{}",
            std::time::SystemTime::now().elapsed().unwrap().as_nanos()
        ));
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

        let res = execute_script_sync(&script, ScriptLang::Rhai, Some(&test_file), &[]);
        assert!(res.success);
        assert_eq!(res.return_value, "17");
        assert!(res.logs[0].contains("exists=true"));
        assert!(res.logs[0].contains("sz=17"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_script_lang_roundtrip() {
        assert_eq!(ScriptLang::from_str_opt("lua"), ScriptLang::Lua);
        assert_eq!(ScriptLang::from_str_opt("LUA"), ScriptLang::Lua);
        assert_eq!(ScriptLang::from_str_opt("rhai"), ScriptLang::Rhai);
        assert_eq!(ScriptLang::from_str_opt("unknown"), ScriptLang::Rhai);
        assert_eq!(ScriptLang::Rhai.as_str(), "rhai");
        assert_eq!(ScriptLang::Lua.as_str(), "lua");
        assert_eq!(ScriptLang::Rhai.label(), "Rhai");
        assert_eq!(ScriptLang::Lua.label(), "Lua");
    }

    #[test]
    fn test_lua_sandboxed_math_and_print() {
        let script = "local a = 10\nlocal b = 25\nprint(\"Sum is \" .. (a + b))\nreturn a + b";
        let res = execute_script_sync(script, ScriptLang::Lua, None, &[]);
        assert!(res.success);
        assert_eq!(res.return_value, "35");
        assert_eq!(res.logs.len(), 1);
        assert_eq!(res.logs[0], "Sum is 35");
    }

    #[test]
    fn test_lua_infinite_loop_is_bounded() {
        let script = "local x = 0\nwhile true do\n    x = x + 1\nend";
        let res = execute_script_sync(script, ScriptLang::Lua, None, &[]);
        assert!(!res.success);
        assert!(res.error.is_some());
        let err = res.error.unwrap();
        assert!(err.contains("Too many operations"), "Error was: {}", err);
    }

    #[test]
    fn test_lua_sandbox_denies_unsafe_libs() {
        let res = execute_script_sync("return os.getenv(\"PATH\")", ScriptLang::Lua, None, &[]);
        assert!(
            !res.success,
            "os library must not be present in the sandbox"
        );
        let res = execute_script_sync(
            "return io.open(\"Cargo.toml\", \"r\")",
            ScriptLang::Lua,
            None,
            &[],
        );
        assert!(
            !res.success,
            "io library must not be present in the sandbox"
        );
    }
}
