//! CLI handler for presets management
//!
//! Pure ASCII. Implements `mossambi presets` list, export, import, delete.

use crate::cli::output::ExitCode;
use crate::core::presets::{
    delete_custom_preset, export_preset_to_file, import_preset_from_file, list_all_presets,
};
use anyhow::Result;
use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand, Debug, PartialEq)]
pub enum PresetsCommand {
    /// List all available conversion presets (built-in and custom).
    List {
        /// Optional category filter (e.g. Image, Document, Exam, Media).
        #[arg(value_name = "CATEGORY")]
        category: Option<String>,
    },
    /// Export a preset to a standalone JSON file.
    Export {
        /// Unique preset identifier (e.g. web-ultra-webp, upsc-photo).
        #[arg(value_name = "ID")]
        id: String,
        /// Destination output path for the JSON file.
        #[arg(short = 'o', long = "output", value_name = "OUTPUT")]
        output: PathBuf,
    },
    /// Import a preset from a JSON file into user configuration.
    Import {
        /// Source path to the preset JSON file.
        #[arg(value_name = "FILE")]
        path: PathBuf,
    },
    /// Delete a custom preset.
    Delete {
        /// Unique preset identifier to delete.
        #[arg(value_name = "ID")]
        id: String,
    },
}

pub fn execute(cmd: PresetsCommand) -> Result<ExitCode> {
    match cmd {
        PresetsCommand::List { category } => {
            let presets = list_all_presets(category.as_deref());
            println!("Mossambi Presets ({} total):", presets.len());
            println!(
                "{:<24} {:<10} {:<8} {:<36}",
                "ID", "CATEGORY", "TARGET", "NAME"
            );
            println!("{}", "-".repeat(80));
            for p in &presets {
                let badge = if p.is_builtin { "builtin" } else { "user" };
                println!(
                    "{:<24} {:<10} {:<8} {:<28} [{}]",
                    p.id, p.category, p.output_format, p.name, badge
                );
            }
            Ok(ExitCode::Ok)
        }
        PresetsCommand::Export { id, output } => match export_preset_to_file(&id, &output) {
            Ok(()) => {
                println!("Successfully exported preset '{}' to {:?}", id, output);
                Ok(ExitCode::Ok)
            }
            Err(e) => {
                eprintln!("Export failed: {}", e);
                Ok(ExitCode::Error)
            }
        },
        PresetsCommand::Import { path } => match import_preset_from_file(&path) {
            Ok(p) => {
                println!("Successfully imported preset '{}' ({})", p.name, p.id);
                Ok(ExitCode::Ok)
            }
            Err(e) => {
                eprintln!("Import failed: {}", e);
                Ok(ExitCode::Error)
            }
        },
        PresetsCommand::Delete { id } => match delete_custom_preset(&id) {
            Ok(true) => {
                println!("Successfully deleted custom preset '{}'", id);
                Ok(ExitCode::Ok)
            }
            Ok(false) => {
                eprintln!(
                    "Preset '{}' was not found or is a protected built-in preset",
                    id
                );
                Ok(ExitCode::Error)
            }
            Err(e) => {
                eprintln!("Deletion failed: {}", e);
                Ok(ExitCode::Error)
            }
        },
    }
}
