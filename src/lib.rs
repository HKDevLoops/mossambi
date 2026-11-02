//! Mossambi: Native High-Performance Universal File Explorer, Zero-Loss File Converter,
//! Archive Suite, and Indian Exam Form Tuner.

pub mod archive;
pub mod codec;
pub mod core;
pub mod daemon;
pub mod exam;
pub mod gui;
pub mod hwaccel;
pub mod widget;

// Module aliases for seamless internal resolution
pub use archive as mossambi_archive;
pub use codec as mossambi_codec;
pub use core as mossambi_core;
pub use exam as mossambi_exam;
pub use gui as mossambi_gui;
pub use hwaccel as mossambi_hwaccel;
pub use widget as mossambi_widget;
