//! Multi-Hop Format Conversion Route Planner
//! Implements Dijkstra's shortest path graph search algorithm (up to 3 hops)
//! for multi-stage conversion pipelines (e.g., HWP -> HWPX -> PDF -> PNG).
//!
//! 100% pure ASCII implementation.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteStep {
    pub from: String,
    pub to: String,
    pub cost: u32,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversionRoute {
    pub steps: Vec<RouteStep>,
    pub total_cost: u32,
}

impl ConversionRoute {
    pub fn hop_count(&self) -> usize {
        self.steps.len()
    }

    pub fn display_path(&self) -> String {
        if self.steps.is_empty() {
            return "Direct (No conversion)".to_string();
        }
        let mut path = self.steps[0].from.clone();
        for step in &self.steps {
            path.push_str(" -> ");
            path.push_str(&step.to);
        }
        path
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
struct State {
    cost: u32,
    node_idx: usize,
}

impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse for min-heap
        other.cost.cmp(&self.cost)
    }
}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone)]
struct Edge {
    target_idx: usize,
    target_name: String,
    cost: u32,
    description: String,
}

pub struct RoutePlanner {
    node_names: Vec<String>,
    name_to_idx: HashMap<String, usize>,
    adjacency: Vec<Vec<Edge>>,
}

impl Default for RoutePlanner {
    fn default() -> Self {
        Self::new_with_standard_matrix()
    }
}

impl RoutePlanner {
    pub fn new() -> Self {
        Self {
            node_names: Vec::new(),
            name_to_idx: HashMap::new(),
            adjacency: Vec::new(),
        }
    }

    pub fn get_or_add_node(&mut self, name: &str) -> usize {
        let u = name.to_uppercase();
        if let Some(&idx) = self.name_to_idx.get(&u) {
            idx
        } else {
            let idx = self.node_names.len();
            self.node_names.push(u.clone());
            self.name_to_idx.insert(u, idx);
            self.adjacency.push(Vec::new());
            idx
        }
    }

    pub fn add_edge(&mut self, from: &str, to: &str, cost: u32, description: &str) {
        let from_idx = self.get_or_add_node(from);
        let to_idx = self.get_or_add_node(to);
        self.adjacency[from_idx].push(Edge {
            target_idx: to_idx,
            target_name: to.to_uppercase(),
            cost,
            description: description.to_string(),
        });
    }

    /// Finds optimal conversion route using Dijkstra's shortest path algorithm.
    /// Supports up to `max_hops` intermediate bridge steps.
    pub fn find_route(&self, src: &str, dst: &str, max_hops: usize) -> Option<ConversionRoute> {
        let src_u = src.to_uppercase();
        let dst_u = dst.to_uppercase();

        if src_u == dst_u {
            return Some(ConversionRoute {
                steps: Vec::new(),
                total_cost: 0,
            });
        }

        let start_idx = *self.name_to_idx.get(&src_u)?;
        let end_idx = *self.name_to_idx.get(&dst_u)?;

        let num_nodes = self.node_names.len();
        let mut dist: Vec<u32> = vec![u32::MAX; num_nodes];
        let mut hops: Vec<usize> = vec![0; num_nodes];
        let mut prev: Vec<Option<(usize, RouteStep)>> = vec![None; num_nodes];

        let mut heap = BinaryHeap::new();
        dist[start_idx] = 0;
        heap.push(State {
            cost: 0,
            node_idx: start_idx,
        });

        while let Some(State { cost, node_idx }) = heap.pop() {
            if node_idx == end_idx {
                break;
            }

            if cost > dist[node_idx] {
                continue;
            }

            if hops[node_idx] >= max_hops {
                continue;
            }

            for edge in &self.adjacency[node_idx] {
                let next_idx = edge.target_idx;
                let next_cost = cost + edge.cost;

                if next_cost < dist[next_idx] {
                    dist[next_idx] = next_cost;
                    hops[next_idx] = hops[node_idx] + 1;
                    prev[next_idx] = Some((
                        node_idx,
                        RouteStep {
                            from: self.node_names[node_idx].clone(),
                            to: edge.target_name.clone(),
                            cost: edge.cost,
                            description: edge.description.clone(),
                        },
                    ));
                    heap.push(State {
                        cost: next_cost,
                        node_idx: next_idx,
                    });
                }
            }
        }

        if dist[end_idx] == u32::MAX {
            return None;
        }

        // Reconstruct path
        let mut steps = Vec::new();
        let mut curr = end_idx;
        while let Some((p_idx, step)) = &prev[curr] {
            steps.push(step.clone());
            curr = *p_idx;
        }
        steps.reverse();

        Some(ConversionRoute {
            total_cost: dist[end_idx],
            steps,
        })
    }

