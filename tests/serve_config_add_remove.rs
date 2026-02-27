//! Add/remove mappings.

mod common;

use roost::serve::config::ServeConfig;

#[test]
fn add_remove_mappings() {
    let dir = common::temp_roost_home();
    let rc_path = dir.path().join("test.roostrc");

    let mut cfg = ServeConfig::default();
    cfg.add("api.test".into(), 5001);
    cfg.add("app.test".into(), 3000);
    cfg.save(&rc_path).unwrap();

    let loaded = ServeConfig::load(&rc_path).unwrap();
    let list = loaded.list();
    assert_eq!(list.len(), 2);
    assert!(list.iter().any(|(d, p)| *d == "api.test" && *p == 5001));
    assert!(list.iter().any(|(d, p)| *d == "app.test" && *p == 3000));

    let mut cfg2 = ServeConfig::load(&rc_path).unwrap();
    cfg2.remove("api.test");
    cfg2.save(&rc_path).unwrap();

    let loaded2 = ServeConfig::load(&rc_path).unwrap();
    let list2 = loaded2.list();
    assert_eq!(list2.len(), 1);
    assert_eq!(list2[0], ("app.test", 3000));
}

#[test]
fn add_same_domain_replaces() {
    let dir = common::temp_roost_home();
    let rc_path = dir.path().join("test.roostrc");

    let mut cfg = ServeConfig::default();
    cfg.add("api.test".into(), 5001);
    cfg.add("api.test".into(), 5002);
    assert_eq!(cfg.list().len(), 1);
    assert_eq!(cfg.list()[0], ("api.test", 5002));
}
