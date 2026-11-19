//! Path-safety primitives shared by GUI, CLI, codecs and archive layers.
//!
//! Every function returns `Result<_, PathSafetyError>` and never panics.
//! The module enforces:
//! - No `..` / absolute / drive / UNC components (Zip-Slip prevention).
//! - Rejection of reserved Windows device names (CON, NUL, COM1-9, LPT1-9).
//! - Rejection of trailing dot / space in components (NTFS creation quirk).
//! - Rejection of NUL bytes and overlong names (> 255 bytes per component).
//! - Symlink-aware file creation (`create_new` semantics).
//! - Extraction budget tracking on **actual bytes written**.

use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Component, Path, PathBuf};

// ---- error type -----------------------------------------------------------

/// All path-safety violations funnelled through one enum so callers can
/// pattern-match on the class of rejection.
#[derive(Debug)]
pub enum PathSafetyError {
    /// A `..` component was found in the untrusted path.
    ParentTraversal,
    /// An absolute, root, drive-letter or UNC prefix was found.
    AbsoluteOrPrefix,
    /// A path component matches a reserved Windows device name.
    ReservedName(String),
    /// A path component ends with `.` or ` ` (NTFS creation quirk).
    TrailingDotOrSpace,
    /// A NUL byte (`\0`) was found anywhere in the path.
    NulByte,
    /// A single component exceeds 255 bytes.
    OverlongComponent(usize),
    /// The resolved (canonicalized) path escapes the allowed root.
    EscapesRoot,
    /// A symlink was detected where only regular files/dirs are allowed.
    SymlinkDetected,
    /// The extraction budget (bytes, entries or ratio) was exceeded.
    BudgetExceeded(String),
    /// Wrapped I/O error from the filesystem.
    Io(io::Error),
}

impl std::fmt::Display for PathSafetyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ParentTraversal => write!(f, "path contains `..` traversal"),
            Self::AbsoluteOrPrefix => write!(f, "path is absolute or has a drive/UNC prefix"),
            Self::ReservedName(n) => write!(f, "reserved Windows device name: {}", n),
            Self::TrailingDotOrSpace => {
                write!(f, "path component ends with trailing dot or space")
            }
            Self::NulByte => write!(f, "path contains NUL byte"),
            Self::OverlongComponent(len) => {
                write!(f, "path component is {} bytes, limit is 255", len)
            }
            Self::EscapesRoot => write!(f, "resolved path escapes the allowed root directory"),
            Self::SymlinkDetected => write!(f, "symlink detected where not allowed"),
            Self::BudgetExceeded(msg) => write!(f, "extraction budget exceeded: {}", msg),
            Self::Io(e) => write!(f, "I/O error: {}", e),
        }
    }
}

impl std::error::Error for PathSafetyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for PathSafetyError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

// ---- constants ------------------------------------------------------------

/// Maximum length of a single path component in bytes.
const MAX_COMPONENT_BYTES: usize = 255;

/// Reserved Windows device names (case-insensitive, with or without extension).
const RESERVED_NAMES: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

// ---- component validation -------------------------------------------------

/// Validate a single file-name component.
///
/// Rejects: NUL bytes, reserved Windows device names, trailing `.` or ` `,
/// and names longer than 255 bytes.
pub fn validate_component(name: &str) -> Result<(), PathSafetyError> {
    if name.is_empty() {
        return Ok(());
    }

    // NUL byte
    if name.bytes().any(|b| b == 0) {
        return Err(PathSafetyError::NulByte);
    }

    // Overlong
    if name.len() > MAX_COMPONENT_BYTES {
        return Err(PathSafetyError::OverlongComponent(name.len()));
    }

    // Trailing dot or space
    if name.ends_with('.') || name.ends_with(' ') {
        return Err(PathSafetyError::TrailingDotOrSpace);
    }

    // Reserved Windows device names: strip extension then compare.
    let stem = match name.rfind('.') {
        Some(pos) if pos > 0 => &name[..pos],
        _ => name,
    };
    let upper = stem.to_ascii_uppercase();
    if RESERVED_NAMES.contains(&upper.as_str()) {
        return Err(PathSafetyError::ReservedName(name.to_owned()));
    }

    Ok(())
}

// ---- safe_join ------------------------------------------------------------

