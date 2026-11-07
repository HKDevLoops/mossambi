//! Pure ASCII Regression Gate Test
//! Asserts that every source file under `src/gui/` contains strictly ASCII characters (0..=127).
//! Disallows any emoji, smart quotes, em-dashes, or unicode symbols in production GUI code
//! to guarantee zero font-fallback glitches and pristine rendering across all OSes.

use std::fs;
use std::path::Path;

#[test]
fn test_gui_source_files_are_pure_ascii() {
    let gui_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join("gui");
    assert!(gui_dir.exists(), "src/gui directory must exist");

    let mut violations = Vec::new();

    for entry in walkdir::WalkDir::new(&gui_dir) {
        let entry = entry.expect("failed to traverse src/gui directory");
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            let content = fs::read_to_string(path)
                .unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e));

            for (line_idx, line) in content.lines().enumerate() {
                for (col_idx, ch) in line.chars().enumerate() {
                    if !ch.is_ascii() {
                        violations.push(format!(
                            "{}:{}:{} -> non-ASCII character '{}' (U+{:04X})",
                            path.strip_prefix(env!("CARGO_MANIFEST_DIR")).unwrap_or(path).display(),
                            line_idx + 1,
                            col_idx + 1,
                            ch,
                            ch as u32
                        ));
                    }
                }
            }
        }
    }

    if !violations.is_empty() {
        panic!(
            "Pure ASCII Regression Gate FAILED with {} violations:\n{}",
            violations.len(),
            violations.join("\n")
        );
    }
}
