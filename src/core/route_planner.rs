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
        let raster_imgs = ["PNG", "WEBP", "JPG", "JPEG", "BMP", "TIFF", "AVIF", "ICO"];
        for &a in &raster_imgs {
            for &b in &raster_imgs {
                if a != b {
                    p.add_edge(a, b, 5, "Direct pure-Rust SIMD image transcode");
                }
            }
        }

        // Vector Image to Raster & PDF (Cost: 4)
        p.add_edge("SVG", "PNG", 4, "Pure-Rust resvg rasterization");
        p.add_edge("SVG", "PDF", 4, "Vector geometry preservation via lopdf");

        // Camera RAW to DNG / TIFF / PNG (Cost: 6)
        let raws = ["CR2", "CR3", "NEF", "ARW", "ORF", "RW2", "RAF", "DNG"];
        for &r in &raws {
            p.add_edge(r, "DNG", 3, "Lossless Camera RAW container encapsulation");
            p.add_edge(r, "TIFF", 5, "16-bit linear demosaic extraction");
            p.add_edge(r, "PNG", 6, "16-bit RGB rasterization");
        }

        // 2. Documents & Office
        p.add_edge("DOCX", "PDF", 8, "Native OOXML layout engine to PDF");
        p.add_edge("DOCX", "TXT", 2, "XML paragraph extraction");
        p.add_edge("DOCX", "HTML", 5, "OOXML to clean Semantic HTML5");
        p.add_edge("DOCX", "MD", 4, "OOXML to CommonMark Markdown");

        p.add_edge("MD", "HTML", 2, "Markdown parser to Semantic HTML5");
        p.add_edge("MD", "PDF", 6, "Markdown to Styled Vector PDF");
        p.add_edge("HTML", "PDF", 7, "HTML5 CSS paged media layout");

        // Korean Hangul HWP / HWPX
        p.add_edge(
            "HWP",
            "HWPX",
            3,
            "Decompress Compound Document OLE to OOXML HWPX",
        );
        p.add_edge("HWPX", "PDF", 7, "HWPX layout engine to PDF");
        p.add_edge("HWPX", "TXT", 2, "HWPX XML section text extraction");

        // CAD (DXF, DWG)
        p.add_edge("DWG", "DXF", 5, "Autodesk drawing entity translation");
        p.add_edge("DXF", "SVG", 4, "CAD 2D vector entity export");
        p.add_edge("DXF", "PDF", 5, "CAD scaled technical vector plot");

        // Spreadsheets
        let sheets = ["XLSX", "XLS", "ODS", "CSV", "TSV"];
        for &s in &sheets {
            p.add_edge(s, "CSV", 2, "Calamine table extraction");
            p.add_edge(s, "XLSX", 3, "Workbook generation via rust_xlsxwriter");
            p.add_edge(s, "PDF", 6, "Spreadsheet grid layout rendering");
            p.add_edge(s, "HTML", 3, "HTML table markup conversion");
        }

        // 3D Models
        p.add_edge("STL", "OBJ", 3, "Facet mesh re-indexing");
        p.add_edge("OBJ", "GLTF", 4, "glTF 2.0 PBR asset conversion");
        p.add_edge("GLTF", "GLB", 2, "Binary glTF container pack");
        p.add_edge("GLB", "GLTF", 2, "JSON glTF unpack with binary buffer");
        p.add_edge(
            "3MF",
            "STL",
            3,
            "Additive manufacturing triangular mesh export",
        );

        // PDF Vector / Text
        p.add_edge("PDF", "PNG", 8, "High-DPI rasterization (150-300 DPI)");
        p.add_edge("PDF", "TXT", 2, "lopdf content stream character extraction");

        // Audio Transcode Matrix (Cost: 2 for stream copy, 5 for lossless, 8 for lossy)
        let audio_lossless = ["FLAC", "WAV", "ALAC", "AIFF"];
        let audio_lossy = ["MP3", "AAC", "OGG", "OPUS", "M4A"];

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
        }

        // Video Transcodes & Audio Extraction
        let video_fmts = ["MP4", "MKV", "MOV", "WEBM", "AVI"];
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

        // Subtitles
        let subs = ["SRT", "VTT", "ASS", "SSA", "SUB"];
        for &s1 in &subs {
            for &s2 in &subs {
                if s1 != s2 {
                    p.add_edge(s1, s2, 2, "Subtitle timestamp translation");
                }
            }
        }

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
}