    /// Constructs the standard Mossambi format transformation graph.
    pub fn new_with_standard_matrix() -> Self {
        let mut p = Self::new();

        // 1. Direct Image Transcodes (Cost: 5 for lossless, 8 for lossy)
        let raster_imgs = [
            "PNG", "WEBP", "JPG", "JPEG", "BMP", "TIFF", "AVIF", "ICO", "QOI", "TGA", "PPM",
            "ICNS", "CUR", "PSD", "PSB", "DDS", "PCX",
        ];
        for &a in &raster_imgs {
            for &b in &raster_imgs {
                if a != b {
                    p.add_edge(a, b, 5, "Direct pure-Rust SIMD image transcode");
                }
            }
        }

        // Specialized raster decoders to standard image representations
        p.add_edge("XCF", "PNG", 5, "GIMP multi-layer composite flatten");
        p.add_edge("PDN", "PNG", 5, "Paint.NET layer composite flatten");
        p.add_edge("VTF", "PNG", 5, "Valve Source texture mipmap decode");
        p.add_edge("ANI", "PNG", 4, "Animated cursor first frame extraction");
        p.add_edge("ANI", "GIF", 6, "Animated cursor multi-frame sequence");
        p.add_edge("FITS", "PNG", 5, "FITS astronomy linear stretch to PNG");
        p.add_edge("FITS", "TIFF", 4, "16-bit scientific FITS preserve");
        p.add_edge("DCM", "PNG", 5, "DICOM medical grayscale slice render");
        p.add_edge("DCM", "TIFF", 4, "16-bit medical DICOM preserve");
        p.add_edge("JXR", "PNG", 5, "Windows HDR JPEG XR transcode");
        p.add_edge("KRA", "PNG", 4, "Krita merged image archive extraction");

        // Vector Graphics to Raster & PDF (Cost: 4)
        p.add_edge("SVG", "PNG", 4, "Pure-Rust resvg rasterization");
        p.add_edge("SVG", "PDF", 4, "Vector geometry preservation via lopdf");
        p.add_edge("SVG", "DXF", 5, "Vector path to CAD polyline translation");
        p.add_edge("SVG", "EPS", 5, "PostScript vector path encapsulation");
        p.add_edge("EPS", "SVG", 5, "Encapsulated PostScript vector parse");
        p.add_edge("EPS", "PDF", 4, "PostScript level 3 vector transcode");
        p.add_edge("PS", "PDF", 4, "PostScript interpreter to PDF");

        // Camera RAW to DNG / TIFF / PNG (Cost: 6)
        let raws = ["CR2", "CR3", "NEF", "ARW", "ORF", "RW2", "RAF", "DNG"];
        for &r in &raws {
            p.add_edge(r, "DNG", 3, "Lossless Camera RAW container encapsulation");
            p.add_edge(r, "TIFF", 5, "16-bit linear demosaic extraction");
            p.add_edge(r, "PNG", 6, "16-bit RGB rasterization");
        }

        // 2. Documents & Publishing
        p.add_edge("DOCX", "PDF", 8, "Native OOXML layout engine to PDF");
        p.add_edge("DOCX", "TXT", 2, "XML paragraph extraction");
        p.add_edge("DOCX", "HTML", 5, "OOXML to clean Semantic HTML5");
        p.add_edge("DOCX", "MD", 4, "OOXML to CommonMark Markdown");

        p.add_edge("ODT", "PDF", 8, "OpenDocument XML layout to PDF");
        p.add_edge("ODT", "TXT", 2, "OpenDocument text body extraction");
        p.add_edge("RTF", "PDF", 7, "Rich Text Format layout to PDF");
        p.add_edge("RTF", "TXT", 2, "Rich Text control-word strip");

        p.add_edge("OFD", "PDF", 6, "Chinese OFD standard fixed layout to PDF");
        p.add_edge("P7M", "PDF", 3, "PKCS#7 signed envelope extraction");
        p.add_edge("P7M", "XML", 3, "PKCS#7 e-invoice payload unwrapping");

        p.add_edge("MD", "HTML", 2, "Markdown parser to Semantic HTML5");
        p.add_edge("MD", "PDF", 6, "Markdown to Styled Vector PDF");
        p.add_edge("HTML", "PDF", 7, "HTML5 CSS paged media layout");

        // E-Books
        p.add_edge("EPUB", "PDF", 7, "EPUB paged layout render to PDF");
        p.add_edge("EPUB", "TXT", 3, "EPUB chapter text extraction");
        p.add_edge("EPUB", "HTML", 3, "EPUB spine package to single HTML");
        p.add_edge("MOBI", "EPUB", 5, "Mobipocket palm database to EPUB");
        p.add_edge("AZW3", "EPUB", 5, "Kindle format 8 to EPUB");
        p.add_edge("FB2", "EPUB", 4, "FictionBook XML to EPUB");
        p.add_edge("CBZ", "PDF", 6, "Comic book ZIP archive to paged PDF");

        // Korean Hangul HWP / HWPX
        p.add_edge(
            "HWP",
            "HWPX",
            3,
            "Decompress Compound Document OLE to OOXML HWPX",
        );
        p.add_edge("HWPX", "PDF", 7, "HWPX layout engine to PDF");
        p.add_edge("HWPX", "TXT", 2, "HWPX XML section text extraction");

        // CAD (DXF, DWG, PLT, DST, GERBER, CANVAS)
        p.add_edge("DWG", "DXF", 5, "Autodesk drawing entity translation");
        p.add_edge("DXF", "SVG", 4, "CAD 2D vector entity export");
        p.add_edge("DXF", "PDF", 5, "CAD scaled technical vector plot");
        p.add_edge("PLT", "SVG", 4, "HP-GL plotter instructions to SVG vector");
        p.add_edge("DST", "SVG", 4, "Tajima embroidery stitch paths to SVG");
        p.add_edge(
            "GERBER",
            "SVG",
            4,
            "RS-274X PCB copper traces to SVG vector",
        );
        p.add_edge("CANVAS", "SVG", 3, "Obsidian visual canvas graph to SVG");
        p.add_edge("CANVAS", "MD", 2, "Obsidian card text nodes to Markdown");

        // G-Code CNC / 3D Printer Toolpaths
        p.add_edge("GCODE", "SVG", 4, "G-code extrusion toolpath plot to SVG");
        p.add_edge("GCODE", "PNG", 5, "G-code layer preview rasterization");

        // Spreadsheets
        let sheets = ["XLSX", "XLS", "ODS", "CSV", "TSV"];
        for &s in &sheets {
            p.add_edge(s, "CSV", 2, "Calamine table extraction");
            p.add_edge(s, "XLSX", 3, "Workbook generation via rust_xlsxwriter");
            p.add_edge(s, "PDF", 6, "Spreadsheet grid layout rendering");
            p.add_edge(s, "HTML", 3, "HTML table markup conversion");
        }

        // 3D Models
        p.add_edge("STL", "OBJ", 3, "Facet mesh re-indexing to OBJ");
        p.add_edge("OBJ", "STL", 3, "Facet mesh re-indexing to STL");
        p.add_edge("STL", "PLY", 3, "Stanford polygon mesh conversion");
        p.add_edge("OBJ", "PLY", 3, "Stanford polygon mesh conversion");
        p.add_edge("PLY", "STL", 3, "Polygon mesh to STL triangles");
        p.add_edge("PLY", "OBJ", 3, "Polygon mesh to Wavefront OBJ");
        p.add_edge("OBJ", "GLTF", 4, "glTF 2.0 PBR asset conversion");
        p.add_edge("GLTF", "GLB", 2, "Binary glTF container pack");
        p.add_edge("GLB", "GLTF", 2, "JSON glTF unpack with binary buffer");
        p.add_edge(
            "3MF",
            "STL",
            3,
            "Additive manufacturing triangular mesh export",
        );
        p.add_edge("PNG", "STL", 8, "Luminance heightmap 3D relief generation");
        p.add_edge("JPG", "STL", 8, "Luminance heightmap 3D relief generation");

        // Geospatial Vector Data
        let geo = ["GEOJSON", "KML", "KMZ", "GPX", "CSV"];
        for &g1 in &geo {
            for &g2 in &geo {
                if g1 != g2 {
                    p.add_edge(g1, g2, 3, "Geospatial vector coordinate translation");
                }
            }
        }

        // Structured Data Exchange
        p.add_edge(
            "JSON",
            "CSV",
            2,
            "Structured JSON object array to tabular CSV",
        );
        p.add_edge("CSV", "JSON", 2, "Tabular CSV records to JSON array");
        p.add_edge(
            "JSON",
            "JSONL",
            2,
            "Pretty JSON to newline-delimited stream",
        );
        p.add_edge("JSONL", "JSON", 2, "Newline-delimited stream to JSON array");
        p.add_edge("YAML", "JSON", 2, "YAML parser to structured JSON tree");
        p.add_edge("JSON", "YAML", 2, "JSON tree to formatted YAML document");
        p.add_edge("TOML", "JSON", 2, "TOML parser to structured JSON tree");
        p.add_edge("JSON", "TOML", 2, "JSON tree to valid TOML configuration");
        p.add_edge("PLIST", "JSON", 2, "Apple property list to JSON tree");
        p.add_edge("HAR", "JSON", 2, "HTTP Archive network log unpack");
        p.add_edge("HAR", "CSV", 3, "HTTP Archive network requests table");
        p.add_edge("XML", "JSON", 3, "Hierarchical XML tree to structured JSON");
        p.add_edge("JSON", "XML", 3, "Structured JSON to hierarchical XML tree");
        p.add_edge("VCF", "JSON", 3, "vCard contact parsing to JSON schema");
        p.add_edge("ICS", "JSON", 3, "iCalendar events to JSON schema");
        p.add_edge("VCF", "CSV", 4, "vCard contact book to CSV spreadsheet");
        p.add_edge("ICS", "CSV", 4, "iCalendar events to CSV agenda");

        // Fonts
        let fonts = ["TTF", "OTF", "WOFF", "WOFF2"];
        for &f1 in &fonts {
            for &f2 in &fonts {
                if f1 != f2 {
                    p.add_edge(f1, f2, 4, "SFNT font table transform and compression");
                }
            }
        }
        p.add_edge("TTF", "EOT", 5, "Embedded OpenType compression");
        p.add_edge("OTF", "EOT", 5, "Embedded OpenType compression");

        // Email & MIME Archives
        p.add_edge("EML", "HTML", 3, "MIME multipart email body extraction");
        p.add_edge("EML", "TXT", 2, "MIME plain-text email body extraction");
        p.add_edge("EML", "ZIP", 3, "Email attachments bundle packaging");
        p.add_edge("MSG", "HTML", 3, "Outlook message body extraction");
        p.add_edge("MSG", "TXT", 2, "Outlook message text extraction");
        p.add_edge("MSG", "ZIP", 3, "Outlook attachments package");
        p.add_edge("MHTML", "HTML", 2, "MIME HTML multipart unpack");
        p.add_edge("MHTML", "TXT", 2, "MIME HTML text strip");
        p.add_edge("MHTML", "ZIP", 3, "MHTML resources bundle");

        // PDF Vector / Text / Image Rasterization
        p.add_edge("PDF", "PNG", 8, "High-DPI rasterization (150-300 DPI)");
        p.add_edge("PDF", "JPG", 8, "High-DPI rasterization (150-300 DPI)");
        p.add_edge("PDF", "WEBP", 8, "High-DPI rasterization (150-300 DPI)");
        p.add_edge("PDF", "TXT", 2, "lopdf content stream character extraction");

        // Audio Transcode Matrix (Cost: 2 for stream copy, 5 for lossless, 8 for lossy)
        let audio_lossless = ["FLAC", "WAV", "ALAC", "AIFF", "WV", "TAK", "APE"];
        let audio_lossy = [
            "MP3", "AAC", "OGG", "OPUS", "M4A", "WMA", "M4R", "WEBA", "MP2",
        ];

        for &a in &audio_lossless {
            for &b in &audio_lossless {
                if a != b {
                    p.add_edge(a, b, 3, "Bit-exact lossless audio transcode");
                }
            }
            for &l in &audio_lossy {
                p.add_edge(a, l, 6, "High-fidelity perceptual audio encode");
            }
        }
        for &l in &audio_lossy {
            for &b in &audio_lossless {
                p.add_edge(l, b, 5, "Lossy-to-lossless audio decode");
            }
            for &l2 in &audio_lossy {
                if l != l2 {
                    p.add_edge(l, l2, 6, "Perceptual transcode");
                }
            }
        }

        // Video Transcodes & Audio Extraction
        let video_fmts = [
            "MP4", "MKV", "MOV", "WEBM", "AVI", "FLV", "TS", "M2TS", "M4V",
        ];
        for &v1 in &video_fmts {
            for &v2 in &video_fmts {
                if v1 != v2 {
                    p.add_edge(v1, v2, 6, "Hardware accelerated remux/transcode");
                }
            }
            p.add_edge(v1, "MP3", 4, "Direct audio stream demux");
            p.add_edge(v1, "WAV", 3, "Uncompressed audio demux");
            p.add_edge(v1, "GIF", 9, "Palette-optimized video clip conversion");
        }
        p.add_edge("H264", "MP4", 3, "Containerize raw H.264 stream to MP4");
        p.add_edge("H265", "MP4", 3, "Containerize raw HEVC stream to MP4");

        // Subtitles
        let subs = [
            "SRT", "VTT", "ASS", "SSA", "SUB", "SMI", "LRC", "TTML", "SCC",
        ];
        for &s1 in &subs {
            for &s2 in &subs {
                if s1 != s2 {
                    p.add_edge(s1, s2, 2, "Subtitle timestamp translation");
                }
            }
        }

        // ArtCraft Pro High-Fidelity Ecosystem Routes
        p.add_edge(
            "AFDESIGN",
            "SVG",
            3,
            "VectorCraft pure-Rust Affinity vector export",
        );
        p.add_edge(
            "AFDESIGN",
            "PDF",
            4,
            "VectorCraft Affinity Designer to Vector PDF",
        );
        p.add_edge(
            "CDR",
            "SVG",
            4,
            "VectorCraft CorelDraw precision vector export",
        );
        p.add_edge(
            "AI",
            "SVG",
            3,
            "VectorCraft Adobe Illustrator precision export",
        );
        p.add_edge(
            "IDML",
            "PDF",
            4,
            "DesignCraft InDesign package layout to PDF/X",
        );
        p.add_edge(
            "PPTX",
            "PDF",
            4,
            "DeckCraft vector presentation render to PDF",
        );
        p.add_edge(
            "PPTX",
            "PNG",
            5,
            "DeckCraft slide high-resolution rasterization",
        );

        p
    }
}

