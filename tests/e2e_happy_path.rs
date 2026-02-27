//! E2E: init -> add -> list -> get-cert -> remove -> list empty.

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

        // get-cert (parseable output)
        let out = Command::cargo_bin("roost")
            .unwrap()
            .current_dir(project_dir)
            .args(["domain", "get-cert", "api.example.test"])
            .assert()
            .success();
        let stdout = String::from_utf8_lossy(&out.get_output().stdout);
        assert!(stdout.contains("cert:"));
        assert!(stdout.contains("key:"));
        assert!(stdout.contains("api.example.test"));

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
