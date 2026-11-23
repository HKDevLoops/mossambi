use mossambi::codec::{BenchmarkEngine, FileQualityChecker, QualityVerdict};
use std::fs;
use std::path::PathBuf;

fn create_valid_wav(path: &std::path::Path) {
    let sample_rate: u32 = 44100;
    let num_samples: usize = 44100; // 1 second of audio
    let mut data = Vec::with_capacity(num_samples * 2);

    for i in 0..num_samples {
        // 440 Hz Sine wave
        let t = i as f64 / sample_rate as f64;
        let sample = (f64::sin(t * 440.0 * 2.0 * std::f64::consts::PI) * 16000.0) as i16;
        data.extend_from_slice(&sample.to_le_bytes());
    }

    let mut wav = Vec::new();
    wav.extend_from_slice(b"RIFF");
    let file_size: u32 = 36 + data.len() as u32;
    wav.extend_from_slice(&file_size.to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes()); // subchunk1 size
    wav.extend_from_slice(&1u16.to_le_bytes()); // PCM format
    wav.extend_from_slice(&1u16.to_le_bytes()); // Mono channel
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    let byte_rate = sample_rate * 2;
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes()); // block align
    wav.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&(data.len() as u32).to_le_bytes());
    wav.extend_from_slice(&data);

    fs::write(path, wav).expect("Failed to write test WAV");
}

fn create_valid_png(path: &std::path::Path, width: u32, height: u32) {
    let mut img = image::RgbaImage::new(width, height);
    for x in 0..width {
        for y in 0..height {
            // Pantone 376 C (#78BE20)
            img.put_pixel(x, y, image::Rgba([120, 190, 32, 255]));
        }
    }
    img.save(path).expect("Failed to save test PNG");
}

#[test]
fn test_vault_generation_and_inspection() {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let vault_dir = workspace_root.join("test_vault");
    let audio_path = vault_dir.join("audio").join("sample_audio.wav");
    let image_path = vault_dir.join("image").join("sample_photo.png");
    let exam_photo_path = vault_dir.join("exam").join("sample_candidate_photo.png");
    let doc_path = vault_dir.join("document").join("sample_notes.txt");
    let archive_path = vault_dir.join("archive").join("sample_bundle.zip");

    // 1. Create reference binaries in test_vault
    if let Some(parent) = audio_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Some(parent) = image_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Some(parent) = exam_photo_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Some(parent) = doc_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Some(parent) = archive_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    create_valid_wav(&audio_path);
    create_valid_png(&image_path, 400, 400);
    create_valid_png(&exam_photo_path, 500, 300);
    fs::write(
        &doc_path,
        "Mossambi Universal File Explorer & Zero-Loss Conformer Notes",
    )
    .unwrap();

    // Create zip archive
    let sample_content = b"Mossambi Pure Rust Zero-Loss Archive Suite";
    let temp_txt = vault_dir.join("archive").join("temp_item.txt");
    fs::write(&temp_txt, sample_content).unwrap();
    let _ = mossambi::archive::pack_archive(
        std::slice::from_ref(&temp_txt),
        &archive_path,
        mossambi::archive::ArchiveType::Zip,
    );
    let _ = fs::remove_file(&temp_txt);

    // 2. Test FileQualityChecker on Audio (WAV)
    let audio_report =
        FileQualityChecker::inspect_file(&audio_path).expect("WAV inspection failed");
    assert_eq!(audio_report.verdict, QualityVerdict::Pass);
    assert!(audio_report.overall_score >= 95.0);
    assert!(audio_report.integrity.magic_bytes_valid);
    assert!(!audio_report.integrity.corruption_detected);
    assert!(audio_report.format.contains("WAV"));
    println!("{}", audio_report.format_terminal());

    // 3. Test FileQualityChecker on Image (PNG)
    let img_report = FileQualityChecker::inspect_file(&image_path).expect("PNG inspection failed");
    assert_eq!(img_report.verdict, QualityVerdict::Pass);
    assert!(img_report.overall_score >= 98.0);
    assert!(img_report.integrity.magic_bytes_valid);
    assert!(img_report.integrity.eof_marker_valid);
    assert!(img_report.parseability.can_parse);
    println!("{}", img_report.format_terminal());

    // 4. Test FileQualityChecker on Document (TXT)
    let doc_report = FileQualityChecker::inspect_file(&doc_path).expect("TXT inspection failed");
    assert_eq!(doc_report.verdict, QualityVerdict::Pass);
    assert!(doc_report.overall_score >= 90.0);
    assert!(doc_report.parseability.can_parse);
    println!("{}", doc_report.format_terminal());

    // 5. Test FileQualityChecker on Archive (ZIP)
    let zip_report =
        FileQualityChecker::inspect_file(&archive_path).expect("ZIP inspection failed");
    assert_eq!(zip_report.verdict, QualityVerdict::Pass);
    assert!(zip_report.integrity.magic_bytes_valid);
    assert!(zip_report.integrity.eof_marker_valid);
    println!("{}", zip_report.format_terminal());

    // 6-7. BenchmarkEngine tests require an external FFmpeg binary. CI installs
    // FFmpeg on every runner, but headless/local environments may not have it.
    // Degrade gracefully when FFmpeg is absent so the rest of the quality-check
    // assertions still validate on any platform.
    let ffmpeg_available = std::process::Command::new("ffmpeg")
        .arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if ffmpeg_available {
        let bench_audio = BenchmarkEngine::benchmark_file(&audio_path, Some("FLAC"))
            .expect("Benchmark WAV failed");
        assert!(bench_audio.duration_ms > 0.0);
        assert!(bench_audio.throughput_mb_per_sec > 0.0);
        assert_eq!(bench_audio.target_format, "FLAC");
        assert!(!bench_audio.comparisons.is_empty());
        println!("{}", bench_audio.format_terminal());

        let bench_img = BenchmarkEngine::benchmark_file(&image_path, Some("WEBP"))
            .expect("Benchmark PNG failed");
        assert!(bench_img.duration_ms > 0.0);
        assert_eq!(bench_img.target_format, "WEBP");
        println!("{}", bench_img.format_terminal());
    } else {
        eprintln!("FFmpeg not available; skipping BenchmarkEngine portion of vault test");
    }
}
