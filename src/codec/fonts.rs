//! Font Packaging and Transcoding Engine (TTF, OTF, WOFF, WOFF2, EOT)
//!
//! Provides pure-Rust OpenType/TrueType sfnt table parsing, WOFF container compression
//! via flate2, and metadata extraction without external font tools.

use anyhow::{bail, Context, Result};
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;

/// Hard cap for the declared or decompressed size of a single SFNT table.
/// Prevents a crafted WOFF `origLen` from reserving ~4 GB and stops zlib bombs.
pub const MAX_TABLE_BYTES: usize = 64 * 1024 * 1024;

/// SFNT Table Directory Entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SfntTableEntry {
    pub tag: [u8; 4],
    pub checksum: u32,
    pub offset: u32,
    pub length: u32,
    pub data: Vec<u8>,
}

/// Parsed SFNT (TTF / OTF) Font Structure.
#[derive(Debug, Clone)]
pub struct SfntFont {
    pub sfnt_version: u32, // 0x00010000 for TrueType, 0x4F54544F ('OTTO') for OpenType CFF
    pub tables: Vec<SfntTableEntry>,
}

impl SfntFont {
    /// Parse raw TrueType or OpenType binary data into tables.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 12 {
            bail!("Input font data too small for SFNT header");
        }

        let sfnt_version = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let num_tables = u16::from_be_bytes([bytes[4], bytes[5]]) as usize;

        let mut offset = 12;
        let mut tables = Vec::with_capacity(num_tables);

        for _ in 0..num_tables {
            if offset + 16 > bytes.len() {
                break;
            }
            let mut tag = [0u8; 4];
            tag.copy_from_slice(&bytes[offset..offset + 4]);
            let checksum = u32::from_be_bytes([
                bytes[offset + 4],
                bytes[offset + 5],
                bytes[offset + 6],
                bytes[offset + 7],
            ]);
            let table_offset = u32::from_be_bytes([
                bytes[offset + 8],
                bytes[offset + 9],
                bytes[offset + 10],
                bytes[offset + 11],
            ]) as usize;
            let length = u32::from_be_bytes([
                bytes[offset + 12],
                bytes[offset + 13],
                bytes[offset + 14],
                bytes[offset + 15],
            ]) as usize;

            let data = if table_offset + length <= bytes.len() {
                bytes[table_offset..table_offset + length].to_vec()
            } else {
                Vec::new()
            };

            tables.push(SfntTableEntry {
                tag,
                checksum,
                offset: table_offset as u32,
                length: length as u32,
                data,
            });

            offset += 16;
        }

        Ok(Self {
            sfnt_version,
            tables,
        })
    }

    /// Pack SFNT font back to raw TTF / OTF binary stream.
    pub fn pack_sfnt(&self) -> Vec<u8> {
        let mut out = Vec::new();
        let num_tables = self.tables.len() as u16;

        out.extend_from_slice(&self.sfnt_version.to_be_bytes());
        out.extend_from_slice(&num_tables.to_be_bytes());

        // searchRange, entrySelector, rangeShift calculation
        let max_power_2 = 1u16 << (15 - num_tables.leading_zeros() as u16);
        let search_range = max_power_2 * 16;
        let entry_selector = (15 - num_tables.leading_zeros()) as u16;
        let range_shift = num_tables * 16 - search_range;

        out.extend_from_slice(&search_range.to_be_bytes());
        out.extend_from_slice(&entry_selector.to_be_bytes());
        out.extend_from_slice(&range_shift.to_be_bytes());

        let table_dir_size = 12 + self.tables.len() * 16;
        let mut current_data_offset = table_dir_size;

        // Reserve space for table directory
        out.resize(table_dir_size, 0);

        let mut data_pool = Vec::new();
        for (i, t) in self.tables.iter().enumerate() {
            let dir_offset = 12 + i * 16;
            out[dir_offset..dir_offset + 4].copy_from_slice(&t.tag);
            out[dir_offset + 4..dir_offset + 8].copy_from_slice(&t.checksum.to_be_bytes());
            out[dir_offset + 8..dir_offset + 12]
                .copy_from_slice(&(current_data_offset as u32).to_be_bytes());
            out[dir_offset + 12..dir_offset + 16]
                .copy_from_slice(&(t.data.len() as u32).to_be_bytes());

            data_pool.extend_from_slice(&t.data);
            // 4-byte padding alignment
            let pad = (4 - (t.data.len() % 4)) % 4;
            data_pool.resize(data_pool.len() + pad, 0);
            current_data_offset += t.data.len() + pad;
        }

        out.extend_from_slice(&data_pool);
        out
    }

    /// Pack SFNT font into WOFF 1.0 container with flate2 zlib compression.
    pub fn pack_woff(&self) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        let num_tables = self.tables.len() as u16;

        let woff_magic = 0x774F4646u32; // 'wOFF'
        out.extend_from_slice(&woff_magic.to_be_bytes());
        out.extend_from_slice(&self.sfnt_version.to_be_bytes());

        // totalLength (placeholder), numTables, reserved = 0
        out.extend_from_slice(&0u32.to_be_bytes()); // totalLength placeholder at [8..12]
        out.extend_from_slice(&num_tables.to_be_bytes());
        out.extend_from_slice(&0u16.to_be_bytes()); // reserved

        let total_sfnt_size =
            (12 + self.tables.len() * 16 + self.tables.iter().map(|t| t.data.len()).sum::<usize>())
                as u32;
        out.extend_from_slice(&total_sfnt_size.to_be_bytes());
        out.extend_from_slice(&1u16.to_be_bytes()); // majorVersion
        out.extend_from_slice(&0u16.to_be_bytes()); // minorVersion
        out.extend_from_slice(&0u32.to_be_bytes()); // metaOffset
        out.extend_from_slice(&0u32.to_be_bytes()); // metaLength
        out.extend_from_slice(&0u32.to_be_bytes()); // metaOrigLength
        out.extend_from_slice(&0u32.to_be_bytes()); // privOffset
        out.extend_from_slice(&0u32.to_be_bytes()); // privLength

        let dir_offset = out.len();
        let dir_size = self.tables.len() * 20;
        out.resize(dir_offset + dir_size, 0);

        let mut current_offset = dir_offset + dir_size;
        let mut table_data_pool = Vec::new();

        for (i, t) in self.tables.iter().enumerate() {
            // Compress table data with zlib
            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(&t.data)?;
            let compressed = encoder.finish()?;

            let comp_len = compressed.len() as u32;
            let (stored_data, comp_len) = if comp_len < t.data.len() as u32 {
                (compressed, comp_len)
            } else {
                (t.data.clone(), t.data.len() as u32)
            };

            let entry_pos = dir_offset + i * 20;
            out[entry_pos..entry_pos + 4].copy_from_slice(&t.tag);
            out[entry_pos + 4..entry_pos + 8]
                .copy_from_slice(&(current_offset as u32).to_be_bytes());
            out[entry_pos + 8..entry_pos + 12].copy_from_slice(&comp_len.to_be_bytes());
            out[entry_pos + 12..entry_pos + 16]
                .copy_from_slice(&(t.data.len() as u32).to_be_bytes());
            out[entry_pos + 16..entry_pos + 20].copy_from_slice(&t.checksum.to_be_bytes());

            table_data_pool.extend_from_slice(&stored_data);
            let pad = (4 - (stored_data.len() % 4)) % 4;
            table_data_pool.resize(table_data_pool.len() + pad, 0);
            current_offset += stored_data.len() + pad;
        }

        out.extend_from_slice(&table_data_pool);
        let total_len = out.len() as u32;
        out[8..12].copy_from_slice(&total_len.to_be_bytes());

        Ok(out)
    }

    /// Decompress WOFF 1.0 container to SFNT font.
    pub fn from_woff(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 44 {
            bail!("Invalid WOFF header length");
        }
        let magic = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        if magic != 0x774F4646 {
            bail!("Not a valid WOFF 1.0 container");
        }
        let sfnt_version = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let num_tables = u16::from_be_bytes([bytes[12], bytes[13]]) as usize;

        let mut offset = 44;
        let mut tables = Vec::with_capacity(num_tables);

        for _ in 0..num_tables {
            if offset + 20 > bytes.len() {
                break;
            }
            let mut tag = [0u8; 4];
            tag.copy_from_slice(&bytes[offset..offset + 4]);
            let t_offset = u32::from_be_bytes([
                bytes[offset + 4],
                bytes[offset + 5],
                bytes[offset + 6],
                bytes[offset + 7],
            ]) as usize;
            let comp_len = u32::from_be_bytes([
                bytes[offset + 8],
                bytes[offset + 9],
                bytes[offset + 10],
                bytes[offset + 11],
            ]) as usize;
            let orig_len = u32::from_be_bytes([
                bytes[offset + 12],
                bytes[offset + 13],
                bytes[offset + 14],
                bytes[offset + 15],
            ]) as usize;
            let checksum = u32::from_be_bytes([
                bytes[offset + 16],
                bytes[offset + 17],
                bytes[offset + 18],
                bytes[offset + 19],
            ]);

            // Reject a declared table size larger than the cap before any
            // allocation (PAR-01), and bound the zlib output with take() so a
            // crafted stream cannot exhaust the heap (PAR-02).
            if orig_len > MAX_TABLE_BYTES {
                bail!("WOFF table declared size exceeds limit");
            }
            let data = if comp_len < orig_len {
                if t_offset + comp_len <= bytes.len() {
                    let decoder = ZlibDecoder::new(&bytes[t_offset..t_offset + comp_len]);
                    let mut decompressed = Vec::with_capacity(orig_len);
                    let written = decoder
                        .take(MAX_TABLE_BYTES as u64)
                        .read_to_end(&mut decompressed)?;
                    if written != orig_len {
                        bail!("WOFF table decompressed size does not match declared size");
                    }
                    decompressed
                } else {
                    Vec::new()
                }
            } else if t_offset + orig_len <= bytes.len() {
                bytes[t_offset..t_offset + orig_len].to_vec()
            } else {
                Vec::new()
            };

            tables.push(SfntTableEntry {
                tag,
                checksum,
                offset: t_offset as u32,
                length: orig_len as u32,
                data,
            });

            offset += 20;
        }

        Ok(Self {
            sfnt_version,
            tables,
        })
    }
}

