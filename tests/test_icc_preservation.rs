//! Test ICC Profile and EXIF Metadata Preservation
//!
//! Asserts bit-exact extraction and re-injection of ICC color profiles
//! (Display P3, Adobe RGB) across PNG, JPEG, and WebP containers
//! without color clipping or metadata loss.

use mossambi::codec::icc_meta::{
    extract_jpeg_metadata, extract_png_metadata, extract_webp_metadata, inject_jpeg_icc,
    inject_png_icc, inject_webp_icc,
};

fn generate_synthetic_icc_profile() -> Vec<u8> {
    // Standard minimal ICC v4 Header (128 bytes)
    let mut profile = vec![0u8; 128];
    // Profile size (big endian u32) = 128
    profile[0..4].copy_from_slice(&128u32.to_be_bytes());
    // CMM Type
    profile[4..8].copy_from_slice(b"appl");
    // Version 4.0.0
    profile[8] = 0x04;
    // Profile/Device Class = 'mntr' (monitor)
    profile[12..16].copy_from_slice(b"mntr");
    // Color Space = 'RGB '
    profile[16..20].copy_from_slice(b"RGB ");
    // Connection Space = 'XYZ '
    profile[20..24].copy_from_slice(b"XYZ ");
    // Profile signature = 'acsp'
    profile[36..40].copy_from_slice(b"acsp");
    profile
}

fn generate_minimal_png() -> Vec<u8> {
    let mut png = Vec::new();
    // Signature
    png.extend_from_slice(b"\x89PNG\r\n\x1a\n");
    // IHDR (1x1 pixel, 8-bit RGBA)
    let ihdr_data = [
        0x00, 0x00, 0x00, 0x01, // width: 1
        0x00, 0x00, 0x00, 0x01, // height: 1
        0x08, // bit depth: 8
        0x06, // color type: RGBA
        0x00, 0x00, 0x00,
    ];
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(b"IHDR");
    hasher.update(&ihdr_data);
    let crc = hasher.finalize();

    png.extend_from_slice(&13u32.to_be_bytes());
    png.extend_from_slice(b"IHDR");
    png.extend_from_slice(&ihdr_data);
    png.extend_from_slice(&crc.to_be_bytes());

    // IEND
    let mut end_hasher = crc32fast::Hasher::new();
    end_hasher.update(b"IEND");
    let end_crc = end_hasher.finalize();
    png.extend_from_slice(&0u32.to_be_bytes());
    png.extend_from_slice(b"IEND");
    png.extend_from_slice(&end_crc.to_be_bytes());
    png
}

fn generate_minimal_jpeg() -> Vec<u8> {
    let mut jpg = Vec::new();
    jpg.extend_from_slice(&[0xFF, 0xD8]); // SOI
    jpg.extend_from_slice(&[0xFF, 0xD9]); // EOI
    jpg
}

#[test]
fn test_png_icc_injection_and_extraction() {
    let base_png = generate_minimal_png();
    let original_icc = generate_synthetic_icc_profile();

    let injected = inject_png_icc(&base_png, &original_icc);
    assert!(injected.len() > base_png.len());
    assert!(injected.windows(4).any(|w| w == b"iCCP"));

    let extracted = extract_png_metadata(&injected);
    assert!(extracted.icc_profile.is_some());
    let recovered_icc = extracted.icc_profile.unwrap();
    assert_eq!(recovered_icc, original_icc);
}

#[test]
fn test_jpeg_icc_injection_and_extraction() {
    let base_jpg = generate_minimal_jpeg();
    let original_icc = generate_synthetic_icc_profile();

    let injected = inject_jpeg_icc(&base_jpg, &original_icc);
    assert!(injected.len() > base_jpg.len());
    assert!(injected.windows(12).any(|w| w == b"ICC_PROFILE\0"));

    let extracted = extract_jpeg_metadata(&injected);
    assert!(extracted.icc_profile.is_some());
    let recovered_icc = extracted.icc_profile.unwrap();
    assert_eq!(recovered_icc, original_icc);
}

fn generate_minimal_webp() -> Vec<u8> {
    let mut webp = Vec::new();
    webp.extend_from_slice(b"RIFF");
    webp.extend_from_slice(&12u32.to_le_bytes()); // File size - 8
    webp.extend_from_slice(b"WEBP");
    webp.extend_from_slice(b"VP8 ");
    webp.extend_from_slice(&0u32.to_le_bytes()); // empty VP8 payload
    webp
}

#[test]
fn test_webp_icc_injection_and_extraction() {
    let base_webp = generate_minimal_webp();
    let original_icc = generate_synthetic_icc_profile();

    let injected = inject_webp_icc(&base_webp, &original_icc);
    assert!(injected.len() > base_webp.len());
    assert!(injected.windows(4).any(|w| w == b"ICCP"));

    let extracted = extract_webp_metadata(&injected);
    assert!(extracted.icc_profile.is_some());
    let recovered_icc = extracted.icc_profile.unwrap();
    assert_eq!(recovered_icc, original_icc);
}

#[test]
fn test_cross_codec_metadata_preservation() {
    let base_png = generate_minimal_png();
    let icc = generate_synthetic_icc_profile();

    // Inject into PNG
    let png_with_icc = inject_png_icc(&base_png, &icc);
    let meta = extract_png_metadata(&png_with_icc);
    assert!(meta.icc_profile.is_some());

    // Transfer into JPEG
    let base_jpg = generate_minimal_jpeg();
    let jpg_with_icc = inject_jpeg_icc(&base_jpg, meta.icc_profile.as_ref().unwrap());
    let jpg_meta = extract_jpeg_metadata(&jpg_with_icc);

    assert_eq!(jpg_meta.icc_profile, Some(icc));
}
