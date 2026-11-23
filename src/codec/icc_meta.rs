//! Industry-Grade ICC Color Profile & EXIF Metadata Preservation Engine
//!
//! Pure ASCII. Preserves wide-gamut ICC profiles (Display P3, Adobe RGB, ProPhoto)
//! and EXIF metadata across PNG, JPEG, and WebP containers without color clipping.

use crc32fast::Hasher as Crc32Hasher;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::io::{Read, Write};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EmbeddedMetadata {
    pub icc_profile: Option<Vec<u8>>,
    pub exif: Option<Vec<u8>>,
    pub dpi: Option<u32>,
}

// ============================================================================
// PNG ICC & EXIF Handlers
// ============================================================================

/// Extracts embedded ICC profile, EXIF, and DPI from a PNG byte stream.
pub fn extract_png_metadata(bytes: &[u8]) -> EmbeddedMetadata {
    let mut meta = EmbeddedMetadata::default();
    const PNG_MAGIC: &[u8] = b"\x89PNG\r\n\x1a\n";
    if bytes.len() < 8 || !bytes.starts_with(PNG_MAGIC) {
        return meta;
    }

    let mut offset = 8;
    while offset + 8 <= bytes.len() {
        let length = u32::from_be_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]) as usize;
        let chunk_type = &bytes[offset + 4..offset + 8];
        let data_start = offset + 8;
        let data_end = data_start + length;

        if data_end + 4 > bytes.len() {
            break;
        }

        if chunk_type == b"iCCP" && length > 2 {
            let chunk_data = &bytes[data_start..data_end];
            // Profile name is null-terminated (1..79 bytes)
            if let Some(null_pos) = chunk_data.iter().position(|&b| b == 0) {
                if null_pos + 2 < chunk_data.len() {
                    let compression_method = chunk_data[null_pos + 1];
                    let compressed_profile = &chunk_data[null_pos + 2..];
                    if compression_method == 0 {
                        let mut decoder = ZlibDecoder::new(compressed_profile);
                        let mut decompressed = Vec::new();
                        if decoder.read_to_end(&mut decompressed).is_ok()
                            && !decompressed.is_empty()
                        {
                            meta.icc_profile = Some(decompressed);
                        }
                    }
                }
            }
        } else if chunk_type == b"eXIf" && length > 0 {
            meta.exif = Some(bytes[data_start..data_end].to_vec());
        } else if chunk_type == b"pHYs" && length == 9 {
            let ppu_x = u32::from_be_bytes([
                bytes[data_start],
                bytes[data_start + 1],
                bytes[data_start + 2],
                bytes[data_start + 3],
            ]);
            let unit = bytes[data_start + 8];
            if unit == 1 && ppu_x > 0 {
                // Meter to inch conversion: 1 meter = 39.3701 inches
                let dpi = (ppu_x as f64 * 0.0254).round() as u32;
                meta.dpi = Some(dpi);
            }
        } else if chunk_type == b"IEND" {
            break;
        }

        offset = data_end + 4; // Skip data + 4-byte CRC
    }

    meta
}

