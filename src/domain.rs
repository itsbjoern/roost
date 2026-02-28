//! Domain validation, add/remove, set-ca.

use anyhow::Result;
use std::path::PathBuf;

use crate::cert;
use crate::config::{Config, RoostPaths};
use crate::hosts;
use crate::platform::HostsEditor;

/// TLD allowlist for safe dev domains.
pub const TLD_ALLOWLIST: &[&str] = &[
    "test", "example", "invalid", "localhost", "local", "internal", "lan", "home",
    "localdomain", "corp", "private", "docker", "dev",
];

/// Validate domain against TLD allowlist.
pub fn validate_domain(domain: &str, allow_any_tld: bool) -> Result<()> {
    if allow_any_tld {
        return validate_hostname(domain);
    }
    let parts: Vec<&str> = domain.split('.').collect();
    if parts.is_empty() {
        anyhow::bail!("empty domain");
    }
    let tld = parts.last().unwrap().to_lowercase();
    if !TLD_ALLOWLIST.contains(&tld.as_str()) {
        anyhow::bail!("TLD .{tld} not in allowlist; use --allow to override");
    }
    validate_hostname(domain)
}

/// Validate hostname format.
pub fn validate_hostname(domain: &str) -> Result<()> {
    if domain.is_empty() {
        anyhow::bail!("empty hostname");
    }
    if domain.contains("..") {
        anyhow::bail!("invalid hostname: consecutive dots");
    }
    if domain == "localhost" {
        anyhow::bail!("bare localhost not allowed");
    }
    for label in domain.split('.') {
        if label.is_empty() {
            anyhow::bail!("invalid hostname: empty label");
        }
        for c in label.chars() {
            if !c.is_ascii_alphanumeric() && c != '-' {
                anyhow::bail!("invalid hostname: illegal char {c:?}");
            }
        }
        if label.starts_with('-') || label.ends_with('-') {
            anyhow::bail!("invalid hostname: label cannot start/end with hyphen");
        }
    }
    Ok(())
}

/// Add domain to config, create cert, and optionally update hosts.
pub fn add_domain(
    paths: &RoostPaths,
    config: &mut Config,
    domain: &str,
    exact: bool,
    hosts_editor: Option<&dyn HostsEditor>,
) -> Result<()> {
    let ca_name = if config.default_ca.is_empty() {
        config.default_ca = "default".to_string();
        "default".to_string()
    } else {
        config.default_ca.clone()
    };
    if !crate::ca::ca_exists(paths, &ca_name) {
        anyhow::bail!("CA '{ca_name}' does not exist; run 'roost ca create {ca_name}' first");
    }

    cert::ensure_cert_valid(paths, domain, &ca_name, exact)?;

    // Update hosts before config so we don't leave partial state on failure
    if let Some(editor) = hosts_editor {
        hosts::add_domain_to_hosts(editor, domain)?;
    }

    config.domains.insert(domain.to_string(), ca_name);

    Ok(())
}

/// Remove domain from config, hosts, and delete cert files.
pub fn remove_domain(
    paths: &RoostPaths,
    config: &mut Config,
    domain: &str,
    hosts_editor: Option<&dyn HostsEditor>,
) -> Result<()> {
    config.domains.remove(domain);

    if let Some(editor) = hosts_editor {
        hosts::remove_domain_from_hosts(editor, domain)?;
    }

    let cert_path = paths.certs_dir.join(format!("{domain}.pem"));
    let key_path = paths.certs_dir.join(format!("{domain}-key.pem"));
    let _ = std::fs::remove_file(&cert_path);
    let _ = std::fs::remove_file(&key_path);

    Ok(())
}

/// Re-sign domain cert with different CA.
pub fn set_ca(paths: &RoostPaths, config: &mut Config, domain: &str, ca_name: &str) -> Result<()> {
    if !config.domains.contains_key(domain) {
        anyhow::bail!("domain '{domain}' not found");
    }
    if !crate::ca::ca_exists(paths, ca_name) {
        anyhow::bail!("CA '{ca_name}' does not exist");
    }

    config.domains.insert(domain.to_string(), ca_name.to_string());
    // Always regenerate when CA changes (don't use ensure_cert_valid which skips if cert exists)
    let (ca_pem, ca_key_pem) = crate::ca::load_ca(paths, ca_name)?;
    let (cert_pem, key_pem) = cert::generate_domain_cert(domain, &ca_pem, &ca_key_pem, false)?;
    cert::save_domain_cert(paths, domain, &cert_pem, &key_pem)?;

    Ok(())
}

/// List domains from config.
pub fn list_domains(config: &Config) -> Vec<(String, String)> {
    let mut v: Vec<_> = config.domains.iter().map(|(d, c)| (d.clone(), c.clone())).collect();
    v.sort_by(|a, b| a.0.cmp(&b.0));
    v
}

/// Get cert and key paths for domain.
pub fn get_cert_paths(paths: &RoostPaths, domain: &str) -> (PathBuf, PathBuf) {
    (
        paths.certs_dir.join(format!("{domain}.pem")),
        paths.certs_dir.join(format!("{domain}-key.pem")),
    )
}
