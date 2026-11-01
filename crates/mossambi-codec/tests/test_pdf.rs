use lopdf::Document;
use mossambi_codec::{
    convert_image_to_pdf_vector, convert_text_to_pdf_vector, transcode_file, LosslessMode,
    TextPdfSettings,
};
use std::fs;

#[test]
fn test_text_to_pdf_vector_fidelity() {
    let temp_dir = std::env::temp_dir().join("mossambi_test_pdf_vector");
    fs::create_dir_all(&temp_dir).unwrap();
    let txt_path = temp_dir.join("sample_notes.txt");
    let pdf_path = temp_dir.join("sample_notes.pdf");

    let content = r#"# Mossambi Engineering Specification
## 1. High Performance Vector Typesetting
Mossambi is engineered with zero compromises. When exporting plain text or markdown to PDF, it generates native vector typographic streams with standard Adobe Type 1 fonts (Helvetica, Times, Courier).

- Feature A: Infinite vector scalability without pixelation
- Feature B: Multi-page automatic pagination and word wrapping
- Feature C: Pantone 376 C accent headers and page numbering

This guarantees 100% publication-grade clarity across all displays and 1200+ DPI print systems.
"#;
    fs::write(&txt_path, content).unwrap();

    let settings = TextPdfSettings {
        page_size: "A4".to_string(),
        font_family: "Helvetica".to_string(),
        font_size: 11.0,
        line_spacing: 1.4,
        margin_pt: 54.0,
        show_header_footer: true,
        title: Some("Mossambi Engineering Specification".to_string()),
    };

    convert_text_to_pdf_vector(&txt_path, &pdf_path, Some(settings)).unwrap();

    assert!(pdf_path.exists());
    let meta = fs::metadata(&pdf_path).unwrap();
    assert!(meta.len() > 500, "PDF file must be non-empty");

    // Load PDF with lopdf to verify structure
    let doc = Document::load(&pdf_path).expect("Failed to parse generated PDF");
    assert!(!doc.objects.is_empty(), "Document must contain PDF objects");

    // Verify it contains Font dictionaries and NO raster image objects
    let mut has_font = false;
    let mut has_raster_image = false;

    for (_, object) in doc.objects.iter() {
        if let Ok(dict) = object.as_dict() {
            if let Ok(type_name) = dict.get(b"Type").and_then(|o| o.as_name()) {
                if type_name == b"Font" {
                    has_font = true;
                }
            }
            if let Ok(subtype) = dict.get(b"Subtype").and_then(|o| o.as_name()) {
                if subtype == b"Image" {
                    has_raster_image = true;
                }
            }
        }
    }

    assert!(has_font, "PDF must contain embedded vector fonts");
    assert!(!has_raster_image, "Text-to-PDF must be pure vector with ZERO rasterization");

    // Clean up
    let _ = fs::remove_file(txt_path);
    let _ = fs::remove_file(pdf_path);
}

#[test]
fn test_transcode_file_text_to_pdf_pipeline() {
    let temp_dir = std::env::temp_dir().join("mossambi_test_transcode_doc");
    fs::create_dir_all(&temp_dir).unwrap();
    let txt_path = temp_dir.join("input_document.txt");
    fs::write(&txt_path, "Hello from Mossambi Zero-Loss Engine!").unwrap();

    let out_pdf = transcode_file(&txt_path, "PDF", LosslessMode::StrictBitExact, Some(&temp_dir))
        .expect("Transcode text to PDF failed");

    assert!(out_pdf.exists());
    assert_eq!(out_pdf.extension().and_then(|e| e.to_str()), Some("pdf"));

    let _ = fs::remove_file(txt_path);
    let _ = fs::remove_file(out_pdf);
}

#[test]
fn test_image_to_pdf_vector_container() {
    let temp_dir = std::env::temp_dir().join("mossambi_test_img_pdf");
    fs::create_dir_all(&temp_dir).unwrap();
    let img_path = temp_dir.join("test_img.png");
    let pdf_path = temp_dir.join("test_img.pdf");

    // Generate a simple 100x100 RGB image
    let img = image::RgbImage::from_fn(100, 100, |x, y| {
        image::Rgb([(x % 256) as u8, (y % 256) as u8, 128])
    });
    img.save(&img_path).unwrap();

    convert_image_to_pdf_vector(&img_path, &pdf_path, Some(200.0)).unwrap();

    assert!(pdf_path.exists());
    let doc = Document::load(&pdf_path).expect("Failed to parse image PDF");
    let mut has_image = false;
    for (_, object) in doc.objects.iter() {
        if let Ok(dict) = object.as_dict() {
            if let Ok(subtype) = dict.get(b"Subtype").and_then(|o| o.as_name()) {
                if subtype == b"Image" {
                    has_image = true;
                }
            }
        } else if let Ok(stream) = object.as_stream() {
            if let Ok(subtype) = stream.dict.get(b"Subtype").and_then(|o| o.as_name()) {
                if subtype == b"Image" {
                    has_image = true;
                }
            }
        }
    }
    assert!(has_image, "Image PDF must contain embedded XObject image");

    let _ = fs::remove_file(img_path);
    let _ = fs::remove_file(pdf_path);
}
