//! Add/remove domain in temp hosts file; 127.0.0.1 and ::1.

mod common;

use roost::config::RoostPaths;
use roost::hosts;
use roost::platform::FileHostsEditor;
use std::fs;

#[test]
fn add_remove_domain_in_temp_hosts() {
    let dir = common::temp_roost_home();
    let hosts_path = dir.path().join("hosts");
    fs::write(&hosts_path, "127.0.0.1\tlocalhost\n").unwrap();

    let editor = FileHostsEditor::new(&hosts_path);

    hosts::add_domain_to_hosts(&editor, "api.test").unwrap();

    let content = fs::read_to_string(&hosts_path).unwrap();
    assert!(content.contains("127.0.0.1\tapi.test"));
    assert!(content.contains("::1\tapi.test"));

    hosts::remove_domain_from_hosts(&editor, "api.test").unwrap();

    let content = fs::read_to_string(&hosts_path).unwrap();
    assert!(!content.contains("api.test"));
}
