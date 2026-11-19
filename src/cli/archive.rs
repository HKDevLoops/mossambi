//! Archive pack and extract command handlers. Delegates to the archive module.
//! Pure ASCII by contract.

use anyhow::{bail, Context, Result};
use serde_json::json;
use std::path::{Path, PathBuf};

use crate::archive::{pack_archive, unpack_archive, ArchiveType};

pub fn handle_archive(
    inputs: &[PathBuf],
    output: &Path,
    archive_type_str: Option<&str>,
    as_json: bool,
) -> Result<()> {
    for inp in inputs {
        if !inp.exists() {
            bail!("Input path does not exist: {}", inp.display());
        }
    }

    let archive_type = if let Some(type_str) = archive_type_str {
        ArchiveType::from_extension(type_str)
            .with_context(|| format!("Unknown archive type specified: {}", type_str))?
    } else {
        let ext = output.extension().and_then(|e| e.to_str()).unwrap_or("zip");
        ArchiveType::from_extension(ext).unwrap_or(ArchiveType::Zip)
    };

    if !as_json {
        println!(
            "Compressing {} input(s) into '{}'...",
            inputs.len(),
            output.display()
        );
    }

    pack_archive(inputs, output, archive_type)?;
    let size = std::fs::metadata(output).map(|m| m.len()).unwrap_or(0);

    if as_json {
        let in_strs: Vec<String> = inputs.iter().map(|p| p.display().to_string()).collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "status": "success",
                "archive": output.display().to_string(),
                "bytes": size,
                "inputs": in_strs
            }))?
        );
    } else {
        println!(
            "Archive successfully created: {} ({:.1} KB)",
            output.display(),
            size as f64 / 1024.0
        );
    }

    Ok(())
}

pub fn handle_extract(archive: &Path, output_dir: Option<&Path>, as_json: bool) -> Result<()> {
    if !archive.exists() {
        bail!("Archive file does not exist: {}", archive.display());
    }

    let dest = match output_dir {
        Some(d) => d.to_path_buf(),
        None => {
            let stem = archive
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("extracted");
            archive
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(stem)
        }
    };

    if !as_json {
        println!(
            "Extracting archive '{}' into '{}' with Zip-Slip protection...",
            archive.display(),
            dest.display()
        );
    }

    unpack_archive(archive, &dest)?;

    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "status": "success",
                "archive": archive.display().to_string(),
                "destination": dest.display().to_string()
            }))?
        );
    } else {
        println!("Extraction complete into: {}", dest.display());
    }

    Ok(())
}
