//! Windows platform implementations.

use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

use super::{HostsEditor, TrustStore};

pub struct WindowsTrustStore;

impl TrustStore for WindowsTrustStore {
    fn install_ca(&self, ca_pem_path: &Path) -> Result<()> {
        // certutil -addstore -user "ROOT" path
        Command::new("certutil")
            .args(["-addstore", "-user", "ROOT", ca_pem_path.to_str().unwrap_or("")])
            .status()
            .context("certutil addstore")?;
        Ok(())
    }

    fn uninstall_ca(&self, ca_pem_path: &Path) -> Result<()> {
        // Would need to get cert hash first - simplified
        let _ = ca_pem_path;
        Ok(())
    }
}

pub struct WindowsHostsEditor;

impl HostsEditor for WindowsHostsEditor {
    fn add_domain(&self, _domain: &str) -> Result<()> {
        let hosts_path = r"C:\Windows\System32\drivers\etc\hosts";
        let _ = hosts_path;
        Ok(())
    }

    fn remove_domain(&self, _domain: &str) -> Result<()> {
        Ok(())
    }
}
