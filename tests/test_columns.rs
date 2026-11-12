//! Columns-wave unit tests: the DetailColumn model, ColumnLayout
//! persistence semantics, per-column filter predicates, and the
//! MossambiConfig column persistence roundtrip.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use chrono::NaiveDate;
use mossambi::core::file_icons::resolve_file_icon;
use mossambi::core::fs_utils::FileItem;
use mossambi::core::MossambiConfig;
use mossambi::gui::explorer_view::{
    cmp_opt_last, date_in_bucket, matches_column_filter, ColumnFilterValue, ColumnLayout,
    DateBucket, DetailColumn, TriState,
};

/// Fixed calendar date (from_ymd_opt unwrap is safe for literal dates).
fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).expect("valid fixed date")
}

/// Minimal FileItem via struct literal (from_path would touch the disk).
fn item(name: &str, is_dir: bool, size: u64, hidden: bool) -> FileItem {
    FileItem {
        path: PathBuf::from(name),
        name: name.to_string(),
        is_dir,
        size_bytes: size,
        modified: None,
        created: None,
        extension: String::new(),
        is_hidden: hidden,
        is_symlink: false,
        symlink_target: None,
        permissions_mode: 0,
    }
}

#[test]
fn test_column_ids_labels_and_geometry() {
    let all = DetailColumn::all();
    assert_eq!(all.len(), 11);
    for (i, col) in all.iter().enumerate() {
        assert_eq!(col.id(), i as u8, "column ids must be dense 0..=10");
        let label = col.label();
        assert!(!label.is_empty());
        assert!(label.is_ascii(), "labels must be ASCII");
        assert!(
            col.min_w() <= col.default_w(),
            "min width must not exceed default"
        );
    }
    assert_eq!(all[10].label(), "Path");
}

#[test]
fn test_from_id_bounds() {
    assert_eq!(DetailColumn::from_id(0), Some(DetailColumn::Name));
    assert_eq!(DetailColumn::from_id(3), Some(DetailColumn::Kind));
    assert_eq!(DetailColumn::from_id(10), Some(DetailColumn::Path));
    assert_eq!(DetailColumn::from_id(11), None);
    assert_eq!(DetailColumn::from_id(255), None);
}

#[test]
fn test_default_visible_and_alignment() {
    assert_eq!(DetailColumn::default_visible_ids(), [0, 1, 2, 3]);
    for col in DetailColumn::all().iter().copied() {
        assert_eq!(col.aligns_right(), col == DetailColumn::Size);
    }
}

#[test]
fn test_sort_mapping_injective() {
    let mapped: Vec<u8> = DetailColumn::all()
        .iter()
        .map(|c| c.sort_column() as u8)
        .collect();
    let mut sorted = mapped.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(
        sorted.len(),
        11,
        "every column must drive a distinct sort axis"
    );
}

#[test]
fn test_cmp_opt_last_none_last() {
    use std::cmp::Ordering;
    assert_eq!(cmp_opt_last(&Some(1u8), &Some(2)), Ordering::Less);
    assert_eq!(cmp_opt_last(&Some(2u8), &Some(1)), Ordering::Greater);
    assert_eq!(cmp_opt_last(&None::<u8>, &Some(1)), Ordering::Greater);
    assert_eq!(cmp_opt_last(&Some(1u8), &None), Ordering::Less);
    assert_eq!(cmp_opt_last(&None::<u8>, &None), Ordering::Equal);
}

