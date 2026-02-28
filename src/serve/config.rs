//! Serve config: mapping add/remove/list, config merge.

use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};

/// Source of a mapping for list output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MappingSource {
    Project,
    Global,
}

/// Single mapping: domain -> port.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapping {
    pub domain: String,
    pub port: u16,
}

/// Top-level .roostrc file format (has [serve] section).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct RoostRc {
    #[serde(default)]
    serve: ServeConfig,
}

/// Default ports when none configured: 80 (HTTP redirect) and 443 (HTTPS).
pub const DEFAULT_PORTS: [u16; 2] = [80, 443];

/// Serve config (from .roostrc or global).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServeConfig {
    #[serde(default)]
    pub mappings: Vec<Mapping>,
    /// Ports to listen on. Empty means use DEFAULT_PORTS ([80, 443]).
    #[serde(default)]
    pub ports: Vec<u16>,
}

impl ServeConfig {
    /// Load serve config from path. Uses advisory lock when file exists.
    pub fn load(path: &Path) -> Result<Self> {
        if path.is_file() {
            let mut file = fs::OpenOptions::new().read(true).open(path)?;
            fs2::FileExt::lock_shared(&file)?;
            let mut s = String::new();
            file.read_to_string(&mut s)?;
            let rc: RoostRc = toml::from_str(&s)?;
            let mut cfg = rc.serve;
            cfg.mappings.retain(|m| !m.domain.is_empty());
            Ok(cfg)
        } else {
            Ok(ServeConfig::default())
        }
    }

    /// Save serve config to path. Uses advisory lock. Creates parent dirs if needed.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(p) = path.parent() {
            fs::create_dir_all(p)?;
        }
        let rc = RoostRc {
            serve: self.clone(),
        };
        let s = toml::to_string_pretty(&rc)?;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)?;
        fs2::FileExt::lock_exclusive(&file)?;
        file.write_all(s.as_bytes())?;
        Ok(())
    }

    pub fn add(&mut self, domain: String, port: u16) {
        self.mappings.retain(|m| m.domain != domain);
        self.mappings.push(Mapping { domain, port });
    }

    pub fn remove(&mut self, domain: &str) {
        self.mappings.retain(|m| m.domain != domain);
    }

    pub fn list(&self) -> Vec<(&str, u16)> {
        self.mappings
            .iter()
            .map(|m| (m.domain.as_str(), m.port))
            .collect()
    }

    /// Effective ports: config ports if non-empty, else DEFAULT_PORTS.
    fn effective_ports(&self) -> Vec<u16> {
        if self.ports.is_empty() {
            DEFAULT_PORTS.to_vec()
        } else {
            let mut p = self.ports.clone();
            p.sort();
            p.dedup();
            p
        }
    }

    pub fn ports_add(&mut self, port: u16) {
        let mut effective = self.effective_ports();
        if !effective.contains(&port) {
            effective.push(port);
            effective.sort();
            self.ports = effective;
        }
    }

    pub fn ports_remove(&mut self, port: u16) {
        if self.ports.is_empty() {
            self.ports = DEFAULT_PORTS
                .iter()
                .filter(|&&p| p != port)
                .copied()
                .collect();
        } else {
            self.ports.retain(|&p| p != port);
        }
    }

    /// Replace ports list entirely (for scripting / tests).
    pub fn ports_set(&mut self, ports: Vec<u16>) {
        let mut p = ports;
        p.sort();
        p.dedup();
        self.ports = p;
    }

    pub fn ports_list(&self) -> Vec<u16> {
        self.effective_ports()
    }
}

/// Merge ports from project and global configs (union). Uses DEFAULT_PORTS when both empty.
pub fn merge_ports(project: &ServeConfig, global: &ServeConfig) -> Vec<u16> {
    use std::collections::HashSet;
    let project_eff = project.effective_ports();
    let global_eff = global.effective_ports();
    let p: HashSet<u16> = project_eff.into_iter().chain(global_eff).collect();
    if p.is_empty() {
        DEFAULT_PORTS.to_vec()
    } else {
        let mut v: Vec<u16> = p.into_iter().collect();
        v.sort();
        v
    }
}

/// Merge project and global configs; project overrides on conflict.
pub fn merge_configs(project: &ServeConfig, global: &ServeConfig) -> HashMap<String, u16> {
    let mut out = HashMap::new();
    for m in &global.mappings {
        out.insert(m.domain.clone(), m.port);
    }
    for m in &project.mappings {
        out.insert(m.domain.clone(), m.port);
    }
    out
}

/// Merged mapping with source for list output.
#[derive(Debug, Clone)]
pub struct MergedMapping {
    pub domain: String,
    pub port: u16,
    pub source: MappingSource,
}

/// Merge project and global configs; returns list with source per mapping.
/// Project overrides global on conflict; source reflects which file provided the value.
pub fn merge_configs_with_source(
    project: &ServeConfig,
    global: &ServeConfig,
) -> Vec<MergedMapping> {
    let mut by_domain: HashMap<String, (u16, MappingSource)> = HashMap::new();
    for m in &global.mappings {
        if !m.domain.is_empty() {
            by_domain.insert(m.domain.clone(), (m.port, MappingSource::Global));
        }
    }
    for m in &project.mappings {
        if !m.domain.is_empty() {
            by_domain.insert(m.domain.clone(), (m.port, MappingSource::Project));
        }
    }
    let mut out: Vec<MergedMapping> = by_domain
        .into_iter()
        .map(|(domain, (port, source))| MergedMapping {
            domain,
            port,
            source,
        })
        .collect();
    out.sort_by(|a, b| a.domain.cmp(&b.domain));
    out
}
