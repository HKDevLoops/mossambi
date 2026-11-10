// tests/test_filee_engines.rs
//
// Integration test suite for the Filee pure-Rust single-binary conversion engines.
// Validates CAD, 3D, Geo, Fonts, Data Exchange, Email, Subtitles, and Multi-Hop Route Execution.
// Pure ASCII.

use mossambi::codec::{transcode_file, LosslessMode};
use mossambi::core::registry::{FileCategory, UniversalRegistry};
use std::fs;
use std::path::PathBuf;

fn scratch_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("mossambi_qa_filee_tests");
    let _ = fs::create_dir_all(&dir);
    dir
}

#[test]
fn test_cad_dxf_conversions_and_multi_hop() {
    let dir = scratch_dir().join("cad");
    let _ = fs::create_dir_all(&dir);

    // Minimal valid ASCII DXF with LINE and CIRCLE entities
    let dxf_content = "\
0\nSECTION\n2\nENTITIES\n\
0\nLINE\n8\n0\n10\n0.0\n20\n0.0\n11\n100.0\n21\n100.0\n\
0\nCIRCLE\n8\n0\n10\n50.0\n20\n50.0\n40\n25.0\n\
0\nENDSEC\n0\nEOF\n";

    let dxf_path = dir.join("drawing.dxf");
    fs::write(&dxf_path, dxf_content).expect("Failed to write test DXF");

    // 1. DXF -> SVG (Direct CAD vector transcode)
    let svg_out = transcode_file(&dxf_path, "SVG", LosslessMode::StrictBitExact, Some(&dir))
        .expect("DXF to SVG transcode failed");
    assert!(svg_out.exists());
    let svg_text = fs::read_to_string(&svg_out).expect("Failed to read SVG");
    assert!(svg_text.contains("<svg"));
    assert!(svg_text.contains("<line"));
    assert!(svg_text.contains("<circle"));

    // 2. DXF -> PDF (Direct CAD scaled vector plot)
    let pdf_out = transcode_file(&dxf_path, "PDF", LosslessMode::StrictBitExact, Some(&dir))
        .expect("DXF to PDF transcode failed");
    assert!(pdf_out.exists());
    let pdf_bytes = fs::read(&pdf_out).expect("Failed to read PDF");
    assert!(pdf_bytes.starts_with(b"%PDF-"));

    // 3. DXF -> PNG (Multi-hop CAD rendering: DXF -> SVG -> PNG via RoutePlanner)
    let png_out = transcode_file(&dxf_path, "PNG", LosslessMode::StrictBitExact, Some(&dir))
        .expect("DXF to PNG multi-hop transcode failed");
    assert!(png_out.exists());
    let png_bytes = fs::read(&png_out).expect("Failed to read PNG");
    assert!(png_bytes.starts_with(b"\x89PNG\r\n\x1a\n"));

    // Clean up
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_models_3d_stl_and_obj_conversions() {
    let dir = scratch_dir().join("models3d");
    let _ = fs::create_dir_all(&dir);

    // Minimal valid ASCII STL tetrahedron facet
    let stl_content = "\
solid test_cube\n\
  facet normal 0.0 0.0 1.0\n\
    outer loop\n\
      vertex 0.0 0.0 0.0\n\
      vertex 1.0 0.0 0.0\n\
      vertex 0.0 1.0 0.0\n\
    endloop\n\
  endfacet\n\
endsolid test_cube\n";

    let stl_path = dir.join("triangle.stl");
    fs::write(&stl_path, stl_content).expect("Failed to write test STL");

    // 1. STL -> OBJ (Wavefront 3D mesh)
    let obj_out = transcode_file(&stl_path, "OBJ", LosslessMode::StrictBitExact, Some(&dir))
        .expect("STL to OBJ transcode failed");
    assert!(obj_out.exists());
    let obj_text = fs::read_to_string(&obj_out).expect("Failed to read OBJ");
    assert!(obj_text.contains("v "));
    assert!(obj_text.contains("f "));

    // 2. STL -> PLY (Stanford polygon format)
    let ply_out = transcode_file(&stl_path, "PLY", LosslessMode::StrictBitExact, Some(&dir))
        .expect("STL to PLY transcode failed");
    assert!(ply_out.exists());
    let ply_text = fs::read_to_string(&ply_out).expect("Failed to read PLY");
    assert!(ply_text.contains("ply"));
    assert!(ply_text.contains("element vertex"));

    // Clean up
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_geo_vector_conversions() {
    let dir = scratch_dir().join("geo");
    let _ = fs::create_dir_all(&dir);

    // Standard GeoJSON FeatureCollection with landmark coordinates (New Delhi)
    let geojson_content = r#"{
  "type": "FeatureCollection",
  "features": [
    {
      "type": "Feature",
      "geometry": {
        "type": "Point",
        "coordinates": [77.2090, 28.6139]
      },
      "properties": {
        "name": "India Gate",
        "category": "Landmark"
      }
    }
  ]
}"#;

    let geo_path = dir.join("landmarks.geojson");
    fs::write(&geo_path, geojson_content).expect("Failed to write test GeoJSON");

    // 1. GeoJSON -> KML (Google Earth vector)
    let kml_out = transcode_file(&geo_path, "KML", LosslessMode::StrictBitExact, Some(&dir))
        .expect("GeoJSON to KML transcode failed");
    assert!(kml_out.exists());
    let kml_text = fs::read_to_string(&kml_out).expect("Failed to read KML");
    assert!(kml_text.contains("<kml"));
    assert!(kml_text.contains("<coordinates>77.209000,28.613900</coordinates>"));

    // 2. GeoJSON -> GPX (GPS exchange format)
    let gpx_out = transcode_file(&geo_path, "GPX", LosslessMode::StrictBitExact, Some(&dir))
        .expect("GeoJSON to GPX transcode failed");
    assert!(gpx_out.exists());
    let gpx_text = fs::read_to_string(&gpx_out).expect("Failed to read GPX");
    assert!(gpx_text.contains("<gpx"));
    assert!(gpx_text.contains("lat=\"28.613900\" lon=\"77.209000\""));

    // 3. GeoJSON -> CSV (Geospatial coordinate table)
    let csv_out = transcode_file(&geo_path, "CSV", LosslessMode::StrictBitExact, Some(&dir))
        .expect("GeoJSON to CSV transcode failed");
    assert!(csv_out.exists());
    let csv_text = fs::read_to_string(&csv_out).expect("Failed to read CSV");
    assert!(csv_text.to_lowercase().contains("latitude") || csv_text.contains("Latitude"));

    // Clean up
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_data_exchange_conversions() {
    let dir = scratch_dir().join("data");
    let _ = fs::create_dir_all(&dir);

    // Standard tabular JSON dataset
    let json_content = r#"[
  {"id": 1, "product": "Mossambi Studio", "price": 49},
  {"id": 2, "product": "Filee Engine", "price": 0}
]"#;

    let json_path = dir.join("inventory.json");
    fs::write(&json_path, json_content).expect("Failed to write test JSON");

    // 1. JSON -> CSV (Tabular record conversion)
    let csv_out = transcode_file(&json_path, "CSV", LosslessMode::StrictBitExact, Some(&dir))
        .expect("JSON to CSV transcode failed");
    assert!(csv_out.exists());
    let csv_text = fs::read_to_string(&csv_out).expect("Failed to read CSV");
    assert!(csv_text.contains("id,product,price") || csv_text.contains("product"));
    assert!(csv_text.contains("Mossambi Studio"));

    // 2. JSON -> JSONL (Newline-delimited stream)
    let jsonl_out = transcode_file(
        &json_path,
        "JSONL",
        LosslessMode::StrictBitExact,
        Some(&dir),
    )
    .expect("JSON to JSONL transcode failed");
    assert!(jsonl_out.exists());
    let jsonl_text = fs::read_to_string(&jsonl_out).expect("Failed to read JSONL");
    let lines: Vec<&str> = jsonl_text.lines().collect();
    assert_eq!(lines.len(), 2);

    // Clean up
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_email_conversions() {
    let dir = scratch_dir().join("email");
    let _ = fs::create_dir_all(&dir);

    // Standard RFC 822 MIME email file
    let eml_content = "\
From: sender@mossambi.org\r\n\
To: recipient@mossambi.org\r\n\
Subject: Filee Architecture Test\r\n\
Content-Type: text/plain; charset=utf-8\r\n\
\r\n\
Hello from the pure Rust conversion suite.\r\n";

    let eml_path = dir.join("message.eml");
    fs::write(&eml_path, eml_content).expect("Failed to write test EML");

    // 1. EML -> HTML (Formatted message rendering)
    let html_out = transcode_file(&eml_path, "HTML", LosslessMode::StrictBitExact, Some(&dir))
        .expect("EML to HTML transcode failed");
    assert!(html_out.exists());
    let html_text = fs::read_to_string(&html_out).expect("Failed to read HTML");
    assert!(html_text.contains("Filee Architecture Test"));
    assert!(html_text.contains("Hello from the pure Rust conversion suite"));

    // 2. EML -> TXT (Plain text extraction)
    let txt_out = transcode_file(&eml_path, "TXT", LosslessMode::StrictBitExact, Some(&dir))
        .expect("EML to TXT transcode failed");
    assert!(txt_out.exists());
    let txt_text = fs::read_to_string(&txt_out).expect("Failed to read TXT");
    assert!(txt_text.contains("Hello from the pure Rust conversion suite"));

    // Clean up
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_subtitles_conversions() {
    let dir = scratch_dir().join("subtitles");
    let _ = fs::create_dir_all(&dir);

    // Standard SubRip SRT file
    let srt_content = "\
1\n\
00:00:01,000 --> 00:00:04,000\n\
Pure Rust Single-Binary Converter\n\n\
2\n\
00:00:05,000 --> 00:00:08,000\n\
Apple HIG macOS Finder Theme\n";

    let srt_path = dir.join("captions.srt");
    fs::write(&srt_path, srt_content).expect("Failed to write test SRT");

    // 1. SRT -> VTT (WebVTT subtitle stream)
    let vtt_out = transcode_file(&srt_path, "VTT", LosslessMode::StrictBitExact, Some(&dir))
        .expect("SRT to VTT transcode failed");
    assert!(vtt_out.exists());
    let vtt_text = fs::read_to_string(&vtt_out).expect("Failed to read VTT");
    assert!(vtt_text.contains("WEBVTT"));
    assert!(vtt_text.contains("Pure Rust Single-Binary Converter"));

    // 2. SRT -> LRC (Synced lyrics format)
    let lrc_out = transcode_file(&srt_path, "LRC", LosslessMode::StrictBitExact, Some(&dir))
        .expect("SRT to LRC transcode failed");
    assert!(lrc_out.exists());
    let lrc_text = fs::read_to_string(&lrc_out).expect("Failed to read LRC");
    assert!(lrc_text.contains("[00:01.00]Pure Rust Single-Binary Converter"));

    // Clean up
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_registry_18_category_classification() {
    assert_eq!(
        UniversalRegistry::classify_extension("DXF"),
        FileCategory::Cad
    );
    assert_eq!(
        UniversalRegistry::classify_extension("DWG"),
        FileCategory::Cad
    );
    assert_eq!(
        UniversalRegistry::classify_extension("STL"),
        FileCategory::Models3D
    );
    assert_eq!(
        UniversalRegistry::classify_extension("OBJ"),
        FileCategory::Models3D
    );
    assert_eq!(
        UniversalRegistry::classify_extension("GEOJSON"),
        FileCategory::Geo
    );
    assert_eq!(
        UniversalRegistry::classify_extension("KML"),
        FileCategory::Geo
    );
    assert_eq!(
        UniversalRegistry::classify_extension("TTF"),
        FileCategory::Fonts
    );
    assert_eq!(
        UniversalRegistry::classify_extension("WOFF2"),
        FileCategory::Fonts
    );
    assert_eq!(
        UniversalRegistry::classify_extension("JSON"),
        FileCategory::Data
    );
    assert_eq!(
        UniversalRegistry::classify_extension("VCF"),
        FileCategory::Data
    );
    assert_eq!(
        UniversalRegistry::classify_extension("EML"),
        FileCategory::Email
    );
    assert_eq!(
        UniversalRegistry::classify_extension("SRT"),
        FileCategory::Subtitle
    );
}
