//! CA creation, loading, and removal.

use anyhow::{Context, Result};
use rcgen::{Certificate, CertificateParams, IsCa, KeyPair};
use std::fs;
use std::io::Write;

use crate::config::RoostPaths;
use crate::store;

/// Create a new CA with the given name.
pub fn create_ca(paths: &RoostPaths, name: &str) -> Result<()> {
    store::ensure_dirs(paths)?;
    let ca_dir = paths.ca_dir.join(name);
    fs::create_dir_all(&ca_dir)?;

    let key_pair = KeyPair::generate()
        .context("generate CA key pair")?;

    let mut params = CertificateParams::default();
    params.distinguished_name = rcgen::DistinguishedName::new();
    params.distinguished_name.push(
        rcgen::DnType::CommonName,
        rcgen::DnValue::Utf8String(format!("Roost CA ({})", name)),
    );
    params.is_ca = IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
    params.key_usages = vec![
        rcgen::KeyUsagePurpose::KeyCertSign,
        rcgen::KeyUsagePurpose::CrlSign,
    ];

    let cert = params.self_signed(&key_pair).context("create CA certificate")?;

    let ca_pem = cert.pem();
    let key_pem = key_pair.serialize_pem();

    let ca_path = ca_dir.join("ca.pem");
    let key_path = ca_dir.join("ca-key.pem");

    let mut f = fs::File::create(&ca_path)?;
    f.write_all(ca_pem.as_bytes())?;

    let mut f = fs::File::create(&key_path)?;
    f.write_all(key_pem.as_bytes())?;

    Ok(())
}

/// List all CAs.
pub fn list_cas(paths: &RoostPaths) -> Result<Vec<String>> {
    let mut names = Vec::new();
    if paths.ca_dir.is_dir() {
        for e in fs::read_dir(&paths.ca_dir)? {
            let e = e?;
            let name = e.file_name().into_string().unwrap_or_default();
            if ca_exists(paths, &name) {
                names.push(name);
            }
        }
    }
    names.sort();
    Ok(names)
}

/// Load CA certificate and key as PEM bytes.
pub fn load_ca(paths: &RoostPaths, name: &str) -> Result<(Vec<u8>, Vec<u8>)> {
    let ca_path = paths.ca_dir.join(name).join("ca.pem");
    let key_path = paths.ca_dir.join(name).join("ca-key.pem");

    let ca_pem = fs::read(&ca_path).with_context(|| format!("read CA cert: {}", ca_path.display()))?;
    let key_pem =
        fs::read(&key_path).with_context(|| format!("read CA key: {}", key_path.display()))?;

    Ok((ca_pem, key_pem))
}

/// Remove a CA (fails if domains use it).
pub fn remove_ca(paths: &RoostPaths, name: &str) -> Result<()> {
    let config = store::load_config(paths)?;
    for (_domain, ca) in &config.domains {
        if ca == name {
            anyhow::bail!("cannot remove CA '{}': domain '{}' uses it", name, _domain);
        }
    }
    let ca_dir = paths.ca_dir.join(name);
    if ca_dir.is_dir() {
        fs::remove_dir_all(&ca_dir)?;
    }
    Ok(())
}

/// Check if CA exists.
pub fn ca_exists(paths: &RoostPaths, name: &str) -> bool {
    let dir = paths.ca_dir.join(name);
    dir.is_dir() && dir.join("ca.pem").is_file() && dir.join("ca-key.pem").is_file()
}
