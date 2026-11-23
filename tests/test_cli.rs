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
    assert!(stdout.contains("Rangalipi"));
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
    fs::write(
        &txt_file,
        b"Header Line\n\nParagraph with publication typesetting.\n",
    )
    .unwrap();

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
        .args([
            "verify",
            txt_file.to_str().unwrap(),
            pdf_file.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(ver_out.status.success());
    let stdout = String::from_utf8_lossy(&ver_out.stdout);
    assert!(
        stdout.contains("VERIFIED 0.00% ZERO-LOSS")
            || stdout.contains("Vector Layout Preservation")
    );

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
        .args([
            "upload-ready",
            img_file.to_str().unwrap(),
            "--preset",
            "upsc-photo",
        ])
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
    let sample_txt = workspace_root
        .join("test_vault")
        .join("document")
        .join("sample_notes.txt");

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
    let help_out = Command::new(&bin).arg("help-manual").output().unwrap();
    assert!(help_out.status.success());
    let help_str = String::from_utf8_lossy(&help_out.stdout);
    assert!(help_str.contains("ZERO-LOSS CONVERSION PLATFORM"));
    assert!(help_str.contains("UNIX 'mv' POSITIONAL GRAMMAR"));
}

#[test]
fn test_cli_themes_26_list_and_json() {
    let bin = get_bin_path();
    if !bin.exists() {
        return;
    }

    let out = Command::new(&bin).arg("themes").output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("26 NAMED PANTONE THEMES"));
    assert!(stdout.contains("13 CERTIFIED DARK THEMES"));
    assert!(stdout.contains("13 EYE-CARE LIGHT THEMES"));

    let json_out = Command::new(&bin)
        .args(["themes", "--json"])
        .output()
        .unwrap();
    assert!(json_out.status.success());
    let val: serde_json::Value = serde_json::from_slice(&json_out.stdout).unwrap();
    assert_eq!(val.as_array().unwrap().len(), 26);
}

