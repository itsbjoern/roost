//! roost init exit code 0.

mod common;

use assert_cmd::Command;

#[test]
fn cli_init_exit_0() {
    let dir = common::temp_roost_home();
    common::with_test_env(dir.path(), || {
        std::env::set_var("ROOST_SKIP_TRUST_INSTALL", "1");
        Command::cargo_bin("roost")
            .unwrap()
            .arg("init")
            .assert()
            .success();
        let _ = std::env::remove_var("ROOST_SKIP_TRUST_INSTALL");
    });
}
