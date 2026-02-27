//! SIGHUP reloads config.

mod common;

use assert_cmd::Command;

#[test]
fn reload_when_not_running_fails_gracefully() {
    let dir = common::temp_roost_home();
    common::with_test_env(dir.path(), || {
        Command::cargo_bin("roost")
            .unwrap()
            .args(["serve", "daemon", "reload"])
            .assert()
            .failure()
            .stderr(predicates::str::contains("not running"));
    });
}
