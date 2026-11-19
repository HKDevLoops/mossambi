//! Open With command handlers: registered handlers and custom context items.
//! Wraps gui::context_menu. Pure ASCII by contract.

use anyhow::{anyhow, bail, Result};
use serde_json::json;
use std::path::Path;

use crate::cli::OpenWithCommand;
use crate::core::MossambiConfig;
use crate::gui::context_menu::{launch_open_with_cmd, open_with_candidates, run_custom_item};

pub fn handle_open_with(action: OpenWithCommand, as_json: bool) -> Result<()> {
    match action {
        OpenWithCommand::List { file } => {
            if !file.exists() {
                bail!("File does not exist: {}", file.display());
            }
            let candidates = open_with_candidates(&file);
            let items: Vec<Value> = candidates
                .iter()
                .map(|(label, command)| json!({ "label": label, "command": command }))
                .collect();
            if as_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(
                        &json!({ "count": items.len(), "handlers": items })
                    )?
                );
            } else {
                if items.is_empty() {
                    println!("No registered handlers found for {}.", file.display());
                } else {
                    println!("Handlers for {}:", file.display());
                    for (i, item) in items.iter().enumerate() {
                        println!(
                            "  {}. {}  [{}]",
                            i + 1,
                            item["label"].as_str().unwrap_or(""),
                            item["command"].as_str().unwrap_or("")
                        );
                    }
                }
            }
            Ok(())
        }
        OpenWithCommand::Run { file, index } => {
            if !file.exists() {
                bail!("File does not exist: {}", file.display());
            }
            let candidates = open_with_candidates(&file);
            if index == 0 || index > candidates.len() {
                bail!(
                    "handler index {} out of range (1..={}); use 'open-with list' first",
                    index,
                    candidates.len()
                );
            }
            let (label, command) = &candidates[index - 1];
            let ok = launch_open_with_cmd(command, &file);
            if !ok {
                bail!("failed to launch handler '{}'", label);
            }
            if as_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "status": "success", "handler": label, "file": file.display().to_string()
                    }))?
                );
            } else {
                println!("Opened {} with {}", file.display(), label);
            }
            Ok(())
        }
        OpenWithCommand::CustomList => {
            let cfg = MossambiConfig::load();
            let items: Vec<Value> = cfg
                .context_menu_items
                .iter()
                .map(|item| {
                    json!({
                        "label": item.label,
                        "kind": format!("{:?}", item.kind),
                        "value": item.value
                    })
                })
                .collect();
            if as_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({ "count": items.len(), "items": items }))?
                );
            } else {
                if items.is_empty() {
                    println!("No custom context-menu items configured.");
                } else {
                    for item in &items {
                        println!(
                            "  {} [{}] {}",
                            item["label"].as_str().unwrap_or(""),
                            item["kind"].as_str().unwrap_or(""),
                            item["value"].as_str().unwrap_or("")
                        );
                    }
                }
            }
            Ok(())
        }
        OpenWithCommand::CustomRun { label, file } => {
            let cfg = MossambiConfig::load();
            let needle = label.to_ascii_lowercase();
            let item = cfg
                .context_menu_items
                .iter()
                .find(|i| i.label.to_ascii_lowercase().contains(&needle))
                .ok_or_else(|| anyhow!("no custom item matching '{}'", label))?;
            let path_ref: Option<&Path> = file.as_deref();
            let ok = run_custom_item(item, path_ref);
            if !ok {
                bail!("custom item '{}' did not run successfully", item.label);
            }
            if as_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "status": "success", "item": item.label
                    }))?
                );
            } else {
                println!("Ran custom item '{}'", item.label);
            }
            Ok(())
        }
    }
}

type Value = serde_json::Value;
