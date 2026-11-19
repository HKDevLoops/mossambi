//! Preview command handler: text metadata + extracted text preview.
//! Wraps core::preview. Pure ASCII by contract.

use anyhow::{bail, Result};
use serde_json::json;
use std::path::Path;

use crate::core::preview::{extract_preview_text, preview_kind};

pub fn handle_preview(file: &Path, as_json: bool) -> Result<()> {
    if !file.exists() {
        bail!("File does not exist: {}", file.display());
    }
    let kind = preview_kind(file);
    let meta = std::fs::metadata(file)?;
    let text =
        extract_preview_text(file).unwrap_or_else(|e| format!("[preview unavailable: {}]", e));

    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "path": file.display().to_string(),
                "kind": kind,
                "size_bytes": meta.len(),
                "is_dir": meta.is_dir(),
                "preview": text
            }))?
        );
    } else {
        println!("Preview: {}", file.display());
        println!("  Kind:       {}", kind);
        println!("  Size:       {} bytes", meta.len());
        println!("  Directory:  {}", meta.is_dir());
        println!("---------- preview ----------");
        println!("{}", text);
    }
    Ok(())
}
