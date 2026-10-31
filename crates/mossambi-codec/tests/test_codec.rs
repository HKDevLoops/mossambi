use image::{ImageBuffer, Rgb};
use mossambi_codec::{transcode_file, transcode_to_all_formats, LosslessMode};
use std::fs;

#[test]
fn test_image_zero_loss_transcode() {
    let test_dir = std::env::temp_dir().join("mossambi_test_codec");
    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(&test_dir).unwrap();

    let mut img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(100, 100);
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        *pixel = Rgb([x as u8, y as u8, 128]);
    }
    let png_path = test_dir.join("source.png");
    img.save(&png_path).unwrap();

    // 1. Transcode PNG -> WEBP Lossless
    let webp_out = transcode_file(
        &png_path,
        "WEBP",
        LosslessMode::StrictBitExact,
        Some(&test_dir),
    )
    .unwrap();
    assert!(webp_out.exists());

    // 2. Transcode PNG -> BMP
    let bmp_out = transcode_file(
        &png_path,
        "BMP",
        LosslessMode::StrictBitExact,
        Some(&test_dir),
    )
    .unwrap();
    assert!(bmp_out.exists());

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_transcode_to_all_formats_batch() {
    let test_dir = std::env::temp_dir().join("mossambi_test_codec_all");
    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(&test_dir).unwrap();

    let mut img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(50, 50);
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        *pixel = Rgb([x as u8, y as u8, 200]);
    }
    let png_path = test_dir.join("mini.png");
    img.save(&png_path).unwrap();

    // Execute ALL sector
    let outputs = transcode_to_all_formats(
        &png_path,
        LosslessMode::StrictBitExact,
        Some(&test_dir),
    )
    .unwrap();

    assert!(!outputs.is_empty(), "ALL sector should generate outputs");
    for out in &outputs {
        assert!(out.exists(), "Output {} must exist", out.display());
    }

    let _ = fs::remove_dir_all(&test_dir);
}
