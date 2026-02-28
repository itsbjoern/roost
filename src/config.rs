//! Configuration loading and path resolution.
//!
//! Supports ROOST_HOME env var override for testing.

use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Paths for roost data store.
#[derive(Debug, Clone)]
pub struct RoostPaths {
    pub config_dir: PathBuf,
    pub config_file: PathBuf,
    pub ca_dir: PathBuf,
    pub certs_dir: PathBuf,
    pub roostrc_global: PathBuf,
}

impl RoostPaths {
    /// Build paths from base directory (e.g. ProjectDirs data dir or ROOST_HOME).
    pub fn from_base(base: PathBuf) -> Self {
        let config_dir = base.clone();
        let config_file = base.join("config.toml");
        let ca_dir = base.join("ca");
        let certs_dir = base.join("certs");
        let roostrc_global = base.join(".roostrc");
        Self {
            config_dir,
            config_file,
            ca_dir,
            certs_dir,
            roostrc_global,
        }
    }

    /// Paths for testing: use a temp dir as base.
    pub fn for_test(base: impl AsRef<Path>) -> Self {
        Self::from_base(base.as_ref().to_path_buf())
    }

    /// Get default roost paths (respects ROOST_HOME).
    pub fn default_paths() -> Self {
        let base = if let Ok(home) = std::env::var("ROOST_HOME") {
            PathBuf::from(home)
        } else if let Some(dirs) = directories::ProjectDirs::from("com", "bjoernf", "roost") {
            dirs.data_dir().to_path_buf()
        } else {
            PathBuf::from(".roost")
        };
        Self::from_base(base)
    }
}

/// Main config.toml structure.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Config {
    #[serde(default)]
    pub default_ca: String,
    #[serde(default)]
    pub domains: HashMap<String, String>,
}

/// Path to config.toml (respects ROOST_HOME).
pub fn config_path() -> PathBuf {
    RoostPaths::default_paths().config_file
}

/// Find .roostrc in cwd only (no walk-up).
pub fn project_roostrc(cwd: &Path) -> Option<PathBuf> {
    let rc = cwd.join(".roostrc");
    if rc.is_file() {
        Some(rc)
    } else {
        None
    }
}

impl Config {
    /// Load config from paths (with shared lock when file exists).
    pub fn load(paths: &RoostPaths) -> Result<Config> {
        if paths.config_file.is_file() {
            let mut file = fs::OpenOptions::new().read(true).open(&paths.config_file)?;
            fs2::FileExt::lock_shared(&file)?;
            use std::io::Read;
            let mut s = String::new();
            file.read_to_string(&mut s)?;
            let cfg: Config = toml::from_str(&s)?;
            Ok(cfg)
        } else {
            Ok(Config::default())
        }
    }

    /// Save config to paths (with exclusive lock). Creates parent dirs if needed.
    pub fn save(&self, paths: &RoostPaths) -> Result<()> {
        if let Some(p) = paths.config_file.parent() {
            fs::create_dir_all(p)?;
        }
        let mut file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&paths.config_file)?;
        fs2::FileExt::lock_exclusive(&file)?;
        let s = toml::to_string_pretty(self)?;
        use std::io::Write;
        file.write_all(s.as_bytes())?;
        Ok(())
    }
}
