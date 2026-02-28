//! Unix (macOS, Linux) platform implementations.

use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;
#[cfg(target_os = "macos")]
use x509_parser::pem::Pem;

use super::{HostsEditor, TrustStore};

/// Extract Common Name from a CA PEM file (e.g. "Roost CA (default)").
#[cfg(target_os = "macos")]
fn cert_cn_from_pem(ca_pem_path: &Path) -> Result<Option<String>> {
    let pem_bytes = std::fs::read(ca_pem_path)
        .with_context(|| format!("read CA cert: {}", ca_pem_path.display()))?;
    let pem = Pem::iter_from_buffer(&pem_bytes)
        .next()
        .ok_or_else(|| anyhow::anyhow!("no PEM block in certificate"))??;
    let x509 = pem
        .parse_x509()
        .context("parse X.509 certificate")?;
    let cn = x509
        .subject()
        .iter_common_name()
        .next()
        .and_then(|c| c.as_str().ok())
        .map(String::from);
    Ok(cn)
}

/// Sanitize CA name for use in Linux trust store filename.
#[cfg(not(target_os = "macos"))]
fn sanitize_ca_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Get CA name from path (e.g. .../cas/default/ca.pem -> "default").
#[cfg(not(target_os = "macos"))]
fn ca_name_from_path(ca_pem_path: &Path) -> Option<String> {
    ca_pem_path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .map(String::from)
}

pub struct UnixTrustStore;

impl TrustStore for UnixTrustStore {
    fn install_ca(&self, ca_pem_path: &Path) -> Result<()> {
        #[cfg(target_os = "macos")]
        {
            // Use user's login keychain (not System) to avoid SecTrustSettings double-prompt.
            // System keychain with -d triggers "no user interaction was possible" when osascript
            // can't show the second auth dialog. User keychain needs no admin privileges.
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
            let keychain = format!("{home}/Library/Keychains/login.keychain-db");
            let status = Command::new("security")
                .args([
                    "add-trusted-cert",
                    "-r",
                    "trustRoot",
                    "-k",
                    &keychain,
                    ca_pem_path.to_str().unwrap_or(""),
                ])
                .status()
                .context("security add-trusted-cert")?;
            if !status.success() {
                anyhow::bail!("security add-trusted-cert failed");
            }
        }

        #[cfg(not(target_os = "macos"))]
        {
            // Linux: copy to /usr/local/share/ca-certificates/ with per-CA filename
            let name = ca_name_from_path(ca_pem_path).unwrap_or_else(|| "default".into());
            let safe = sanitize_ca_name(&name);
            let dest = format!("/usr/local/share/ca-certificates/roost-{safe}.crt");
            let cp_status = Command::new("sudo")
                .args(["cp", ca_pem_path.to_str().unwrap_or(""), &dest])
                .status()
                .context("sudo cp ca")?;
            if !cp_status.success() {
                anyhow::bail!("Failed to copy CA to trust store");
            }
            Command::new("sudo")
                .args(["update-ca-certificates"])
                .status()
                .context("sudo update-ca-certificates")?;
        }
        Ok(())
    }

    fn uninstall_ca(&self, ca_pem_path: &Path) -> Result<()> {
        #[cfg(target_os = "macos")]
        {
            let cn = cert_cn_from_pem(ca_pem_path)?
                .ok_or_else(|| anyhow::anyhow!("CA certificate has no Common Name"))?;
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
            let keychain = format!("{home}/Library/Keychains/login.keychain-db");
            let status = Command::new("security")
                .args(["delete-certificate", "-c", &cn, "-t"])
                .arg(&keychain)
                .status()
                .context("security delete-certificate")?;
            if !status.success() {
                anyhow::bail!("security delete-certificate failed (cert may not be installed)");
            }
        }

        #[cfg(not(target_os = "macos"))]
        {
            let name = ca_name_from_path(ca_pem_path).unwrap_or_else(|| "default".into());
            let safe = sanitize_ca_name(&name);
            let dest = format!("/usr/local/share/ca-certificates/roost-{safe}.crt");
            let rm_status = Command::new("sudo")
                .args(["rm", "-f", &dest])
                .status()
                .context("sudo rm ca")?;
            if !rm_status.success() {
                anyhow::bail!("Failed to remove CA from trust store");
            }
            Command::new("sudo")
                .args(["update-ca-certificates"])
                .status()
                .context("sudo update-ca-certificates")?;
        }
        Ok(())
    }

    fn is_ca_installed(&self, ca_pem_path: &Path) -> Result<bool> {
        #[cfg(target_os = "macos")]
        {
            let cn = match cert_cn_from_pem(ca_pem_path)? {
                Some(c) => c,
                None => return Ok(false),
            };
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
            let keychain = format!("{home}/Library/Keychains/login.keychain-db");
            let output = Command::new("security")
                .args(["find-certificate", "-c", &cn, "-a"])
                .arg(&keychain)
                .output()
                .context("security find-certificate")?;
            Ok(output.status.success() && !output.stdout.is_empty())
        }

        #[cfg(not(target_os = "macos"))]
        {
            let name = ca_name_from_path(ca_pem_path).unwrap_or_else(|| "default".into());
            let safe = sanitize_ca_name(&name);
            let dest = format!("/usr/local/share/ca-certificates/roost-{safe}.crt");
            Ok(std::path::Path::new(&dest).exists())
        }
    }
}

pub struct UnixHostsEditor;

impl HostsEditor for UnixHostsEditor {
    fn add_domain(&self, domain: &str) -> Result<()> {
        let hosts_path = "/etc/hosts";
        let content = std::fs::read_to_string(hosts_path)?;
        let line1 = format!("127.0.0.1\t{domain}");
        let line2 = format!("::1\t{domain}");
        if content.contains(&line1) || content.contains(&line2) {
            return Ok(());
        }
        let new_content = format!("{content}\n{line1}\n{line2}\n");

        let temp = std::env::temp_dir().join("roost-hosts");
        std::fs::write(&temp, &new_content)?;

        #[cfg(target_os = "macos")]
        {
            // Pass path via env var so we never embed it in a shell string (avoids escaping issues)
            let status = Command::new("osascript")
                .env("ROOST_HOSTS_TMP", temp.as_os_str())
                .args([
                    "-e",
                    "do shell script \"cp \\\"$ROOST_HOSTS_TMP\\\" /etc/hosts && (killall -HUP mDNSResponder || true)\" with administrator privileges",
                ])
                .status()
                .context("osascript write hosts")?;
            if !status.success() {
                anyhow::bail!(
                    "Failed to update hosts file (user cancelled or permission denied)"
                );
            }
        }

        #[cfg(not(target_os = "macos"))]
        {
            // Linux: sudo is standard on virtually all distros
            let status = Command::new("sudo")
                .args(["cp", temp.to_str().unwrap(), hosts_path])
                .status()
                .context("sudo cp hosts")?;
            if !status.success() {
                anyhow::bail!("Failed to write hosts");
            }
        }

        let _ = std::fs::remove_file(&temp);
        Ok(())
    }

    fn remove_domain(&self, _domain: &str) -> Result<()> {
        // Filter out lines for domain
        Ok(())
    }

    fn has_domain(&self, domain: &str) -> Result<bool> {
        let hosts_path = "/etc/hosts";
        let content = std::fs::read_to_string(hosts_path).unwrap_or_default();
        Ok(super::domain_in_hosts_content(&content, domain))
    }
}
