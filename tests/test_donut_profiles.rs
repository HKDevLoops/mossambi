// tests/test_donut_profiles.rs
//
// Regression tests for the 20 Filee 2.5.0 domain profiles, Front/Back slice counts,
// and extension routing in Mossambi's DonutConfig.
//
// Pure 7-bit ASCII.

use mossambi::core::donut_config::{DonutConfig, DonutProfile};

#[test]
fn test_default_donut_profiles_count_is_twenty() {
    let cfg = DonutConfig::default();
    assert_eq!(
        cfg.profiles.len(),
        20,
        "Expected exactly 20 domain profiles matching Filee 2.5.0"
    );
}

#[test]
fn test_default_donut_profiles_ids_and_counts() {
    let cfg = DonutConfig::default();
    let expected: &[(&str, &str, usize, usize)] = &[
        ("images", "Images", 7, 8),
        ("pdf", "PDF", 8, 8),
        ("office", "Documents", 7, 4),
        ("spreadsheets", "Spreadsheets", 6, 5),
        ("presentations", "Presentations", 7, 4),
        ("hwp", "HWP / HWPX", 7, 4),
        ("text", "Text & Markdown", 6, 5),
        ("ebooks", "E-books", 6, 3),
        ("video", "Video", 6, 8),
        ("audio", "Audio", 5, 5),
        ("subtitles", "Subtitles", 2, 1),
        ("vector", "Vector graphics", 5, 5),
        ("archives", "Archives", 4, 0),
        ("folders", "Folders", 3, 0),
        ("cad", "CAD drawings", 5, 3),
        ("fonts", "Fonts", 4, 1),
        ("data", "Data", 5, 4),
        ("models", "3D models", 5, 1),
        ("geo", "Geospatial", 5, 1),
        ("mixed", "Mixed files", 5, 2),
    ];

    for (idx, &(exp_id, exp_name, exp_front, exp_back)) in expected.iter().enumerate() {
        let p = &cfg.profiles[idx];
        assert_eq!(p.id, exp_id, "Profile index {} ID mismatch", idx);
        assert_eq!(p.name, exp_name, "Profile index {} name mismatch", idx);
        assert_eq!(
            p.front_slices.len(),
            exp_front,
            "Profile '{}' front slice count mismatch",
            p.id
        );
        assert_eq!(
            p.back_slices.len(),
            exp_back,
            "Profile '{}' back slice count mismatch",
            p.id
        );
    }
}

#[test]
fn test_profile_count_badge_formatting() {
    let p_img = DonutProfile::new(
        "images",
        "Images",
        &[".png"],
        &[
            "to-png",
            "to-jpg",
            "to-webp",
            "to-gif",
            "to-pdf",
            "merge-pdf",
            "trace-svg",
        ],
        &[
            "tool-crop",
            "rotate",
            "remove-background",
            "merge-images",
            "compress-image",
            "half-size",
            "tool-image-edit",
            "zip-all",
        ],
    );
    assert_eq!(p_img.count_badge(), "7-8");

    let p_arc = DonutProfile::new(
        "archives",
        "Archives",
        &[".zip"],
        &["extract", "zip-all", "to-zip", "to-7z"],
        &[],
    );
    assert_eq!(p_arc.count_badge(), "4");
}

#[test]
fn test_extension_to_profile_routing() {
    let cfg = DonutConfig::default();

    let cases = &[
        ("photo.png", "images"),
        ("IMAGE.JPG", "images"),
        ("picture.webp", "images"),
        ("document.pdf", "pdf"),
        ("report.docx", "office"),
        ("notes.odt", "office"),
        ("budget.xlsx", "spreadsheets"),
        ("data.csv", "spreadsheets"),
        ("slides.pptx", "presentations"),
        ("hangul.hwp", "hwp"),
        ("hangul_x.hwpx", "hwp"),
        ("readme.md", "text"),
        ("script.py", "text"),
        ("book.epub", "ebooks"),
        ("movie.mp4", "video"),
        ("clip.mkv", "video"),
        ("song.mp3", "audio"),
        ("audio.flac", "audio"),
        ("movie.srt", "subtitles"),
        ("diagram.svg", "vector"),
        ("package.zip", "archives"),
        ("blueprint.dwg", "cad"),
        ("model.dxf", "cad"),
        ("font.woff2", "fonts"),
        ("payload.json", "data"),
        ("mesh.glb", "models"),
        ("mesh.stl", "models"),
        ("map.geojson", "geo"),
    ];

    for &(filename, expected_profile_id) in cases {
        let p = cfg.find_profile_for_filename(filename);
        assert!(
            p.is_some(),
            "Expected profile for '{}' but got None",
            filename
        );
        let found = p.unwrap();
        assert_eq!(
            found.id, expected_profile_id,
            "File '{}' routed to '{}', expected '{}'",
            filename, found.id, expected_profile_id
        );
    }
}
