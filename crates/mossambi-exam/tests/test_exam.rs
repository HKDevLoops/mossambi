use image::{ImageBuffer, Rgb};
use mossambi_exam::{optimize_exam_media, ExamConstraint};
use std::fs;

#[test]
fn test_exam_presets() {
    let upsc_p = ExamConstraint::preset_upsc_photo();
    assert_eq!(upsc_p.width_px, 350);
    assert_eq!(upsc_p.height_px, 450);
    assert_eq!(upsc_p.dpi, 300);
    assert_eq!(upsc_p.min_bytes, 20 * 1024);
    assert_eq!(upsc_p.max_bytes, 100 * 1024);

    let upsc_s = ExamConstraint::preset_upsc_signature();
    assert_eq!(upsc_s.width_px, 350);
    assert_eq!(upsc_s.height_px, 150);
    assert_eq!(upsc_s.min_bytes, 10 * 1024);
    assert_eq!(upsc_s.max_bytes, 40 * 1024);

    let ssc_p = ExamConstraint::preset_ssc_photo();
    assert_eq!(ssc_p.width_px, 140);
    assert_eq!(ssc_p.height_px, 190);

    let ibps_p = ExamConstraint::preset_ibps_photo();
    assert_eq!(ibps_p.width_px, 200);
    assert_eq!(ibps_p.height_px, 230);
}

#[test]
fn test_exam_optimizer_binary_search_convergence() {
    let test_dir = std::env::temp_dir().join("mossambi_test_exam");
    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(&test_dir).unwrap();

    // Create a 800x1000 synthetic RGB image
    let mut img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(800, 1000);
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        *pixel = Rgb([((x * 7) % 256) as u8, ((y * 11) % 256) as u8, 180]);
    }

    let input_path = test_dir.join("input_photo.png");
    img.save(&input_path).unwrap();

    let output_path = test_dir.join("upsc_ready.jpg");
    let constraint = ExamConstraint::preset_upsc_photo();

    let optimized = optimize_exam_media(&input_path, &output_path, &constraint).unwrap();
    assert!(optimized.exists());

    let final_bytes = fs::metadata(&optimized).unwrap().len();
    assert!(
        final_bytes <= constraint.max_bytes,
        "Final bytes ({}) must not exceed max boundary ({})",
        final_bytes,
        constraint.max_bytes
    );

    // Verify dimensions of result
    let res_img = image::open(&optimized).unwrap();
    assert_eq!(res_img.width(), constraint.width_px);
    assert_eq!(res_img.height(), constraint.height_px);

    let _ = fs::remove_dir_all(&test_dir);
}
