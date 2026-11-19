//! Integration Tests for Settings & State Persistence (M1 / R3)
//!
//! Validates:
//! 1. `apply_look_preset()` sets both `view_mode` and `default_explorer_layout` for all presets.
//! 2. Serde round-trip JSON serialization preserves `view_mode`, `row_density`, `theme_mode`, `look_preset`, and `design_style`.
//! 3. Backward compatibility: Deserializing a config without `view_mode` defaults cleanly and migrates `default_explorer_layout`.
//! 4. Config file reading and writing at the reconciled path and corruption resilience.

use mossambi::core::config::{
    DesignStyle, ExplorerStyle, LookPreset, MossambiConfig, RowDensity, SidebarStyle, ThemeMode,
    ViewMode,
};
use std::fs;
use tempfile::tempdir;

#[test]
fn test_look_presets_set_view_mode_and_layout() {
    let mut cfg = MossambiConfig::default();

    // 1. Finder Preset -> Columns (3)
    cfg.apply_look_preset(LookPreset::Finder);
    assert_eq!(cfg.view_mode, ViewMode::Columns);
    assert_eq!(cfg.default_explorer_layout, 3);
    assert_eq!(cfg.sidebar_style, SidebarStyle::Finder);
    assert_eq!(cfg.explorer_style, ExplorerStyle::MacOS);
    assert_eq!(cfg.row_density, RowDensity::Default);
    assert_eq!(cfg.icon_pack, "macos");
    assert_eq!(cfg.theme_mode, ThemeMode::Rangalipi);

    // 2. Windows Explorer Preset -> Details (0)
    cfg.apply_look_preset(LookPreset::WindowsExplorer);
    assert_eq!(cfg.view_mode, ViewMode::Details);
    assert_eq!(cfg.default_explorer_layout, 0);
    assert_eq!(cfg.sidebar_style, SidebarStyle::Windows);
    assert_eq!(cfg.explorer_style, ExplorerStyle::Windows);
    assert_eq!(cfg.row_density, RowDensity::Standard);
    assert_eq!(cfg.icon_pack, "win11");
    assert_eq!(cfg.theme_mode, ThemeMode::DarkPlus);

    // 3. Nautilus Preset -> Grid (2)
    cfg.apply_look_preset(LookPreset::Nautilus);
    assert_eq!(cfg.view_mode, ViewMode::Grid);
    assert_eq!(cfg.default_explorer_layout, 2);
    assert_eq!(cfg.sidebar_style, SidebarStyle::Nautilus);
    assert_eq!(cfg.explorer_style, ExplorerStyle::Nautilus);
    assert_eq!(cfg.row_density, RowDensity::Spacious);
    assert_eq!(cfg.icon_pack, "adwaita");
    assert_eq!(cfg.theme_mode, ThemeMode::TokyoNight);

    // 4. Thunar Preset -> Details (0)
    cfg.apply_look_preset(LookPreset::Thunar);
    assert_eq!(cfg.view_mode, ViewMode::Details);
    assert_eq!(cfg.default_explorer_layout, 0);
    assert_eq!(cfg.sidebar_style, SidebarStyle::Thunar);
    assert_eq!(cfg.explorer_style, ExplorerStyle::Thunar);
    assert_eq!(cfg.row_density, RowDensity::Compact);
    assert_eq!(cfg.icon_pack, "cinnamon");
    assert_eq!(cfg.theme_mode, ThemeMode::Kanagawa);
}

#[test]
fn test_serde_roundtrip_preserves_all_core_settings() {
    let test_modes = [ViewMode::Details, ViewMode::Grid, ViewMode::Columns];
    let test_densities = [
        RowDensity::Compact,
        RowDensity::Default,
        RowDensity::Standard,
        RowDensity::Spacious,
    ];
    let test_styles = [
        DesignStyle::ArcaneFluid,
        DesignStyle::ZenithPro,
        DesignStyle::HyperionSlate,
    ];
    let test_presets = [
        Some(LookPreset::Finder),
        Some(LookPreset::WindowsExplorer),
        Some(LookPreset::Nautilus),
        Some(LookPreset::Thunar),
        None,
    ];
    let test_themes = [
        ThemeMode::DarkPlus,
        ThemeMode::Rangalipi,
        ThemeMode::TokyoNight,
        ThemeMode::Kanagawa,
        ThemeMode::Gruvbox,
    ];

    for &view_mode in &test_modes {
        for &row_density in &test_densities {
            for &design_style in &test_styles {
                for &look_preset in &test_presets {
                    for &theme_mode in &test_themes {
                        let cfg = MossambiConfig {
                            view_mode,
                            row_density,
                            design_style,
                            look_preset,
                            theme_mode,
                            ..Default::default()
                        };

                        let serialized = serde_json::to_string(&cfg)
                            .expect("MossambiConfig must serialize to JSON");
                        let deserialized: MossambiConfig = serde_json::from_str(&serialized)
                            .expect("MossambiConfig must deserialize from JSON");

                        assert_eq!(deserialized.view_mode, view_mode);
                        assert_eq!(deserialized.row_density, row_density);
                        assert_eq!(deserialized.design_style, design_style);
                        assert_eq!(deserialized.look_preset, look_preset);
                        assert_eq!(deserialized.theme_mode, theme_mode);
                    }
                }
            }
        }
    }
}

