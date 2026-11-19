//! Scripts command handlers: list and run user scripts.
//! Wraps core::scripting. Pure ASCII by contract.

use anyhow::{anyhow, bail, Result};
use serde_json::json;

use crate::cli::ScriptsCommand;
use crate::core::scripting::{execute_script_sync, load_all_user_scripts, ScriptLang};

pub fn handle_scripts(action: ScriptsCommand, as_json: bool) -> Result<()> {
    match action {
        ScriptsCommand::List => {
            let scripts = load_all_user_scripts();
            let items: Vec<Value> = scripts
                .iter()
                .map(|s| {
                    json!({
                        "id": s.id,
                        "title": s.title,
                        "lang": s.lang.as_str(),
                        "shortcut": s.shortcut
                    })
                })
                .collect();
            if as_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(
                        &json!({ "count": items.len(), "scripts": items })
                    )?
                );
            } else {
                if items.is_empty() {
                    println!("No user scripts found.");
                } else {
                    for item in &items {
                        println!(
                            "  {} [{}] {}",
                            item["id"].as_str().unwrap_or(""),
                            item["lang"].as_str().unwrap_or(""),
                            item["title"].as_str().unwrap_or("")
                        );
                    }
                }
            }
            Ok(())
        }
        ScriptsCommand::Run { id, file } => {
            let scripts = load_all_user_scripts();
            let needle = id.to_ascii_lowercase();
            let script = scripts
                .iter()
                .find(|s| {
                    s.id.to_ascii_lowercase().contains(&needle)
                        || s.title.to_ascii_lowercase().contains(&needle)
                })
                .ok_or_else(|| anyhow!("no script matching '{}'", id))?;

            let lang = ScriptLang::from_str_opt(script.lang.as_str());
            let current = file.as_deref();
            let outcome = execute_script_sync(&script.body, lang, current, &[]);

            if as_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "script": script.title,
                        "success": outcome.success,
                        "return_value": outcome.return_value,
                        "logs": outcome.logs,
                        "error": outcome.error
                    }))?
                );
            } else {
                if !outcome.success {
                    if let Some(err) = &outcome.error {
                        eprintln!("Script '{}' failed: {}", script.title, err);
                    }
                    bail!("script execution failed");
                }
                println!("Script '{}' completed.", script.title);
                for line in &outcome.logs {
                    println!("  {}", line);
                }
                if !outcome.return_value.is_empty() {
                    println!("  => {}", outcome.return_value);
                }
            }
            Ok(())
        }
    }
}

type Value = serde_json::Value;
