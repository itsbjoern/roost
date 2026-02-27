//! Add mapping auto-adds domain and cert.

mod common;

use assert_cmd::Command;
use roost::ca;
use roost::config::RoostPaths;
use roost::store;
use std::fs;

#[test]
fn config_add_auto_adds_domain_and_cert() {
    let dir = common::temp_roost_home();
    let paths = RoostPaths::for_test(dir.path());
    let hosts_path = dir.path().join("hosts");
    fs::write(&hosts_path, "").unwrap();

    common::with_test_env(dir.path(), || {
        std::env::set_var("ROOST_HOSTS_FILE", hosts_path.to_str().unwrap());

        // Init: create CA, config
        ca::create_ca(&paths, "default").unwrap();
        store::ensure_dirs(&paths).unwrap();
        let mut config = store::load_config(&paths).unwrap();
        config.default_ca = "default".to_string();
        store::save_config(&paths, &config).unwrap();

        // Add mapping for unregistered domain via CLI
        let mut cmd = Command::cargo_bin("roost").unwrap();
        cmd.current_dir(dir.path())
            .args(["serve", "config", "add", "api.test", "5001"])
            .assert()
            .success();

        // Domain should now exist in config
        let config = store::load_config(&paths).unwrap();
        assert!(
            config.domains.contains_key("api.test"),
            "domain should be auto-added"
        );

        // Cert should exist
        assert!(
            paths.certs_dir.join("api.test.pem").is_file(),
            "cert should be created"
        );
        assert!(
            paths.certs_dir.join("api.test-key.pem").is_file(),
            "key should be created"
        );

        // Mapping should be in project .roostrc
        let rc_path = dir.path().join(".roostrc");
        assert!(rc_path.is_file(), ".roostrc should be created");
        let content = fs::read_to_string(&rc_path).unwrap();
        assert!(content.contains("api.test"));
        assert!(content.contains("5001"));
        let _ = std::env::remove_var("ROOST_HOSTS_FILE");
    });
}