#[test]
fn test_date_in_bucket_fixed_today() {
    let today = date(2026, 11, 12);
    assert!(date_in_bucket(today, today, DateBucket::Today));
    assert!(!date_in_bucket(
        date(2026, 11, 11),
        today,
        DateBucket::Today
    ));

    // Rolling 7-day window: today-6 is inside, today-7 is outside.
    assert!(date_in_bucket(
        date(2026, 11, 6),
        today,
        DateBucket::ThisWeek
    ));
    assert!(!date_in_bucket(
        date(2026, 11, 5),
        today,
        DateBucket::ThisWeek
    ));

    assert!(date_in_bucket(
        date(2026, 11, 1),
        today,
        DateBucket::ThisMonth
    ));
    assert!(!date_in_bucket(
        date(2026, 10, 31),
        today,
        DateBucket::ThisMonth
    ));

    assert!(date_in_bucket(date(2026, 10, 31), today, DateBucket::Older));
    assert!(!date_in_bucket(date(2026, 11, 1), today, DateBucket::Older));
    assert!(!date_in_bucket(today, today, DateBucket::Older));

    for d in [today, date(2025, 1, 1)].iter().copied() {
        assert!(date_in_bucket(d, today, DateBucket::Any));
    }
}

#[test]
fn test_filter_text_case_insensitive_and_empty() {
    let mut it = item("ReportQ4.pdf", false, 12, false);
    it.extension = "PDF".to_string();
    it.path = PathBuf::from("Z:/docs/ReportQ4.pdf");

    for col in [
        DetailColumn::Name,
        DetailColumn::Extension,
        DetailColumn::Path,
    ]
    .iter()
    .copied()
    {
        assert!(matches_column_filter(
            &it,
            col,
            &ColumnFilterValue::Text(String::new())
        ));
        assert!(matches_column_filter(
            &it,
            col,
            &ColumnFilterValue::Text("   ".to_string())
        ));
    }
    assert!(matches_column_filter(
        &it,
        DetailColumn::Name,
        &ColumnFilterValue::Text("report".to_string())
    ));
    assert!(!matches_column_filter(
        &it,
        DetailColumn::Name,
        &ColumnFilterValue::Text("zzz".to_string())
    ));
    assert!(matches_column_filter(
        &it,
        DetailColumn::Extension,
        &ColumnFilterValue::Text("pdf".to_string())
    ));
    assert!(matches_column_filter(
        &it,
        DetailColumn::Path,
        &ColumnFilterValue::Text("DOCS".to_string())
    ));
}

#[test]
fn test_filter_kind_matches_derived_category() {
    let mut it = item("photo.jpg", false, 9, false);
    it.extension = "JPG".to_string();
    let cat = resolve_file_icon(&it.name, &it.extension, it.is_dir).category;
    let f = ColumnFilterValue::Text(cat.to_string());
    assert!(matches_column_filter(&it, DetailColumn::Kind, &f));

    let dir = item("Documents", true, 0, false);
    let dir_cat = resolve_file_icon(&dir.name, &dir.extension, dir.is_dir).category;
    assert_ne!(cat, dir_cat, "file and folder categories must differ");
    assert!(!matches_column_filter(
        &it,
        DetailColumn::Kind,
        &ColumnFilterValue::Text(dir_cat.to_string())
    ));
}

#[test]
fn test_filter_tri_state() {
    let hidden = item("secret.txt", false, 3, true);
    let plain = item("normal.txt", false, 4, false);
    for (tri, expected) in [
        (TriState::Yes, true),
        (TriState::No, false),
        (TriState::Any, true),
    ]
    .iter()
    .copied()
    {
        assert_eq!(
            matches_column_filter(
                &hidden,
                DetailColumn::Hidden,
                &ColumnFilterValue::TriState(tri)
            ),
            expected
        );
    }
    assert!(matches_column_filter(
        &plain,
        DetailColumn::Hidden,
        &ColumnFilterValue::TriState(TriState::No)
    ));
    assert!(!matches_column_filter(
        &plain,
        DetailColumn::Hidden,
        &ColumnFilterValue::TriState(TriState::Yes)
    ));

    let mut link = item("alias", false, 0, false);
    link.is_symlink = true;
    assert!(matches_column_filter(
        &link,
        DetailColumn::Symlink,
        &ColumnFilterValue::TriState(TriState::Yes)
    ));
    assert!(!matches_column_filter(
        &plain,
        DetailColumn::Symlink,
        &ColumnFilterValue::TriState(TriState::Yes)
    ));
}

