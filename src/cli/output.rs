//! Shared CLI output helpers: process exit codes and machine-readable JSON.
//! Pure ASCII by contract (strict-ASCII rule for all new CLI sources).

use serde::Serialize;

/// Process exit codes shared by every CLI command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ExitCode {
    /// Command completed successfully.
    Ok = 0,
    /// Generic runtime error.
    Error = 1,
    /// Invalid command-line usage (clap uses the same code).
    Usage = 2,
    /// A referenced path was missing or an I/O operation failed.
    NotFound = 3,
}

impl ExitCode {
    /// Return the numeric code suitable for `std::process::exit`.
    pub fn code(self) -> i32 {
        self as i32
    }
}

/// Print a value as pretty JSON (the global --json contract).
pub fn print_json<T: Serialize>(value: &T) -> anyhow::Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

/// Map an anyhow error to an exit code: missing paths / raw I/O errors
/// become NotFound, every other failure becomes a generic error.
pub fn exit_code_for_error(err: &anyhow::Error) -> ExitCode {
    for cause in err.chain() {
        if let Some(io) = cause.downcast_ref::<std::io::Error>() {
            if matches!(io.kind(), std::io::ErrorKind::NotFound) {
                return ExitCode::NotFound;
            }
        }
    }
    ExitCode::Error
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_codes_match_contract() {
        assert_eq!(ExitCode::Ok.code(), 0);
        assert_eq!(ExitCode::Error.code(), 1);
        assert_eq!(ExitCode::Usage.code(), 2);
        assert_eq!(ExitCode::NotFound.code(), 3);
    }

    #[test]
    fn not_found_io_error_maps_to_not_found() {
        let io = std::io::Error::new(std::io::ErrorKind::NotFound, "missing");
        let err = anyhow::Error::new(io);
        assert_eq!(exit_code_for_error(&err), ExitCode::NotFound);
    }

    #[test]
    fn generic_error_maps_to_error() {
        let err = anyhow::anyhow!("boom");
        assert_eq!(exit_code_for_error(&err), ExitCode::Error);
    }
}
