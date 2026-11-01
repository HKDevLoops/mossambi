//! Turnkey Distribution Package Builder for Mossambi
//! Generates official binary distribution packages:
//! - target/dist/mossambi_2.1.0_amd64.deb (Standard Debian/Ubuntu binary package)
//! - target/dist/mossambi-v2.1.0-linux-x86_64.tar.gz (Universal Linux distribution archive)
//! - target/dist/mossambi-v2.1.0-windows-x64.zip (Universal Windows distribution bundle)

use anyhow::{Context, Result};
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

const VERSION: &str = "2.1.0";

fn main() -> Result<()> {
    println!("=======================================================");
    println!("  MOSSAMBI MULTI-PLATFORM DISTRIBUTION PACKAGE BUILDER ");
    println!("=======================================================\n");

    let dist_dir = Path::new("target/dist");
    fs::create_dir_all(dist_dir)?;

    let release_dir = Path::new("target/release");

    // 1. Gather binaries
    let bin_names = ["mossambi", "mossambi-gui", "mossambi-widget", "mossambi-tui"];
    let mut binary_data = Vec::new();

    for name in &bin_names {
        let exe_path = release_dir.join(format!("{}.exe", name));
        let unix_path = release_dir.join(name);

        let (src, is_exe) = if exe_path.exists() {
            (exe_path, true)
        } else if unix_path.exists() {
            (unix_path, false)
        } else {
            println!("Warning: binary '{}' not found in target/release, skipping.", name);
            continue;
        };

        let bytes = fs::read(&src)
            .with_context(|| format!("Failed to read binary: {}", src.display()))?;
        binary_data.push((*name, bytes, is_exe));
    }

    if binary_data.is_empty() {
        println!("No release binaries found. Please run 'cargo build --release' first.");
        return Ok(());
    }

    // 2. Build Debian / Ubuntu (.deb) package
    build_debian_package(dist_dir, &binary_data)?;

    // 3. Build Linux Tarball (.tar.gz)
    build_linux_tarball(dist_dir, &binary_data)?;

    // 4. Build Windows ZIP Bundle (.zip)
    build_windows_zip(dist_dir, &binary_data)?;

    println!("\n✓ All distribution packages successfully generated in target/dist/:\n");
    for entry in fs::read_dir(dist_dir)? {
        let entry = entry?;
        let meta = entry.metadata()?;
        let size_mb = meta.len() as f64 / (1024.0 * 1024.0);
        println!("  • {:<40} {:>7.2} MB", entry.file_name().to_string_lossy(), size_mb);
    }

    Ok(())
}

