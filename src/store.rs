//! Data store operations and directory layout.

use anyhow::Result;

use crate::config::{Config, RoostPaths};

/// Ensure all roost directories exist.
pub fn ensure_dirs(paths: &RoostPaths) -> Result<()> {
    std::fs::create_dir_all(&paths.ca_dir)?;
    std::fs::create_dir_all(&paths.certs_dir)?;
    if let Some(p) = paths.config_file.parent() {
        std::fs::create_dir_all(p)?;
    }
    Ok(())
}

/// Load config from store.
pub fn load_config(paths: &RoostPaths) -> Result<Config> {
    Config::load(paths)
}

/// Save config to store.
pub fn save_config(paths: &RoostPaths, config: &Config) -> Result<()> {
    Config::save(config, paths)
}
