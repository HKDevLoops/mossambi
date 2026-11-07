//! Data Exchange Engine (JSON, JSON Lines, CSV, XML, vCard, iCalendar)
//!
//! Provides pure-Rust cross-conversions across tabular and structured data formats
//! without external runtime dependencies.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

/// Convert a JSON array of objects to CSV formatted text.
pub fn json_to_csv(json_str: &str) -> Result<String> {
    let val: Value = serde_json::from_str(json_str).context("Invalid JSON for CSV conversion")?;
    let arr = match val {
        Value::Array(a) => a,
        Value::Object(o) => vec![Value::Object(o)],
        _ => bail!("JSON must be an object or an array of objects to convert to CSV"),
    };

    if arr.is_empty() {
        return Ok(String::new());
    }

    // Collect all unique keys from all objects
    let mut headers = Vec::new();
    for item in &arr {
        if let Value::Object(map) = item {
            for k in map.keys() {
                if !headers.contains(k) {
                    headers.push(k.clone());
                }
            }
        }
    }

    let mut wtr = csv::Writer::from_writer(vec![]);
    wtr.write_record(&headers)?;

    for item in &arr {
        if let Value::Object(map) = item {
            let row: Vec<String> = headers
                .iter()
                .map(|h| match map.get(h) {
                    Some(Value::String(s)) => s.clone(),
                    Some(Value::Number(n)) => n.to_string(),
                    Some(Value::Bool(b)) => b.to_string(),
                    Some(Value::Null) | None => String::new(),
                    Some(other) => other.to_string(),
                })
                .collect();
            wtr.write_record(&row)?;
        }
    }

    let bytes = wtr.into_inner()?;
    String::from_utf8(bytes).context("CSV generated non-UTF8 output")
}

/// Convert CSV formatted text to a JSON array of objects.
pub fn csv_to_json(csv_str: &str) -> Result<String> {
    let mut rdr = csv::Reader::from_reader(csv_str.as_bytes());
    let headers = rdr.headers()?.clone();

    let mut records = Vec::new();
    for result in rdr.records() {
        let record = result?;
        let mut obj = serde_json::Map::new();
        for (i, field) in record.iter().enumerate() {
            if let Some(header) = headers.get(i) {
                // Infer basic types: int, float, bool, or string
                if let Ok(n) = field.parse::<i64>() {
                    obj.insert(header.to_string(), json!(n));
                } else if let Ok(f) = field.parse::<f64>() {
                    obj.insert(header.to_string(), json!(f));
                } else if let Ok(b) = field.parse::<bool>() {
                    obj.insert(header.to_string(), json!(b));
                } else {
                    obj.insert(header.to_string(), json!(field));
                }
            }
        }
        records.push(Value::Object(obj));
    }

    serde_json::to_string_pretty(&records).context("Failed to serialize JSON records")
}

/// Convert JSON to JSON Lines (one line per record).
pub fn json_to_jsonl(json_str: &str) -> Result<String> {
    let val: Value = serde_json::from_str(json_str).context("Invalid JSON for JSONL conversion")?;
    let arr = match val {
        Value::Array(a) => a,
        Value::Object(o) => vec![Value::Object(o)],
        _ => bail!("JSON must be an array or object to convert to JSONL"),
    };

    let mut lines = Vec::new();
    for item in arr {
        lines.push(serde_json::to_string(&item)?);
    }
    Ok(lines.join("\n"))
}

/// Convert JSON Lines to JSON Array.
pub fn jsonl_to_json(jsonl_str: &str) -> Result<String> {
    let mut arr = Vec::new();
    for line in jsonl_str.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let val: Value = serde_json::from_str(trimmed).context("Invalid JSON line in JSONL")?;
        arr.push(val);
    }
    serde_json::to_string_pretty(&arr).context("Failed to serialize JSON array from JSONL")
}

/// Convert JSON to basic XML.
pub fn json_to_xml(json_str: &str) -> Result<String> {
    let val: Value = serde_json::from_str(json_str).context("Invalid JSON for XML conversion")?;
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<root>\n");

    fn render_val(key: &str, val: &Value, indent: usize, out: &mut String) {
        let pad = " ".repeat(indent);
        match val {
            Value::Object(map) => {
                out.push_str(&format!("{}<{}>\n", pad, key));
                for (k, v) in map {
                    render_val(k, v, indent + 2, out);
                }
                out.push_str(&format!("{}</{}>\n", pad, key));
            }
            Value::Array(arr) => {
                for item in arr {
                    render_val("item", item, indent, out);
                }
            }
            Value::String(s) => {
                let escaped = s
                    .replace('&', "&amp;")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;");
                out.push_str(&format!("{}<{}>{}</{}>\n", pad, key, escaped, key));
            }
            Value::Number(n) => {
                out.push_str(&format!("{}<{}>{}</{}>\n", pad, key, n, key));
            }
            Value::Bool(b) => {
                out.push_str(&format!("{}<{}>{}</{}>\n", pad, key, b, key));
            }
            Value::Null => {
                out.push_str(&format!("{}<{}/>\n", pad, key));
            }
        }
    }

    match val {
        Value::Array(arr) => {
            for item in arr {
                render_val("item", &item, 2, &mut xml);
            }
        }
        Value::Object(map) => {
            for (k, v) in map {
                render_val(&k, &v, 2, &mut xml);
            }
        }
        _ => bail!("JSON must be an object or array to convert to XML"),
    }

    xml.push_str("</root>\n");
    Ok(xml)
}

