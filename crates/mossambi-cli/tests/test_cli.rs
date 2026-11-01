use std::fs;
use std::process::Command;

fn get_bin_path() -> std::path::PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop(); // drop test binary name
    if path.ends_with("deps") {
        path.pop();
    }
    path.join(format!("mossambi{}", std::env::consts::EXE_SUFFIX))
}

#[test]
fn test_cli_help_and_version() {
    let bin = get_bin_path();
    if !bin.exists() {
        return; // Skip if bin not yet in target
    }

    let output = Command::new(&bin).arg("--help").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Mossambi"));
    assert!(stdout.contains("Usage:"));
}

#[test]
fn test_cli_hardware_diagnostics() {
    let bin = get_bin_path();
    if !bin.exists() {
        return;
    }

    let output = Command::new(&bin).arg("hw").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Hardware Acceleration"));
    assert!(stdout.contains("CPU Architecture"));
}

#[test]
fn test_cli_config_commands() {
    let bin = get_bin_path();
    if !bin.exists() {
        return;
    }

    // Set theme
    let set_out = Command::new(&bin)
        .args(["config", "set", "theme", "liquid_glass"])
        .output()
        .unwrap();
    assert!(set_out.status.success());

    // Get theme
    let get_out = Command::new(&bin)
        .args(["config", "get", "theme"])
        .output()
        .unwrap();
    assert!(get_out.status.success());
    let stdout = String::from_utf8_lossy(&get_out.stdout);
    assert!(stdout.contains("LiquidGlass"));
}

#[test]
fn test_cli_text_to_pdf_mv_grammar() {
    let bin = get_bin_path();
    if !bin.exists() {
        return;
    }

    let test_dir = std::env::temp_dir().join("mossambi_cli_test_pdf");
    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(&test_dir).unwrap();

    let txt_file = test_dir.join("notes.txt");
    fs::write(&txt_file, b"Header Line\n\nParagraph with publication typesetting.\n").unwrap();

    // Run: mossambi notes.txt pdf
    let conv_out = Command::new(&bin)
        .args([txt_file.to_str().unwrap(), "pdf"])
        .output()
        .unwrap();
    assert!(conv_out.status.success());

    let expected_pdf = test_dir.join("notes_converted.pdf");
    assert!(expected_pdf.exists());
    assert!(fs::metadata(&expected_pdf).unwrap().len() > 100);

    let _ = fs::remove_dir_all(&test_dir);
}
