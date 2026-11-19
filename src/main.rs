//! Mossambi process entry point.
//!
//! All command-line parsing and dispatch lives in the `mossambi::cli` module.
//! This binary only forwards the process exit code produced by the CLI.

fn main() -> std::process::ExitCode {
    let code = mossambi::cli::run();
    std::process::ExitCode::from(code as u8)
}
