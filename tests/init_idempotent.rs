//! Run init twice; no overwrite.

mod common;

use assert_cmd::Command;
use roost::ca;
use roost::config::RoostPaths;
use roost::store;
use std::fs;

#[test]
fn init_idempotent() {
    let dir = common::temp_roost_home();
    let paths = RoostPaths::for_test(dir.path());

    common::with_test_env(dir.path(), || {
        std::env::set_var("ROOST_SKIP_TRUST_INSTALL", "1");
        Command::cargo_bin("roost").unwrap().arg("init").assert().success();
        let ca_before = fs::read(paths.ca_dir.join("default").join("ca.pem")).unwrap();
        Command::cargo_bin("roost").unwrap().arg("init").assert().success();
        let ca_after = fs::read(paths.ca_dir.join("default").join("ca.pem")).unwrap();
        let _ = std::env::remove_var("ROOST_SKIP_TRUST_INSTALL");
        assert_eq!(ca_before, ca_after, "second init should not overwrite CA");
    });
}
