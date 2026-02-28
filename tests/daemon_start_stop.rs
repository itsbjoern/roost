//! Start and stop daemon.

mod common;

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn daemon_start_stop() {
    let dir = common::temp_roost_home();
    let hosts_path = dir.path().join("hosts");
    std::fs::write(&hosts_path, "").unwrap();
    let project_dir = dir.path();

    common::with_test_env(project_dir, || {
        std::env::set_var("ROOST_SKIP_TRUST_INSTALL", "1");
        std::env::set_var("ROOST_HOSTS_FILE", hosts_path.to_str().unwrap());

        let mut init_cmd = Command::cargo_bin("roost").unwrap();
        init_cmd.current_dir(project_dir).arg("init").assert().success();

        // Add a domain and mapping so proxy can start
        Command::cargo_bin("roost")
            .unwrap()
            .current_dir(project_dir)
            .args(["domain", "add", "api.test"])
            .assert()
            .success();
        Command::cargo_bin("roost")
            .unwrap()
            .current_dir(project_dir)
            .args(["serve", "config", "add", "api.test", "8080"])
            .assert()
            .success();

        // Use non-privileged port (80/443 need root)
        Command::cargo_bin("roost")
            .unwrap()
            .current_dir(project_dir)
            .args(["serve", "config", "ports", "set", "18443"])
            .assert()
            .success();

        // Start daemon
        Command::cargo_bin("roost")
            .unwrap()
            .current_dir(project_dir)
            .args(["serve", "daemon", "start"])
            .assert()
            .success();

        std::thread::sleep(std::time::Duration::from_millis(500));

        // Status should show running
        Command::cargo_bin("roost")
            .unwrap()
            .current_dir(project_dir)
            .args(["serve", "daemon", "status"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Daemon running"));

        // Stop daemon
        Command::cargo_bin("roost")
            .unwrap()
            .current_dir(project_dir)
            .args(["serve", "daemon", "stop"])
            .assert()
            .success();

        // Status should show not running
        Command::cargo_bin("roost")
            .unwrap()
            .current_dir(project_dir)
            .args(["serve", "daemon", "status"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Daemon not running"));

        let _ = std::env::remove_var("ROOST_SKIP_TRUST_INSTALL");
        let _ = std::env::remove_var("ROOST_HOSTS_FILE");
    });
}