/// Join `base` with an untrusted relative path, rejecting traversal attacks.
///
/// Rejects `..`, absolute paths, drive/UNC prefixes, reserved names, trailing
/// dot/space, NUL bytes and overlong components.  Returns the joined
/// (non-canonicalized) path on success.
pub fn safe_join(base: &Path, untrusted: &Path) -> Result<PathBuf, PathSafetyError> {
    // Full path string NUL check
    let raw = untrusted.to_string_lossy();
    if raw.bytes().any(|b| b == 0) {
        return Err(PathSafetyError::NulByte);
    }

    let mut result = base.to_path_buf();

    for comp in untrusted.components() {
        match comp {
            Component::ParentDir => return Err(PathSafetyError::ParentTraversal),
            Component::RootDir | Component::Prefix(_) => {
                return Err(PathSafetyError::AbsoluteOrPrefix);
            }
            Component::CurDir => { /* skip `.` */ }
            Component::Normal(segment) => {
                let s = segment.to_string_lossy();
                validate_component(&s)?;
                result.push(segment);
            }
        }
    }

    Ok(result)
}

// ---- symlink check --------------------------------------------------------

/// Return `true` when `path` is a symbolic link (uses `symlink_metadata` to
/// avoid following the link).
pub fn is_symlink(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
}

/// Verify that every existing ancestor of `path` down from (and including)
/// `root` is **not** a symlink.  Returns `Err(SymlinkDetected)` if any
/// intermediate directory is a symlink.
pub fn verify_no_symlink_components(root: &Path, path: &Path) -> Result<(), PathSafetyError> {
    // Walk from root downward; if any prefix component is a symlink the
    // extraction destination could be redirected.
    let rel = path.strip_prefix(root).unwrap_or(path);
    let mut cursor = root.to_path_buf();
    for comp in rel.components() {
        if let Component::Normal(seg) = comp {
            cursor.push(seg);
            if cursor.exists() && is_symlink(&cursor) {
                return Err(PathSafetyError::SymlinkDetected);
            }
        }
    }
    Ok(())
}

// ---- create_file_no_follow ------------------------------------------------

/// Create a new file at `base / rel` without following symlinks.
///
/// 1. Validates `rel` with `safe_join`.
/// 2. Creates parent directories (rejecting symlink components under `base`).
/// 3. Opens the file with `create_new(true)` so pre-existing symlinks are
///    never followed.
pub fn create_file_no_follow(base: &Path, rel: &Path) -> Result<File, PathSafetyError> {
    let full = safe_join(base, rel)?;

    if let Some(parent) = full.parent() {
        // Ensure parent dirs exist
        fs::create_dir_all(parent)?;
        // Verify no symlink components between base and the parent
        verify_no_symlink_components(base, parent)?;
    }

    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&full)?;
    Ok(file)
}

// ---- create_dir_no_follow -------------------------------------------------

/// Create a directory at `base / rel` without following symlinks.
///
/// 1. Validates `rel` with `safe_join`.
/// 2. Creates the directory tree.
/// 3. Verifies no symlink components exist under `base`.
pub fn create_dir_no_follow(base: &Path, rel: &Path) -> Result<PathBuf, PathSafetyError> {
    let full = safe_join(base, rel)?;
    fs::create_dir_all(&full)?;
    verify_no_symlink_components(base, &full)?;
    Ok(full)
}

// ---- extraction budget ----------------------------------------------------

/// Configurable limits for archive extraction, counted on **actual bytes
/// written** (not attacker-declared sizes).
#[derive(Debug, Clone, Copy)]
pub struct ExtractLimits {
    /// Maximum total bytes that may be written across all entries.
    pub max_total_bytes: u64,
    /// Maximum number of entries (files + directories).
    pub max_entries: usize,
    /// Maximum compression ratio (uncompressed / compressed); 0 = unlimited.
    pub max_ratio: f64,
}

impl Default for ExtractLimits {
    fn default() -> Self {
        Self {
            max_total_bytes: 4 * 1024 * 1024 * 1024, // 4 GB
            max_entries: 20_000,
            max_ratio: 0.0,
        }
    }
}

/// Mutable budget tracker for an extraction session.
#[derive(Debug)]
pub struct ExtractBudget {
    limits: ExtractLimits,
    /// Total bytes actually written so far.
    pub bytes_written: u64,
    /// Entries processed so far.
    pub entries: usize,
    /// Compressed (input) size, set once by the caller for ratio checks.
    pub compressed_size: u64,
}