/// Injects or replaces an embedded iCCP chunk in a PNG byte buffer.
pub fn inject_png_icc(bytes: &[u8], icc_profile: &[u8]) -> Vec<u8> {
    const PNG_MAGIC: &[u8] = b"\x89PNG\r\n\x1a\n";
    if bytes.len() < 33 || !bytes.starts_with(PNG_MAGIC) || icc_profile.is_empty() {
        return bytes.to_vec();
    }

    // Compress ICC profile data with zlib
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::best());
    if encoder.write_all(icc_profile).is_err() {
        return bytes.to_vec();
    }
    let compressed = match encoder.finish() {
        Ok(c) => c,
        Err(_) => return bytes.to_vec(),
    };

    // Construct iCCP chunk payload: "ICC Profile\0\0" + compressed
    let profile_name = b"ICC Profile\0\0";
    let mut chunk_data = Vec::with_capacity(profile_name.len() + compressed.len());
    chunk_data.extend_from_slice(profile_name);
    chunk_data.extend_from_slice(&compressed);

    let mut hasher = Crc32Hasher::new();
    hasher.update(b"iCCP");
    hasher.update(&chunk_data);
    let crc = hasher.finalize();

    let mut chunk = Vec::with_capacity(chunk_data.len() + 12);
    chunk.extend_from_slice(&(chunk_data.len() as u32).to_be_bytes());
    chunk.extend_from_slice(b"iCCP");
    chunk.extend_from_slice(&chunk_data);
    chunk.extend_from_slice(&crc.to_be_bytes());

    // Filter out any existing iCCP chunk from source
    let mut clean = Vec::with_capacity(bytes.len() + chunk.len());
    clean.extend_from_slice(PNG_MAGIC);

    let mut offset = 8;
    let mut inserted = false;

    while offset + 8 <= bytes.len() {
        let length = u32::from_be_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]) as usize;
        let chunk_type = &bytes[offset + 4..offset + 8];
        let total_chunk_len = length + 12;

        if offset + total_chunk_len > bytes.len() {
            clean.extend_from_slice(&bytes[offset..]);
            break;
        }

        if chunk_type == b"iCCP" {
            // Drop existing chunk
            offset += total_chunk_len;
            continue;
        }

        clean.extend_from_slice(&bytes[offset..offset + total_chunk_len]);

        if chunk_type == b"IHDR" && !inserted {
            // Insert immediately after IHDR
            clean.extend_from_slice(&chunk);
            inserted = true;
        }

        offset += total_chunk_len;
    }

    if !inserted {
        return bytes.to_vec();
    }

    clean
}

// ============================================================================
// JPEG ICC & EXIF Handlers
// ============================================================================

/// Extracts embedded ICC profile from a JPEG byte stream.
pub fn extract_jpeg_metadata(bytes: &[u8]) -> EmbeddedMetadata {
    let mut meta = EmbeddedMetadata::default();
    if bytes.len() < 4 || bytes[0] != 0xFF || bytes[1] != 0xD8 {
        return meta;
    }

    let mut offset = 2;
    let mut icc_parts: Vec<(u8, u8, Vec<u8>)> = Vec::new();

    while offset + 4 <= bytes.len() {
        if bytes[offset] != 0xFF {
            break;
        }
        let marker = bytes[offset + 1];
        if marker == 0xD9 || marker == 0xDA {
            // End of Image or Start of Scan
            break;
        }
        if marker == 0x00 || (0xD0..=0xD7).contains(&marker) {
            offset += 2;
            continue;
        }

        let length = u16::from_be_bytes([bytes[offset + 2], bytes[offset + 3]]) as usize;
        if length < 2 || offset + 2 + length > bytes.len() {
            break;
        }

        let segment_data = &bytes[offset + 4..offset + 2 + length];

        // APP2 with "ICC_PROFILE\0"
        if marker == 0xE2 && segment_data.len() > 14 && &segment_data[0..12] == b"ICC_PROFILE\0" {
            let seq_no = segment_data[12];
            let num_markers = segment_data[13];
            let part_data = segment_data[14..].to_vec();
            icc_parts.push((seq_no, num_markers, part_data));
        } else if marker == 0xE1 && segment_data.len() > 6 && &segment_data[0..6] == b"Exif\0\0" {
            meta.exif = Some(segment_data[6..].to_vec());
        }

        offset += 2 + length;
    }

    if !icc_parts.is_empty() {
        icc_parts.sort_by_key(|&(seq, _, _)| seq);
        let mut assembled = Vec::new();
        for (_, _, data) in icc_parts {
            assembled.extend_from_slice(&data);
        }
        if !assembled.is_empty() {
            meta.icc_profile = Some(assembled);
        }
    }

    meta
}

