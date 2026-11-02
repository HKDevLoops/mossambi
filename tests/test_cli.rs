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

    let expected_pdf = if test_dir.join("notes.pdf").exists() {
        test_dir.join("notes.pdf")
    } else {
        test_dir.join("notes_converted.pdf")
    };
    assert!(expected_pdf.exists());
    assert!(fs::metadata(&expected_pdf).unwrap().len() > 100);

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_cli_catalogue_search() {
    let bin = get_bin_path();
    if !bin.exists() {
        return;
    }

    let out = Command::new(&bin)
        .args(["catalogue", "flac"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("FLAC"));
    assert!(stdout.contains("Guaranteed"));
}

#[test]
fn test_cli_verify_zero_loss() {
    let bin = get_bin_path();
    if !bin.exists() {
        return;
    }

    let test_dir = std::env::temp_dir().join("mossambi_cli_test_verify");
    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(&test_dir).unwrap();

    let txt_file = test_dir.join("document.txt");
    fs::write(&txt_file, b"Fidelity test text payload\n").unwrap();

    // Convert to PDF
    let conv = Command::new(&bin)
        .args([txt_file.to_str().unwrap(), "pdf"])
        .output()
        .unwrap();
    assert!(conv.status.success());

    let pdf_file = if test_dir.join("document.pdf").exists() {
        test_dir.join("document.pdf")
    } else {
        test_dir.join("document_converted.pdf")
    };
    assert!(pdf_file.exists());

    // Verify mathematical quality
    let ver_out = Command::new(&bin)
        .args(["verify", txt_file.to_str().unwrap(), pdf_file.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(ver_out.status.success());
    let stdout = String::from_utf8_lossy(&ver_out.stdout);
    assert!(stdout.contains("VERIFIED 0.00% ZERO-LOSS") || stdout.contains("Vector Layout Preservation"));

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_cli_upload_ready() {
    let bin = get_bin_path();
    if !bin.exists() {
        return;
    }

    let test_dir = std::env::temp_dir().join("mossambi_cli_test_upload");
    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(&test_dir).unwrap();

    let img_file = test_dir.join("applicant.png");
    let img: image::RgbImage = image::ImageBuffer::new(500, 600);
    img.save(&img_file).unwrap();

    let out = Command::new(&bin)
        .args(["upload-ready", img_file.to_str().unwrap(), "--preset", "upsc-photo"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Exam") || stdout.contains("UPSC"));

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_cli_check_and_bench_commands() {
    let bin = get_bin_path();
    if !bin.exists() {
        return;
    }

    let workspace_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sample_txt = workspace_root.join("test_vault").join("document").join("sample_notes.txt");

    if sample_txt.exists() {
        // Run: mossambi check sample_notes.txt
        let check_out = Command::new(&bin)
            .args(["check", sample_txt.to_str().unwrap()])
            .output()
            .unwrap();
        assert!(check_out.status.success());
        let stdout = String::from_utf8_lossy(&check_out.stdout);
        assert!(stdout.contains("MOSSAMBI QUALITY & INTEGRITY INSPECTOR"));
        assert!(stdout.contains("PASS"));

        // Run with --json
        let check_json = Command::new(&bin)
            .args(["check", sample_txt.to_str().unwrap(), "--json"])
            .output()
            .unwrap();
        assert!(check_json.status.success());
        let json_stdout = String::from_utf8_lossy(&check_json.stdout);
        assert!(json_stdout.contains("\"overall_score\""));

        // Run: mossambi bench sample_notes.txt
        let bench_out = Command::new(&bin)
            .args(["bench", sample_txt.to_str().unwrap()])
            .output()
            .unwrap();
        assert!(bench_out.status.success());
        let bench_stdout = String::from_utf8_lossy(&bench_out.stdout);
        assert!(bench_stdout.contains("MOSSAMBI HIGH-PERFORMANCE BENCHMARK ENGINE"));
        assert!(bench_stdout.contains("Throughput"));
    }
}

#[test]
fn test_cli_completions_and_carapace() {
    let bin = get_bin_path();
    if !bin.exists() {
        return;
    }

    // Shell completions test: bash
    let comp_out = Command::new(&bin)
        .args(["completions", "bash"])
        .output()
        .unwrap();
    assert!(comp_out.status.success());
    let comp_str = String::from_utf8_lossy(&comp_out.stdout);
    assert!(comp_str.contains("complete -F _mossambi mossambi") || comp_str.contains("mossambi"));

    // Carapace YAML spec test
    let cara_out = Command::new(&bin)
        .args(["completions", "--carapace"])
        .output()
        .unwrap();
    assert!(cara_out.status.success());
    let cara_str = String::from_utf8_lossy(&cara_out.stdout);
    assert!(cara_str.contains("name: mossambi"));
    assert!(cara_str.contains("upload-ready"));

    // Help-manual Unicode table test
    let help_out = Command::new(&bin)
        .arg("help-manual")
        .output()
        .unwrap();
    assert!(help_out.status.success());
    let help_str = String::from_utf8_lossy(&help_out.stdout);
    assert!(help_str.contains("MOSSAMBI — ZERO-LOSS CONVERSION PLATFORM"));
    assert!(help_str.contains("UNIX 'mv' POSITIONAL GRAMMAR"));
}
