#![allow(unused_imports, dead_code)]

//! Shared Test Harness Modules for Mossambi E2E Test Suite.
//!
//! Provides reusable test utilities:
//! - `gui_harness`: Headless egui Context, RawInput generators, frame runner.
//! - `fs_harness`: TempDir fixture generator, 10k item generator, symlinks, filename validation.
//! - `contrast_harness`: WCAG 2.1 AA/AAA luminance & contrast ratio calculation and assertion helpers.
//! - `config_harness`: Sandboxed config environment & JSON serializer fixtures.

pub mod config_harness;
pub mod contrast_harness;
pub mod fs_harness;
pub mod gui_harness;

pub use config_harness::IsolatedConfigEnv;
pub use contrast_harness::{
    assert_wcag_aa, assert_wcag_aa_large, assert_wcag_aaa, contrast_ratio, relative_luminance,
};
pub use fs_harness::TestFsTree;
pub use gui_harness::HeadlessTestApp;