/// Representation of a Contact (vCard / VCF).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ContactCard {
    pub formatted_name: String,
    pub given_name: String,
    pub family_name: String,
    pub email: String,
    pub telephone: String,
    pub organization: String,
    pub title: String,
    pub note: String,
}

/// Parse vCard (.vcf) formatted text into contact cards.
pub fn parse_vcard(text: &str) -> Vec<ContactCard> {
    let mut cards = Vec::new();
    let mut current: Option<ContactCard> = None;

    for line in text.lines() {
        let clean = line.trim();
        if clean.eq_ignore_ascii_case("BEGIN:VCARD") {
            current = Some(ContactCard::default());
            continue;
        }
        if clean.eq_ignore_ascii_case("END:VCARD") {
            if let Some(card) = current.take() {
                cards.push(card);
            }
            continue;
        }

        if let Some(card) = &mut current {
            if let Some((raw_key, val)) = clean.split_once(':') {
                let key = raw_key
                    .split(';')
                    .next()
                    .unwrap_or(raw_key)
                    .to_ascii_uppercase();
                match key.as_str() {
                    "FN" => card.formatted_name = val.to_string(),
                    "N" => {
                        let parts: Vec<&str> = val.split(';').collect();
                        if !parts.is_empty() {
                            card.family_name = parts[0].to_string();
                        }
                        if parts.len() > 1 {
                            card.given_name = parts[1].to_string();
                        }
                    }
                    "EMAIL" => card.email = val.to_string(),
                    "TEL" => card.telephone = val.to_string(),
                    "ORG" => card.organization = val.to_string(),
                    "TITLE" => card.title = val.to_string(),
                    "NOTE" => card.note = val.to_string(),
                    _ => {}
                }
            }
        }
    }

    cards
}

/// Render contact cards to vCard 3.0 format.
pub fn render_vcard(cards: &[ContactCard]) -> String {
    let mut out = String::new();
    for c in cards {
        out.push_str("BEGIN:VCARD\r\nVERSION:3.0\r\n");
        if !c.formatted_name.is_empty() {
            out.push_str(&format!("FN:{}\r\n", c.formatted_name));
        } else if !c.given_name.is_empty() || !c.family_name.is_empty() {
            out.push_str(format!("FN:{} {}\r\n", c.given_name, c.family_name).trim_start());
        }
        out.push_str(&format!("N:{};{};;;\r\n", c.family_name, c.given_name));
        if !c.email.is_empty() {
            out.push_str(&format!("EMAIL;TYPE=INTERNET:{}\r\n", c.email));
        }
        if !c.telephone.is_empty() {
            out.push_str(&format!("TEL;TYPE=CELL:{}\r\n", c.telephone));
        }
        if !c.organization.is_empty() {
            out.push_str(&format!("ORG:{}\r\n", c.organization));
        }
        if !c.title.is_empty() {
            out.push_str(&format!("TITLE:{}\r\n", c.title));
        }
        if !c.note.is_empty() {
            out.push_str(&format!("NOTE:{}\r\n", c.note));
        }
        out.push_str("END:VCARD\r\n");
    }
    out
}

/// Representation of an iCalendar Event (.ics).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CalendarEvent {
    pub summary: String,
    pub description: String,
    pub dtstart: String,
    pub dtend: String,
    pub location: String,
    pub uid: String,
}

/// Parse iCalendar (.ics) text into calendar events.
pub fn parse_icalendar(text: &str) -> Vec<CalendarEvent> {
    let mut events = Vec::new();
    let mut current: Option<CalendarEvent> = None;

    for line in text.lines() {
        let clean = line.trim();
        if clean.eq_ignore_ascii_case("BEGIN:VEVENT") {
            current = Some(CalendarEvent::default());
            continue;
        }
        if clean.eq_ignore_ascii_case("END:VEVENT") {
            if let Some(event) = current.take() {
                events.push(event);
            }
            continue;
        }

        if let Some(event) = &mut current {
            if let Some((raw_key, val)) = clean.split_once(':') {
                let key = raw_key
                    .split(';')
                    .next()
                    .unwrap_or(raw_key)
                    .to_ascii_uppercase();
                match key.as_str() {
                    "SUMMARY" => event.summary = val.to_string(),
                    "DESCRIPTION" => event.description = val.to_string(),
                    "DTSTART" => event.dtstart = val.to_string(),
                    "DTEND" => event.dtend = val.to_string(),
                    "LOCATION" => event.location = val.to_string(),
                    "UID" => event.uid = val.to_string(),
                    _ => {}
                }
            }
        }
    }

    events
}

