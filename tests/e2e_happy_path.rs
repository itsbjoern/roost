//! E2E: init -> add -> list -> get-path -> remove -> list empty.

mod common;

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn e2e_happy_path() {
    let dir = common::temp_roost_home();
    let hosts_path = dir.path().join("hosts");
    std::fs::write(&hosts_path, "").unwrap();
    let project_dir = dir.path();

    common::with_test_env(project_dir, || {
        std::env::set_var("ROOST_SKIP_TRUST_INSTALL", "1");
        std::env::set_var("ROOST_HOSTS_FILE", hosts_path.to_str().unwrap());

        // init
        Command::cargo_bin("roost")
            .unwrap()
            .current_dir(project_dir)
            .arg("init")
            .assert()
            .success();

        // add
        Command::cargo_bin("roost")
            .unwrap()
            .current_dir(project_dir)
            .args(["domain", "add", "api.example.test"])
            .assert()
            .success();

        // list (should show api.example.test)
        Command::cargo_bin("roost")
            .unwrap()
            .current_dir(project_dir)
            .args(["domain", "list"])
            .assert()
            .success()
            .stdout(predicate::str::contains("api.example.test"));

        // get-path (parseable output)
        let cert_out = Command::cargo_bin("roost")
            .unwrap()
            .current_dir(project_dir)
            .args(["domain", "get-path", "cert", "api.example.test"])
            .output()
            .unwrap();
        assert!(cert_out.status.success());
        let cert_stdout = String::from_utf8_lossy(&cert_out.stdout);
        assert!(cert_stdout.trim().ends_with("api.example.test.pem"));
        let key_out = Command::cargo_bin("roost")
            .unwrap()
            .current_dir(project_dir)
            .args(["domain", "get-path", "key", "api.example.test"])
            .output()
            .unwrap();
        assert!(key_out.status.success());
        let key_stdout = String::from_utf8_lossy(&key_out.stdout);
        assert!(key_stdout.trim().ends_with("api.example.test-key.pem"));

        // remove
        Command::cargo_bin("roost")
            .unwrap()
            .current_dir(project_dir)
            .args(["domain", "remove", "api.example.test"])
            .assert()
            .success();

        // list (should be empty)
        Command::cargo_bin("roost")
            .unwrap()
            .current_dir(project_dir)
            .args(["domain", "list"])
            .assert()
            .success()
            .stdout(predicate::str::contains("api.example.test").not());

        let _ = std::env::remove_var("ROOST_SKIP_TRUST_INSTALL");
        let _ = std::env::remove_var("ROOST_HOSTS_FILE");
    });
}
