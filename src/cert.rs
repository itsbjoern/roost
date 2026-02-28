//! Certificate generation and renewal.

use anyhow::{Context, Result};
use rcgen::{CertificateParams, KeyPair};
use std::fs;
use std::io::Write;
use std::path::Path;
use x509_parser::prelude::FromDer;

use crate::config::RoostPaths;

/// Generate domain cert; SANs = [domain, *.domain] or [domain] if exact.
pub fn generate_domain_cert(
    domain: &str,
    ca_pem: &[u8],
    ca_key_pem: &[u8],
    exact: bool,
) -> Result<(Vec<u8>, Vec<u8>)> {
    let ca_str = String::from_utf8(ca_pem.to_vec())?;
    let ca_key_str = String::from_utf8(ca_key_pem.to_vec())?;

    let issuer_params =
        CertificateParams::from_ca_cert_pem(&ca_str).context("parse CA cert")?;
    let issuer_key = KeyPair::from_pem(&ca_key_str).context("parse CA key")?;
    let issuer_cert = issuer_params.self_signed(&issuer_key).context("reconstruct issuer cert")?;

    let subject_key = KeyPair::generate().context("generate domain key")?;

    let subject_alt_names: Vec<String> = if exact {
        vec![domain.to_string()]
    } else {
        vec![domain.to_string(), format!("*.{}", domain)]
    };

    let mut params =
        CertificateParams::new(subject_alt_names).context("create cert params")?;
    params.distinguished_name = rcgen::DistinguishedName::new();
    params.distinguished_name.push(
        rcgen::DnType::CommonName,
        rcgen::DnValue::Utf8String(domain.to_string()),
    );
    params.is_ca = rcgen::IsCa::NoCa;

    let cert = params
        .signed_by(&subject_key, &issuer_cert, &issuer_key)
        .context("sign domain cert")?;

    let cert_pem = cert.pem();
    let key_pem = subject_key.serialize_pem();

    Ok((cert_pem.into_bytes(), key_pem.into_bytes()))
}

/// Generate domain cert that expires in `validity_days` days. For testing renewal.
#[doc(hidden)]
pub fn generate_domain_cert_with_validity(
    domain: &str,
    ca_pem: &[u8],
    ca_key_pem: &[u8],
    exact: bool,
    validity_days: u32,
) -> Result<(Vec<u8>, Vec<u8>)> {
    let ca_str = String::from_utf8(ca_pem.to_vec())?;
    let ca_key_str = String::from_utf8(ca_key_pem.to_vec())?;

    let issuer_params =
        CertificateParams::from_ca_cert_pem(&ca_str).context("parse CA cert")?;
    let issuer_key = KeyPair::from_pem(&ca_key_str).context("parse CA key")?;
    let issuer_cert = issuer_params.self_signed(&issuer_key).context("reconstruct issuer cert")?;

    let subject_key = KeyPair::generate().context("generate domain key")?;

    let subject_alt_names: Vec<String> = if exact {
        vec![domain.to_string()]
    } else {
        vec![domain.to_string(), format!("*.{}", domain)]
    };

    let mut params =
        CertificateParams::new(subject_alt_names).context("create cert params")?;
    params.distinguished_name = rcgen::DistinguishedName::new();
    params.distinguished_name.push(
        rcgen::DnType::CommonName,
        rcgen::DnValue::Utf8String(domain.to_string()),
    );
    params.is_ca = rcgen::IsCa::NoCa;

    let now = time::OffsetDateTime::now_utc();
    params.not_after = now.saturating_add(time::Duration::days(validity_days as i64));

    let cert = params
        .signed_by(&subject_key, &issuer_cert, &issuer_key)
        .context("sign domain cert")?;

    let cert_pem = cert.pem();
    let key_pem = subject_key.serialize_pem();

    Ok((cert_pem.into_bytes(), key_pem.into_bytes()))
}

/// Save domain cert and key to store.
pub fn save_domain_cert(
    paths: &RoostPaths,
    domain: &str,
    cert_pem: &[u8],
    key_pem: &[u8],
) -> Result<()> {
    crate::store::ensure_dirs(paths)?;
    let cert_path = paths.certs_dir.join(format!("{domain}.pem"));
    let key_path = paths.certs_dir.join(format!("{domain}-key.pem"));

    let mut f = fs::File::create(&cert_path)?;
    f.write_all(cert_pem)?;

    let mut f = fs::File::create(&key_path)?;
    f.write_all(key_pem)?;

    Ok(())
}

/// Load domain cert and key.
pub fn load_domain_cert(paths: &RoostPaths, domain: &str) -> Result<(Vec<u8>, Vec<u8>)> {
    let cert_path = paths.certs_dir.join(format!("{domain}.pem"));
    let key_path = paths.certs_dir.join(format!("{domain}-key.pem"));

    let cert = fs::read(&cert_path)
        .with_context(|| format!("read cert: {}", cert_path.display()))?;
    let key = fs::read(&key_path)
        .with_context(|| format!("read key: {}", key_path.display()))?;

    Ok((cert, key))
}

/// Check if cert expires within N days.
pub fn cert_expires_within_days(path: &Path, days: u32) -> Result<bool> {
    let pem = fs::read_to_string(path)?;
    let cert_der = rustls_pemfile::certs(&mut pem.as_bytes())
        .next()
        .and_then(|r| r.ok())
        .context("parse cert PEM")?;

    let (_, cert) = x509_parser::prelude::X509Certificate::from_der(cert_der.as_ref())
        .map_err(|e| anyhow::anyhow!("parse X.509: {e:?}"))?;

    let now = time::OffsetDateTime::now_utc();
    let validity = cert.validity();
    let expiry_ts = validity.not_after.timestamp();
    let expiry_ot = time::OffsetDateTime::from_unix_timestamp(expiry_ts)
        .map_err(|e| anyhow::anyhow!("invalid expiry: {e:?}"))?;

    let threshold = now + time::Duration::days(days as i64);
    Ok(expiry_ot < threshold)
}

/// Ensure cert is valid; regenerate if missing or expiry < 30 days.
pub fn ensure_cert_valid(
    paths: &RoostPaths,
    domain: &str,
    ca_name: &str,
    exact: bool,
) -> Result<()> {
    let cert_path = paths.certs_dir.join(format!("{domain}.pem"));

    let needs_regen = if cert_path.is_file() {
        cert_expires_within_days(&cert_path, 30)?
    } else {
        true
    };

    if needs_regen {
        let (ca_pem, ca_key_pem) = crate::ca::load_ca(paths, ca_name)?;
        let (cert_pem, key_pem) = generate_domain_cert(domain, &ca_pem, &ca_key_pem, exact)?;
        save_domain_cert(paths, domain, &cert_pem, &key_pem)?;
    }

    Ok(())
}
