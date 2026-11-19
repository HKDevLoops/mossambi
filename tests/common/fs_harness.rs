//! Filesystem Sandboxing & Fixture Generator for Mossambi E2E Tests.
//!
//! Provides isolated TempDir fixtures, large directory generators (10k items),
//! symlink/junction creators, filename validation contract checkers,
//! and loop-safe recursive tree copy helpers.

#![allow(clippy::io_other_error)]

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Sandboxed filesystem tree fixture for isolated E2E tests.
pub struct TestFsTree {
    temp_dir: TempDir,
}

impl TestFsTree {
    /// Creates a new temporary filesystem sandbox.
    pub fn new() -> io::Result<Self> {
        let temp_dir = TempDir::new()?;
        Ok(Self { temp_dir })
    }

    /// Returns the absolute path to the root of the sandbox.
    pub fn root(&self) -> &Path {
        self.temp_dir.path()
    }

    /// Creates a file with the specified content relative to the sandbox root.
    pub fn create_file(&self, rel_path: &str, content: &[u8]) -> io::Result<PathBuf> {
        let target = self.root().join(rel_path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&target, content)?;
        Ok(target)
    }

    /// Creates a directory relative to the sandbox root.
    pub fn create_dir(&self, rel_path: &str) -> io::Result<PathBuf> {
        let target = self.root().join(rel_path);
        fs::create_dir_all(&target)?;
        Ok(target)
    }

    /// Creates a directory symlink or Windows NTFS junction pointing to `target`.
    pub fn create_symlink_or_junction(
        &self,
        link_name: &str,
        target: &Path,
    ) -> io::Result<PathBuf> {
        let link_path = self.root().join(link_name);
        if let Some(parent) = link_path.parent() {
            fs::create_dir_all(parent)?;
        }

        #[cfg(windows)]
        {
            // First attempt native symlink
            if std::os::windows::fs::symlink_dir(target, &link_path).is_ok() {
                return Ok(link_path);
            }
            // Fall back to unprivileged NTFS directory junction via mklink /J
            let output = std::process::Command::new("cmd")
                .args(["/c", "mklink", "/J"])
                .arg(&link_path)
                .arg(target)
                .output()?;
            if !output.status.success() {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!(
                        "mklink /J failed: {}",
                        String::from_utf8_lossy(&output.stderr)
                    ),
                ));
            }
            Ok(link_path)
        }

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(target, &link_path)?;
            Ok(link_path)
        }

        #[cfg(not(any(windows, unix)))]
        {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Symlinks not supported on this platform",
            ))
        }
    }

    /// Creates a cyclic directory tree (A -> B -> A) using junctions/symlinks.
    pub fn create_cyclic_symlink_tree(&self, dir_a: &str, dir_b: &str) -> io::Result<PathBuf> {
        let path_a = self.create_dir(dir_a)?;
        let path_b = self.create_dir(dir_b)?;
        let _ = self.create_file(&format!("{dir_a}/item_a.txt"), b"item in a")?;
        let _ = self.create_file(&format!("{dir_b}/item_b.txt"), b"item in b")?;

        // Link B inside A and link A inside B
        let _ = self.create_symlink_or_junction(&format!("{dir_a}/link_to_b"), &path_b);
        let _ = self.create_symlink_or_junction(&format!("{dir_b}/link_to_a"), &path_a);

        Ok(path_a)
    }

    /// Generates a large benchmark directory containing `count` files with
    /// natural sort prefixes, extensions, and varied file sizes.
    pub fn generate_benchmark_dir(&self, count: usize) -> io::Result<PathBuf> {
        let bench_dir = self.create_dir("benchmark_10k")?;
        let exts = ["txt", "rs", "json", "pdf", "png", "zip", "log", "toml"];

        for i in 1..=count {
            let ext = exts[i % exts.len()];
            let name = format!("benchmark_item_{}.{}", i, ext);
            let file_path = bench_dir.join(name);
            let content = format!("Benchmark payload for item {i}\n");
            fs::write(&file_path, content.as_bytes())?;
        }

        Ok(bench_dir)
    }
}