#[test]
fn test_view_mode_serde_token_representations() {
    assert_eq!(
        serde_json::to_string(&ViewMode::Details).unwrap(),
        "\"details\""
    );
    assert_eq!(serde_json::to_string(&ViewMode::Grid).unwrap(), "\"grid\"");
    assert_eq!(
        serde_json::to_string(&ViewMode::Columns).unwrap(),
        "\"columns\""
    );

    let parsed_details: ViewMode = serde_json::from_str("\"details\"").unwrap();
    assert_eq!(parsed_details, ViewMode::Details);
    let parsed_grid: ViewMode = serde_json::from_str("\"grid\"").unwrap();
    assert_eq!(parsed_grid, ViewMode::Grid);
    let parsed_columns: ViewMode = serde_json::from_str("\"columns\"").unwrap();
    assert_eq!(parsed_columns, ViewMode::Columns);
}

#[test]
fn test_backward_compatibility_missing_view_mode() {
    // 1. Deserializing legacy JSON with default_explorer_layout = 2 (Grid) and missing view_mode
    let mut val = serde_json::to_value(MossambiConfig::default()).unwrap();
    let obj = val.as_object_mut().unwrap();
    obj.remove("view_mode");
    obj.insert("default_explorer_layout".to_string(), serde_json::json!(2));
    obj.insert("prefs_version".to_string(), serde_json::json!(1));

    let json_grid = serde_json::to_string(&val).unwrap();
    assert!(!json_grid.contains("\"view_mode\""));

    let mut cfg_grid: MossambiConfig =
        serde_json::from_str(&json_grid).expect("must parse legacy grid json");
    assert_eq!(cfg_grid.view_mode, ViewMode::Details); // serde default before migration
    assert!(mossambi::core::config::migrate_prefs(&mut cfg_grid));
    assert_eq!(cfg_grid.view_mode, ViewMode::Grid);
    assert_eq!(cfg_grid.prefs_version, 2);

    // 2. Deserializing legacy JSON with default_explorer_layout = 3 (Columns) and missing view_mode
    let mut val = serde_json::to_value(MossambiConfig::default()).unwrap();
    let obj = val.as_object_mut().unwrap();
    obj.remove("view_mode");
    obj.insert("default_explorer_layout".to_string(), serde_json::json!(3));
    obj.insert("prefs_version".to_string(), serde_json::json!(1));

    let json_columns = serde_json::to_string(&val).unwrap();
    assert!(!json_columns.contains("\"view_mode\""));

    let mut cfg_columns: MossambiConfig =
        serde_json::from_str(&json_columns).expect("must parse legacy columns json");
    assert_eq!(cfg_columns.view_mode, ViewMode::Details);
    assert!(mossambi::core::config::migrate_prefs(&mut cfg_columns));
    assert_eq!(cfg_columns.view_mode, ViewMode::Columns);
    assert_eq!(cfg_columns.prefs_version, 2);

    // 3. Deserializing legacy JSON with default_explorer_layout = 0 (Details) and missing view_mode
    let mut val = serde_json::to_value(MossambiConfig::default()).unwrap();
    let obj = val.as_object_mut().unwrap();
    obj.remove("view_mode");
    obj.insert("default_explorer_layout".to_string(), serde_json::json!(0));
    obj.insert("prefs_version".to_string(), serde_json::json!(1));

    let json_details = serde_json::to_string(&val).unwrap();
    assert!(!json_details.contains("\"view_mode\""));

    let mut cfg_details: MossambiConfig =
        serde_json::from_str(&json_details).expect("must parse legacy details json");
    assert_eq!(cfg_details.view_mode, ViewMode::Details);
    assert!(mossambi::core::config::migrate_prefs(&mut cfg_details));
    assert_eq!(cfg_details.view_mode, ViewMode::Details);
    assert_eq!(cfg_details.prefs_version, 2);
}

#[test]
fn test_config_disk_io_roundtrip_and_corruption_resilience() {
    let tmp = tempdir().expect("tempdir creation");
    let test_cfg_path = tmp.path().join("config.json");

    // 1. Write custom config
    let custom = MossambiConfig {
        view_mode: ViewMode::Grid,
        row_density: RowDensity::Spacious,
        theme_mode: ThemeMode::TokyoNight,
        design_style: DesignStyle::ZenithPro,
        ..Default::default()
    };

    let serialized = serde_json::to_string_pretty(&custom).unwrap();
    fs::write(&test_cfg_path, &serialized).unwrap();

    // 2. Read back from disk
    let content = fs::read_to_string(&test_cfg_path).unwrap();
    let loaded: MossambiConfig = serde_json::from_str(&content).unwrap();
    assert_eq!(loaded.view_mode, ViewMode::Grid);
    assert_eq!(loaded.row_density, RowDensity::Spacious);
    assert_eq!(loaded.theme_mode, ThemeMode::TokyoNight);
    assert_eq!(loaded.design_style, DesignStyle::ZenithPro);

    // 3. Corrupt config handling (fallback and safety)
    let corrupt_content = "{ \"view_mode\": \"invalid_mode_syntax\" ... ";
    fs::write(&test_cfg_path, corrupt_content).unwrap();

    let parse_result =
        serde_json::from_str::<MossambiConfig>(&fs::read_to_string(&test_cfg_path).unwrap());
    assert!(
        parse_result.is_err(),
        "Corrupted JSON must produce parse error"
    );
}

#[test]
fn test_canonical_and_legacy_paths_reconciled() {
    let canonical = MossambiConfig::config_path();
    assert!(
        canonical.ends_with(
            std::path::Path::new(".config")
                .join("mossambi")
                .join("config.json")
        ),
        "Canonical path {:?} must end with .config/mossambi/config.json",
        canonical
    );

    let legacy = MossambiConfig::legacy_config_path();
    assert!(
        legacy.ends_with(std::path::Path::new(".mossambi").join("config.json")),
        "Legacy path {:?} must end with .mossambi/config.json",
        legacy
    );
}
