//! Drag and Drop (W2) Regression and Headless Acceptance Tests
//!
//! Enforces:
//! 1. Data object for 2 temp files exposes CF_HDROP with both paths.
//! 2. Badge text and ghost layout logic unit-tested.
//! 3. Drop handler copy into temp dir via file-op path works.
//!
//! Pure 7-bit ASCII.

use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

#[test]
fn test_badge_and_ghost_layout_logic() {
    use mossambi::core::config::DropActionPreference;
    use mossambi::core::drag_out::compute_drag_ghost_layout;

    // Single item, default move
    let layout1 =
        compute_drag_ghost_layout(1, false, false, false, DropActionPreference::SmartDefault);
    assert_eq!(layout1.card_count, 1);
    assert_eq!(layout1.card_offsets, vec![(0.0, 0.0)]);
    assert_eq!(layout1.badge_text, "Move");
    assert!(layout1.pill_width >= 24.0);
    assert_eq!(layout1.pill_height, 20.0);

    // Multi-item count badge (3 items)
    let layout3 =
        compute_drag_ghost_layout(3, false, false, false, DropActionPreference::SmartDefault);
    assert_eq!(layout3.card_count, 3);
    assert_eq!(
        layout3.card_offsets,
        vec![(0.0, 0.0), (-4.0, -4.0), (-8.0, -8.0)]
    );
    assert_eq!(layout3.badge_text, "3");

    // Max 3 card stack clamping
    let layout10 =
        compute_drag_ghost_layout(10, false, false, false, DropActionPreference::SmartDefault);
    assert_eq!(layout10.card_count, 3);
    assert_eq!(layout10.badge_text, "10");

    // Modifiers override count text:
    // Ctrl -> Copy
    let layout_ctrl =
        compute_drag_ghost_layout(5, true, false, false, DropActionPreference::SmartDefault);
    assert_eq!(layout_ctrl.badge_text, "Copy");

    // Shift -> Move
    let layout_shift =
        compute_drag_ghost_layout(5, false, true, false, DropActionPreference::AlwaysCopy);
    assert_eq!(layout_shift.badge_text, "Move");

    // Alt -> Link
    let layout_alt =
        compute_drag_ghost_layout(5, false, false, true, DropActionPreference::AlwaysCopy);
    assert_eq!(layout_alt.badge_text, "Link");

    // Ctrl + Shift -> Link
    let layout_ctrl_shift =
        compute_drag_ghost_layout(5, true, true, false, DropActionPreference::AlwaysCopy);
    assert_eq!(layout_ctrl_shift.badge_text, "Link");
}

#[test]
fn test_drop_copy_into_temp_dir_via_file_op_path() {
    let src_dir = tempdir().expect("src tempdir");
    let dest_dir = tempdir().expect("dest tempdir");

    let file_a = src_dir.path().join("doc_a.txt");
    let file_b = src_dir.path().join("doc_b.dat");

    let mut fa = File::create(&file_a).expect("create file_a");
    fa.write_all(b"Hello Mossambi Drag And Drop A")
        .expect("write a");

    let mut fb = File::create(&file_b).expect("create file_b");
    fb.write_all(b"Payload Data B 123456").expect("write b");

    // Test file-op path: copy_entry_recursive
    let target_a = dest_dir.path().join("doc_a.txt");
    let target_b = dest_dir.path().join("doc_b.dat");

    mossambi::core::fs_utils::copy_entry_recursive(&file_a, &target_a).expect("copy a");
    mossambi::core::fs_utils::copy_entry_recursive(&file_b, &target_b).expect("copy b");

    assert!(target_a.exists(), "target_a must exist");
    assert!(target_b.exists(), "target_b must exist");

    let content_a = std::fs::read(&target_a).expect("read target_a");
    let content_b = std::fs::read(&target_b).expect("read target_b");

    assert_eq!(content_a, b"Hello Mossambi Drag And Drop A");
    assert_eq!(content_b, b"Payload Data B 123456");
}

#[cfg(windows)]
#[test]
fn test_data_object_exposes_cf_hdrop_with_both_paths() {
    use mossambi::core::ole_drag_out::{create_files_data_object, extract_paths_from_hdrop};
    use windows::Win32::System::Com::{DVASPECT_CONTENT, FORMATETC, TYMED_HGLOBAL};
    use windows::Win32::System::Ole::{ReleaseStgMedium, CF_HDROP};

    let dir = tempdir().expect("tempdir");
    let f1 = dir.path().join("file1.txt");
    let f2 = dir.path().join("file2.bin");

    File::create(&f1)
        .expect("create f1")
        .write_all(b"f1")
        .expect("write f1");
    File::create(&f2)
        .expect("create f2")
        .write_all(b"f2")
        .expect("write f2");

    let paths = vec![f1.clone(), f2.clone()];
    let data_obj = create_files_data_object(&paths).expect("create_files_data_object");

    let fmt = FORMATETC {
        cfFormat: CF_HDROP.0,
        ptd: std::ptr::null_mut(),
        dwAspect: DVASPECT_CONTENT.0,
        lindex: -1,
        tymed: TYMED_HGLOBAL.0 as u32,
    };

    unsafe {
        let mut medium = data_obj.GetData(&fmt).expect("GetData CF_HDROP");
        let extracted = extract_paths_from_hdrop(medium.u.hGlobal);
        ReleaseStgMedium(&mut medium);

        assert_eq!(extracted.len(), 2, "Must extract 2 paths");
        assert_eq!(extracted[0].file_name(), f1.file_name());
        assert_eq!(extracted[1].file_name(), f2.file_name());
    }
}