/// Builds an authentic, standard Debian (.deb) binary archive using pure Rust.
/// A .deb file is an 'ar' archive containing:
///  1. debian-binary: "2.0\n"
///  2. control.tar.gz: ./control metadata
///  3. data.tar.gz: filesystem tree (/usr/bin, /usr/share/applications, /usr/share/icons)
fn build_debian_package(
    dist_dir: &Path,
    binaries: &[(&str, Vec<u8>, bool)],
) -> Result<()> {
    let deb_path = dist_dir.join(format!("mossambi_{}_amd64.deb", VERSION));

    // A. Build control.tar.gz
    let control_content = format!(
        "Package: mossambi\n\
         Version: {}\n\
         Section: utils\n\
         Priority: optional\n\
         Architecture: amd64\n\
         Depends: ffmpeg\n\
         Maintainer: Mossambi Systems Team <maintainers@mossambi.org>\n\
         Homepage: https://github.com/mossambi/mossambi\n\
         Description: Universal lossless file converter and Indian exam tuner\n\
          Zero-loss native file conversion, archive suite, and form optimizer.\n",
        VERSION
    );

    let mut control_tar_gz = Vec::new();
    {
        let enc = GzEncoder::new(&mut control_tar_gz, Compression::best());
        let mut tar = tar::Builder::new(enc);

        let mut header = tar::Header::new_gnu();
        header.set_size(control_content.len() as u64);
        header.set_mode(0o644);
        header.set_mtime(1700000000);
        header.set_cksum();
        tar.append_data(&mut header, "./control", control_content.as_bytes())?;
        tar.into_inner()?.finish()?;
    }

    // B. Build data.tar.gz
    let mut data_tar_gz = Vec::new();
    {
        let enc = GzEncoder::new(&mut data_tar_gz, Compression::best());
        let mut tar = tar::Builder::new(enc);

        // Binaries in /usr/bin/
        for (name, data, _) in binaries {
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_mode(0o755); // Executable
            header.set_mtime(1700000000);
            header.set_cksum();
            tar.append_data(&mut header, format!("./usr/bin/{}", name), data.as_slice())?;
        }

        // Desktop Entry in /usr/share/applications/
        let desktop_file = Path::new("packaging/assets/mossambi.desktop");
        if desktop_file.exists() {
            let content = fs::read(desktop_file)?;
            let mut header = tar::Header::new_gnu();
            header.set_size(content.len() as u64);
            header.set_mode(0o644);
            header.set_mtime(1700000000);
            header.set_cksum();
            tar.append_data(&mut header, "./usr/share/applications/mossambi.desktop", content.as_slice())?;
        }

        // Icons in /usr/share/icons/hicolor/
        let icon_sizes = [16, 32, 48, 64, 128, 256, 512];
        for size in icon_sizes {
            let icon_path = PathBuf::from(format!("assets/icons/mossambi-{}x{}.png", size, size));
            if icon_path.exists() {
                let bytes = fs::read(&icon_path)?;
                let mut header = tar::Header::new_gnu();
                header.set_size(bytes.len() as u64);
                header.set_mode(0o644);
                header.set_mtime(1700000000);
                header.set_cksum();
                tar.append_data(
                    &mut header,
                    format!("./usr/share/icons/hicolor/{}x{}/apps/mossambi.png", size, size),
                    bytes.as_slice(),
                )?;
            }
        }

        // Scalable SVG icon
        let svg_path = Path::new("assets/icons/mossambi.svg");
        if svg_path.exists() {
            let bytes = fs::read(svg_path)?;
            let mut header = tar::Header::new_gnu();
            header.set_size(bytes.len() as u64);
            header.set_mode(0o644);
            header.set_mtime(1700000000);
            header.set_cksum();
            tar.append_data(&mut header, "./usr/share/icons/hicolor/scalable/apps/mossambi.svg", bytes.as_slice())?;
        }

        tar.into_inner()?.finish()?;
    }

    // C. Write 'ar' archive
    let mut deb_file = File::create(&deb_path)?;
    deb_file.write_all(b"!<arch>\n")?;

    write_ar_member(&mut deb_file, "debian-binary", b"2.0\n")?;
    write_ar_member(&mut deb_file, "control.tar.gz", &control_tar_gz)?;
    write_ar_member(&mut deb_file, "data.tar.gz", &data_tar_gz)?;

    println!("✓ Generated Debian (.deb) Package: {}", deb_path.display());
    Ok(())
}

fn write_ar_member(out: &mut File, name: &str, data: &[u8]) -> Result<()> {
    // 60-byte ar header format
    let header = format!(
        "{:<16}{:<12}{:<6}{:<6}{:<8}{:<10}`\n",
        name,
        1700000000, // Timestamp
        0,          // Owner ID
        0,          // Group ID
        "100644",   // File mode
        data.len(), // File size
    );
    out.write_all(header.as_bytes())?;
    out.write_all(data)?;
    if data.len() % 2 != 0 {
        out.write_all(b"\n")?; // Even padding
    }
    Ok(())
}

