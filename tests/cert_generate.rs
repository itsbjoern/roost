//! Cert SANs: wildcard has domain+*.domain, exact has only domain.

mod common;

use roost::ca;
use roost::cert;
use roost::config::RoostPaths;
use x509_parser::extensions::GeneralName;
use x509_parser::prelude::FromDer;

fn get_sans(cert_pem: &[u8]) -> Vec<String> {
    let mut cursor = std::io::Cursor::new(cert_pem);
    let cert_der = rustls_pemfile::certs(&mut cursor)
        .next()
        .and_then(|r| r.ok())
        .unwrap();
    let (_, x509) = x509_parser::prelude::X509Certificate::from_der(cert_der.as_ref()).unwrap();
    let mut sans = Vec::new();
    if let Ok(Some(ext)) = x509.subject_alternative_name() {
        for name in ext.value.general_names.iter() {
            if let GeneralName::DNSName(s) = name {
                sans.push(s.to_string());
            }
        }
    }
    sans.sort();
    sans
}

#[test]
fn wildcard_cert_has_both_domain_and_star() {
    let dir = common::temp_roost_home();
    let paths = RoostPaths::for_test(dir.path());
    ca::create_ca(&paths, "default").unwrap();
    let (ca_pem, ca_key_pem) = ca::load_ca(&paths, "default").unwrap();

    let (cert_pem, _key_pem) =
        cert::generate_domain_cert("api.example.local", &ca_pem, &ca_key_pem, false).unwrap();

    let sans = get_sans(&cert_pem);
    assert_eq!(sans, vec!["*.api.example.local", "api.example.local"]);
}

#[test]
fn exact_cert_has_only_domain() {
    let dir = common::temp_roost_home();
    let paths = RoostPaths::for_test(dir.path());
    ca::create_ca(&paths, "default").unwrap();
    let (ca_pem, ca_key_pem) = ca::load_ca(&paths, "default").unwrap();

    let (cert_pem, _key_pem) =
        cert::generate_domain_cert("api.test", &ca_pem, &ca_key_pem, true).unwrap();

    let sans = get_sans(&cert_pem);
    assert_eq!(sans, vec!["api.test"]);
}
