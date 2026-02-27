//! CA remove: succeeds when unused, fails when domain uses it.

mod common;

use roost::ca;
use roost::config::{Config, RoostPaths};
use roost::store;
#[test]
fn remove_unused_ca_succeeds() {
    let dir = common::temp_roost_home();
    let paths = RoostPaths::for_test(dir.path());

    ca::create_ca(&paths, "toremove").unwrap();
    assert!(ca::ca_exists(&paths, "toremove"));

    ca::remove_ca(&paths, "toremove").unwrap();
    assert!(!ca::ca_exists(&paths, "toremove"));
}

#[test]
fn remove_ca_with_domain_fails() {
    let dir = common::temp_roost_home();
    let paths = RoostPaths::for_test(dir.path());

    ca::create_ca(&paths, "inuse").unwrap();
    store::ensure_dirs(&paths).unwrap();

    let mut config = Config::default();
    config.default_ca = "inuse".to_string();
    config.domains.insert("api.test".to_string(), "inuse".to_string());
    config.save(&paths).unwrap();

    let err = ca::remove_ca(&paths, "inuse").unwrap_err();
    assert!(err.to_string().contains("api.test"));
    assert!(err.to_string().contains("inuse"));
}