impl ExtractBudget {
    pub fn new(limits: ExtractLimits) -> Self {
        Self {
            limits,
            bytes_written: 0,
            entries: 0,
            compressed_size: 0,
        }
    }

    /// Record one entry.  Returns an error if the entry limit is exceeded.
    pub fn count_entry(&mut self) -> Result<(), PathSafetyError> {
        self.entries += 1;
        if self.entries > self.limits.max_entries {
            return Err(PathSafetyError::BudgetExceeded(format!(
                "entry count {} exceeds limit {}",
                self.entries, self.limits.max_entries
            )));
        }
        Ok(())
    }

    /// Record `n` bytes actually written.  Returns an error if the byte or
    /// ratio limit is exceeded.
    pub fn count_bytes(&mut self, n: u64) -> Result<(), PathSafetyError> {
        self.bytes_written += n;
        if self.bytes_written > self.limits.max_total_bytes {
            return Err(PathSafetyError::BudgetExceeded(format!(
                "total bytes {} exceed limit {}",
                self.bytes_written, self.limits.max_total_bytes
            )));
        }
        if self.limits.max_ratio > 0.0 && self.compressed_size > 0 {
            let ratio = self.bytes_written as f64 / self.compressed_size as f64;
            if ratio > self.limits.max_ratio {
                return Err(PathSafetyError::BudgetExceeded(format!(
                    "compression ratio {:.1} exceeds limit {:.1}",
                    ratio, self.limits.max_ratio
                )));
            }
        }
        Ok(())
    }

    /// How many bytes remain before the total byte ceiling.
    pub fn remaining_bytes(&self) -> u64 {
        self.limits
            .max_total_bytes
            .saturating_sub(self.bytes_written)
    }
}

