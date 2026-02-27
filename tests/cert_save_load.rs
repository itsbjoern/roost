//! Save/load roundtrip; ensure_cert_valid creates when missing.

mod common;

use roost::ca;
use roost::cert;
use roost::config::RoostPaths;

#[test]
fn save_load_roundtrip() {
    let dir = common::temp_roost_home();
    let paths = RoostPaths::for_test(dir.path());
    ca::create_ca(&paths, "default").unwrap();
    let (ca_pem, ca_key_pem) = ca::load_ca(&paths, "default").unwrap();
    let (cert_pem, key_pem) =
        cert::generate_domain_cert("api.test", &ca_pem, &ca_key_pem, true).unwrap();

    cert::save_domain_cert(&paths, "api.test", &cert_pem, &key_pem).unwrap();

    let (loaded_cert, loaded_key) = cert::load_domain_cert(&paths, "api.test").unwrap();
    assert_eq!(loaded_cert, cert_pem);
    assert_eq!(loaded_key, key_pem);
}

#[test]
fn ensure_cert_valid_creates_when_missing() {
    let dir = common::temp_roost_home();
    let paths = RoostPaths::for_test(dir.path());
    ca::create_ca(&paths, "default").unwrap();

    cert::ensure_cert_valid(&paths, "newdomain.test", "default", false).unwrap();

    let (cert, key) = cert::load_domain_cert(&paths, "newdomain.test").unwrap();
    assert!(!cert.is_empty());
    assert!(!key.is_empty());
}