/// Render calendar events into standard iCalendar RFC 5545 format.
pub fn render_icalendar(events: &[CalendarEvent]) -> String {
    let mut out = String::new();
    out.push_str(
        "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//Mossambi Systems//NONSGML v2.1//EN\r\n",
    );
    for (i, ev) in events.iter().enumerate() {
        out.push_str("BEGIN:VEVENT\r\n");
        let uid = if !ev.uid.is_empty() {
            ev.uid.clone()
        } else {
            format!("mossambi-evt-{}@mossambi.local", i + 1)
        };
        out.push_str(&format!("UID:{}\r\n", uid));
        out.push_str(&format!("SUMMARY:{}\r\n", ev.summary));
        if !ev.description.is_empty() {
            out.push_str(&format!("DESCRIPTION:{}\r\n", ev.description));
        }
        if !ev.dtstart.is_empty() {
            out.push_str(&format!("DTSTART:{}\r\n", ev.dtstart));
        }
        if !ev.dtend.is_empty() {
            out.push_str(&format!("DTEND:{}\r\n", ev.dtend));
        }
        if !ev.location.is_empty() {
            out.push_str(&format!("LOCATION:{}\r\n", ev.location));
        }
        out.push_str("END:VEVENT\r\n");
    }
    out.push_str("END:VCALENDAR\r\n");
    out
}

/// Transcode data exchange file between JSON, CSV, JSONL, XML, VCF, and ICS.
pub fn transcode_data_exchange(input: &Path, output: &Path) -> Result<()> {
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

    let content = fs::read_to_string(input).context("Failed to read input data exchange file")?;

    match (in_ext.as_str(), out_ext.as_str()) {
        ("JSON", "CSV") => {
            let csv = json_to_csv(&content)?;
            fs::write(output, csv)?;
        }
        ("CSV", "JSON") => {
            let json = csv_to_json(&content)?;
            fs::write(output, json)?;
        }
        ("JSON", "JSONL") => {
            let jsonl = json_to_jsonl(&content)?;
            fs::write(output, jsonl)?;
        }
        ("JSONL", "JSON") => {
            let json = jsonl_to_json(&content)?;
            fs::write(output, json)?;
        }
        ("JSON", "XML") => {
            let xml = json_to_xml(&content)?;
            fs::write(output, xml)?;
        }
        ("VCF", "JSON") => {
            let cards = parse_vcard(&content);
            let json = serde_json::to_string_pretty(&cards)?;
            fs::write(output, json)?;
        }
        ("VCF", "CSV") => {
            let cards = parse_vcard(&content);
            let json = serde_json::to_string(&cards)?;
            let csv = json_to_csv(&json)?;
            fs::write(output, csv)?;
        }
        ("JSON", "VCF") => {
            let cards: Vec<ContactCard> = serde_json::from_str(&content)?;
            let vcf = render_vcard(&cards);
            fs::write(output, vcf)?;
        }
        ("ICS", "JSON") => {
            let events = parse_icalendar(&content);
            let json = serde_json::to_string_pretty(&events)?;
            fs::write(output, json)?;
        }
        ("ICS", "CSV") => {
            let events = parse_icalendar(&content);
            let json = serde_json::to_string(&events)?;
            let csv = json_to_csv(&json)?;
            fs::write(output, csv)?;
        }
        ("JSON", "ICS") => {
            let events: Vec<CalendarEvent> = serde_json::from_str(&content)?;
            let ics = render_icalendar(&events);
            fs::write(output, ics)?;
        }
        _ => {
            bail!(
                "Unsupported data exchange transcoding route: {} -> {}",
                in_ext,
                out_ext
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_csv_roundtrip() {
        let json_input = r#"[{"id":1,"name":"Alice","role":"Engineer"},{"id":2,"name":"Bob","role":"Designer"}]"#;
        let csv_output = json_to_csv(json_input).expect("json to csv failed");
        assert!(csv_output.contains("Alice"));
        assert!(csv_output.contains("Designer"));

        let json_again = csv_to_json(&csv_output).expect("csv to json failed");
        assert!(json_again.contains("Alice"));
        assert!(json_again.contains("Bob"));
    }

    #[test]
    fn test_vcard_parse_and_render() {
        let vcard_text = "BEGIN:VCARD\r\nVERSION:3.0\r\nFN:John Doe\r\nEMAIL:john@example.com\r\nTEL:1234567890\r\nEND:VCARD\r\n";
        let cards = parse_vcard(vcard_text);
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].formatted_name, "John Doe");
        assert_eq!(cards[0].email, "john@example.com");

        let rendered = render_vcard(&cards);
        assert!(rendered.contains("FN:John Doe"));
        assert!(rendered.contains("EMAIL;TYPE=INTERNET:john@example.com"));
    }

    #[test]
    fn test_icalendar_parse_and_render() {
        let ics_text = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nSUMMARY:Team Meeting\r\nDTSTART:20261108T100000Z\r\nLOCATION:Conference Room\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        let events = parse_icalendar(ics_text);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].summary, "Team Meeting");

        let rendered = render_icalendar(&events);
        assert!(rendered.contains("SUMMARY:Team Meeting"));
        assert!(rendered.contains("LOCATION:Conference Room"));
    }
}
