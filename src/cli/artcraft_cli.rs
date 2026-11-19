//! CLI handler for ArtCraft ecosystem integration
//!
//! Pure ASCII. Implements `mossambi artcraft` status and convert.

use crate::cli::output::ExitCode;
use crate::core::artcraft::{
    deckcraft_convert, designcraft_export, vectorcraft_convert, ArtcraftApp, ArtcraftDiscovery,
};
use anyhow::Result;
use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand, Debug, PartialEq)]
pub enum ArtcraftCommand {
    /// Show installed ArtCraft application status and detected CLI paths.
    Status,
    /// Convert a file using installed ArtCraft tools (VectorCraft, DeckCraft, DesignCraft).
    Convert {
        /// Source input file.
        input: PathBuf,
        /// Destination output file.
        output: PathBuf,
        /// Optional scale factor for raster outputs.
        #[arg(long = "scale")]
        scale: Option<f32>,
        /// Optional pixels-per-inch (PPI) for raster outputs.
        #[arg(long = "ppi")]
        ppi: Option<u32>,
    },
}

pub fn execute(cmd: ArtcraftCommand) -> Result<ExitCode> {
    match cmd {
        ArtcraftCommand::Status => {
            let disc = ArtcraftDiscovery::detect();
            println!("ArtCraft Ecosystem Detection Status:");
            println!("{}", "-".repeat(70));

            let apps = [
                ArtcraftApp::VectorCraft,
                ArtcraftApp::DesignCraft,
                ArtcraftApp::DeckCraft,
                ArtcraftApp::FilmCraft,
                ArtcraftApp::EffectCraft,
                ArtcraftApp::AniCraft,
            ];

            let mut count = 0;
            for app in apps {
                if let Some(p) = disc.get_cli_path(app) {
                    println!("[INSTALLED] {:<14} -> {:?}", app.name(), p);
                    count += 1;
                } else {
                    println!("[NOT FOUND] {:<14} ({})", app.name(), app.cli_binary_name());
                }
            }

            println!("{}", "-".repeat(70));
            println!("Total ArtCraft apps detected: {} of {}", count, apps.len());
            Ok(ExitCode::Ok)
        }
        ArtcraftCommand::Convert {
            input,
            output,
            scale,
            ppi,
        } => {
            let in_ext = input
                .extension()
                .map(|e| e.to_string_lossy().to_string())
                .unwrap_or_default();

            let out_ext = output
                .extension()
                .map(|e| e.to_string_lossy().to_string())
                .unwrap_or_default();

            println!(
                "Routing conversion {:?} -> {:?} via ArtCraft Engine...",
                input, output
            );

            // VectorCraft handles vector, CAD, raster, afdesign, ai, dxf
            if crate::core::artcraft::is_vectorcraft_readable(&in_ext)
                && crate::core::artcraft::is_vectorcraft_writable(&out_ext)
            {
                match vectorcraft_convert(&input, &output, scale, ppi) {
                    Ok(()) => {
                        println!(
                            "VectorCraft successfully converted {:?} to {:?}",
                            input, output
                        );
                        return Ok(ExitCode::Ok);
                    }
                    Err(e) => eprintln!("VectorCraft warning: {}", e),
                }
            }

            // DeckCraft handles presentations (pptx, deckcraft)
            if crate::core::artcraft::is_deckcraft_readable(&in_ext) {
                match deckcraft_convert(&input, &output) {
                    Ok(()) => {
                        println!(
                            "DeckCraft successfully converted {:?} to {:?}",
                            input, output
                        );
                        return Ok(ExitCode::Ok);
                    }
                    Err(e) => eprintln!("DeckCraft warning: {}", e),
                }
            }

            // DesignCraft handles desktop publishing (idml, designcraft)
            if crate::core::artcraft::is_designcraft_readable(&in_ext) {
                match designcraft_export(&input, &output) {
                    Ok(()) => {
                        println!(
                            "DesignCraft successfully exported {:?} to {:?}",
                            input, output
                        );
                        return Ok(ExitCode::Ok);
                    }
                    Err(e) => eprintln!("DesignCraft warning: {}", e),
                }
            }

            eprintln!(
                "No compatible ArtCraft application was able to convert this format pairing."
            );
            Ok(ExitCode::Error)
        }
    }
}
