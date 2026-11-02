use mossambi_core::{get_system_drives, parse_breadcrumbs, FileItem};
use std::path::{Path, PathBuf};

#[test]
fn test_get_system_drives_non_empty() {
    let drives = get_system_drives();
    assert!(!drives.is_empty(), "System drives list must not be empty");
    for d in &drives {
        assert!(d.letter != ' ' || d.letter != '\0');
        assert!(!d.display_name.is_empty());
    }
}

#[test]
fn test_parse_breadcrumbs_windows_and_unix() {
    let p = Path::new("Z:/Projects/mossambi/crates");
    let breadcrumbs = parse_breadcrumbs(p);
    assert!(breadcrumbs.len() >= 3);
    assert_eq!(breadcrumbs[0].label, "Z:");
    assert_eq!(breadcrumbs[1].label, "Projects");
    assert_eq!(breadcrumbs[2].label, "mossambi");
}

#[test]
fn test_file_item_formatted_size() {
    let mut item = FileItem::from_path(PathBuf::from("test.txt"));
    item.is_dir = true;
    assert_eq!(item.formatted_size(), "Folder");

    item.is_dir = false;
    item.size_bytes = 500;
    assert_eq!(item.formatted_size(), "500 B");

    item.size_bytes = 2048;
    assert_eq!(item.formatted_size(), "2.0 KB");

    item.size_bytes = 5 * 1024 * 1024;
    assert_eq!(item.formatted_size(), "5.0 MB");
}

#[test]
fn test_linux_desktop_entry_generation() {
    let exec = Path::new("/usr/bin/mossambi-gui");
    let icon = Path::new("/usr/share/icons/mossambi.png");
    let entry = mossambi_core::linux_dna::generate_xdg_desktop_entry(exec, icon);
    assert!(entry.contains("[Desktop Entry]"));
    assert!(entry.contains("Name=Mossambi"));
    assert!(entry.contains("StartupWMClass=mossambi-gui"));
    assert!(entry.contains("Actions=OpenStudio;SummonWidget;InspectQuality;"));
}
