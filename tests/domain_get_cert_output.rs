//! get-path returns cert and key paths (one per invocation).

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
            .args(["domain", "get-path", "cert", "api.test"])
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
            .args(["domain", "get-path", "key", "api.test"])
            .output()
            .unwrap();
        assert!(out.status.success());
        let stdout = String::from_utf8(out.stdout).unwrap();
        let path = stdout.trim();
        assert!(path.ends_with("api.test-key.pem"), "got: {path}");
    });
}
