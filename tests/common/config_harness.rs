//! Sandboxed Configuration Environment Harness for Mossambi E2E Tests.
//!
//! Provides isolated configuration paths and JSON serialization fixtures to guarantee
//! zero contamination of user settings or %APPDATA%/mossambi.

use mossambi::core::MossambiConfig;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Sandboxed configuration environment for isolated persistence tests.
pub struct IsolatedConfigEnv {
    temp_dir: TempDir,
    config_file: PathBuf,
}

impl IsolatedConfigEnv {
    /// Creates a new isolated configuration environment in a temporary folder.
    pub fn new() -> io::Result<Self> {
        let temp_dir = TempDir::new()?;
        let config_file = temp_dir.path().join("config.json");
        Ok(Self {
            temp_dir,
            config_file,
        })
    }

    /// Returns the absolute path to the isolated config directory.
    pub fn config_dir(&self) -> &Path {
        self.temp_dir.path()
    }

    /// Returns the absolute path to the isolated `config.json` file.
    pub fn config_path(&self) -> PathBuf {
        self.config_file.clone()
    }

    /// Writes raw JSON string directly into the isolated configuration file.
    pub fn write_config_json(&self, json: &str) -> io::Result<PathBuf> {
        fs::write(&self.config_file, json)?;
        Ok(self.config_file.clone())
    }

    /// Reads the raw JSON content of the isolated configuration file.
    pub fn read_config_json(&self) -> io::Result<String> {
        fs::read_to_string(&self.config_file)
    }

    /// Loads and deserializes `MossambiConfig` from the isolated configuration file.
    pub fn load_config(&self) -> io::Result<MossambiConfig> {
        let content = self.read_config_json()?;
        serde_json::from_str(&content).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// Serializes and saves `MossambiConfig` to the isolated configuration file.
    pub fn save_config(&self, cfg: &MossambiConfig) -> io::Result<()> {
        let json = serde_json::to_string_pretty(cfg)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        fs::write(&self.config_file, json)?;
        Ok(())
    }
}
