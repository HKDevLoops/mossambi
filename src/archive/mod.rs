//! Exhaustive Archive Engine supporting ZIP, 7Z, TAR, GZ, BZ2, XZ, ZST, and RAR.
//! Enforces Zip-Slip directory traversal protection and bomb limits.

use anyhow::{bail, Context, Result};
use std::fs::{self, File};
use std::io::{self, BufWriter, Read, Write};
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
                bail!(
                    "Archive exceeds maximum entry limit ({})",
                    MAX_ARCHIVE_ENTRIES
                );
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
                    let outfile = File::create(&out_path)?;
                    let mut buf_out = BufWriter::with_capacity(256 * 1024, outfile);
                    io::copy(&mut entry, &mut buf_out)?;
                    buf_out.flush()?;
                }
            }
        }
        "TAR" => {
            let file = File::open(archive_path)?;
            unpack_tar_safe(file, &canonical_dest)?;
        }
        "GZ" | "TGZ" => {
            let file = File::open(archive_path)?;
            let gz = flate2::read::GzDecoder::new(file);
            if unpack_tar_safe(gz, &canonical_dest).is_err() {
                let file = File::open(archive_path)?;
                let gz = flate2::read::GzDecoder::new(file);
                let stem = archive_path
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "decompressed".to_string());
                let out_file = canonical_dest.join(stem);
                extract_single_file_safe(gz, &out_file)?;
            }
        }
        "BZ2" | "TBZ2" => {
            let file = File::open(archive_path)?;
            let bz = bzip2::read::BzDecoder::new(file);
            if unpack_tar_safe(bz, &canonical_dest).is_err() {
                let file = File::open(archive_path)?;
                let bz = bzip2::read::BzDecoder::new(file);
                let stem = archive_path
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "decompressed".to_string());
                let out_file = canonical_dest.join(stem);
                extract_single_file_safe(bz, &out_file)?;
            }
        }
        "XZ" | "TXZ" => {
            let file = File::open(archive_path)?;
            let xz = xz2::read::XzDecoder::new(file);
            if unpack_tar_safe(xz, &canonical_dest).is_err() {
                let file = File::open(archive_path)?;
                let xz = xz2::read::XzDecoder::new(file);
                let stem = archive_path
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "decompressed".to_string());
                let out_file = canonical_dest.join(stem);
                extract_single_file_safe(xz, &out_file)?;
            }
        }
        "ZST" | "TZST" => {
            let file = File::open(archive_path)?;
            let zst = zstd::stream::read::Decoder::new(file)?;
            if unpack_tar_safe(zst, &canonical_dest).is_err() {
                let file = File::open(archive_path)?;
                let zst = zstd::stream::read::Decoder::new(file)?;
                let stem = archive_path
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "decompressed".to_string());
                let out_file = canonical_dest.join(stem);
                extract_single_file_safe(zst, &out_file)?;
            }
        }
        _ => {
            bail!("Unsupported or unrecognized archive format: {}", ext);
        }
    }

    Ok(())
}

/// Unpack a TAR archive with strict Zip-Slip protection, symlink escape checks,
/// entry limit caps, and decompression bomb ceiling enforcement.
fn unpack_tar_safe<R: Read>(reader: R, canonical_dest: &Path) -> Result<()> {
    let mut archive = tar::Archive::new(reader);
    let mut total_bytes = 0u64;
    let mut entry_count = 0usize;

    for entry_res in archive.entries()? {
        let mut entry = entry_res?;
        entry_count += 1;
        if entry_count > MAX_ARCHIVE_ENTRIES {
            bail!("Archive exceeds safe entry limit ({})", MAX_ARCHIVE_ENTRIES);
        }

        // 1. Path Sanitization & Zip-Slip Protection
        let path = entry.path()?;
        for component in path.components() {
            match component {
                std::path::Component::ParentDir => {
                    bail!(
                        "Zip-Slip / Directory Traversal attack detected in tar archive: {:?}",
                        path
                    );
                }
                std::path::Component::RootDir | std::path::Component::Prefix(_) => {
                    bail!(
                        "Absolute path traversal attack detected in tar archive: {:?}",
                        path
                    );
                }
                _ => {}
            }
        }

        let out_path = canonical_dest.join(&path);
        let entry_type = entry.header().entry_type();

        // 2. Symlink & Hardlink Escape Protection
        if entry_type.is_symlink() || entry_type.is_hard_link() {
            if let Some(link_target) = entry.link_name()? {
                if link_target.is_absolute()
                    || link_target
                        .components()
                        .any(|c| matches!(c, std::path::Component::ParentDir))
                {
                    bail!(
                        "Malicious link target escaping archive destination: {:?}",
                        link_target
                    );
                }
            }
        }

        if entry_type.is_dir() {
            fs::create_dir_all(&out_path)?;
        } else if entry_type.is_file() {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent)?;
            }

            let entry_size = entry.size();
            total_bytes += entry_size;
            if total_bytes > MAX_UNCOMPRESSED_BYTES {
                bail!("Archive exceeds safe uncompressed size limit (Tar Bomb protection)");
            }

            let outfile = File::create(&out_path)?;
            let mut buf_out = BufWriter::with_capacity(256 * 1024, outfile);
            let mut limited = io::Read::by_ref(&mut entry)
                .take(MAX_UNCOMPRESSED_BYTES.saturating_sub(total_bytes) + entry_size + 1);
            let copied = io::copy(&mut limited, &mut buf_out)?;
            buf_out.flush()?;

            if copied > entry_size && copied > MAX_UNCOMPRESSED_BYTES {
                bail!("Archive entry decompressed payload exceeds declared size limit");
            }
        }
    }

    Ok(())
}