// ---- unit tests -----------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    // -- validate_component --

    #[test]
    fn valid_component() {
        assert!(validate_component("readme.txt").is_ok());
        assert!(validate_component("My File (1)").is_ok());
    }

    #[test]
    fn reject_nul_byte() {
        assert!(matches!(
            validate_component("bad\0name"),
            Err(PathSafetyError::NulByte)
        ));
    }

    #[test]
    fn reject_reserved_name_bare() {
        assert!(matches!(
            validate_component("CON"),
            Err(PathSafetyError::ReservedName(_))
        ));
        assert!(matches!(
            validate_component("nul"),
            Err(PathSafetyError::ReservedName(_))
        ));
    }

    #[test]
    fn reject_reserved_name_with_extension() {
        assert!(matches!(
            validate_component("COM1.txt"),
            Err(PathSafetyError::ReservedName(_))
        ));
        assert!(matches!(
            validate_component("lpt9.log"),
            Err(PathSafetyError::ReservedName(_))
        ));
    }

    #[test]
    fn reject_trailing_dot() {
        assert!(matches!(
            validate_component("bad."),
            Err(PathSafetyError::TrailingDotOrSpace)
        ));
    }

    #[test]
    fn reject_trailing_space() {
        assert!(matches!(
            validate_component("bad "),
            Err(PathSafetyError::TrailingDotOrSpace)
        ));
    }

    #[test]
    fn reject_overlong_component() {
        let long = "a".repeat(256);
        assert!(matches!(
            validate_component(&long),
            Err(PathSafetyError::OverlongComponent(256))
        ));
    }

    #[test]
    fn allow_max_length_component() {
        let exact = "b".repeat(255);
        assert!(validate_component(&exact).is_ok());
    }

    // -- safe_join --

    #[test]
    fn safe_join_normal() {
        let base = Path::new("/tmp/extract");
        let result = safe_join(base, Path::new("subdir/file.txt")).unwrap();
        assert!(result.starts_with(base));
        assert!(result.ends_with("file.txt"));
    }

    #[test]
    fn safe_join_rejects_parent_dir() {
        let base = Path::new("/tmp/extract");
        assert!(matches!(
            safe_join(base, Path::new("../escape")),
            Err(PathSafetyError::ParentTraversal)
        ));
    }

    #[test]
    fn safe_join_rejects_embedded_parent() {
        let base = Path::new("/tmp/extract");
        assert!(matches!(
            safe_join(base, Path::new("ok/../../../escape")),
            Err(PathSafetyError::ParentTraversal)
        ));
    }

    #[test]
    fn safe_join_rejects_absolute() {
        let base = Path::new("/tmp/extract");
        assert!(matches!(
            safe_join(base, Path::new("/etc/passwd")),
            Err(PathSafetyError::AbsoluteOrPrefix)
        ));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn safe_join_rejects_drive_prefix() {
        let base = Path::new("C:\\extract");
        assert!(matches!(
            safe_join(base, Path::new("D:\\evil")),
            Err(PathSafetyError::AbsoluteOrPrefix)
        ));
    }

    #[test]
    fn safe_join_rejects_reserved_in_path() {
        let base = Path::new("/tmp/extract");
        assert!(matches!(
            safe_join(base, Path::new("subdir/CON/file.txt")),
            Err(PathSafetyError::ReservedName(_))
        ));
    }

    #[test]
    fn safe_join_allows_curdir() {
        let base = Path::new("/tmp/extract");
        let result = safe_join(base, Path::new("./file.txt")).unwrap();
        assert!(result.ends_with("file.txt"));
    }

    // -- ExtractBudget --

    #[test]
    fn budget_counts_actual_bytes() {
        let limits = ExtractLimits {
            max_total_bytes: 1000,
            max_entries: 10,
            max_ratio: 0.0,
        };
        let mut budget = ExtractBudget::new(limits);
        assert!(budget.count_bytes(500).is_ok());
        assert_eq!(budget.bytes_written, 500);
        assert_eq!(budget.remaining_bytes(), 500);
        assert!(budget.count_bytes(500).is_ok());
        assert_eq!(budget.remaining_bytes(), 0);
        // Next byte exceeds
        assert!(matches!(
            budget.count_bytes(1),
            Err(PathSafetyError::BudgetExceeded(_))
        ));
    }

    #[test]
    fn budget_counts_entries() {
        let limits = ExtractLimits {
            max_total_bytes: u64::MAX,
            max_entries: 2,
            max_ratio: 0.0,
        };
        let mut budget = ExtractBudget::new(limits);
        assert!(budget.count_entry().is_ok());
        assert!(budget.count_entry().is_ok());
        assert!(matches!(
            budget.count_entry(),
            Err(PathSafetyError::BudgetExceeded(_))
        ));
    }

    #[test]
    fn budget_ratio_check() {
        let limits = ExtractLimits {
            max_total_bytes: u64::MAX,
            max_entries: usize::MAX,
            max_ratio: 100.0,
        };
        let mut budget = ExtractBudget::new(limits);
        budget.compressed_size = 100;
        // exactly 100x is within the limit
        assert!(budget.count_bytes(10_000).is_ok());
        // one byte past the 100x ratio limit fails
        assert!(matches!(
            budget.count_bytes(1),
            Err(PathSafetyError::BudgetExceeded(_))
        ));
    }

    // -- create_file_no_follow / create_dir_no_follow (filesystem tests) --

    #[test]
    fn create_file_no_follow_basic() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path();
        let file = create_file_no_follow(base, Path::new("sub/test.txt"));
        assert!(file.is_ok());
        assert!(base.join("sub").join("test.txt").exists());
    }

    #[test]
    fn create_file_no_follow_rejects_traversal() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path();
        let result = create_file_no_follow(base, Path::new("../escape.txt"));
        assert!(matches!(result, Err(PathSafetyError::ParentTraversal)));
    }

    #[test]
    fn create_file_no_follow_rejects_reserved() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path();
        let result = create_file_no_follow(base, Path::new("CON"));
        assert!(matches!(result, Err(PathSafetyError::ReservedName(_))));
    }

    #[test]
    fn create_dir_no_follow_basic() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path();
        let dir = create_dir_no_follow(base, Path::new("a/b/c")).unwrap();
        assert!(dir.is_dir());
    }

    #[cfg(unix)]
    #[test]
    fn create_file_rejects_symlink_component() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path();
        // Create a directory and a symlink pointing outside
        let legit = base.join("legit");
        fs::create_dir_all(&legit).unwrap();
        let link_path = base.join("link");
        std::os::unix::fs::symlink("/tmp", &link_path).unwrap();
        let result = create_file_no_follow(base, Path::new("link/pwned.txt"));
        // The symlink component should be detected
        assert!(result.is_err());
    }

    // -- is_symlink --

    #[test]
    fn is_symlink_regular_file() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("regular.txt");
        File::create(&p).unwrap();
        assert!(!is_symlink(&p));
    }
}
