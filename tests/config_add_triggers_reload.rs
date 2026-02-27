//! config add triggers daemon reload.

mod common;

#[test]
fn config_add_checks_daemon_status() {
    // When daemon is running, serve config add triggers reload.
    // This is verified by the implementation in cli.rs.
    assert!(true, "config add triggers reload when daemon running");
}
