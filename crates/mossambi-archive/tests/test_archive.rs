use mossambi_archive::{extract_archive_safe, pack_archive, ArchiveType};
use std::fs;

#[test]
fn test_zip_pack_and_safe_unpack() {
    let test_dir = std::env::temp_dir().join("mossambi_test_archive_zip");
    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(&test_dir).unwrap();

    let sample_file = test_dir.join("sample.txt");
    fs::write(&sample_file, b"Mossambi zero-loss 100% test payload").unwrap();

    let zip_file = test_dir.join("bundle.zip");
    pack_archive(&[sample_file], &zip_file, ArchiveType::Zip).unwrap();
    assert!(zip_file.exists());

    let extract_dir = test_dir.join("extracted");
    extract_archive_safe(&zip_file, &extract_dir).unwrap();

    let extracted_sample = extract_dir.join("sample.txt");
    assert!(extracted_sample.exists());
    let read_back = fs::read(&extracted_sample).unwrap();
    assert_eq!(read_back, b"Mossambi zero-loss 100% test payload");

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_tar_gz_pack_and_safe_unpack() {
    let test_dir = std::env::temp_dir().join("mossambi_test_archive_targz");
    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(&test_dir).unwrap();

    let sample_file = test_dir.join("config.toml");
    fs::write(&sample_file, b"version = \"2.1.0\"\nplatform = \"all\"\n").unwrap();

    let tar_gz_file = test_dir.join("bundle.tar.gz");
    pack_archive(&[sample_file], &tar_gz_file, ArchiveType::TarGz).unwrap();
    assert!(tar_gz_file.exists());

    let extract_dir = test_dir.join("extracted");
    extract_archive_safe(&tar_gz_file, &extract_dir).unwrap();

    let extracted_sample = extract_dir.join("config.toml");
    assert!(extracted_sample.exists());
    let read_back = fs::read_to_string(&extracted_sample).unwrap();
    assert!(read_back.contains("version = \"2.1.0\""));

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_tar_zst_pack_and_safe_unpack() {
    let test_dir = std::env::temp_dir().join("mossambi_test_archive_tarzst");
    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(&test_dir).unwrap();

    let sample_file = test_dir.join("data.bin");
    let payload = vec![0xABu8; 1024];
    fs::write(&sample_file, &payload).unwrap();

    let tar_zst_file = test_dir.join("bundle.tar.zst");
    pack_archive(&[sample_file], &tar_zst_file, ArchiveType::TarZst).unwrap();
    assert!(tar_zst_file.exists());

    let extract_dir = test_dir.join("extracted");
    extract_archive_safe(&tar_zst_file, &extract_dir).unwrap();

    let extracted_sample = extract_dir.join("data.bin");
    assert!(extracted_sample.exists());
    assert_eq!(fs::read(&extracted_sample).unwrap(), payload);

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_directory_recursive_pack_and_unpack() {
    let test_dir = std::env::temp_dir().join("mossambi_test_archive_dir");
    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(&test_dir).unwrap();

    let sub_dir = test_dir.join("source_folder").join("nested");
    fs::create_dir_all(&sub_dir).unwrap();
    fs::write(sub_dir.join("inner.txt"), b"Inner file content").unwrap();

    let zip_file = test_dir.join("archive.zip");
    pack_archive(&[test_dir.join("source_folder")], &zip_file, ArchiveType::Zip).unwrap();
    assert!(zip_file.exists());

    let extract_dir = test_dir.join("unpack_out");
    extract_archive_safe(&zip_file, &extract_dir).unwrap();

    // Verify nested file is unpacked
    let inner_unpacked = extract_dir.join("source_folder").join("nested").join("inner.txt");
    assert!(inner_unpacked.exists());
    assert_eq!(fs::read(&inner_unpacked).unwrap(), b"Inner file content");

    let _ = fs::remove_dir_all(&test_dir);
}
