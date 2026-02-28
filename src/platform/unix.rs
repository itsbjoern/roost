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
            // Linux: copy to /usr/local/share/ca-certificates/ and run update-ca-certificates
            // Uses sudo (standard on Linux) rather than pkexec (requires polkit, not on all distros)
            let dest = "/usr/local/share/ca-certificates/roost-ca.crt";
            let cp_status = Command::new("sudo")
                .args(["cp", ca_pem_path.to_str().unwrap_or(""), dest])
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

    fn uninstall_ca(&self, _ca_pem_path: &Path) -> Result<()> {
        #[cfg(target_os = "macos")]
        {
            // Would need to find cert by subject and remove - simplified for now
            // security delete-certificate -c "Roost" /Library/Keychains/System.keychain
            // For now we leave uninstall as no-op; full impl would parse cert and delete by identity
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = Command::new("sudo")
                .args(["rm", "-f", "/usr/local/share/ca-certificates/roost-ca.crt"])
                .status();
            let _ = Command::new("sudo")
                .args(["update-ca-certificates"])
                .status();
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

        let temp = std::env::temp_dir().join("roost-hosts");
        std::fs::write(&temp, &new_content)?;

        #[cfg(target_os = "macos")]
        {
            // Pass path via env var so we never embed it in a shell string (avoids escaping issues)
            Command::new("osascript")
                .env("ROOST_HOSTS_TMP", temp.as_os_str())
                .args([
                    "-e",
                    "do shell script \"cp \\\"$ROOST_HOSTS_TMP\\\" /etc/hosts && (killall -HUP mDNSResponder || true)\" with administrator privileges",
                ])
                .status()
                .context("osascript write hosts")?;
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
}