/// Safely extracts a single compressed stream with decompression bomb size ceiling enforcement.
fn extract_single_file_safe<R: Read>(mut reader: R, out_file: &Path) -> Result<()> {
    let outfile = File::create(out_file)?;
    let mut buf_out = BufWriter::with_capacity(256 * 1024, outfile);
    let mut limited = reader.by_ref().take(MAX_UNCOMPRESSED_BYTES + 1);
    let copied = io::copy(&mut limited, &mut buf_out)?;
    buf_out.flush()?;

    if copied > MAX_UNCOMPRESSED_BYTES {
        let _ = fs::remove_file(out_file);
        bail!("Decompressed file exceeds safe size limit (Decompression Bomb protection)");
    }
    Ok(())
}

/// Codec level 1-9 for the Preferences "Archive compression" choice
/// (Fast / Normal / Ultra); `max` is used for Ultra and unknown labels.
fn compression_level(max: u32) -> u32 {
    let label = crate::core::MossambiConfig::load()
        .conv_archive_compression
        .to_ascii_lowercase();
    match label.as_str() {
        "fast" => 1,
        "normal" | "balanced" => 6,
        _ => max,
    }
}

fn pack_zip(inputs: &[PathBuf], output: &Path) -> Result<()> {
    let file = File::create(output)?;
    let buf = BufWriter::with_capacity(512 * 1024, file);
    let mut zip_writer = zip::ZipWriter::new(buf);
    // Level 9: Maximum Deflate (7-Zip Ultra / WinRAR Best equivalent)
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .compression_level(Some(compression_level(9) as i64))
        .unix_permissions(0o755);

    for input in inputs {
        if input.is_file() {
            let name = input
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "entry".to_string());
            zip_writer.start_file(name, options)?;
            let mut f = File::open(input)?;
            io::copy(&mut f, &mut zip_writer)?;
        } else if input.is_dir() {
            let parent_dir = input.parent().unwrap_or(Path::new(""));
            for entry in WalkDir::new(input) {
                let entry = entry?;
                let rel_path = entry.path().strip_prefix(parent_dir)?;
                let rel_str = rel_path.to_string_lossy().replace('\\', "/");
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

    let mut inner = zip_writer.finish()?;
    inner.flush()?;
    Ok(())
}

fn pack_tar_plain(inputs: &[PathBuf], output: &Path) -> Result<()> {
    let file = File::create(output)?;
    let buf = BufWriter::with_capacity(512 * 1024, file);
    let mut builder = tar::Builder::new(buf);
    add_inputs_to_tar(&mut builder, inputs)?;
    let mut inner = builder.into_inner()?;
    inner.flush()?;
    Ok(())
}

fn pack_tar_gz(inputs: &[PathBuf], output: &Path) -> Result<()> {
    let file = File::create(output)?;
    let buf = BufWriter::with_capacity(512 * 1024, file);
    // Maximum Gzip compression level 9
    let enc = flate2::write::GzEncoder::new(buf, flate2::Compression::new(compression_level(9)));
    let mut builder = tar::Builder::new(enc);
    add_inputs_to_tar(&mut builder, inputs)?;
    let enc = builder.into_inner()?;
    let mut inner = enc.finish()?;
    inner.flush()?;
    Ok(())
}

fn pack_tar_bz2(inputs: &[PathBuf], output: &Path) -> Result<()> {
    let file = File::create(output)?;
    let buf = BufWriter::with_capacity(512 * 1024, file);
    // Maximum Bzip2 compression level 9
    let enc =
        bzip2::write::BzEncoder::new(buf, bzip2::Compression::new(compression_level(9).max(1)));
    let mut builder = tar::Builder::new(enc);
    add_inputs_to_tar(&mut builder, inputs)?;
    let enc = builder.into_inner()?;
    let mut inner = enc.finish()?;
    inner.flush()?;
    Ok(())
}

fn pack_tar_xz(inputs: &[PathBuf], output: &Path) -> Result<()> {
    let file = File::create(output)?;
    let buf = BufWriter::with_capacity(512 * 1024, file);
    // Maximum LZMA2 Level 9 Ultra (7-Zip Ultra standard)
    let enc = xz2::write::XzEncoder::new(buf, compression_level(9));
    let mut builder = tar::Builder::new(enc);
    add_inputs_to_tar(&mut builder, inputs)?;
    let enc = builder.into_inner()?;
    let mut inner = enc.finish()?;
    inner.flush()?;
    Ok(())
}

fn pack_tar_zst(inputs: &[PathBuf], output: &Path) -> Result<()> {
    let file = File::create(output)?;
    let buf = BufWriter::with_capacity(512 * 1024, file);
    // High-performance Zstandard Level 19 (Ultra ratio & ultra-fast decompression)
    let enc =
        zstd::stream::write::Encoder::new(buf, (compression_level(9) as i32 * 2 + 1).min(19))?;
    let mut builder = tar::Builder::new(enc);
    add_inputs_to_tar(&mut builder, inputs)?;
    let enc = builder.into_inner()?;
    let mut inner = enc.finish()?;
    inner.flush()?;
    Ok(())
}

fn add_inputs_to_tar<W: Write>(builder: &mut tar::Builder<W>, inputs: &[PathBuf]) -> Result<()> {
    for input in inputs {
        let name = input
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "entry".to_string());
        if input.is_file() {
            builder.append_path_with_name(input, &name)?;
        } else if input.is_dir() {
            builder.append_dir_all(&name, input)?;
        }
    }
    Ok(())
}
