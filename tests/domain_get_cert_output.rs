//! domain path returns cert and key paths (one per invocation).

mod common;

use assert_cmd::Command;
use roost::ca;
use roost::config::RoostPaths;
use roost::domain;
use roost::store;

#[test]
fn get_path_cert() {
    let dir = common::temp_roost_home();
    let paths = RoostPaths::for_test(dir.path());

    ca::create_ca(&paths, "default").unwrap();
    store::ensure_dirs(&paths).unwrap();

    let mut config = store::load_config(&paths).unwrap();
    domain::add_domain(&paths, &mut config, "api.test", false, None).unwrap();
    store::save_config(&paths, &config).unwrap();

    common::with_test_env(dir.path(), || {
        let out = Command::cargo_bin("roost")
            .unwrap()
            .args(["domain", "path", "cert", "api.test"])
            .output()
            .unwrap();
        assert!(out.status.success());
        let stdout = String::from_utf8(out.stdout).unwrap();
        let path = stdout.trim();
        assert!(path.ends_with("api.test.pem"), "got: {path}");
    });
}

#[test]
fn get_path_key() {
    let dir = common::temp_roost_home();
    let paths = RoostPaths::for_test(dir.path());

    ca::create_ca(&paths, "default").unwrap();
    store::ensure_dirs(&paths).unwrap();

    let mut config = store::load_config(&paths).unwrap();
    domain::add_domain(&paths, &mut config, "api.test", false, None).unwrap();
    store::save_config(&paths, &config).unwrap();

    common::with_test_env(dir.path(), || {
        let out = Command::cargo_bin("roost")
            .unwrap()
            .args(["domain", "path", "key", "api.test"])
            .output()
            .unwrap();
        assert!(out.status.success());
        let stdout = String::from_utf8(out.stdout).unwrap();
        let path = stdout.trim();
        assert!(path.ends_with("api.test-key.pem"), "got: {path}");
    });
}

#[test]
fn path_with_generate_creates_domain() {
    let dir = common::temp_roost_home();
    let paths = RoostPaths::for_test(dir.path());
    let hosts_path = dir.path().join("hosts");
    std::fs::write(&hosts_path, "").unwrap();

    ca::create_ca(&paths, "default").unwrap();
    store::ensure_dirs(&paths).unwrap();
    let mut config = store::load_config(&paths).unwrap();
    config.default_ca = "default".to_string();
    store::save_config(&paths, &config).unwrap();

    common::with_test_env(dir.path(), || {
        std::env::set_var("ROOST_SKIP_TRUST_INSTALL", "1");
        std::env::set_var("ROOST_HOSTS_FILE", hosts_path.to_str().unwrap());

        // Domain not added yet; --generate should create it and return path
        let out = Command::cargo_bin("roost")
            .unwrap()
            .args(["domain", "path", "cert", "gen.test", "--generate"])
            .output()
            .unwrap();
        assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
        let stdout = String::from_utf8(out.stdout).unwrap();
        let path = stdout.trim();
        assert!(path.ends_with("gen.test.pem"), "got: {path}");

        let config_after = store::load_config(&paths).unwrap();
        assert!(config_after.domains.contains_key("gen.test"));
    });
}
