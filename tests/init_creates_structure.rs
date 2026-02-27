//! ROOST_HOME temp dir; ca/default/, certs/, config.toml exist.

mod common;

use assert_cmd::Command;
use roost::config::RoostPaths;

#[test]
fn init_creates_structure() {
    let dir = common::temp_roost_home();
    let paths = RoostPaths::for_test(dir.path());

    common::with_test_env(dir.path(), || {
        std::env::set_var("ROOST_SKIP_TRUST_INSTALL", "1");
        let result = Command::cargo_bin("roost")
            .unwrap()
            .arg("init")
            .output();
        let _ = std::env::remove_var("ROOST_SKIP_TRUST_INSTALL");
        result.unwrap();
    });

    assert!(paths.ca_dir.join("default").is_dir());
    assert!(paths.ca_dir.join("default").join("ca.pem").is_file());
    assert!(paths.certs_dir.is_dir());
    assert!(paths.config_file.is_file());
}