#[test]
fn test_filter_size_bounds_files_only() {
    let file = item("big.bin", false, 100, false);
    assert!(matches_column_filter(
        &file,
        DetailColumn::Size,
        &ColumnFilterValue::SizeMin(99)
    ));
    assert!(matches_column_filter(
        &file,
        DetailColumn::Size,
        &ColumnFilterValue::SizeMin(100)
    ));
    assert!(!matches_column_filter(
        &file,
        DetailColumn::Size,
        &ColumnFilterValue::SizeMin(101)
    ));
    assert!(matches_column_filter(
        &file,
        DetailColumn::Size,
        &ColumnFilterValue::SizeMax(100)
    ));
    assert!(!matches_column_filter(
        &file,
        DetailColumn::Size,
        &ColumnFilterValue::SizeMax(99)
    ));

    // Size bounds are files-only: directories are never excluded/included.
    let dir = item("Folder", true, 0, false);
    assert!(!matches_column_filter(
        &dir,
        DetailColumn::Size,
        &ColumnFilterValue::SizeMin(1)
    ));
    assert!(!matches_column_filter(
        &dir,
        DetailColumn::Size,
        &ColumnFilterValue::SizeMax(1000)
    ));
}

#[test]
fn test_filter_date_buckets() {
    let two_days_ago = SystemTime::now() - Duration::from_secs(48 * 3600);

    let mut it = item("recent.txt", false, 1, false);
    it.modified = Some(two_days_ago);
    assert!(matches_column_filter(
        &it,
        DetailColumn::DateModified,
        &ColumnFilterValue::Date(DateBucket::ThisWeek)
    ));
    assert!(!matches_column_filter(
        &it,
        DetailColumn::DateModified,
        &ColumnFilterValue::Date(DateBucket::Today)
    ));
    assert!(!matches_column_filter(
        &it,
        DetailColumn::DateModified,
        &ColumnFilterValue::Date(DateBucket::Older)
    ));

    let mut c = item("made.txt", false, 1, false);
    c.created = Some(two_days_ago);
    assert!(matches_column_filter(
        &c,
        DetailColumn::DateCreated,
        &ColumnFilterValue::Date(DateBucket::ThisWeek)
    ));

    // Missing dates: concrete buckets exclude the item, Any includes it.
    let bare = item("bare.txt", false, 1, false);
    assert!(!matches_column_filter(
        &bare,
        DetailColumn::DateModified,
        &ColumnFilterValue::Date(DateBucket::ThisWeek)
    ));
    assert!(matches_column_filter(
        &bare,
        DetailColumn::DateModified,
        &ColumnFilterValue::Date(DateBucket::Any)
    ));
    assert!(!matches_column_filter(
        &bare,
        DetailColumn::DateCreated,
        &ColumnFilterValue::Date(DateBucket::ThisMonth)
    ));
}

#[test]
fn test_layout_finder_defaults() {
    let layout = ColumnLayout::finder_default();
    assert_eq!(
        layout.visible,
        vec![
            DetailColumn::Name,
            DetailColumn::DateModified,
            DetailColumn::Size,
            DetailColumn::Kind,
        ]
    );
    assert_eq!(
        layout.widths.len(),
        11,
        "finder default carries a width per column"
    );
    assert_eq!(layout.width(DetailColumn::Kind), 140.0);

    let fallback: ColumnLayout = Default::default();
    assert_eq!(fallback.visible, layout.visible);
    assert_eq!(fallback.widths, layout.widths);

    // A hand-built layout with no widths falls back to per-column defaults.
    let empty = ColumnLayout {
        visible: Vec::new(),
        widths: BTreeMap::new(),
    };
    assert_eq!(empty.width(DetailColumn::Name), 260.0);
    assert_eq!(empty.width(DetailColumn::Path), 240.0);
}

