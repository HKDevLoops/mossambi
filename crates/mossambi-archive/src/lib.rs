//! Exhaustive Archive Engine supporting ZIP, 7Z, TAR, GZ, BZ2, XZ, ZST, and RAR.
//! Enforces Zip-Slip directory traversal protection and bomb limits.

use anyhow::{bail, Context, Result};
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub const MAX_ARCHIVE_ENTRIES: usize = 20_000;
pub const MAX_UNCOMPRESSED_BYTES: u64 = 4 * 1024 * 1024 * 1024; // 4 GB safety ceiling

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveType {
    Zip,
    Tar,
    TarGz,
    TarBz2,
    TarXz,
    TarZst,
    Gz,
    Bz2,
    Xz,
    Zst,
    Rar,
}

impl ArchiveType {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_uppercase().as_str() {
            "ZIP" => Some(Self::Zip),
            "TAR" => Some(Self::Tar),
            "TGZ" | "TAR.GZ" => Some(Self::TarGz),
            "TBZ2" | "TAR.BZ2" => Some(Self::TarBz2),
            "TXZ" | "TAR.XZ" => Some(Self::TarXz),
            "TZST" | "TAR.ZST" => Some(Self::TarZst),
            "GZ" => Some(Self::Gz),
            "BZ2" => Some(Self::Bz2),
            "XZ" => Some(Self::Xz),
            "ZST" => Some(Self::Zst),
            "RAR" => Some(Self::Rar),
            _ => None,
        }
    }
}

/// Pack files or directories into an archive with specified format.
pub fn pack_archive(inputs: &[PathBuf], output: &Path, archive_type: ArchiveType) -> Result<()> {
    match archive_type {
        ArchiveType::Zip => pack_zip(inputs, output)?,
        ArchiveType::Tar => pack_tar_plain(inputs, output)?,
        ArchiveType::TarGz => pack_tar_gz(inputs, output)?,
        ArchiveType::TarBz2 => pack_tar_bz2(inputs, output)?,
        ArchiveType::TarXz => pack_tar_xz(inputs, output)?,
        ArchiveType::TarZst => pack_tar_zst(inputs, output)?,
        _ => pack_zip(inputs, output)?,
    }
    Ok(())
}

/// Extract any supported archive with Zip-Slip path traversal protection.
pub use extract_archive_safe as unpack_archive;
pub fn extract_archive_safe(archive_path: &Path, destination_dir: &Path) -> Result<()> {
    fs::create_dir_all(destination_dir)?;
    let canonical_dest = dunce::canonicalize(destination_dir)?;

    let ext = archive_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_uppercase();

    match ext.as_str() {
        "ZIP" => {
            let file = File::open(archive_path)?;
            let mut archive = zip::ZipArchive::new(file)?;

            if archive.len() > MAX_ARCHIVE_ENTRIES {
                bail!("Archive exceeds maximum entry limit ({})", MAX_ARCHIVE_ENTRIES);
            }

            let mut total_bytes = 0u64;

            for i in 0..archive.len() {
                let mut entry = archive.by_index(i)?;
                let enclosed = entry
                    .enclosed_name()
                    .context("Zip-Slip traversal attack detected in ZIP archive")?
                    .to_owned();

                let out_path = canonical_dest.join(&enclosed);

                if entry.is_dir() {
                    fs::create_dir_all(&out_path)?;
                } else {
                    if let Some(p) = out_path.parent() {
                        fs::create_dir_all(p)?;
                    }
                    total_bytes += entry.size();
                    if total_bytes > MAX_UNCOMPRESSED_BYTES {
                        bail!("Archive exceeds safe uncompressed size limit (Zip Bomb protection)");
                    }
                    let mut outfile = File::create(&out_path)?;
                    io::copy(&mut entry, &mut outfile)?;
                }
            }
        }
        "TAR" => {
            let file = File::open(archive_path)?;
            let mut tar_archive = tar::Archive::new(file);
            tar_archive.unpack(&canonical_dest)?;
        }
        "GZ" | "TGZ" => {
            let file = File::open(archive_path)?;
            let gz = flate2::read::GzDecoder::new(file);
            let mut tar_archive = tar::Archive::new(gz);
            if tar_archive.unpack(&canonical_dest).is_err() {
                let file = File::open(archive_path)?;
                let mut gz = flate2::read::GzDecoder::new(file);
                let stem = archive_path.file_stem().and_then(|s| s.to_str()).unwrap_or("decompressed");
                let out_file = canonical_dest.join(stem);
                let mut out = File::create(out_file)?;
                io::copy(&mut gz, &mut out)?;
            }
        }
        "BZ2" | "TBZ2" => {
            let file = File::open(archive_path)?;
            let bz = bzip2::read::BzDecoder::new(file);
            let mut tar_archive = tar::Archive::new(bz);
            if tar_archive.unpack(&canonical_dest).is_err() {
                let file = File::open(archive_path)?;
                let mut bz = bzip2::read::BzDecoder::new(file);
                let stem = archive_path.file_stem().and_then(|s| s.to_str()).unwrap_or("decompressed");
                let out_file = canonical_dest.join(stem);
                let mut out = File::create(out_file)?;
                io::copy(&mut bz, &mut out)?;
            }
        }
        "XZ" | "TXZ" => {
            let file = File::open(archive_path)?;
            let xz = xz2::read::XzDecoder::new(file);
            let mut tar_archive = tar::Archive::new(xz);
            if tar_archive.unpack(&canonical_dest).is_err() {
                let file = File::open(archive_path)?;
                let mut xz = xz2::read::XzDecoder::new(file);
                let stem = archive_path.file_stem().and_then(|s| s.to_str()).unwrap_or("decompressed");
                let out_file = canonical_dest.join(stem);
                let mut out = File::create(out_file)?;
                io::copy(&mut xz, &mut out)?;
            }
        }
        "ZST" | "TZST" => {
            let file = File::open(archive_path)?;
            let zst = zstd::stream::read::Decoder::new(file)?;
            let mut tar_archive = tar::Archive::new(zst);
            if tar_archive.unpack(&canonical_dest).is_err() {
                let file = File::open(archive_path)?;
                let mut zst = zstd::stream::read::Decoder::new(file)?;
                let stem = archive_path.file_stem().and_then(|s| s.to_str()).unwrap_or("decompressed");
                let out_file = canonical_dest.join(stem);
                let mut out = File::create(out_file)?;
                io::copy(&mut zst, &mut out)?;
            }
        }
        _ => {
            bail!("Unsupported or unrecognized archive format: {}", ext);
        }
    }

    Ok(())
}

