use image::{ImageBuffer, Rgb, Rgba};
use mossambi_codec::{
    extract_text_from_docx, extract_text_from_pdf, transcode_file, transcode_to_all_formats,
    LosslessMode,
};
use std::fs::{self, File};
use std::io::Write;

#[test]
fn test_image_zero_loss_transcode() {
    let test_dir = std::env::temp_dir().join("mossambi_test_codec_img");
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

    // 3. Transcode PNG -> TIFF
    let tiff_out = transcode_file(
        &png_path,
        "TIFF",
        LosslessMode::StrictBitExact,
        Some(&test_dir),
    )
    .unwrap();
    assert!(tiff_out.exists());

    // 4. Transcode PNG -> GIF
    let gif_out = transcode_file(
        &png_path,
        "GIF",
        LosslessMode::StrictBitExact,
        Some(&test_dir),
    )
    .unwrap();
    assert!(gif_out.exists());

    // 5. Transcode PNG -> PDF (Vector embedded container)
    let pdf_out = transcode_file(
        &png_path,
        "PDF",
        LosslessMode::StrictBitExact,
        Some(&test_dir),
    )
    .unwrap();
    assert!(pdf_out.exists());

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_transparent_png_to_jpeg_no_black_background() {
    let test_dir = std::env::temp_dir().join("mossambi_test_codec_transparency");
    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(&test_dir).unwrap();

    // Create an image with transparent border and red square in the center
    let mut rgba_img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(50, 50);
    for (x, y, pixel) in rgba_img.enumerate_pixels_mut() {
        if x > 15 && x < 35 && y > 15 && y < 35 {
            *pixel = Rgba([255, 0, 0, 255]); // Opaque Red
        } else {
            *pixel = Rgba([0, 0, 0, 0]); // Fully Transparent
        }
    }
    let png_path = test_dir.join("transparent_logo.png");
    rgba_img.save(&png_path).unwrap();

    let jpg_out = transcode_file(
        &png_path,
        "JPG",
        LosslessMode::StrictBitExact,
        Some(&test_dir),
    )
    .unwrap();

    assert!(jpg_out.exists());
    // Decode back and check corner pixel is clean white (255, 255, 255), NOT pitch black (0, 0, 0)
    let decoded = image::open(&jpg_out).unwrap().to_rgb8();
    let corner = decoded.get_pixel(0, 0);
    assert!(
        corner[0] >= 240 && corner[1] >= 240 && corner[2] >= 240,
        "Transparent pixel on JPEG conversion must blend onto white background, got {:?}",
        corner
    );

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

    assert!(outputs.len() >= 5, "ALL sector must generate multiple valid formats, got {}", outputs.len());
    for out in &outputs {
        assert!(out.exists(), "Output {} must exist", out.display());
        let meta = fs::metadata(out).unwrap();
        assert!(meta.len() > 0, "Output {} must not be 0 bytes", out.display());
    }

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_docx_extraction_and_conversion() {
    let test_dir = std::env::temp_dir().join("mossambi_test_docx");
    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(&test_dir).unwrap();

    // Create a mock DOCX file (ZIP archive containing word/document.xml)
    let docx_path = test_dir.join("mock_sample.docx");
    let file = File::create(&docx_path).unwrap();
    let mut zip = zip::ZipWriter::new(file);

    let options = zip::write::SimpleFileOptions::default();
    zip.start_file("word/document.xml", options).unwrap();
    let xml_content = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
    <w:body>
        <w:p><w:r><w:t>Mossambi Publication Standard</w:t></w:r></w:p>
        <w:p><w:r><w:t>Zero loss document conversion &amp; typesetting.</w:t></w:r></w:p>
    </w:body>
</w:document>"#;
    zip.write_all(xml_content.as_bytes()).unwrap();
    zip.finish().unwrap();

    // 1. Verify text extraction
    let text = extract_text_from_docx(&docx_path).unwrap();
    assert!(text.contains("Mossambi Publication Standard"));
    assert!(text.contains("Zero loss document conversion & typesetting."));

    // 2. Transcode DOCX -> TXT
    let txt_out = transcode_file(&docx_path, "TXT", LosslessMode::StrictBitExact, Some(&test_dir)).unwrap();
    assert!(txt_out.exists());
    let txt_content = fs::read_to_string(&txt_out).unwrap();
    assert!(txt_content.contains("Mossambi Publication Standard"));

    // 3. Transcode DOCX -> HTML
    let html_out = transcode_file(&docx_path, "HTML", LosslessMode::StrictBitExact, Some(&test_dir)).unwrap();
    assert!(html_out.exists());
    let html_content = fs::read_to_string(&html_out).unwrap();
    assert!(html_content.contains("<h1>mock_sample</h1>"));
    assert!(html_content.contains("<p>Mossambi Publication Standard</p>"));

    // 4. Transcode DOCX -> PDF (Vector)
    let pdf_out = transcode_file(&docx_path, "PDF", LosslessMode::StrictBitExact, Some(&test_dir)).unwrap();
    assert!(pdf_out.exists());
    let pdf_meta = fs::metadata(&pdf_out).unwrap();
    assert!(pdf_meta.len() > 100);

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_spreadsheet_conversions() {
    let test_dir = std::env::temp_dir().join("mossambi_test_spreadsheet");
    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(&test_dir).unwrap();

    // Create a CSV file
    let csv_path = test_dir.join("financial_data.csv");
    let csv_content = "Year,Revenue,Profit,Growth\n2024,1000000,250000,25%\n2025,1500000,450000,50%\n";
    fs::write(&csv_path, csv_content).unwrap();

    // 1. CSV -> TSV
    let tsv_out = transcode_file(&csv_path, "TSV", LosslessMode::StrictBitExact, Some(&test_dir)).unwrap();
    assert!(tsv_out.exists());
    let tsv_content = fs::read_to_string(&tsv_out).unwrap();
    assert!(tsv_content.contains("Year\tRevenue\tProfit\tGrowth"));

    // 2. CSV -> XLSX
    let xlsx_out = transcode_file(&csv_path, "XLSX", LosslessMode::StrictBitExact, Some(&test_dir)).unwrap();
    assert!(xlsx_out.exists());
    assert!(fs::metadata(&xlsx_out).unwrap().len() > 100);

    // 3. CSV -> HTML (Table)
    let html_out = transcode_file(&csv_path, "HTML", LosslessMode::StrictBitExact, Some(&test_dir)).unwrap();
    assert!(html_out.exists());
    let html_content = fs::read_to_string(&html_out).unwrap();
    assert!(html_content.contains("<table>"));
    assert!(html_content.contains("<th>Year</th>"));
    assert!(html_content.contains("<td>2024</td>"));

    // 4. CSV -> PDF (Vector Table)
    let pdf_out = transcode_file(&csv_path, "PDF", LosslessMode::StrictBitExact, Some(&test_dir)).unwrap();
    assert!(pdf_out.exists());
    assert!(fs::metadata(&pdf_out).unwrap().len() > 100);

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_archive_transcode_via_codec() {
    let test_dir = std::env::temp_dir().join("mossambi_test_codec_archive");
    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(&test_dir).unwrap();

    // Create files in a directory
    let data_dir = test_dir.join("bundle_data");
    fs::create_dir_all(&data_dir).unwrap();
    fs::write(data_dir.join("report.txt"), b"Quarterly Audit Data").unwrap();

    // Pack directory to ZIP via transcode_file
    let zip_out = transcode_file(&data_dir, "ZIP", LosslessMode::StrictBitExact, Some(&test_dir)).unwrap();
    assert!(zip_out.exists());

    // Extract ZIP via transcode_file (target "EXTRACT")
    let extract_out = transcode_file(&zip_out, "EXTRACT", LosslessMode::StrictBitExact, None).unwrap();
    assert!(extract_out.exists());
    assert!(extract_out.join("bundle_data").join("report.txt").exists());

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_pdf_text_extraction() {
    let test_dir = std::env::temp_dir().join("mossambi_test_pdf_extract");
    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(&test_dir).unwrap();

    let txt_path = test_dir.join("sample.txt");
    fs::write(&txt_path, b"Hello PDF Vector Text\nSecond Line of Content\n").unwrap();

    let pdf_out = transcode_file(&txt_path, "PDF", LosslessMode::StrictBitExact, Some(&test_dir)).unwrap();
    assert!(pdf_out.exists());

    let extracted = extract_text_from_pdf(&pdf_out).unwrap();
    assert!(extracted.contains("Hello") || extracted.contains("PDF"));

    let _ = fs::remove_dir_all(&test_dir);
}
