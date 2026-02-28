//! Platform abstraction for trust store, hosts file, etc.

use std::path::PathBuf;

#[cfg(unix)]
pub mod unix;

#[cfg(windows)]
pub mod windows;

use anyhow::Result;
use std::path::Path;

/// Trait for trust store operations (install/uninstall CA).
pub trait TrustStore: Send + Sync {
    /// Install CA PEM into system trust store.
    fn install_ca(&self, ca_pem_path: &Path) -> Result<()>;
    /// Remove CA from system trust store (by cert subject/hash).
    fn uninstall_ca(&self, ca_pem_path: &Path) -> Result<()>;
    /// Check if CA is installed in system trust store.
    fn is_ca_installed(&self, ca_pem_path: &Path) -> Result<bool>;
}

/// Trait for hosts file operations.
pub trait HostsEditor: Send + Sync {
    /// Add domain to hosts file (127.0.0.1 and ::1).
    fn add_domain(&self, domain: &str) -> Result<()>;
    /// Remove domain from hosts file.
    fn remove_domain(&self, domain: &str) -> Result<()>;
}

/// Get platform TrustStore implementation.
pub fn default_trust_store() -> Box<dyn TrustStore> {
    #[cfg(unix)]
    return Box::new(unix::UnixTrustStore);

    #[cfg(windows)]
    return Box::new(windows::WindowsTrustStore);
}

/// Get platform HostsEditor implementation.
/// If ROOST_HOSTS_FILE is set (e.g. in tests), uses FileHostsEditor with that path.
pub fn default_hosts_editor() -> Box<dyn HostsEditor> {
    if let Ok(path) = std::env::var("ROOST_HOSTS_FILE") {
        return Box::new(FileHostsEditor::new(path));
    }
    #[cfg(unix)]
    return Box::new(unix::UnixHostsEditor);

    #[cfg(windows)]
    return Box::new(windows::WindowsHostsEditor);
}

/// HostsEditor that reads/writes a file at the given path (for tests).
#[derive(Clone)]
pub struct FileHostsEditor {
    path: PathBuf,
}

impl FileHostsEditor {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &std::path::Path {
        &self.path
    }
}

impl HostsEditor for FileHostsEditor {
    fn add_domain(&self, domain: &str) -> Result<()> {
        let content = std::fs::read_to_string(&self.path).unwrap_or_default();
        let line1 = format!("127.0.0.1\t{domain}");
        let line2 = format!("::1\t{domain}");
        if content.contains(&line1) && content.contains(&line2) {
            return Ok(());
        }
        let mut lines: Vec<String> = content.lines().map(String::from).collect();
        if !lines.iter().any(|l| l.contains(domain)) {
            lines.push(line1);
            lines.push(line2);
        }
        let new_content = lines.join("\n");
        std::fs::write(&self.path, format!("{}\n", new_content.trim_end()))?;
        Ok(())
    }

    fn remove_domain(&self, domain: &str) -> Result<()> {
        let content = std::fs::read_to_string(&self.path).unwrap_or_default();
        let lines: Vec<&str> = content.lines().filter(|l| !l.contains(domain)).collect();
        std::fs::write(&self.path, lines.join("\n"))?;
        Ok(())
    }
}
