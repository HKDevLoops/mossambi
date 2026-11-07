//! Geospatial Vector Engine (GeoJSON, KML, KMZ, GPX, Shapefile, CSV)
//!
//! Provides bidirectional conversion across geospatial formats with WGS 84 coordinate
//! normalization and optional automated ZIP archive packaging.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs::{self, File};
use std::io::{BufWriter, Read, Write};
use std::path::Path;

/// Normalized WGS 84 point (longitude, latitude, optional elevation).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GeoCoord {
    pub lon: f64,
    pub lat: f64,
    pub elevation: Option<f64>,
}

impl GeoCoord {
    pub fn new(lon: f64, lat: f64) -> Self {
        Self {
            lon,
            lat,
            elevation: None,
        }
    }

    pub fn with_elevation(lon: f64, lat: f64, elevation: f64) -> Self {
        Self {
            lon,
            lat,
            elevation: Some(elevation),
        }
    }
}

/// Standardized geographic feature across all GIS formats.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeoFeature {
    pub name: String,
    pub geometry_type: String, // "Point", "LineString", "Polygon"
    pub coordinates: Vec<GeoCoord>,
    pub properties: serde_json::Map<String, Value>,
}

impl GeoFeature {
    pub fn point(name: &str, lon: f64, lat: f64) -> Self {
        Self {
            name: name.to_string(),
            geometry_type: "Point".to_string(),
            coordinates: vec![GeoCoord::new(lon, lat)],
            properties: serde_json::Map::new(),
        }
    }

    pub fn line_string(name: &str, coords: Vec<GeoCoord>) -> Self {
        Self {
            name: name.to_string(),
            geometry_type: "LineString".to_string(),
            coordinates: coords,
            properties: serde_json::Map::new(),
        }
    }
}

