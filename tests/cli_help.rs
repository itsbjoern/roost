//! CLI help strings succeed.

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn roost_help() {
    Command::cargo_bin("roost").unwrap().arg("--help").assert().success();
}

#[test]
fn roost_ca_help() {
    Command::cargo_bin("roost")
        .unwrap()
        .args(["ca", "--help"])
        .assert()
        .success();
}

#[test]
fn roost_domain_help() {
    Command::cargo_bin("roost")
        .unwrap()
        .args(["domain", "--help"])
        .assert()
        .success();
}

#[test]
fn roost_serve_help() {
    Command::cargo_bin("roost")
        .unwrap()
        .args(["serve", "--help"])
        .assert()
        .success();
}
