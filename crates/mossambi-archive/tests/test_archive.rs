use mossambi_archive::{extract_archive_safe, pack_archive, ArchiveType};
use std::fs;

#[test]
fn test_zip_pack_and_safe_unpack() {
    let test_dir = std::env::temp_dir().join("mossambi_test_archive");
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
