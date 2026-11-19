//! Crop command handler: normalized image cropping.
//! Wraps gui::crop_canvas. Pure ASCII by contract.

use anyhow::{anyhow, bail, Result};
use serde_json::json;
use std::path::{Path, PathBuf};

use crate::gui::crop_canvas::{apply_crop_to_image, CropAspectRatio, CropCanvas, NormalizedCrop};

fn parse_aspect(s: &str) -> Result<CropAspectRatio> {
    match s.to_ascii_lowercase().replace(' ', "").as_str() {
        "free" | "none" => Ok(CropAspectRatio::Free),
        "1:1" | "square" => Ok(CropAspectRatio::Square),
        "4:3" => Ok(CropAspectRatio::FourThree),
        "16:9" => Ok(CropAspectRatio::SixteenNine),
        "3:2" => Ok(CropAspectRatio::ThreeTwo),
        "5:4" => Ok(CropAspectRatio::FiveFour),
        other => bail!(
            "unknown aspect ratio '{}' (use free, 1:1, 4:3, 16:9, 3:2, 5:4)",
            other
        ),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn handle_crop(
    file: &Path,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    aspect: Option<&str>,
    output: Option<&Path>,
    as_json: bool,
) -> Result<()> {
    if !file.exists() {
        bail!("Image file does not exist: {}", file.display());
    }
    let img = image::open(file).map_err(|e| anyhow!("failed to open image: {}", e))?;

    let mut crop = NormalizedCrop { x, y, w, h };
    if let Some(a) = aspect {
        let mut canvas = CropCanvas::new(img.width(), img.height());
        canvas.crop = crop;
        canvas.set_aspect_ratio(parse_aspect(a)?);
        crop = canvas.crop;
    } else {
        crop.clamp();
    }

    let cropped = apply_crop_to_image(&img, &crop);
    let out_path: PathBuf = match output {
        Some(p) => p.to_path_buf(),
        None => {
            let parent = file.parent().unwrap_or_else(|| Path::new("."));
            let stem = file.file_stem().and_then(|s| s.to_str()).unwrap_or("image");
            let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("png");
            parent.join(format!("{}_crop.{}", stem, ext))
        }
    };

    cropped
        .save(&out_path)
        .map_err(|e| anyhow!("failed to save cropped image: {}", e))?;

    let (px, py, pw, ph) = crop.to_pixel_rect(img.width(), img.height());
    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "status": "success",
                "source": file.display().to_string(),
                "output": out_path.display().to_string(),
                "rect": { "x": px, "y": py, "w": pw, "h": ph }
            }))?
        );
    } else {
        println!(
            "Cropped {} -> {} ({}x{} at {},{})",
            file.display(),
            out_path.display(),
            pw,
            ph,
            px,
            py
        );
    }
    Ok(())
}
