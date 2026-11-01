use anyhow::Result;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

fn main() -> Result<()> {
    let svg_path = Path::new("assets/icons/mossambi.svg");
    let svg_data = fs::read_to_string(svg_path)?;

    let opt = resvg::usvg::Options {
        resources_dir: None,
        ..Default::default()
    };

    let tree = resvg::usvg::Tree::from_str(&svg_data, &opt)?;

    let sizes: [u32; 8] = [16, 32, 48, 64, 128, 256, 512, 1024];
    let mut png_bytes_by_size = Vec::new();

    let out_dir = Path::new("assets/icons");
    fs::create_dir_all(out_dir)?;

    let orig_size = tree.size();
    let sx = orig_size.width();
    let sy = orig_size.height();

    for &size in &sizes {
        let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size)
            .ok_or_else(|| anyhow::anyhow!("Failed to create pixmap of size {size}"))?;

        let transform = resvg::tiny_skia::Transform::from_scale(
            size as f32 / sx,
            size as f32 / sy,
        );

        resvg::render(&tree, transform, &mut pixmap.as_mut());
        let png_bytes = pixmap.encode_png()?;

        let png_filename = format!("mossambi-{}x{}.png", size, size);
        let png_path = out_dir.join(&png_filename);
        fs::write(&png_path, &png_bytes)?;
        println!("Generated: {}", png_path.display());

        // For packaging assets, also save standard 512x512 as mossambi.png
        if size == 512 {
            fs::write("packaging/assets/mossambi.png", &png_bytes)?;
            fs::write("assets/icons/mossambi.png", &png_bytes)?;
        }

        if size <= 256 {
            png_bytes_by_size.push((size, png_bytes));
        }
    }

    // Generate Windows .ico with 16, 32, 48, 64, 128, 256 embedded PNGs
    let ico_path = out_dir.join("mossambi.ico");
    let mut ico_file = File::create(&ico_path)?;

    // ICONDIR header: 6 bytes
    let count = png_bytes_by_size.len() as u16;
    ico_file.write_all(&0u16.to_le_bytes())?; // Reserved
    ico_file.write_all(&1u16.to_le_bytes())?; // Type (1 = ICO)
    ico_file.write_all(&count.to_le_bytes())?; // Image count

    // Header size + Directory entries size
    let mut offset = 6 + (count as u32 * 16);

    for (size, data) in &png_bytes_by_size {
        let b_size = if *size >= 256 { 0u8 } else { *size as u8 };
        let entry = [
            b_size,                    // Width
            b_size,                    // Height
            0,                         // Color count
            0,                         // Reserved
            1, 0,                      // Color planes (1)
            32, 0,                     // Bits per pixel (32)
        ];
        ico_file.write_all(&entry)?;
        ico_file.write_all(&(data.len() as u32).to_le_bytes())?;
        ico_file.write_all(&offset.to_le_bytes())?;
        offset += data.len() as u32;
    }

    // Write PNG data blocks
    for (_, data) in &png_bytes_by_size {
        ico_file.write_all(data)?;
    }

    fs::copy(&ico_path, "packaging/assets/mossambi.ico")?;
    println!("Generated Windows Multi-Res ICO: {}", ico_path.display());

    Ok(())
}
