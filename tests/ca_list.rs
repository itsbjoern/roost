//! CA list returns created CAs.

mod common;

use roost::ca;
use roost::config::RoostPaths;

#[test]
fn list_cas_returns_both() {
    let dir = common::temp_roost_home();
    let paths = RoostPaths::for_test(dir.path());

    ca::create_ca(&paths, "default").unwrap();
    ca::create_ca(&paths, "custom").unwrap();

    let list = ca::list_cas(&paths).unwrap();
    assert_eq!(list, vec!["custom", "default"]);
}
