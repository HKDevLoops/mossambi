use lopdf::Document;
use mossambi::codec::{
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

    for object in doc.objects.values() {
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
    assert!(
        !has_raster_image,
        "Text-to-PDF must be pure vector with ZERO rasterization"
    );

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

    let out_pdf = transcode_file(
        &txt_path,
        "PDF",
        LosslessMode::StrictBitExact,
        Some(&temp_dir),
    )
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
    for object in doc.objects.values() {
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

#[test]
fn test_pdf_first_page_thumbnail_render() {
    let temp_dir = std::env::temp_dir().join("mossambi_test_pdf_thumb");
    fs::create_dir_all(&temp_dir).unwrap();
    let txt_path = temp_dir.join("test_doc.txt");
    let pdf_path = temp_dir.join("test_doc.pdf");

    fs::write(
        &txt_path,
        "Hello Mossambi PDF Thumbnail Test\nPage 1 Content",
    )
    .unwrap();
    convert_text_to_pdf_vector(&txt_path, &pdf_path, None).unwrap();

    let rgba = mossambi::codec::render_pdf_first_page_thumbnail(&pdf_path, 320)
        .expect("PDF thumbnail failed");
    assert!(rgba.width() > 0);
    assert!(rgba.height() > 0);
    assert!(rgba.into_raw().len() > 100);

    let _ = fs::remove_file(txt_path);
    let _ = fs::remove_file(pdf_path);
}

#[test]
fn test_pdf_to_jpg_transcode() {
    let gate_pdf = std::path::Path::new(r"Z:\HK\Best GATE Free Resources  - ECE.pdf");
    if gate_pdf.exists() {
        let temp_dir = std::env::temp_dir().join("mossambi_test_pdf_jpg");
        let _ = fs::create_dir_all(&temp_dir);
        let out_jpg = temp_dir.join("test_gate.jpg");
        mossambi::codec::pdf_image::transcode_pdf_to_images(gate_pdf, &out_jpg, "JPG").unwrap();
        assert!(out_jpg.exists());
        let meta = fs::metadata(&out_jpg).unwrap();
        println!("out_jpg length: {}", meta.len());
        assert!(meta.len() > 1000);
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
