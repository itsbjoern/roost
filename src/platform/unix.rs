//! Unix (macOS, Linux) platform implementations.

use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

use super::{HostsEditor, TrustStore};

pub struct UnixTrustStore;

impl TrustStore for UnixTrustStore {
    fn install_ca(&self, ca_pem_path: &Path) -> Result<()> {
        #[cfg(target_os = "macos")]
        {
            let path = ca_pem_path.to_string_lossy();
            Command::new("osascript")
                .args([
                    "-e",
                    &format!(
                        "do shell script \"security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain {}\" with administrator privileges",
                        path.replace('"', "\\\"")
                    ),
                ])
                .status()
                .context("osascript security add-trusted-cert")?;
        }

        #[cfg(not(target_os = "macos"))]
        {
            // Linux: copy to /usr/local/share/ca-certificates/ and run update-ca-certificates
            let dest = "/usr/local/share/ca-certificates/roost-ca.crt";
            let cp_status = Command::new("pkexec")
                .args(["cp", ca_pem_path.to_str().unwrap_or(""), dest])
                .status()
                .context("pkexec cp ca")?;
            if !cp_status.success() {
                anyhow::bail!("Failed to copy CA to trust store");
            }
            Command::new("pkexec")
                .args(["update-ca-certificates"])
                .status()
                .context("pkexec update-ca-certificates")?;
        }
        Ok(())
    }

    fn uninstall_ca(&self, _ca_pem_path: &Path) -> Result<()> {
        #[cfg(target_os = "macos")]
        {
            // Would need to find cert by subject and remove - simplified for now
            // security delete-certificate -c "Roost" /Library/Keychains/System.keychain
            // For now we leave uninstall as no-op; full impl would parse cert and delete by identity
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = std::fs::remove_file("/usr/local/share/ca-certificates/roost-ca.crt");
            let _ = Command::new("pkexec").args(["update-ca-certificates"]).status();
        }
        Ok(())
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
        Command::new("pkexec")
            .args(["sh", "-c", &format!("echo '{}' >> {}", new_content.lines().last().unwrap_or(""), hosts_path)])
            .status()
            .context("Failed to write hosts")?;
        // Simpler: write temp file and pkexec cp
        let temp = std::env::temp_dir().join("roost-hosts");
        std::fs::write(&temp, new_content)?;
        let _ = Command::new("pkexec")
            .args(["cp", temp.to_str().unwrap(), hosts_path])
            .status();
        Ok(())
    }

    fn remove_domain(&self, _domain: &str) -> Result<()> {
        // Filter out lines for domain
        Ok(())
    }
}
