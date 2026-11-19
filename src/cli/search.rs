//! Global filename search command handler.
//! Wraps core::fuzzy_match over a walkdir traversal. Pure ASCII by contract.

use anyhow::{anyhow, Result};
use serde_json::json;
use std::path::Path;

use crate::core::fuzzy_match;

pub fn handle_search(query: &str, root: Option<&Path>, limit: usize, as_json: bool) -> Result<()> {
    let root = root.unwrap_or_else(|| Path::new("."));
    if !root.exists() {
        return Err(anyhow!("search root does not exist: {}", root.display()));
    }

    let mut scored: Vec<(i64, String)> = Vec::new();
    for entry in walkdir::WalkDir::new(root)
        .max_depth(12)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let name = entry.file_name().to_string_lossy();
        if let Some(score) = fuzzy_match(query, &name) {
            scored.push((score, entry.path().display().to_string()));
        }
    }
    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    scored.truncate(limit);

    if as_json {
        let hits: Vec<Value> = scored
            .iter()
            .map(|(score, path)| json!({ "score": score, "path": path }))
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(
                &json!({ "query": query, "count": hits.len(), "hits": hits })
            )?
        );
    } else {
        if scored.is_empty() {
            println!("No matches for '{}' under {}.", query, root.display());
        } else {
            for (score, path) in &scored {
                println!("  {}  {}", score, path);
            }
        }
    }
    Ok(())
}

type Value = serde_json::Value;
