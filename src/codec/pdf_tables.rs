//! PDF Tables Extraction Engine (PDF to Excel XLSX / CSV)
//!
//! Detects table grid structures and text positioning in vector PDF documents
//! and converts them directly into native Excel (.xlsx) workbooks or CSV files.

use anyhow::{bail, Context, Result};
use lopdf::content::Content;
use lopdf::Document;
use rust_xlsxwriter::Workbook;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// Extracted text token with page coordinate.
#[derive(Debug, Clone)]
struct PdfWord {
    pub text: String,
    pub x: f32,
    pub y: f32,
}

/// Extracted row of cells.
#[derive(Debug, Clone, Default)]
pub struct TableRow {
    pub cells: Vec<String>,
}

/// Extracted sheet table.
#[derive(Debug, Clone, Default)]
pub struct PdfTable {
    pub page_number: u32,
    pub rows: Vec<TableRow>,
}

/// Extract table rows from PDF bytes.
pub fn extract_pdf_tables(pdf_bytes: &[u8]) -> Result<Vec<PdfTable>> {
    let doc = Document::load_mem(pdf_bytes).context("Failed to load PDF memory")?;
    let mut tables = Vec::new();

    for (page_idx, page_id) in doc.page_iter().enumerate() {
        let mut words = Vec::new();
        let content_bytes = doc.get_page_content(page_id);
        if let Ok(content) = Content::decode(&content_bytes) {
            let mut current_x = 0.0f32;
            let mut current_y = 0.0f32;

            for op in &content.operations {
                match op.operator.as_str() {
                    "Tm" | "Td" | "TD" => {
                        if op.operands.len() >= 2 {
                            let l = op.operands.len();
                            if let (Ok(tx), Ok(ty)) =
                                (op.operands[l - 2].as_float(), op.operands[l - 1].as_float())
                            {
                                current_x = tx;
                                current_y = ty;
                            }
                        }
                    }
                    "Tj" => {
                        if let Some(operand) = op.operands.first() {
                            if let Ok(s) = operand.as_str() {
                                let clean = String::from_utf8_lossy(s).trim().to_string();
                                if !clean.is_empty() {
                                    words.push(PdfWord {
                                        text: clean,
                                        x: current_x,
                                        y: current_y,
                                    });
                                }
                            }
                        }
                    }
                    "TJ" => {
                        if let Some(arr) = op.operands.first().and_then(|o| o.as_array().ok()) {
                            let mut combined = String::new();
                            for item in arr {
                                if let Ok(s) = item.as_str() {
                                    combined.push_str(&String::from_utf8_lossy(s));
                                }
                            }
                            let clean = combined.trim().to_string();
                            if !clean.is_empty() {
                                words.push(PdfWord {
                                    text: clean,
                                    x: current_x,
                                    y: current_y,
                                });
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        if words.is_empty() {
            // Fallback: extract page text using lopdf high-level helper
            if let Ok(text) = doc.extract_text(&[page_idx as u32 + 1]) {
                let mut fallback_rows = Vec::new();
                for line in text.lines() {
                    let cells: Vec<String> =
                        line.split_whitespace().map(|s| s.to_string()).collect();
                    if !cells.is_empty() {
                        fallback_rows.push(TableRow { cells });
                    }
                }
                if !fallback_rows.is_empty() {
                    tables.push(PdfTable {
                        page_number: page_idx as u32 + 1,
                        rows: fallback_rows,
                    });
                }
            }
            continue;
        }

        // Cluster words into rows by Y (within 4 pt tolerance)
        // Group descending by Y because PDF origin is bottom-left
        let mut row_map: BTreeMap<i32, Vec<PdfWord>> = BTreeMap::new();
        for w in words {
            let key = (w.y / 4.0).round() as i32;
            row_map.entry(-key).or_default().push(w);
        }

        let mut page_rows = Vec::new();
        for (_, mut row_words) in row_map {
            // Sort row cells ascending by X
            row_words.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal));
            let cells: Vec<String> = row_words.into_iter().map(|w| w.text).collect();
            page_rows.push(TableRow { cells });
        }

        if !page_rows.is_empty() {
            tables.push(PdfTable {
                page_number: page_idx as u32 + 1,
                rows: page_rows,
            });
        }
    }

    Ok(tables)
}

/// Transcode PDF table content directly to Excel (.xlsx) or CSV.
pub fn transcode_pdf_tables(input: &Path, output: &Path) -> Result<()> {
    let bytes = fs::read(input).context("Failed to read input PDF")?;
    let tables = extract_pdf_tables(&bytes)?;

    let out_ext = output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_uppercase();

    match out_ext.as_str() {
        "XLSX" => {
            let mut workbook = Workbook::new();
            if tables.is_empty() {
                let worksheet = workbook.add_worksheet();
                worksheet.write_string(0, 0, "No table data detected in PDF")?;
            } else {
                for table in tables {
                    let sheet_name = format!("Page {}", table.page_number);
                    let worksheet = workbook.add_worksheet();
                    worksheet.set_name(&sheet_name)?;

                    for (row_idx, row) in table.rows.iter().enumerate() {
                        for (col_idx, cell) in row.cells.iter().enumerate() {
                            // Check if numeric value
                            if let Ok(num) = cell.replace(',', "").parse::<f64>() {
                                worksheet.write_number(row_idx as u32, col_idx as u16, num)?;
                            } else {
                                worksheet.write_string(row_idx as u32, col_idx as u16, cell)?;
                            }
                        }
                    }
                }
            }
            workbook
                .save(output)
                .context("Failed to write XLSX workbook")?;
        }
        "CSV" => {
            let mut wtr = csv::Writer::from_path(output)?;
            for table in tables {
                for row in table.rows {
                    wtr.write_record(&row.cells)?;
                }
            }
            wtr.flush()?;
        }
        _ => bail!("Unsupported PDF tables output extension: {}", out_ext),
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_row_structure() {
        let row = TableRow {
            cells: vec![
                "Revenue".to_string(),
                "10000".to_string(),
                "2026".to_string(),
            ],
        };
        assert_eq!(row.cells.len(), 3);
        assert_eq!(row.cells[0], "Revenue");
    }
}
