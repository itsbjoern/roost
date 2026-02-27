//! cert_expires_within_days true/false for near/far expiry.

mod common;

use roost::ca;
use x509_parser::prelude::FromDer;
use roost::cert;
use roost::config::RoostPaths;
use std::fs;
use std::path::Path;

fn cert_expiry_days(path: &Path) -> i64 {
    let pem = fs::read_to_string(path).unwrap();
    let cert_der = rustls_pemfile::certs(&mut pem.as_bytes())
        .next()
        .and_then(|r| r.ok())
        .unwrap();
    let (_, x509) = x509_parser::prelude::X509Certificate::from_der(cert_der.as_ref()).unwrap();
    let validity = x509.validity();
    let now = time::OffsetDateTime::now_utc();
    let expiry = time::OffsetDateTime::from_unix_timestamp(validity.not_after.timestamp()).unwrap();
    (expiry - now).whole_days()
}

#[test]
fn cert_expires_within_days_near_expiry() {
    let dir = common::temp_roost_home();
    let paths = RoostPaths::for_test(dir.path());
    ca::create_ca(&paths, "default").unwrap();
    let (ca_pem, ca_key_pem) = ca::load_ca(&paths, "default").unwrap();
    let (cert_pem, key_pem) =
        cert::generate_domain_cert("api.test", &ca_pem, &ca_key_pem, true).unwrap();
    cert::save_domain_cert(&paths, "api.test", &cert_pem, &key_pem).unwrap();

    let cert_path = paths.certs_dir.join("api.test.pem");
    let days_left = cert_expiry_days(&cert_path);
    // rcgen default validity is until 4096, so cert is valid for many years
    assert!(days_left > 365 * 10, "cert should be valid for many years");

    // Huge threshold (1M days): cert expires within that -> true
    let expires_soon = cert::cert_expires_within_days(&cert_path, 1_000_000).unwrap();
    assert!(expires_soon);
}

#[test]
fn cert_expires_within_days_far_expiry() {
    let dir = common::temp_roost_home();
    let paths = RoostPaths::for_test(dir.path());
    ca::create_ca(&paths, "default").unwrap();
    let (ca_pem, ca_key_pem) = ca::load_ca(&paths, "default").unwrap();
    let (cert_pem, key_pem) =
        cert::generate_domain_cert("api.test", &ca_pem, &ca_key_pem, true).unwrap();
    cert::save_domain_cert(&paths, "api.test", &cert_pem, &key_pem).unwrap();

    let cert_path = paths.certs_dir.join("api.test.pem");

    // 5 days threshold: cert expires in ~365 days, NOT within 5 days -> false
    let expires_soon = cert::cert_expires_within_days(&cert_path, 5).unwrap();
    assert!(!expires_soon);
}
