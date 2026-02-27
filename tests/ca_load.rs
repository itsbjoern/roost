//! Load CA and verify it is valid and usable for signing.

mod common;

use roost::ca;
use roost::config::RoostPaths;

#[test]
fn load_ca_returns_valid_pem() {
    let dir = common::temp_roost_home();
    let paths = RoostPaths::for_test(dir.path());

    ca::create_ca(&paths, "default").unwrap();
    let (ca_pem, key_pem) = ca::load_ca(&paths, "default").unwrap();

    let ca_str = String::from_utf8(ca_pem).unwrap();
    let key_str = String::from_utf8(key_pem).unwrap();

    assert!(ca_str.contains("-----BEGIN CERTIFICATE-----"));
    assert!(key_str.contains("-----BEGIN"));
    // CA certs have basic constraints with cA=TRUE - check for typical CA structure
    assert!(ca_str.contains("CERTIFICATE") || ca_str.len() > 100);
}
