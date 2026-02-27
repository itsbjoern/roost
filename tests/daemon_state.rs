//! Write/read/clear daemon.json.

mod common;

use roost::config::RoostPaths;
use roost::serve::daemon::daemon_status;
use std::fs;

#[test]
fn daemon_status_none_when_no_file() {
    let dir = common::temp_roost_home();
    let paths = RoostPaths::for_test(dir.path());

    common::with_test_env(dir.path(), || {
        let status = daemon_status(&paths).unwrap();
        assert!(status.is_none());
    });
}