#[test]
fn test_layout_show_hide_lock_clamp() {
    let mut layout = ColumnLayout::finder_default();

    assert!(ColumnLayout::is_locked(DetailColumn::Name));
    assert!(!ColumnLayout::is_locked(DetailColumn::Size));
    layout.hide(DetailColumn::Name);
    assert!(
        layout.visible.contains(&DetailColumn::Name),
        "locked Name cannot be hidden"
    );

    layout.hide(DetailColumn::Size);
    assert!(!layout.visible.contains(&DetailColumn::Size));
    layout.show(DetailColumn::Size);
    layout.show(DetailColumn::Size);
    assert_eq!(
        layout
            .visible
            .iter()
            .filter(|c| **c == DetailColumn::Size)
            .count(),
        1,
        "show must not duplicate an existing column"
    );

    layout.set_width(DetailColumn::Path, 1.0);
    assert_eq!(
        layout.width(DetailColumn::Path),
        120.0,
        "width clamps to min"
    );
    layout.set_width(DetailColumn::Size, 500.0);
    assert_eq!(layout.width(DetailColumn::Size), 500.0);
}

#[test]
fn test_layout_config_roundtrip() {
    let mut layout = ColumnLayout::finder_default();
    layout.hide(DetailColumn::Size);
    layout.show(DetailColumn::Path);
    layout.set_width(DetailColumn::DateModified, 200.0);

    let (cols, widths) = layout.to_config();
    let rebuilt = ColumnLayout::from_config(&cols, &widths);
    assert_eq!(rebuilt.visible, layout.visible);
    assert_eq!(rebuilt.widths, layout.widths);
}

#[test]
fn test_layout_from_config_legacy_and_unknown() {
    // Empty persisted values fall back to the Finder defaults.
    let empty = ColumnLayout::from_config(&[], &BTreeMap::new());
    assert_eq!(empty.visible, ColumnLayout::finder_default().visible);

    // Unknown id 99 is dropped; Name is re-locked first; widths apply.
    let mut widths = BTreeMap::new();
    widths.insert(2u8, 150u32);
    widths.insert(99u8, 999u32);
    let layout = ColumnLayout::from_config(&[2, 7, 0, 99], &widths);
    assert_eq!(
        layout.visible,
        vec![
            DetailColumn::Name,
            DetailColumn::Size,
            DetailColumn::Symlink
        ]
    );
    assert_eq!(layout.width(DetailColumn::Size), 150.0);
    assert_eq!(
        layout.width(DetailColumn::Symlink),
        DetailColumn::Symlink.default_w()
    );
}

#[test]
fn test_config_defaults_and_serde_roundtrip() {
    let cfg = MossambiConfig::default();
    assert_eq!(cfg.explorer_columns, vec![0, 1, 2, 3]);
    assert!(cfg.explorer_column_widths.is_empty());

    let mut custom = MossambiConfig {
        explorer_columns: vec![0, 1, 2, 3, 9],
        ..MossambiConfig::default()
    };
    custom.explorer_column_widths.insert(1, 180);
    let json = serde_json::to_string(&custom).expect("config must serialize");
    let back: MossambiConfig = serde_json::from_str(&json).expect("config must deserialize");
    assert_eq!(back.explorer_columns, vec![0, 1, 2, 3, 9]);
    assert_eq!(back.explorer_column_widths.get(&1), Some(&180));

    // A legacy config without the column keys must load Finder defaults.
    let mut value = serde_json::to_value(&custom).expect("config must serialize to value");
    let obj = value.as_object_mut().expect("config must be a JSON object");
    obj.remove("explorer_columns");
    obj.remove("explorer_column_widths");
    let legacy: MossambiConfig = serde_json::from_value(value).expect("legacy config must load");
    assert_eq!(legacy.explorer_columns, vec![0, 1, 2, 3]);
    assert!(legacy.explorer_column_widths.is_empty());
}
