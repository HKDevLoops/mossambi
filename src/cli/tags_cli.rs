//! Tag command handlers: list, add, remove, find.
//! Wraps core::tags and core::state_db. Pure ASCII by contract.

use anyhow::{anyhow, bail, Result};
use serde_json::json;
use std::path::PathBuf;

use crate::cli::TagsCommand;
use crate::core::state_db;
use crate::core::tags::{available_tags, clear_tag, get_tags, sanitize_new_tag, set_tag};
use crate::core::MossambiConfig;

pub fn handle_tags(action: TagsCommand, as_json: bool) -> Result<()> {
    match action {
        TagsCommand::List { path } => {
            let (label, values): (&str, Vec<String>) = match path {
                Some(p) => {
                    if !p.exists() {
                        bail!("Path does not exist: {}", p.display());
                    }
                    ("tags", get_tags(&p))
                }
                None => ("available", available_tags()),
            };
            if as_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({ label: values }))?
                );
            } else {
                if values.is_empty() {
                    println!("No {} tags.", label);
                } else {
                    for tag in &values {
                        println!("{}", tag);
                    }
                }
            }
            Ok(())
        }
        TagsCommand::Add { path, tags } => {
            if !path.exists() {
                bail!("Path does not exist: {}", path.display());
            }
            let custom = available_tags();
            let mut applied = Vec::new();
            for raw in &tags {
                let tag = sanitize_new_tag(raw, &custom)
                    .map_err(|e| anyhow!("invalid tag '{}': {}", raw, e))?;
                // A brand-new custom tag is validated by sanitize_new_tag but is
                // not yet in config.custom_tags, so set_tag would reject it as
                // unknown. Register it first (unless it is a built-in preset).
                let is_preset = crate::core::tags::TAG_PRESETS
                    .iter()
                    .any(|p| p.eq_ignore_ascii_case(&tag));
                if !is_preset && !custom.iter().any(|c| c.eq_ignore_ascii_case(&tag)) {
                    let mut cfg = MossambiConfig::load();
                    if !cfg.custom_tags.iter().any(|c| c.eq_ignore_ascii_case(&tag)) {
                        cfg.custom_tags.push(tag.clone());
                        cfg.save()?;
                    }
                }
                set_tag(&path, &tag).map_err(|e| anyhow!("failed to add tag: {}", e))?;
                applied.push(tag);
            }
            if as_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "status": "success", "operation": "tag-add",
                        "path": path.display().to_string(), "tags": applied
                    }))?
                );
            } else {
                println!("Added tags to {}: {}", path.display(), applied.join(", "));
            }
            Ok(())
        }
        TagsCommand::Remove { path, tags } => {
            if !path.exists() {
                bail!("Path does not exist: {}", path.display());
            }
            let mut removed = Vec::new();
            for tag in &tags {
                clear_tag(&path, tag).map_err(|e| anyhow!("failed to remove tag: {}", e))?;
                removed.push(tag.clone());
            }
            if as_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "status": "success", "operation": "tag-remove",
                        "path": path.display().to_string(), "tags": removed
                    }))?
                );
            } else {
                println!(
                    "Removed tags from {}: {}",
                    path.display(),
                    removed.join(", ")
                );
            }
            Ok(())
        }
        TagsCommand::Find { tag } => {
            let matches: Vec<String> = state_db::list_files_by_tag(&tag)
                .iter()
                .map(|p: &PathBuf| p.display().to_string())
                .collect();
            if as_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "tag": tag, "count": matches.len(), "paths": matches
                    }))?
                );
            } else {
                if matches.is_empty() {
                    println!("No files found with tag '{}'.", tag);
                } else {
                    for p in &matches {
                        println!("{}", p);
                    }
                }
            }
            Ok(())
        }
    }
}
