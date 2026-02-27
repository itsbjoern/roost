//! Parseable cert: and key: lines.

mod common;

use assert_cmd::Command;
use roost::ca;
use roost::config::RoostPaths;
use roost::domain;
use roost::store;

#[test]
fn get_cert_output_parseable() {
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
            .args(["domain", "get-cert", "api.test"])
            .output()
            .unwrap();
        let stdout = String::from_utf8(out.stdout).unwrap();
        assert!(stdout.contains("cert:"));
        assert!(stdout.contains("key:"));
        let mut cert_path = None;
        let mut key_path = None;
        for line in stdout.lines() {
            if let Some(p) = line.strip_prefix("cert: ") {
                cert_path = Some(p.trim());
            }
            if let Some(p) = line.strip_prefix("key: ") {
                key_path = Some(p.trim());
            }
        }
        assert!(cert_path.is_some());
        assert!(key_path.is_some());
        assert!(cert_path.unwrap().ends_with("api.test.pem"));
        assert!(key_path.unwrap().ends_with("api.test-key.pem"));
    });
}
