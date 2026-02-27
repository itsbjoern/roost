//! CA creation in temp dir.

mod common;

use roost::ca;
use roost::config::RoostPaths;
use std::fs;

#[test]
fn create_ca_produces_pem_files() {
    let dir = common::temp_roost_home();
    let paths = RoostPaths::for_test(dir.path());

    ca::create_ca(&paths, "testca").unwrap();

    let ca_path = paths.ca_dir.join("testca").join("ca.pem");
    let key_path = paths.ca_dir.join("testca").join("ca-key.pem");

    assert!(ca_path.is_file());
    assert!(key_path.is_file());

    let ca_content = fs::read_to_string(&ca_path).unwrap();
    let key_content = fs::read_to_string(&key_path).unwrap();

    assert!(ca_content.contains("-----BEGIN CERTIFICATE-----"));
    assert!(ca_content.contains("-----END CERTIFICATE-----"));
    assert!(key_content.contains("-----BEGIN") && key_content.contains("-----END"));
}
