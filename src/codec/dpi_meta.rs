//! Industry-Grade DPI & Metadata Engine (Adobe Photoshop Standard)
//! Injects standard 300 DPI (or custom target DPI) into JFIF (JPEG) and pHYs (PNG) headers.
//! Guarantees that files opened in Photoshop, Illustrator, InDesign, Windows Explorer,
//! and macOS Preview immediately report publication-grade 300 DPI.

use crc32fast::Hasher as Crc32Hasher;

/// Injects standard 300 DPI JFIF APP0 header into raw JPEG byte buffer.
pub fn inject_jfif_dpi(bytes: &[u8], dpi: u32) -> Vec<u8> {
    let dpi = if dpi == 0 { 300 } else { dpi };
    let dpi_u16 = (dpi.min(65535)) as u16;

    if bytes.len() < 4 || bytes[0] != 0xFF || bytes[1] != 0xD8 {
        return bytes.to_vec();
    }

    // Check if an existing APP0 marker exists immediately after SOI
    if bytes.len() >= 20 && bytes[2] == 0xFF && bytes[3] == 0xE0 {
        // Verify identifier is "JFIF\0"
        if &bytes[6..11] == b"JFIF\0" {
            let mut modified = bytes.to_vec();
            // Offset 13 is units: 1 = dots/inch
            modified[13] = 0x01;
            // Offsets 14..16 are Xdensity (big-endian u16)
            modified[14] = (dpi_u16 >> 8) as u8;
            modified[15] = (dpi_u16 & 0xFF) as u8;
            // Offsets 16..18 are Ydensity (big-endian u16)
            modified[16] = (dpi_u16 >> 8) as u8;
            modified[17] = (dpi_u16 & 0xFF) as u8;
            return modified;
        }
    }

    // Otherwise, construct standard 18-byte JFIF APP0 segment and insert after SOI (bytes 0..2)
    let mut out = Vec::with_capacity(bytes.len() + 18);
    out.extend_from_slice(&bytes[0..2]); // FF D8 (SOI)

    // Standard JFIF APP0 Segment
    out.push(0xFF);
    out.push(0xE0); // APP0
    out.push(0x00);
    out.push(0x10); // Length = 16 bytes
    out.extend_from_slice(b"JFIF\0");
    out.push(0x01);
    out.push(0x02); // Version 1.02
    out.push(0x01); // Units: 1 = dots per inch (DPI)
    out.push((dpi_u16 >> 8) as u8);
    out.push((dpi_u16 & 0xFF) as u8); // Xdensity
    out.push((dpi_u16 >> 8) as u8);
    out.push((dpi_u16 & 0xFF) as u8); // Ydensity
    out.push(0x00); // Thumbnail X
    out.push(0x00); // Thumbnail Y

    // Append remainder of original JPEG stream
    out.extend_from_slice(&bytes[2..]);
    out
}

/// Injects standard 300 DPI pHYs chunk into raw PNG byte buffer.
pub fn inject_png_dpi(bytes: &[u8], dpi: u32) -> Vec<u8> {
    let dpi = if dpi == 0 { 300 } else { dpi };
    // Pixels per meter = DPI / 0.0254 (e.g. 300 DPI = 11811 pixels/meter)
    let ppm = (dpi as f64 / 0.0254).round() as u32;

    const PNG_MAGIC: &[u8] = b"\x89PNG\r\n\x1a\n";
    if bytes.len() < 33 || !bytes.starts_with(PNG_MAGIC) {
        return bytes.to_vec();
    }

    // In a valid PNG, IHDR is 13 bytes data + 4 bytes len + 4 bytes "IHDR" + 4 bytes CRC = 25 bytes.
    // Starting at offset 8, IHDR chunk ends at offset 8 + 25 = 33.
    let ihdr_end = 33;
    if bytes.len() < ihdr_end {
        return bytes.to_vec();
    }

    // Build pHYs chunk:
    // Length: 9 bytes
    // Type: "pHYs"
    // Data: [4 bytes X ppm][4 bytes Y ppm][1 byte unit (1 = meter)]
    // CRC: 4 bytes
    let mut phys_data = Vec::with_capacity(9);
    phys_data.extend_from_slice(&ppm.to_be_bytes());
    phys_data.extend_from_slice(&ppm.to_be_bytes());
    phys_data.push(0x01); // 1 = meter

    let mut hasher = Crc32Hasher::new();
    hasher.update(b"pHYs");
    hasher.update(&phys_data);
    let crc = hasher.finalize();

    let mut chunk = Vec::with_capacity(21);
    chunk.extend_from_slice(&9u32.to_be_bytes()); // Length
    chunk.extend_from_slice(b"pHYs"); // Type
    chunk.extend_from_slice(&phys_data); // Data
    chunk.extend_from_slice(&crc.to_be_bytes()); // CRC

    // Check if an existing pHYs chunk is already present
    if let Some(pos) = bytes.windows(4).position(|w| w == b"pHYs") {
        if pos >= 4 {
            let chunk_start = pos - 4;
            // Existing pHYs is 4 (len) + 4 (type) + 9 (data) + 4 (crc) = 21 bytes
            if chunk_start + 21 <= bytes.len() {
                let mut out = bytes.to_vec();
                out.splice(chunk_start..chunk_start + 21, chunk);
                return out;
            }
        }
    }

    // Insert immediately after IHDR chunk
    let mut out = Vec::with_capacity(bytes.len() + 21);
    out.extend_from_slice(&bytes[..ihdr_end]);
    out.extend_from_slice(&chunk);
    out.extend_from_slice(&bytes[ihdr_end..]);
    out
}