/// Global convenience function to find the shortest conversion path.
pub fn find_conversion_route(src: &str, dst: &str) -> Option<ConversionRoute> {
    let planner = RoutePlanner::new_with_standard_matrix();
    planner.find_route(src, dst, 3)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_direct_single_hop_route() {
        let route = find_conversion_route("PNG", "WEBP").expect("Direct route should exist");
        assert_eq!(route.hop_count(), 1);
        assert_eq!(route.steps[0].from, "PNG");
        assert_eq!(route.steps[0].to, "WEBP");
    }

    #[test]
    fn test_multi_hop_korean_hwp_route() {
        let route = find_conversion_route("HWP", "PNG").expect("HWP to PNG route should exist");
        // Expected multi-hop: HWP -> HWPX -> PDF -> PNG (3 hops)
        assert_eq!(route.hop_count(), 3);
        assert_eq!(route.steps[0].from, "HWP");
        assert_eq!(route.steps[0].to, "HWPX");
        assert_eq!(route.steps[1].to, "PDF");
        assert_eq!(route.steps[2].to, "PNG");
    }

    #[test]
    fn test_multi_hop_cad_dxf_to_png() {
        let route = find_conversion_route("DXF", "PNG").expect("DXF to PNG route should exist");
        // Expected: DXF -> SVG -> PNG (2 hops) or DXF -> PDF -> PNG (2 hops)
        assert_eq!(route.hop_count(), 2);
        assert_eq!(route.steps[0].from, "DXF");
        assert_eq!(route.steps[1].to, "PNG");
    }

    #[test]
    fn test_same_format_zero_hops() {
        let route = find_conversion_route("PNG", "PNG").expect("Same format route should exist");
        assert_eq!(route.hop_count(), 0);
        assert_eq!(route.total_cost, 0);
    }

    #[test]
    fn test_artcraft_afdesign_to_png() {
        let route = find_conversion_route("AFDESIGN", "PNG")
            .expect("AFDESIGN to PNG multi-hop route should exist");
        assert_eq!(route.hop_count(), 2);
        assert_eq!(route.steps[0].from, "AFDESIGN");
        assert_eq!(route.steps[0].to, "SVG");
        assert_eq!(route.steps[1].to, "PNG");
    }

    #[test]
    fn test_canvas_to_png() {
        let route = find_conversion_route("CANVAS", "PNG")
            .expect("Obsidian Canvas to PNG route should exist");
        assert_eq!(route.hop_count(), 2);
        assert_eq!(route.steps[0].from, "CANVAS");
        assert_eq!(route.steps[0].to, "SVG");
        assert_eq!(route.steps[1].to, "PNG");
    }

    #[test]
    fn test_epub_to_pdf() {
        let route = find_conversion_route("EPUB", "PDF").expect("EPUB to PDF route should exist");
        assert_eq!(route.hop_count(), 1);
        assert_eq!(route.steps[0].from, "EPUB");
        assert_eq!(route.steps[0].to, "PDF");
    }
}