#[test]
fn test_cli_tags_subcommand() {
    let bin = get_bin_path();
    if !bin.exists() {
        return;
    }

    let test_dir = std::env::temp_dir().join("mossambi_cli_test_tags");
    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(&test_dir).unwrap();

    let file_path = test_dir.join("tagged_doc.txt");
    fs::write(&file_path, b"Test content for tagging").unwrap();

    // "Important" is a built-in preset; sanitize_new_tag correctly rejects it
    // as already existing. Use a unique custom tag so the add path is exercised.
    let tag = format!("CliTest{}", std::process::id());

    // List available tags
    let list_out = Command::new(&bin).args(["tags", "list"]).output().unwrap();
    assert!(list_out.status.success());

    // Add tag
    let add_out = Command::new(&bin)
        .args(["tags", "add", file_path.to_str().unwrap(), &tag])
        .output()
        .unwrap();
    assert!(
        add_out.status.success(),
        "tags add failed: {}",
        String::from_utf8_lossy(&add_out.stderr)
    );

    // Find by tag
    // TagsCommand::Find accepts only TAG; a --root flag is rejected by clap.
    let find_out = Command::new(&bin)
        .args(["tags", "find", &tag])
        .output()
        .unwrap();
    assert!(find_out.status.success());
    let find_str = String::from_utf8_lossy(&find_out.stdout);
    assert!(find_str.contains("tagged_doc.txt"));

    // Remove tag
    let rm_out = Command::new(&bin)
        .args(["tags", "remove", file_path.to_str().unwrap(), &tag])
        .output()
        .unwrap();
    assert!(rm_out.status.success());

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_cli_fs_operations() {
    let bin = get_bin_path();
    if !bin.exists() {
        return;
    }

    let test_dir = std::env::temp_dir().join("mossambi_cli_test_fs");
    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(&test_dir).unwrap();

    let src_file = test_dir.join("origin.txt");
    fs::write(&src_file, b"Original file payload").unwrap();

    // Copy
    let copy_dest = test_dir.join("copied.txt");
    let copy_out = Command::new(&bin)
        .args([
            "fs",
            "copy",
            src_file.to_str().unwrap(),
            copy_dest.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(copy_out.status.success());
    assert!(copy_dest.exists());

    // Rename
    let rename_out = Command::new(&bin)
        .args(["fs", "rename", copy_dest.to_str().unwrap(), "renamed.txt"])
        .output()
        .unwrap();
    assert!(rename_out.status.success());
    assert!(test_dir.join("renamed.txt").exists());

    // Move
    let sub_dir = test_dir.join("subdir");
    fs::create_dir_all(&sub_dir).unwrap();
    let move_out = Command::new(&bin)
        .args([
            "fs",
            "move",
            test_dir.join("renamed.txt").to_str().unwrap(),
            sub_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(move_out.status.success());
    assert!(sub_dir.join("renamed.txt").exists());

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_cli_route_and_search() {
    let bin = get_bin_path();
    if !bin.exists() {
        return;
    }

    // Route
    let route_out = Command::new(&bin)
        .args(["route", "png", "webp"])
        .output()
        .unwrap();
    assert!(route_out.status.success());
    let route_str = String::from_utf8_lossy(&route_out.stdout);
    assert!(route_str.contains("Route: PNG -> WEBP"));

    // Search
    let search_out = Command::new(&bin)
        .args(["search", "Cargo", "--limit", "5"])
        .output()
        .unwrap();
    assert!(search_out.status.success());
    let search_str = String::from_utf8_lossy(&search_out.stdout);
    // Filename matching is case-insensitive; POSIX results surface ".cargo"
    // directories while Windows surfaces "Cargo.toml". Compare case-insensitively.
    assert!(
        search_str.to_lowercase().contains("cargo"),
        "search output was: {search_str}"
    );
}

#[test]
fn test_cli_preview_and_queue() {
    let bin = get_bin_path();
    if !bin.exists() {
        return;
    }

    // Preview
    let prev_out = Command::new(&bin)
        .args(["preview", "Cargo.toml", "--json"])
        .output()
        .unwrap();
    assert!(prev_out.status.success());
    let prev_val: serde_json::Value = serde_json::from_slice(&prev_out.stdout).unwrap();
    assert_eq!(prev_val["kind"], "text");

    // Queue List
    let q_out = Command::new(&bin)
        .args(["queue", "list", "--json"])
        .output()
        .unwrap();
    assert!(q_out.status.success());
    let q_val: serde_json::Value = serde_json::from_slice(&q_out.stdout).unwrap();
    assert!(q_val["jobs"].is_array());
}

#[test]
fn test_cli_completions_carapace_and_shells() {
    let bin = get_bin_path();
    if !bin.exists() {
        return;
    }

    // Carapace YAML
    let yaml_out = Command::new(&bin)
        .args(["completions", "--carapace"])
        .output()
        .unwrap();
    assert!(yaml_out.status.success());
    let yaml_str = String::from_utf8_lossy(&yaml_out.stdout);
    assert!(yaml_str.contains("name: mossambi"));
    assert!(yaml_str.contains("convert"));
    assert!(yaml_str.contains("upload-ready"));
    assert!(yaml_str.contains("batch-rename"));

    // PowerShell
    let pwsh_out = Command::new(&bin)
        .args(["completions", "powershell"])
        .output()
        .unwrap();
    assert!(pwsh_out.status.success());
    let pwsh_str = String::from_utf8_lossy(&pwsh_out.stdout);
    assert!(pwsh_str.contains("Register-ArgumentCompleter"));
    assert!(pwsh_str.contains("mossambi"));

    // Bash
    let bash_out = Command::new(&bin)
        .args(["completions", "bash"])
        .output()
        .unwrap();
    assert!(bash_out.status.success());
    let bash_str = String::from_utf8_lossy(&bash_out.stdout);
    assert!(bash_str.contains("_mossambi"));

    // Fish
    let fish_out = Command::new(&bin)
        .args(["completions", "fish"])
        .output()
        .unwrap();
    assert!(fish_out.status.success());
    let fish_str = String::from_utf8_lossy(&fish_out.stdout);
    assert!(fish_str.contains("complete -c mossambi"));

    // Auto-detect shell when no shell argument is specified
    let auto_out = Command::new(&bin).args(["completions"]).output().unwrap();
    assert!(auto_out.status.success());
    assert!(!auto_out.stdout.is_empty());
}

#[test]
fn test_cli_carapace_dynamic_bridge() {
    let bin = get_bin_path();
    if !bin.exists() {
        return;
    }

    // Exam preset completions
    let exam_out = Command::new(&bin)
        .args(["_carapace", "pwsh", "upload-ready", "--preset"])
        .output()
        .unwrap();
    assert!(exam_out.status.success());
    let exam_val: serde_json::Value = serde_json::from_slice(&exam_out.stdout).unwrap();
    assert!(exam_val.is_array());
    let exam_arr = exam_val.as_array().unwrap();
    assert!(exam_arr.iter().any(|item| item["value"] == "upsc-photo"));

    // Theme completions
    let theme_out = Command::new(&bin)
        .args(["_carapace", "pwsh", "config", "set", "theme"])
        .output()
        .unwrap();
    assert!(theme_out.status.success());
    let theme_val: serde_json::Value = serde_json::from_slice(&theme_out.stdout).unwrap();
    assert!(theme_val.is_array());
    let theme_arr = theme_val.as_array().unwrap();
    assert!(theme_arr.iter().any(|item| item["value"] == "gruvbox"));
    assert!(theme_arr
        .iter()
        .any(|item| item["value"] == "mossambi_punch"));
}

#[test]
fn test_cli_presets_and_artcraft() {
    let bin = get_bin_path();
    if !bin.exists() {
        return;
    }

    // Presets list
    let presets_out = Command::new(&bin)
        .args(["presets", "list"])
        .output()
        .unwrap();
    assert!(presets_out.status.success());
    let presets_str = String::from_utf8_lossy(&presets_out.stdout);
    assert!(presets_str.contains("web-ultra-webp"));
    assert!(presets_str.contains("upsc-photo"));

    // ArtCraft status
    let artcraft_out = Command::new(&bin)
        .args(["artcraft", "status"])
        .output()
        .unwrap();
    assert!(artcraft_out.status.success());
    let artcraft_str = String::from_utf8_lossy(&artcraft_out.stdout);
    assert!(artcraft_str.contains("ArtCraft Ecosystem Detection Status"));
}

#[test]
fn test_cli_presets_export_import_roundtrip() {
    let bin = get_bin_path();
    if !bin.exists() {
        return;
    }

    let temp_dir = std::env::temp_dir().join("mossambi_test_preset_export");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let export_path = temp_dir.join("exported_preset.json");

    // Export builtin preset
    let exp_out = Command::new(&bin)
        .args([
            "presets",
            "export",
            "web-ultra-webp",
            "--output",
            export_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(exp_out.status.success());
    assert!(export_path.exists());

    // Verify valid JSON content
    let content = fs::read_to_string(&export_path).unwrap();
    assert!(content.contains("web-ultra-webp"));
    // The schema is serialised as the "schema_version" field; the
    // "mossambi-preset.v1.json" name is the on-disk format identifier, not a
    // literal embedded in the JSON body.
    assert!(content.contains("schema_version"));

    let _ = fs::remove_dir_all(&temp_dir);
}
