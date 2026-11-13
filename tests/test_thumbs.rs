//! Grid-preview wave unit tests: the PreviewKind mapping (case
//! insensitive, complete for the listed sets) and the tile-to-decode
//! size bucket used for thumbnail cache keys.

use mossambi::gui::explorer_view::{preview_kind, thumb_max_px, PreviewKind};

#[test]
fn images_map_to_image() {
    for ext in [
        "JPG", "JPEG", "PNG", "WEBP", "GIF", "AVIF", "TIFF", "BMP", "ICO", "HEIC", "RAW",
    ] {
        assert_eq!(preview_kind(ext), PreviewKind::Image, "ext {ext}");
    }
}

#[test]
fn pdf_maps_to_pdf_any_case() {
    assert_eq!(preview_kind("PDF"), PreviewKind::Pdf);
    assert_eq!(preview_kind("pdf"), PreviewKind::Pdf);
    assert_eq!(preview_kind("Pdf"), PreviewKind::Pdf);
}

#[test]
fn video_set_maps_to_video_any_case() {
    for ext in [
        "MP4", "MKV", "AVI", "MOV", "WEBM", "M4V", "WMV", "FLV", "TS", "M2TS", "MTS", "3GP", "3G2",
        "OGV", "VOB", "MPG", "MPEG", "F4V",
    ] {
        assert_eq!(preview_kind(ext), PreviewKind::Video, "ext {ext}");
        let lower = ext.to_ascii_lowercase();
        assert_eq!(preview_kind(&lower), PreviewKind::Video, "ext {lower}");
    }
}

#[test]
fn non_previewable_map_to_none() {
    for ext in [
        "", "TXT", "MD", "MP3", "WAV", "FLAC", "ZIP", "7Z", "RAR", "DOCX", "XLSX", "RS", "EXE",
        "DLL",
    ] {
        assert_eq!(preview_kind(ext), PreviewKind::None, "ext {ext}");
    }
}

#[test]
fn thumb_bucket_clamps() {
    assert_eq!(thumb_max_px(0.0), 128);
    assert_eq!(thumb_max_px(63.9), 128);
    assert_eq!(thumb_max_px(64.0), 128);
    assert_eq!(thumb_max_px(96.0), 128);
    assert_eq!(thumb_max_px(128.0), 128);
    assert_eq!(thumb_max_px(128.4), 129);
    assert_eq!(thumb_max_px(300.0), 300);
    assert_eq!(thumb_max_px(512.0), 512);
    assert_eq!(thumb_max_px(600.0), 512);
}
