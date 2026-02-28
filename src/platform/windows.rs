//! Windows platform implementations.

use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;
use x509_parser::pem::Pem;

use super::{HostsEditor, TrustStore};

/// Extract Common Name from CA PEM bytes (e.g. "Roost CA (default)").
pub fn cert_cn_from_pem(pem_bytes: &[u8]) -> Result<Option<String>> {
    let pem = Pem::iter_from_buffer(pem_bytes)
        .next()
        .ok_or_else(|| anyhow::anyhow!("no PEM block in certificate"))??;
    let x509 = pem.parse_x509().context("parse X.509 certificate")?;
    let cn = x509
        .subject()
        .iter_common_name()
        .next()
        .and_then(|c| c.as_str().ok())
        .map(String::from);
    Ok(cn)
}

pub struct WindowsTrustStore;

impl TrustStore for WindowsTrustStore {
    fn install_ca(&self, ca_pem_path: &Path) -> Result<()> {
        // certutil -addstore -user "ROOT" path
        let status = Command::new("certutil")
            .args(["-addstore", "-user", "ROOT", ca_pem_path.to_str().unwrap_or("")])
            .status()
            .context("certutil addstore")?;
        if !status.success() {
            anyhow::bail!("certutil addstore failed");
        }
        Ok(())
    }

    fn uninstall_ca(&self, ca_pem_path: &Path) -> Result<()> {
        let pem_bytes = std::fs::read(ca_pem_path)
            .with_context(|| format!("read CA cert: {}", ca_pem_path.display()))?;
        let cn = cert_cn_from_pem(&pem_bytes)?
            .ok_or_else(|| anyhow::anyhow!("CA certificate has no Common Name"))?;
        let status = Command::new("certutil")
            .args(["-delstore", "-user", "ROOT", &cn])
            .status()
            .context("certutil delstore")?;
        if !status.success() {
            anyhow::bail!("certutil delstore failed (cert may not be installed)");
        }
        Ok(())
    }

    fn is_ca_installed(&self, ca_pem_path: &Path) -> Result<bool> {
        let pem_bytes = std::fs::read(ca_pem_path)
            .with_context(|| format!("read CA cert: {}", ca_pem_path.display()))?;
        let cn = match cert_cn_from_pem(&pem_bytes)? {
            Some(c) => c,
            None => return Ok(false),
        };
        let output = Command::new("certutil")
            .args(["-verifystore", "-user", "ROOT"])
            .output()
            .context("certutil verifystore")?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.contains(&cn))
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

    fn has_domain(&self, domain: &str) -> Result<bool> {
        let hosts_path = r"C:\Windows\System32\drivers\etc\hosts";
        let content = std::fs::read_to_string(hosts_path).unwrap_or_default();
        Ok(super::domain_in_hosts_content(&content, domain))
    }
}
