//! All subcommands have help.

use assert_cmd::Command;

fn roost() -> Command {
    Command::cargo_bin("roost").unwrap()
}

#[test]
fn help_main() {
    roost().arg("--help").assert().success();
}

#[test]
fn help_init() {
    roost().args(["init", "--help"]).assert().success();
}

#[test]
fn help_ca() {
    roost().args(["ca", "--help"]).assert().success();
}

#[test]
fn help_ca_list() {
    roost().args(["ca", "list", "--help"]).assert().success();
}

#[test]
fn help_ca_create() {
    roost().args(["ca", "create", "--help"]).assert().success();
}

#[test]
fn help_ca_remove() {
    roost().args(["ca", "remove", "--help"]).assert().success();
}

#[test]
fn help_ca_install() {
    roost().args(["ca", "install", "--help"]).assert().success();
}

#[test]
fn help_ca_uninstall() {
    roost().args(["ca", "uninstall", "--help"]).assert().success();
}

#[test]
fn help_domain() {
    roost().args(["domain", "--help"]).assert().success();
}

#[test]
fn help_domain_list() {
    roost().args(["domain", "list", "--help"]).assert().success();
}

#[test]
fn help_domain_add() {
    roost().args(["domain", "add", "--help"]).assert().success();
}

#[test]
fn help_domain_remove() {
    roost().args(["domain", "remove", "--help"]).assert().success();
}

#[test]
fn help_domain_set_ca() {
    roost().args(["domain", "set-ca", "--help"]).assert().success();
}

#[test]
fn help_domain_get_path() {
    roost().args(["domain", "get-path", "--help"]).assert().success();
}

#[test]
fn help_serve() {
    roost().args(["serve", "--help"]).assert().success();
}

#[test]
fn help_serve_config() {
    roost().args(["serve", "config", "--help"]).assert().success();
}

#[test]
fn help_serve_config_add() {
    roost().args(["serve", "config", "add", "--help"]).assert().success();
}

#[test]
fn help_serve_config_remove() {
    roost().args(["serve", "config", "remove", "--help"]).assert().success();
}

#[test]
fn help_serve_config_list() {
    roost().args(["serve", "config", "list", "--help"]).assert().success();
}

#[test]
fn help_serve_config_ports() {
    roost().args(["serve", "config", "ports", "--help"]).assert().success();
}

#[test]
fn help_serve_config_ports_add() {
    roost().args(["serve", "config", "ports", "add", "--help"]).assert().success();
}

#[test]
fn help_serve_config_ports_remove() {
    roost().args(["serve", "config", "ports", "remove", "--help"]).assert().success();
}

#[test]
fn help_serve_config_ports_set() {
    roost().args(["serve", "config", "ports", "set", "--help"]).assert().success();
}

#[test]
fn help_serve_config_ports_list() {
    roost().args(["serve", "config", "ports", "list", "--help"]).assert().success();
}

#[test]
fn help_serve_daemon() {
    roost().args(["serve", "daemon", "--help"]).assert().success();
}

#[test]
fn help_serve_daemon_start() {
    roost().args(["serve", "daemon", "start", "--help"]).assert().success();
}

#[test]
fn help_serve_daemon_stop() {
    roost().args(["serve", "daemon", "stop", "--help"]).assert().success();
}

#[test]
fn help_serve_daemon_status() {
    roost().args(["serve", "daemon", "status", "--help"]).assert().success();
}

#[test]
fn help_serve_daemon_reload() {
    roost().args(["serve", "daemon", "reload", "--help"]).assert().success();
}