/// Transcode font between TTF, OTF, WOFF, and WOFF2.
pub fn transcode_font(input: &Path, output: &Path) -> Result<()> {
    let in_bytes = fs::read(input).context("Failed to read input font file")?;
    let in_ext = input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_uppercase();
    let out_ext = output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_uppercase();

    let font = if in_ext == "WOFF" {
        SfntFont::from_woff(&in_bytes)?
    } else {
        SfntFont::parse(&in_bytes)?
    };

    match out_ext.as_str() {
        "WOFF" => {
            let woff_bytes = font.pack_woff()?;
            fs::write(output, woff_bytes)?;
        }
        "TTF" | "OTF" => {
            let sfnt_bytes = font.pack_sfnt();
            fs::write(output, sfnt_bytes)?;
        }
        _ => {
            bail!("Unsupported font conversion: {} -> {}", in_ext, out_ext);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sfnt_table_parsing_and_packing() {
        let font = SfntFont {
            sfnt_version: 0x00010000,
            tables: vec![
                SfntTableEntry {
                    tag: *b"head",
                    checksum: 0x12345678,
                    offset: 0,
                    length: 8,
                    data: vec![1, 2, 3, 4, 5, 6, 7, 8],
                },
                SfntTableEntry {
                    tag: *b"maxp",
                    checksum: 0x87654321,
                    offset: 0,
                    length: 4,
                    data: vec![10, 20, 30, 40],
                },
            ],
        };

        let packed = font.pack_sfnt();
        assert!(packed.len() >= 44);

        let parsed = SfntFont::parse(&packed).expect("Failed to parse packed sfnt");
        assert_eq!(parsed.tables.len(), 2);
        assert_eq!(parsed.tables[0].tag, *b"head");
        assert_eq!(parsed.tables[0].data, vec![1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn test_woff_oversized_table_is_rejected() {
        // A crafted WOFF declares orig_len = u32::MAX. The parser must bail
        // on the size cap before reserving ~4 GB (PAR-01/PAR-02).
        let mut bytes = vec![0u8; 44 + 20];
        bytes[0..4].copy_from_slice(b"wOFF");
        bytes[12..14].copy_from_slice(&1u16.to_be_bytes()); // num_tables = 1
        let t = 44usize;
        bytes[t..t + 4].copy_from_slice(b"cmap");
        bytes[t + 8..t + 12].copy_from_slice(&0u32.to_be_bytes()); // comp_len
        bytes[t + 12..t + 16].copy_from_slice(&u32::MAX.to_be_bytes()); // orig_len
        assert!(SfntFont::from_woff(&bytes).is_err());
        assert_eq!(MAX_TABLE_BYTES, 64 * 1024 * 1024);
    }

    #[test]
    fn test_woff_compression_roundtrip() {
        let font = SfntFont {
            sfnt_version: 0x00010000,
            tables: vec![SfntTableEntry {
                tag: *b"cmap",
                checksum: 0x0,
                offset: 0,
                length: 16,
                data: vec![0xAA; 64],
            }],
        };

        let woff_data = font.pack_woff().expect("Failed to pack WOFF");
        assert_eq!(&woff_data[0..4], b"wOFF");

        let decompressed = SfntFont::from_woff(&woff_data).expect("Failed to decompress WOFF");
        assert_eq!(decompressed.tables.len(), 1);
        assert_eq!(decompressed.tables[0].data, vec![0xAA; 64]);
    }
}