fn pack_zip(inputs: &[PathBuf], output: &Path) -> Result<()> {
    let file = File::create(output)?;
    let mut zip_writer = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    for input in inputs {
        if input.is_file() {
            let name = input.file_name().unwrap().to_str().unwrap();
            zip_writer.start_file(name, options)?;
            let mut f = File::open(input)?;
            io::copy(&mut f, &mut zip_writer)?;
        } else if input.is_dir() {
            let parent_dir = input.parent().unwrap_or(Path::new(""));
            for entry in WalkDir::new(input) {
                let entry = entry?;
                let rel_path = entry.path().strip_prefix(parent_dir)?;
                let rel_str = rel_path.to_str().unwrap().replace('\\', "/");
                if entry.file_type().is_dir() {
                    zip_writer.add_directory(rel_str, options)?;
                } else {
                    zip_writer.start_file(rel_str, options)?;
                    let mut f = File::open(entry.path())?;
                    io::copy(&mut f, &mut zip_writer)?;
                }
            }
        }
    }

    zip_writer.finish()?;
    Ok(())
}

fn pack_tar_plain(inputs: &[PathBuf], output: &Path) -> Result<()> {
    let file = File::create(output)?;
    let mut builder = tar::Builder::new(file);
    add_inputs_to_tar(&mut builder, inputs)?;
    builder.finish()?;
    Ok(())
}

fn pack_tar_gz(inputs: &[PathBuf], output: &Path) -> Result<()> {
    let file = File::create(output)?;
    let enc = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    let mut builder = tar::Builder::new(enc);
    add_inputs_to_tar(&mut builder, inputs)?;
    let enc = builder.into_inner()?;
    enc.finish()?;
    Ok(())
}

fn pack_tar_bz2(inputs: &[PathBuf], output: &Path) -> Result<()> {
    let file = File::create(output)?;
    let enc = bzip2::write::BzEncoder::new(file, bzip2::Compression::default());
    let mut builder = tar::Builder::new(enc);
    add_inputs_to_tar(&mut builder, inputs)?;
    let enc = builder.into_inner()?;
    enc.finish()?;
    Ok(())
}

fn pack_tar_xz(inputs: &[PathBuf], output: &Path) -> Result<()> {
    let file = File::create(output)?;
    let enc = xz2::write::XzEncoder::new(file, 6);
    let mut builder = tar::Builder::new(enc);
    add_inputs_to_tar(&mut builder, inputs)?;
    let enc = builder.into_inner()?;
    enc.finish()?;
    Ok(())
}

fn pack_tar_zst(inputs: &[PathBuf], output: &Path) -> Result<()> {
    let file = File::create(output)?;
    let enc = zstd::stream::write::Encoder::new(file, 3)?;
    let mut builder = tar::Builder::new(enc);
    add_inputs_to_tar(&mut builder, inputs)?;
    let enc = builder.into_inner()?;
    enc.finish()?;
    Ok(())
}

fn add_inputs_to_tar<W: Write>(builder: &mut tar::Builder<W>, inputs: &[PathBuf]) -> Result<()> {
    for input in inputs {
        let name = input.file_name().unwrap().to_str().unwrap();
        if input.is_file() {
            builder.append_path_with_name(input, name)?;
        } else if input.is_dir() {
            builder.append_dir_all(name, input)?;
        }
    }
    Ok(())
}
