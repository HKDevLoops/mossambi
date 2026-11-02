//! Mossambi Standalone Radial Fruit Widget Library
//! 100% frameless, topbarless, transparent circular desktop widget matching `Image reference.png`.

pub mod widget_geometry;
pub mod widget_app;

pub use widget_geometry::{RadialWidgetGeometry, WidgetState, WidgetTheme};
pub use widget_app::{run_widget, run_widget_with_args, MossambiWidgetApp};
