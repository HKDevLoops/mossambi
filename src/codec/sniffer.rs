//! Binary Format Sniffer (Magic Byte Identification)
//!
//! Pure ASCII. Inspects file headers to identify true underlying container
//! and codec format without trusting untrusted file extensions.

use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SniffedFormat {
    // Images
    Png,
    Jpeg,
    WebP,
    Gif,
    Bmp,
    Tiff,
    Ico,
    Cur,
    Icns,
    Psd,
    Qoi,
    Dds,
    Xcf,
    Pdn,
    Vtf,
    Ani,
    Fits,
    Dicom,
    CameraRaw,
    Avif,
    Heic,
    Svg,

    // Documents
    Pdf,
    PostScript,
    Rtf,
    HwpOle,
    ZipPackage, // DOCX, XLSX, PPTX, EPUB, KRA, ODT, ODS, or ZIP

    // Media
    Mp4,
    Mkv,
    WebM,
    Avi,
    Flv,
    Mp3,
    Flac,
    Wav,
    Ogg,
    Aiff,
    Aac,

    // Archives
    SevenZip,
    Gzip,
    Bzip2,
    Xz,
    Zstd,
    Tar,
    Rar,
    Cab,

    // Fonts
    Ttf,
    Otf,
    Woff,
    Woff2,

    // CAD / 3D / Toolpaths
    Dxf,
    Dwg,
    Gcode,
    StlAscii,
    StlBinary,
    CanvasJson,

    // Data & Code
    Json,
    Xml,
    Sqlite,

    // Unknown or generic
    Unknown,
}

impl SniffedFormat {
    pub fn extension_hint(&self) -> Option<&'static str> {
        match self {
            Self::Png => Some("png"),
            Self::Jpeg => Some("jpg"),
            Self::WebP => Some("webp"),
            Self::Gif => Some("gif"),
            Self::Bmp => Some("bmp"),
            Self::Tiff => Some("tiff"),
            Self::Ico => Some("ico"),
            Self::Cur => Some("cur"),
            Self::Icns => Some("icns"),
            Self::Psd => Some("psd"),
            Self::Qoi => Some("qoi"),
            Self::Dds => Some("dds"),
            Self::Xcf => Some("xcf"),
            Self::Pdn => Some("pdn"),
            Self::Vtf => Some("vtf"),
            Self::Ani => Some("ani"),
            Self::Fits => Some("fits"),
            Self::Dicom => Some("dcm"),
            Self::CameraRaw => Some("raw"),
            Self::Avif => Some("avif"),
            Self::Heic => Some("heic"),
            Self::Svg => Some("svg"),
            Self::Pdf => Some("pdf"),
            Self::PostScript => Some("ps"),
            Self::Rtf => Some("rtf"),
            Self::HwpOle => Some("hwp"),
            Self::ZipPackage => Some("zip"),
            Self::Mp4 => Some("mp4"),
            Self::Mkv => Some("mkv"),
            Self::WebM => Some("webm"),
            Self::Avi => Some("avi"),
            Self::Flv => Some("flv"),
            Self::Mp3 => Some("mp3"),
            Self::Flac => Some("flac"),
            Self::Wav => Some("wav"),
            Self::Ogg => Some("ogg"),
            Self::Aiff => Some("aiff"),
            Self::Aac => Some("aac"),
            Self::SevenZip => Some("7z"),
            Self::Gzip => Some("gz"),
            Self::Bzip2 => Some("bz2"),
            Self::Xz => Some("xz"),
            Self::Zstd => Some("zst"),
            Self::Tar => Some("tar"),
            Self::Rar => Some("rar"),
            Self::Cab => Some("cab"),
            Self::Ttf => Some("ttf"),
            Self::Otf => Some("otf"),
            Self::Woff => Some("woff"),
            Self::Woff2 => Some("woff2"),
            Self::Dxf => Some("dxf"),
            Self::Dwg => Some("dwg"),
            Self::Gcode => Some("gcode"),
            Self::StlAscii | Self::StlBinary => Some("stl"),
            Self::CanvasJson => Some("canvas"),
            Self::Json => Some("json"),
            Self::Xml => Some("xml"),
            Self::Sqlite => Some("db"),
            Self::Unknown => None,
        }
    }
}