/// Injects an ICC profile into a JPEG byte stream using standard APP2 segments.
pub fn inject_jpeg_icc(bytes: &[u8], icc_profile: &[u8]) -> Vec<u8> {
    if bytes.len() < 4 || bytes[0] != 0xFF || bytes[1] != 0xD8 || icc_profile.is_empty() {
        return bytes.to_vec();
    }

    // Maximum payload per APP2 marker: 65535 - 2 (length) - 12 (ident) - 2 (seq) = 65519 bytes
    const MAX_CHUNK_SIZE: usize = 65000;
    let chunks: Vec<&[u8]> = icc_profile.chunks(MAX_CHUNK_SIZE).collect();
    let num_markers = chunks.len().min(255) as u8;

    let mut app2_segments = Vec::new();
    for (idx, chunk) in chunks.iter().enumerate().take(255) {
        let seq_no = (idx + 1) as u8;
        let segment_len = (2 + 12 + 2 + chunk.len()) as u16;

        app2_segments.push(0xFF);
        app2_segments.push(0xE2); // APP2
        app2_segments.extend_from_slice(&segment_len.to_be_bytes());
        app2_segments.extend_from_slice(b"ICC_PROFILE\0");
        app2_segments.push(seq_no);
        app2_segments.push(num_markers);
        app2_segments.extend_from_slice(chunk);
    }

    // Find insertion point (after SOI 0xFF 0xD8, and after APP0 JFIF if present)
    let mut insert_pos = 2;
    if bytes.len() >= 4 && bytes[2] == 0xFF && bytes[3] == 0xE0 {
        if let Some(app0_len) = bytes.get(4..6) {
            let l = u16::from_be_bytes([app0_len[0], app0_len[1]]) as usize;
            insert_pos = (4 + l).min(bytes.len());
        }
    }

    let mut out = Vec::with_capacity(bytes.len() + app2_segments.len());
    out.extend_from_slice(&bytes[..insert_pos]);
    out.extend_from_slice(&app2_segments);
    out.extend_from_slice(&bytes[insert_pos..]);
    out
}

// ============================================================================
// WebP ICC & EXIF Handlers
// ============================================================================

/// Extracts embedded ICC profile from a WebP byte stream.
pub fn extract_webp_metadata(bytes: &[u8]) -> EmbeddedMetadata {
    let mut meta = EmbeddedMetadata::default();
    if bytes.len() < 12 || !bytes.starts_with(b"RIFF") || &bytes[8..12] != b"WEBP" {
        return meta;
    }

    let mut offset = 12;
    while offset + 8 <= bytes.len() {
        let chunk_type = &bytes[offset..offset + 4];
        let chunk_size = u32::from_le_bytes([
            bytes[offset + 4],
            bytes[offset + 5],
            bytes[offset + 6],
            bytes[offset + 7],
        ]) as usize;

        let data_start = offset + 8;
        let data_end = data_start + chunk_size;
        if data_end > bytes.len() {
            break;
        }

        if chunk_type == b"ICCP" && chunk_size > 0 {
            meta.icc_profile = Some(bytes[data_start..data_end].to_vec());
        } else if chunk_type == b"EXIF" && chunk_size > 0 {
            meta.exif = Some(bytes[data_start..data_end].to_vec());
        }

        // WebP chunks are padded to even number of bytes
        let padded_size = (chunk_size + 1) & !1;
        offset = data_start + padded_size;
    }

    meta
}

/// Injects an ICC profile into a WebP byte stream.
pub fn inject_webp_icc(bytes: &[u8], icc_profile: &[u8]) -> Vec<u8> {
    if bytes.len() < 12
        || !bytes.starts_with(b"RIFF")
        || &bytes[8..12] != b"WEBP"
        || icc_profile.is_empty()
    {
        return bytes.to_vec();
    }

    // Build ICCP chunk
    let mut iccp_chunk = Vec::with_capacity(icc_profile.len() + 9);
    iccp_chunk.extend_from_slice(b"ICCP");
    iccp_chunk.extend_from_slice(&(icc_profile.len() as u32).to_le_bytes());
    iccp_chunk.extend_from_slice(icc_profile);
    if icc_profile.len() % 2 != 0 {
        iccp_chunk.push(0x00); // padding byte
    }

    let mut out = Vec::with_capacity(bytes.len() + iccp_chunk.len());
    out.extend_from_slice(b"RIFF");
    let total_file_size = (bytes.len() + iccp_chunk.len() - 8) as u32;
    out.extend_from_slice(&total_file_size.to_le_bytes());
    out.extend_from_slice(b"WEBP");

    // Insert ICCP chunk right after WEBP header
    out.extend_from_slice(&iccp_chunk);
    out.extend_from_slice(&bytes[12..]);
    out
}
