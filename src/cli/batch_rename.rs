//! Pattern-based batch rename command handler (dry-run preview by default).
//! Uses core path_safety validation and action_journal recording.
//! Pure ASCII by contract.

use anyhow::{anyhow, bail, Result};
use serde_json::json;
use std::path::{Path, PathBuf};

use crate::core::action_journal::record_rename;
use crate::core::path_safety::validate_component;

#[allow(clippy::too_many_arguments)]
pub fn handle_batch_rename(
    files: &[PathBuf],
    find: &str,
    replace: &str,
    prefix: Option<&str>,
    suffix: Option<&str>,
    start: usize,
    apply: bool,
    as_json: bool,
) -> Result<()> {
    let mut plan: Vec<Value> = Vec::new();

    for (i, src) in files.iter().enumerate() {
        if !src.exists() {
            bail!("File does not exist: {}", src.display());
        }
        let parent = src.parent().unwrap_or_else(|| Path::new("."));
        let stem = src
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow!("file has no name: {}", src.display()))?;
        let ext = src.extension().and_then(|e| e.to_str()).unwrap_or("");

        let mut new_stem = stem.replace(find, replace);
        if let Some(p) = prefix {
            new_stem = format!("{}{}", p, new_stem);
        }
        if let Some(s) = suffix {
            new_stem = format!("{}{}", new_stem, s);
        }
        new_stem = new_stem.replace("%N", &(start + i).to_string());

        validate_component(&new_stem)
            .map_err(|e| anyhow!("invalid generated name '{}': {}", new_stem, e))?;

        let dest = if ext.is_empty() {
            parent.join(&new_stem)
        } else {
            parent.join(format!("{}.{}", new_stem, ext))
        };
        plan.push(json!({
            "from": src.display().to_string(),
            "to": dest.display().to_string()
        }));
    }

    if apply {
        for item in &plan {
            let from = PathBuf::from(item["from"].as_str().unwrap_or(""));
            let to = PathBuf::from(item["to"].as_str().unwrap_or(""));
            if from == to {
                continue;
            }
            std::fs::rename(&from, &to).map_err(|e| anyhow!("rename failed: {}", e))?;
            record_rename(from, to);
        }
    }

    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "dry_run": !apply,
                "count": plan.len(),
                "plan": plan
            }))?
        );
    } else {
        if !apply {
            println!(
                "Dry-run (pass --apply to commit) - {} rename(s):",
                plan.len()
            );
        } else {
            println!("Applied {} rename(s):", plan.len());
        }
        for item in &plan {
            println!(
                "  {} -> {}",
                item["from"].as_str().unwrap_or(""),
                item["to"].as_str().unwrap_or("")
            );
        }
    }
    Ok(())
}

type Value = serde_json::Value;