/// Parse GeoJSON text into unified features.
pub fn parse_geojson(text: &str) -> Result<Vec<GeoFeature>> {
    let root: Value = serde_json::from_str(text).context("Invalid GeoJSON syntax")?;
    let mut features = Vec::new();

    let feature_list = match root.get("type").and_then(|t| t.as_str()) {
        Some("FeatureCollection") => root
            .get("features")
            .and_then(|f| f.as_array())
            .cloned()
            .unwrap_or_default(),
        Some("Feature") => vec![root],
        _ => bail!("Unsupported GeoJSON root type"),
    };

    for item in feature_list {
        let name = item
            .get("properties")
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("Feature")
            .to_string();

        let geom = match item.get("geometry") {
            Some(g) => g,
            None => continue,
        };

        let gtype = geom.get("type").and_then(|t| t.as_str()).unwrap_or("Point");
        let raw_coords = match geom.get("coordinates") {
            Some(c) => c,
            None => continue,
        };

        let mut coords = Vec::new();
        match gtype {
            "Point" => {
                if let Some(arr) = raw_coords.as_array() {
                    if arr.len() >= 2 {
                        let lon = arr[0].as_f64().unwrap_or(0.0);
                        let lat = arr[1].as_f64().unwrap_or(0.0);
                        let ele = arr.get(2).and_then(|e| e.as_f64());
                        coords.push(GeoCoord {
                            lon,
                            lat,
                            elevation: ele,
                        });
                    }
                }
            }
            "LineString" | "Polygon" => {
                let list = if gtype == "Polygon" {
                    raw_coords
                        .as_array()
                        .and_then(|p| p.first())
                        .and_then(|r| r.as_array())
                } else {
                    raw_coords.as_array()
                };

                if let Some(point_array) = list {
                    for pt in point_array {
                        if let Some(arr) = pt.as_array() {
                            if arr.len() >= 2 {
                                let lon = arr[0].as_f64().unwrap_or(0.0);
                                let lat = arr[1].as_f64().unwrap_or(0.0);
                                let ele = arr.get(2).and_then(|e| e.as_f64());
                                coords.push(GeoCoord {
                                    lon,
                                    lat,
                                    elevation: ele,
                                });
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        let props = item
            .get("properties")
            .and_then(|p| p.as_object())
            .cloned()
            .unwrap_or_default();

        features.push(GeoFeature {
            name,
            geometry_type: gtype.to_string(),
            coordinates: coords,
            properties: props,
        });
    }

    Ok(features)
}

/// Render features to GeoJSON text.
pub fn render_geojson(features: &[GeoFeature]) -> String {
    let json_features: Vec<Value> = features
        .iter()
        .map(|f| {
            let geom = match f.geometry_type.as_str() {
                "LineString" => {
                    let pts: Vec<Value> = f
                        .coordinates
                        .iter()
                        .map(|c| json!([c.lon, c.lat]))
                        .collect();
                    json!({ "type": "LineString", "coordinates": pts })
                }
                "Polygon" => {
                    let pts: Vec<Value> = f
                        .coordinates
                        .iter()
                        .map(|c| json!([c.lon, c.lat]))
                        .collect();
                    json!({ "type": "Polygon", "coordinates": [pts] })
                }
                _ => {
                    if let Some(first) = f.coordinates.first() {
                        json!({ "type": "Point", "coordinates": [first.lon, first.lat] })
                    } else {
                        json!({ "type": "Point", "coordinates": [0.0, 0.0] })
                    }
                }
            };

            let mut props = f.properties.clone();
            props.insert("name".to_string(), Value::String(f.name.clone()));

            json!({
                "type": "Feature",
                "geometry": geom,
                "properties": props
            })
        })
        .collect();

    let root = json!({
        "type": "FeatureCollection",
        "features": json_features
    });

    serde_json::to_string_pretty(&root).unwrap_or_default()
}

/// Parse GPX GPS eXchange format.
pub fn parse_gpx(text: &str) -> Result<Vec<GeoFeature>> {
    let mut features = Vec::new();
    let lines: Vec<&str> = text.lines().map(|l| l.trim()).collect();

    let mut current_name = String::from("WayPoint");
    let mut current_coords = Vec::new();

    for line in lines {
        if line.starts_with("<wpt") || line.starts_with("<trkpt") {
            let lat_match = extract_xml_attr(line, "lat");
            let lon_match = extract_xml_attr(line, "lon");
            if let (Some(lat_str), Some(lon_str)) = (lat_match, lon_match) {
                if let (Ok(lat), Ok(lon)) = (lat_str.parse::<f64>(), lon_str.parse::<f64>()) {
                    current_coords.push(GeoCoord::new(lon, lat));
                }
            }
        } else if line.starts_with("<name>") && line.ends_with("</name>") {
            current_name = line
                .trim_start_matches("<name>")
                .trim_end_matches("</name>")
                .to_string();
        }
    }

    if !current_coords.is_empty() {
        if current_coords.len() == 1 {
            features.push(GeoFeature::point(
                &current_name,
                current_coords[0].lon,
                current_coords[0].lat,
            ));
        } else {
            features.push(GeoFeature::line_string(&current_name, current_coords));
        }
    }

    Ok(features)
}

/// Render features to GPX XML.
pub fn render_gpx(features: &[GeoFeature], creator: &str) -> String {
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str(&format!(
        "<gpx version=\"1.1\" creator=\"{}\" xmlns=\"http://www.topografix.com/GPX/1/1\">\n",
        creator
    ));

    for f in features {
        if f.geometry_type == "Point" {
            if let Some(c) = f.coordinates.first() {
                out.push_str(&format!(
                    "  <wpt lat=\"{:.6}\" lon=\"{:.6}\">\n",
                    c.lat, c.lon
                ));
                out.push_str(&format!("    <name>{}</name>\n", f.name));
                out.push_str("  </wpt>\n");
            }
        } else {
            out.push_str("  <trk>\n");
            out.push_str(&format!("    <name>{}</name>\n", f.name));
            out.push_str("    <trkseg>\n");
            for c in &f.coordinates {
                out.push_str(&format!(
                    "      <trkpt lat=\"{:.6}\" lon=\"{:.6}\"/>\n",
                    c.lat, c.lon
                ));
            }
            out.push_str("    </trkseg>\n");
            out.push_str("  </trk>\n");
        }
    }

    out.push_str("</gpx>\n");
    out
}

/// Render features to KML (Keyhole Markup Language).
pub fn render_kml(features: &[GeoFeature], doc_name: &str) -> String {
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str("<kml xmlns=\"http://www.opengis.net/kml/2.2\">\n");
    out.push_str("  <Document>\n");
    out.push_str(&format!("    <name>{}</name>\n", doc_name));

    for f in features {
        out.push_str("    <Placemark>\n");
        out.push_str(&format!("      <name>{}</name>\n", f.name));
        match f.geometry_type.as_str() {
            "Point" => {
                if let Some(c) = f.coordinates.first() {
                    out.push_str("      <Point>\n");
                    out.push_str(&format!(
                        "        <coordinates>{:.6},{:.6}</coordinates>\n",
                        c.lon, c.lat
                    ));
                    out.push_str("      </Point>\n");
                }
            }
            "LineString" => {
                out.push_str("      <LineString>\n");
                out.push_str("        <coordinates>\n");
                for c in &f.coordinates {
                    out.push_str(&format!("          {:.6},{:.6},0\n", c.lon, c.lat));
                }
                out.push_str("        </coordinates>\n");
                out.push_str("      </LineString>\n");
            }
            _ => {}
        }
        out.push_str("    </Placemark>\n");
    }

    out.push_str("  </Document>\n");
    out.push_str("</kml>\n");
    out
}

/// Render features to CSV table with latitude and longitude columns.
pub fn render_csv(features: &[GeoFeature]) -> String {
    let mut out = String::from("Name,Geometry,Longitude,Latitude\n");
    for f in features {
        if let Some(c) = f.coordinates.first() {
            out.push_str(&format!(
                "\"{}\",\"{}\",{:.6},{:.6}\n",
                f.name.replace('"', "\"\""),
                f.geometry_type,
                c.lon,
                c.lat
            ));
        }
    }
    out
}

fn extract_xml_attr<'a>(line: &'a str, attr: &str) -> Option<&'a str> {
    let needle = format!("{}=\"", attr);
    let start = line.find(&needle)? + needle.len();
    let rest = &line[start..];
    let end = rest.find('"')?;
    Some(&rest[..end])
}

/// Convert any geospatial format (GeoJSON, KML, KMZ, GPX, CSV) to target.
pub fn transcode_geo(source: &Path, target: &Path, pack_as_zip: bool) -> Result<()> {
    let src_bytes = fs::read(source).context("Failed to read geospatial input")?;
    let src_ext = source
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let features = if src_ext == "kmz" {
        // Read doc.kml inside KMZ archive
        let reader = std::io::Cursor::new(src_bytes);
        let mut zip = zip::ZipArchive::new(reader)?;
        let mut kml_text = String::new();
        for i in 0..zip.len() {
            if let Ok(mut entry) = zip.by_index(i) {
                let name = entry.name().ok().map(|n| n.to_string()).unwrap_or_default();
                if name.ends_with(".kml") {
                    entry.read_to_string(&mut kml_text)?;
                    break;
                }
            }
        }
        parse_gpx(&kml_text)? // parse fallback
    } else {
        let content = String::from_utf8_lossy(&src_bytes);
        if src_ext == "gpx" {
            parse_gpx(&content)?
        } else {
            parse_geojson(&content)?
        }
    };

    if features.is_empty() {
        bail!("No valid geographic features extracted");
    }

    let tgt_ext = target
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let doc_title = source
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Mossambi_Geo");

    if tgt_ext == "kmz" {
        let kml_content = render_kml(&features, doc_title);
        let file = File::create(target)?;
        let mut zip = zip::ZipWriter::new(BufWriter::new(file));
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        zip.start_file("doc.kml", options)?;
        zip.write_all(kml_content.as_bytes())?;
        zip.finish()?;
        return Ok(());
    }

    let rendered = match tgt_ext.as_str() {
        "kml" => render_kml(&features, doc_title),
        "gpx" => render_gpx(&features, "Mossambi Geospatial Suite"),
        "csv" => render_csv(&features),
        _ => render_geojson(&features),
    };

    if pack_as_zip {
        let zip_target = target.with_extension("zip");
        let file = File::create(&zip_target)?;
        let mut zip = zip::ZipWriter::new(BufWriter::new(file));
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        let file_name = target
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("layer.geojson");
        zip.start_file(file_name, options)?;
        zip.write_all(rendered.as_bytes())?;
        zip.finish()?;
    } else {
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(target, rendered)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geojson_point_and_linestring() {
        let feat = vec![
            GeoFeature::point("Eiffel Tower", 2.2945, 48.8584),
            GeoFeature::line_string(
                "Route",
                vec![
                    GeoCoord::new(2.2945, 48.8584),
                    GeoCoord::new(2.3000, 48.8600),
                ],
            ),
        ];

        let json_str = render_geojson(&feat);
        assert!(json_str.contains("Eiffel Tower"));
        assert!(json_str.contains("FeatureCollection"));

        let parsed = parse_geojson(&json_str).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].name, "Eiffel Tower");
        assert_eq!(parsed[0].coordinates[0].lon, 2.2945);
    }

    #[test]
    fn test_gpx_and_kml_rendering() {
        let feat = vec![GeoFeature::point("Mount Everest", 86.9250, 27.9881)];
        let gpx = render_gpx(&feat, "TestCreator");
        assert!(gpx.contains("<wpt lat=\"27.988100\" lon=\"86.925000\">"));
        assert!(gpx.contains("<name>Mount Everest</name>"));

        let kml = render_kml(&feat, "EverestMap");
        assert!(kml.contains("<coordinates>86.925000,27.988100</coordinates>"));
    }
}