/// Sniffs the format from the first 512 bytes of a file.
pub fn sniff_file(path: &Path) -> SniffedFormat {
    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return SniffedFormat::Unknown,
    };

    let mut buf = [0u8; 512];
    let n = match file.read(&mut buf) {
        Ok(bytes) => bytes,
        Err(_) => return SniffedFormat::Unknown,
    };

    sniff_magic(&buf[..n])
}

/// Sniffs format from a slice of header bytes.
pub fn sniff_magic(bytes: &[u8]) -> SniffedFormat {
    if bytes.len() < 4 {
        return SniffedFormat::Unknown;
    }

    // PNG: 89 50 4E 47 0D 0A 1A 0A
    if bytes.len() >= 8 && bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return SniffedFormat::Png;
    }

    // JPEG: FF D8 FF
    if bytes.starts_with(b"\xFF\xD8\xFF") {
        return SniffedFormat::Jpeg;
    }

    // GIF: GIF87a or GIF89a
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return SniffedFormat::Gif;
    }

    // BMP: BM (42 4D)
    if bytes.starts_with(b"BM") {
        return SniffedFormat::Bmp;
    }

    // TIFF & Camera RAW
    if bytes.starts_with(b"II*\0") || bytes.starts_with(b"MM\0*") {
        // Canon CR2 header: II*\0 followed by offset to CR at offset 8
        if bytes.len() >= 10 && &bytes[8..10] == b"CR" {
            return SniffedFormat::CameraRaw;
        }
        return SniffedFormat::Tiff;
    }

    // PDF: %PDF-
    if bytes.starts_with(b"%PDF-") {
        return SniffedFormat::Pdf;
    }

    // PostScript: %!PS-
    if bytes.starts_with(b"%!PS") {
        return SniffedFormat::PostScript;
    }

    // RTF: {\rtf
    if bytes.starts_with(b"{\\rtf") {
        return SniffedFormat::Rtf;
    }

    // Photoshop PSD / PSB: 8BPS
    if bytes.starts_with(b"8BPS") {
        return SniffedFormat::Psd;
    }

    // QOI Image: qoif
    if bytes.starts_with(b"qoif") {
        return SniffedFormat::Qoi;
    }

    // DirectDraw Surface: DDS
    if bytes.starts_with(b"DDS ") {
        return SniffedFormat::Dds;
    }

    // GIMP XCF: gimp xcf
    if bytes.starts_with(b"gimp xcf") {
        return SniffedFormat::Xcf;
    }

    // Paint.NET: PDN3
    if bytes.starts_with(b"PDN3") {
        return SniffedFormat::Pdn;
    }

    // Valve Texture: VTF\0
    if bytes.starts_with(b"VTF\0") {
        return SniffedFormat::Vtf;
    }

    // Apple ICNS: icns
    if bytes.starts_with(b"icns") {
        return SniffedFormat::Icns;
    }

    // FITS Astronomy: SIMPLE  =
    if bytes.starts_with(b"SIMPLE  =") {
        return SniffedFormat::Fits;
    }

    // Hangul HWP OLE Compound Document: D0 CF 11 E0 A1 B1 1A E1
    if bytes.starts_with(b"\xD0\xCF\x11\xE0\xA1\xB1\x1A\xE1") {
        return SniffedFormat::HwpOle;
    }

    // 7-Zip: 37 7A BC AF 27 1C
    if bytes.len() >= 6 && bytes.starts_with(b"\x37\x7A\xBC\xAF\x27\x1C") {
        return SniffedFormat::SevenZip;
    }

    // GZIP: 1F 8B
    if bytes.starts_with(b"\x1F\x8B") {
        return SniffedFormat::Gzip;
    }

    // BZIP2: BZh
    if bytes.starts_with(b"BZh") {
        return SniffedFormat::Bzip2;
    }

    // XZ: FD 37 7A 58 5A 00
    if bytes.len() >= 6 && bytes.starts_with(b"\xFD\x37\x7A\x58\x5A\x00") {
        return SniffedFormat::Xz;
    }

    // Zstandard: 28 B5 2F FD
    if bytes.starts_with(b"\x28\xB5\x2F\xFD") {
        return SniffedFormat::Zstd;
    }

    // RAR: Rar!\x1A\x07
    if bytes.len() >= 7
        && (bytes.starts_with(b"Rar!\x1A\x07\x00") || bytes.starts_with(b"Rar!\x1A\x07\x01"))
    {
        return SniffedFormat::Rar;
    }

    // Microsoft CAB: MSCF
    if bytes.starts_with(b"MSCF") {
        return SniffedFormat::Cab;
    }

    // FLAC: fLaC
    if bytes.starts_with(b"fLaC") {
        return SniffedFormat::Flac;
    }

    // Ogg: OggS
    if bytes.starts_with(b"OggS") {
        return SniffedFormat::Ogg;
    }

    // FLV: FLV\x01
    if bytes.starts_with(b"FLV\x01") {
        return SniffedFormat::Flv;
    }

    // MP3 with ID3 tag: ID3 or sync frame
    if bytes.starts_with(b"ID3") || bytes.starts_with(b"\xFF\xFB") || bytes.starts_with(b"\xFF\xF3")
    {
        return SniffedFormat::Mp3;
    }

    // ADTS AAC: FF F1 or FF F9
    if bytes.starts_with(b"\xFF\xF1") || bytes.starts_with(b"\xFF\xF9") {
        return SniffedFormat::Aac;
    }

    // Windows Icons / Cursors
    if bytes.starts_with(b"\x00\x00\x01\x00") {
        return SniffedFormat::Ico;
    }
    if bytes.starts_with(b"\x00\x00\x02\x00") {
        return SniffedFormat::Cur;
    }

    // Fonts: TrueType / OpenType / WOFF / WOFF2
    if bytes.starts_with(b"\x00\x01\x00\x00") {
        return SniffedFormat::Ttf;
    }
    if bytes.starts_with(b"OTTO") {
        return SniffedFormat::Otf;
    }
    if bytes.starts_with(b"wOFF") {
        return SniffedFormat::Woff;
    }
    if bytes.starts_with(b"wOF2") {
        return SniffedFormat::Woff2;
    }

    // AutoCAD DWG: AC10
    if bytes.starts_with(b"AC10") {
        return SniffedFormat::Dwg;
    }

    // SQLite: SQLite format 3\0
    if bytes.len() >= 16 && bytes.starts_with(b"SQLite format 3\0") {
        return SniffedFormat::Sqlite;
    }

    // DICOM: 128 bytes preamble + DICM
    if bytes.len() >= 132 && &bytes[128..132] == b"DICM" {
        return SniffedFormat::Dicom;
    }

    // RIFF containers: WebP, WAV, AVI, or ANI
    if bytes.len() >= 12 && bytes.starts_with(b"RIFF") {
        let fourcc = &bytes[8..12];
        if fourcc == b"WEBP" {
            return SniffedFormat::WebP;
        }
        if fourcc == b"WAVE" {
            return SniffedFormat::Wav;
        }
        if fourcc == b"AVI " {
            return SniffedFormat::Avi;
        }
        if fourcc == b"ACON" {
            return SniffedFormat::Ani;
        }
    }

    // AIFF audio: FORM....AIFF
    if bytes.len() >= 12 && bytes.starts_with(b"FORM") {
        let fourcc = &bytes[8..12];
        if fourcc == b"AIFF" || fourcc == b"AIFC" {
            return SniffedFormat::Aiff;
        }
    }

    // Matroska / WebM: 1A 45 DF A3 (EBML Header)
    if bytes.starts_with(b"\x1A\x45\xDF\xA3") {
        if bytes.len() >= 32 {
            let sample = &bytes[..32.min(bytes.len())];
            if sample.windows(4).any(|w| w == b"webm") {
                return SniffedFormat::WebM;
            }
        }
        return SniffedFormat::Mkv;
    }

    // ISO Media / MP4 / AVIF / HEIC: offset 4 ftyp
    if bytes.len() >= 12 && &bytes[4..8] == b"ftyp" {
        let brand = &bytes[8..12];
        if brand == b"avif" || brand == b"avis" {
            return SniffedFormat::Avif;
        }
        if brand == b"heic" || brand == b"heix" || brand == b"mif1" || brand == b"msf1" {
            return SniffedFormat::Heic;
        }
        return SniffedFormat::Mp4;
    }

    // ZIP (and OpenXML documents): PK\x03\x04
    if bytes.starts_with(b"PK\x03\x04") {
        return SniffedFormat::ZipPackage;
    }

    // TAR: ustar magic at offset 257
    if bytes.len() >= 262 && &bytes[257..262] == b"ustar" {
        return SniffedFormat::Tar;
    }

    // Text-based format sniffing
    let text_sample = String::from_utf8_lossy(&bytes[..bytes.len().min(256)]);
    let trimmed = text_sample.trim_start();

    // XML / SVG detection
    if trimmed.starts_with("<?xml") || trimmed.starts_with("<svg") {
        if text_sample.contains("<svg") {
            return SniffedFormat::Svg;
        }
        return SniffedFormat::Xml;
    }

    // DXF ASCII detection
    if trimmed.starts_with("0\nSECTION") || trimmed.starts_with("0\r\nSECTION") {
        return SniffedFormat::Dxf;
    }

    // STL ASCII detection
    if trimmed.starts_with("solid ") {
        return SniffedFormat::StlAscii;
    }

    // G-code toolpath detection
    if trimmed.starts_with("G0 ")
        || trimmed.starts_with("G1 ")
        || trimmed.starts_with(";FLAVOR")
        || trimmed.starts_with(";Generated")
        || trimmed.starts_with("M104")
        || trimmed.starts_with("M109")
    {
        return SniffedFormat::Gcode;
    }

    // JSON / Canvas detection
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        if text_sample.contains("\"nodes\":") && text_sample.contains("\"edges\":") {
            return SniffedFormat::CanvasJson;
        }
        return SniffedFormat::Json;
    }

    SniffedFormat::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_png_magic() {
        let header = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR";
        assert_eq!(sniff_magic(header), SniffedFormat::Png);
    }

    #[test]
    fn test_jpeg_magic() {
        let header = b"\xFF\xD8\xFF\xE0\x00\x10JFIF";
        assert_eq!(sniff_magic(header), SniffedFormat::Jpeg);
    }

    #[test]
    fn test_pdf_magic() {
        let header = b"%PDF-1.7\r\n1 0 obj";
        assert_eq!(sniff_magic(header), SniffedFormat::Pdf);
    }

    #[test]
    fn test_webp_magic() {
        let header = b"RIFF\x20\x00\x00\x00WEBPVP8 ";
        assert_eq!(sniff_magic(header), SniffedFormat::WebP);
    }

    #[test]
    fn test_zip_magic() {
        let header = b"PK\x03\x04\x14\x00\x00\x00";
        assert_eq!(sniff_magic(header), SniffedFormat::ZipPackage);
    }

    #[test]
    fn test_flac_magic() {
        let header = b"fLaC\x00\x00\x00\x22";
        assert_eq!(sniff_magic(header), SniffedFormat::Flac);
    }

    #[test]
    fn test_psd_magic() {
        let header = b"8BPS\x00\x01\x00\x00";
        assert_eq!(sniff_magic(header), SniffedFormat::Psd);
    }

    #[test]
    fn test_qoi_magic() {
        let header = b"qoif\x00\x00\x01\x00";
        assert_eq!(sniff_magic(header), SniffedFormat::Qoi);
    }

    #[test]
    fn test_ttf_magic() {
        let header = b"\x00\x01\x00\x00\x00\x0c";
        assert_eq!(sniff_magic(header), SniffedFormat::Ttf);
    }

    #[test]
    fn test_gcode_magic() {
        let header = b";FLAVOR:Marlin\nG21\nG90\nM82\nM104 S200\n";
        assert_eq!(sniff_magic(header), SniffedFormat::Gcode);
    }
}
