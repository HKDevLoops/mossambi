//! Filesystem operation command handlers: copy, move, rename, delete-to-trash.
//! User-supplied destination names are validated through core::path_safety.
//! Pure ASCII by contract.

use anyhow::{anyhow, bail, Result};
use serde_json::json;
use std::path::{Path, PathBuf};

use crate::cli::FsCommand;
use crate::core::fs_utils::{
    copy_entry_recursive, delete_to_recycle_bin, move_entry, resolve_unique_paste_path,
};
use crate::core::path_safety::validate_component;

/// Validate a single user-supplied name component (rename target / leaf name).
fn validate_leaf(name: &str, label: &str) -> Result<()> {
    validate_component(name).map_err(|e| anyhow!("invalid {} '{}': {}", label, name, e))?;
    if Path::new(name).components().count() != 1 {
        bail!("{} must be a single path component, got '{}'", label, name);
    }
    Ok(())
}

/// Resolve the destination path for a source item, validating its leaf name.
fn destination_for(source: &Path, destination: &Path) -> Result<PathBuf> {
    let leaf = if destination.is_dir() {
        source
            .file_name()
            .ok_or_else(|| anyhow!("cannot derive a name for {}", source.display()))?
            .to_string_lossy()
            .into_owned()
    } else {
        destination
            .file_name()
            .ok_or_else(|| anyhow!("destination has no file name: {}", destination.display()))?
            .to_string_lossy()
            .into_owned()
    };
    validate_leaf(&leaf, "destination name")?;

    let target = if destination.is_dir() {
        destination.join(&leaf)
    } else {
        destination.to_path_buf()
    };
    Ok(resolve_unique_paste_path(&target))
}

pub fn handle_fs(action: FsCommand, as_json: bool) -> Result<()> {
    match action {
        FsCommand::Copy {
            sources,
            destination,
        } => {
            for src in &sources {
                if !src.exists() {
                    bail!("Source path does not exist: {}", src.display());
                }
            }
            if sources.len() > 1 && !destination.is_dir() {
                bail!("Copying multiple sources requires the destination to be a directory");
            }
            let mut results = Vec::new();
            for src in &sources {
                let target = destination_for(src, &destination)?;
                let bytes = copy_entry_recursive(src, &target)
                    .map_err(|e| anyhow!("copy {} failed: {}", src.display(), e))?;
                results.push(json!({
                    "source": src.display().to_string(),
                    "destination": target.display().to_string(),
                    "bytes": bytes
                }));
            }
            if as_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "status": "success", "operation": "copy", "count": results.len(), "items": results
                    }))?
                );
            } else {
                for r in &results {
                    println!(
                        "Copied {} -> {}",
                        r["source"].as_str().unwrap_or(""),
                        r["destination"].as_str().unwrap_or("")
                    );
                }
            }
            Ok(())
        }
        FsCommand::Move {
            sources,
            destination,
        } => {
            for src in &sources {
                if !src.exists() {
                    bail!("Source path does not exist: {}", src.display());
                }
            }
            if sources.len() > 1 && !destination.is_dir() {
                bail!("Moving multiple sources requires the destination to be a directory");
            }
            let mut results = Vec::new();
            for src in &sources {
                let target = destination_for(src, &destination)?;
                move_entry(src, &target)
                    .map_err(|e| anyhow!("move {} failed: {}", src.display(), e))?;
                results.push(json!({
                    "source": src.display().to_string(),
                    "destination": target.display().to_string()
                }));
            }
            if as_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "status": "success", "operation": "move", "count": results.len(), "items": results
                    }))?
                );
            } else {
                for r in &results {
                    println!(
                        "Moved {} -> {}",
                        r["source"].as_str().unwrap_or(""),
                        r["destination"].as_str().unwrap_or("")
                    );
                }
            }
            Ok(())
        }
        FsCommand::Rename { path, new_name } => {
            if !path.exists() {
                bail!("Path does not exist: {}", path.display());
            }
            validate_leaf(&new_name, "new name")?;
            let parent = path
                .parent()
                .ok_or_else(|| anyhow!("path has no parent directory: {}", path.display()))?;
            let target = parent.join(&new_name);
            let target = resolve_unique_paste_path(&target);
            std::fs::rename(&path, &target).map_err(|e| anyhow!("rename failed: {}", e))?;
            if as_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "status": "success",
                        "operation": "rename",
                        "from": path.display().to_string(),
                        "to": target.display().to_string()
                    }))?
                );
            } else {
                println!("Renamed {} -> {}", path.display(), target.display());
            }
            Ok(())
        }
        FsCommand::Delete { paths, permanent } => {
            for p in &paths {
                if !p.exists() {
                    bail!("Path does not exist: {}", p.display());
                }
            }
            if permanent {
                let mut removed = Vec::new();
                for p in &paths {
                    if p.is_dir() {
                        std::fs::remove_dir_all(p)
                            .map_err(|e| anyhow!("permanent delete failed: {}", e))?;
                    } else {
                        std::fs::remove_file(p)
                            .map_err(|e| anyhow!("permanent delete failed: {}", e))?;
                    }
                    removed.push(p.display().to_string());
                }
                if as_json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&json!({
                            "status": "success", "operation": "delete", "permanent": true, "paths": removed
                        }))?
                    );
                } else {
                    for p in &removed {
                        println!("Permanently deleted {}", p);
                    }
                }
            } else {
                delete_to_recycle_bin(&paths)
                    .map_err(|e| anyhow!("delete to trash failed: {}", e))?;
                let items: Vec<String> = paths.iter().map(|p| p.display().to_string()).collect();
                if as_json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&json!({
                            "status": "success", "operation": "delete", "permanent": false, "paths": items
                        }))?
                    );
                } else {
                    for p in &items {
                        println!("Moved to trash {}", p);
                    }
                }
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_parent_traversal_in_leaf_name() {
        assert!(validate_leaf("..", "new name").is_err());
        assert!(validate_leaf("a/b", "new name").is_err());
    }

    #[test]
    fn accepts_normal_leaf_name() {
        assert!(validate_leaf("report.txt", "new name").is_ok());
    }
}
