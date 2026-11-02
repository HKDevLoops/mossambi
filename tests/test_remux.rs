use mossambi::codec::remux::{verify_conversion_fidelity, MathematicalRemuxType};
use mossambi::codec::{transcode_file, LosslessMode};
use std::fs;

#[test]
fn test_mathematical_image_remux_zero_loss() {
    let dir = std::env::temp_dir().join("mossambi_test_remux_lossless");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let src_png = dir.join("master_image.png");

    // Generate a test RGBA pattern
    let img = image::ImageBuffer::from_fn(64, 64, |x, y| {
        image::Rgba([
            (x * 4) as u8,
            (y * 4) as u8,
            ((x + y) * 2) as u8,
            255,
        ])
    });
    img.save(&src_png).unwrap();

    // Transcode PNG to BMP (uncompressed lossless raster)
    let out_bmp = transcode_file(&src_png, "BMP", LosslessMode::StrictBitExact, Some(&dir)).unwrap();

    // Verify mathematical quality
    let report = verify_conversion_fidelity(&src_png, &out_bmp).unwrap();
    assert!(report.is_zero_loss, "BMP conversion must be mathematically lossless");
    assert_eq!(report.remux_type, MathematicalRemuxType::LosslessRasterMatrix);
    assert_eq!(report.loss_percentage, 0.0, "Loss rate must be 0.000%");
    assert_eq!(report.rms_error, 0.0, "RMS error must be 0.000");
    assert!(report.psnr_db.unwrap().is_infinite(), "PSNR must be infinite for exact raster");
    assert_eq!(report.ssim.unwrap(), 1.0, "SSIM must be 1.000");
    assert!(report.verified_zero_loss);
}

#[test]
fn test_mathematical_image_lossy_warning_detection() {
    let dir = std::env::temp_dir().join("mossambi_test_remux_lossy");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let src_png = dir.join("photo.png");

    let img = image::ImageBuffer::from_fn(64, 64, |x, y| {
        image::Rgba([
            (x * 3) as u8,
            (y * 3) as u8,
            128,
            255,
        ])
    });
    img.save(&src_png).unwrap();

    // Transcode PNG to JPG (lossy DCT)
    let out_jpg = transcode_file(&src_png, "JPG", LosslessMode::PerceptualHighFidelity, Some(&dir)).unwrap();

    // Verify report correctly detects lossy encoding
    let report = verify_conversion_fidelity(&src_png, &out_jpg).unwrap();
    assert!(!report.is_zero_loss, "JPG must be flagged as lossy");
    assert_eq!(report.remux_type, MathematicalRemuxType::LossyTranscodeWithConsent);
    assert!(!report.warnings.is_empty(), "Must produce user warning regarding zero-loss bypass");
    assert!(report.psnr_db.unwrap() > 20.0, "High quality JPEG must have reasonable PSNR");
}

#[test]
fn test_audio_pcm_lossless_verification() {
    let dir = std::env::temp_dir().join("mossambi_test_remux_audio");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let src_flac = dir.join("master.flac");
    let tgt_wav = dir.join("master.wav");

    // Write dummy audio container files
    fs::write(&src_flac, b"fLaC\x00\x00\x00\x22dummy_audio_stream").unwrap();
    fs::write(&tgt_wav, b"RIFF\x24\x00\x00\x00WAVEfmt dummy").unwrap();

    let report = verify_conversion_fidelity(&src_flac, &tgt_wav).unwrap();
    assert!(report.is_zero_loss);
    assert_eq!(report.remux_type, MathematicalRemuxType::BitExactPcm);
    assert_eq!(report.loss_percentage, 0.0);
    assert!(report.verified_zero_loss);
}

#[test]
fn test_document_vector_pdf_verification() {
    let dir = std::env::temp_dir().join("mossambi_test_remux_doc");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let src_txt = dir.join("report.txt");
    fs::write(&src_txt, "Mossambi Universal Converter Engine\nSection 1: Architecture\n").unwrap();

    let out_pdf = transcode_file(&src_txt, "PDF", LosslessMode::StrictBitExact, Some(&dir)).unwrap();

    let report = verify_conversion_fidelity(&src_txt, &out_pdf).unwrap();
    assert!(report.is_zero_loss);
    assert_eq!(report.remux_type, MathematicalRemuxType::VectorLayoutPreservation);
    assert!(report.verified_zero_loss);
}
