//! Mossambi Standalone Desktop Radial Fruit Widget Binary
//! Calls into mossambi_widget library.

use eframe::egui::Pos2;
use std::path::PathBuf;

fn main() -> Result<(), eframe::Error> {
    let args: Vec<String> = std::env::args().collect();
    let mut initial_pos: Option<Pos2> = None;
    let mut initial_file: Option<PathBuf> = None;

    let mut i = 1;
    while i < args.len() {
        if args[i] == "--pos" && i + 1 < args.len() {
            let parts: Vec<&str> = args[i + 1].split(',').collect();
            if parts.len() == 2 {
                if let (Ok(x), Ok(y)) = (parts[0].trim().parse::<f32>(), parts[1].trim().parse::<f32>()) {
                    initial_pos = Some(Pos2::new(x - 200.0, y - 200.0));
                }
            }
            i += 2;
        } else if args[i] == "--active-drag" {
            i += 1;
        } else if !args[i].starts_with('-') {
            let p = PathBuf::from(&args[i]);
            if p.exists() {
                initial_file = Some(p);
            }
            i += 1;
        } else {
            i += 1;
        }
    }

    mossambi_widget::run_widget_with_args(initial_pos, initial_file)
}