/// Builds a universal Linux distribution tarball (.tar.gz).
fn build_linux_tarball(
    dist_dir: &Path,
    binaries: &[(&str, Vec<u8>, bool)],
) -> Result<()> {
    let tar_path = dist_dir.join(format!("mossambi-v{}-linux-x86_64.tar.gz", VERSION));
    let file = File::create(&tar_path)?;
    let enc = GzEncoder::new(file, Compression::best());
    let mut tar = tar::Builder::new(enc);

    let prefix = format!("mossambi-v{}-linux-x86_64", VERSION);

    for (name, data, _) in binaries {
        let mut header = tar::Header::new_gnu();
        header.set_size(data.len() as u64);
        header.set_mode(0o755);
        header.set_mtime(1700000000);
        header.set_cksum();
        tar.append_data(&mut header, format!("{}/{}", prefix, name), data.as_slice())?;
    }

    let install_sh = Path::new("install.sh");
    if install_sh.exists() {
        let bytes = fs::read(install_sh)?;
        let mut header = tar::Header::new_gnu();
        header.set_size(bytes.len() as u64);
        header.set_mode(0o755);
        header.set_mtime(1700000000);
        header.set_cksum();
        tar.append_data(&mut header, format!("{}/install.sh", prefix), bytes.as_slice())?;
    }

    let readme = Path::new("README.md");
    if readme.exists() {
        let bytes = fs::read(readme)?;
        let mut header = tar::Header::new_gnu();
        header.set_size(bytes.len() as u64);
        header.set_mode(0o644);
        header.set_mtime(1700000000);
        header.set_cksum();
        tar.append_data(&mut header, format!("{}/README.md", prefix), bytes.as_slice())?;
    }

    let svg = Path::new("assets/icons/mossambi.svg");
    if svg.exists() {
        let bytes = fs::read(svg)?;
        let mut header = tar::Header::new_gnu();
        header.set_size(bytes.len() as u64);
        header.set_mode(0o644);
        header.set_mtime(1700000000);
        header.set_cksum();
        tar.append_data(&mut header, format!("{}/mossambi.svg", prefix), bytes.as_slice())?;
    }

    tar.into_inner()?.finish()?;
    println!("✓ Generated Linux Distribution Tarball: {}", tar_path.display());
    Ok(())
}

/// Builds a universal Windows portable ZIP distribution bundle.
fn build_windows_zip(
    dist_dir: &Path,
    binaries: &[(&str, Vec<u8>, bool)],
) -> Result<()> {
    let zip_path = dist_dir.join(format!("mossambi-v{}-windows-x64.zip", VERSION));
    let file = File::create(&zip_path)?;
    let mut zip = zip::ZipWriter::new(file);

    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .compression_level(Some(9));

    for (name, data, _) in binaries {
        let filename = format!("{}.exe", name);
        zip.start_file(filename, options)?;
        zip.write_all(data)?;
    }

    let setup_bat = "@echo off\r\necho =======================================================\r\necho   Installing Mossambi Universal Converter & Studio\r\necho =======================================================\r\npowershell -NoProfile -ExecutionPolicy Bypass -File .\\install.ps1\r\npause\r\n";
    zip.start_file("setup.bat", options)?;
    zip.write_all(setup_bat.as_bytes())?;

    let install_ps1 = Path::new("install.ps1");
    if install_ps1.exists() {
        let bytes = fs::read(install_ps1)?;
        zip.start_file("install.ps1", options)?;
        zip.write_all(&bytes)?;
    }

    let ico = Path::new("assets/icons/mossambi.ico");
    if ico.exists() {
        let bytes = fs::read(ico)?;
        zip.start_file("icons/mossambi.ico", options)?;
        zip.write_all(&bytes)?;
    }

    let png = Path::new("assets/icons/mossambi-512x512.png");
    if png.exists() {
        let bytes = fs::read(png)?;
        zip.start_file("icons/mossambi-512x512.png", options)?;
        zip.write_all(&bytes)?;
    }

    let readme = Path::new("README.md");
    if readme.exists() {
        let bytes = fs::read(readme)?;
        zip.start_file("README.md", options)?;
        zip.write_all(&bytes)?;
    }

    zip.finish()?;
    println!("✓ Generated Windows Portable Distribution Bundle: {}", zip_path.display());
    Ok(())
}
