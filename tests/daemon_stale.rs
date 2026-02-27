//! Stale PID clears state.

mod common;

use roost::config::RoostPaths;
use roost::serve::daemon::daemon_status;
use std::fs;

#[test]
fn stale_pid_clears_state() {
    let dir = common::temp_roost_home();
    let paths = RoostPaths::for_test(dir.path());

    common::with_test_env(dir.path(), || {
        std::fs::create_dir_all(&paths.config_dir).unwrap();
        let daemon_json = paths.config_dir.join("daemon.json");
        // Write state with PID that doesn't exist (99999999)
        fs::write(
            &daemon_json,
            r#"{"pid":99999999,"project_path":null,"started_at":"2025-01-01T00:00:00Z"}"#,
        )
        .unwrap();

        let status = daemon_status(&paths).unwrap();
        assert!(status.is_none(), "stale PID should clear state");
        assert!(!daemon_json.is_file(), "stale state should be cleared");
    });
}
