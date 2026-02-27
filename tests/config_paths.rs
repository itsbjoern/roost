//! Verify config_path(), ca_dir, certs_dir resolve correctly under ROOST_HOME.

use roost::config::RoostPaths;
use std::path::Path;

mod common;

#[test]
fn paths_resolve_under_base() {
    let dir = common::temp_roost_home();
    let base = dir.path();
    let paths = RoostPaths::for_test(base);

    assert_eq!(paths.config_dir, base);
    assert!(paths.config_file.ends_with("config.toml"));
    assert!(paths.ca_dir.ends_with("ca"));
    assert!(paths.certs_dir.ends_with("certs"));
    assert!(paths.roostrc_global.ends_with(".roostrc"));

    assert!(paths.config_file.starts_with(base));
    assert!(paths.ca_dir.starts_with(base));
    assert!(paths.certs_dir.starts_with(base));
}

#[test]
fn config_path_uses_roost_home() {
    let dir = common::temp_roost_home();
    let base = dir.path();

    common::with_test_env(base, || {
        let path = roost::config::config_path();
        assert!(path.starts_with(base), "config_path should be under ROOST_HOME");
        assert!(path.ends_with("config.toml"));
    });
}
