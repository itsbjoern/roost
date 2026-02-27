//! Config save/load roundtrip.

use std::collections::HashMap;
use std::fs;

mod common;

use roost::config::{Config, RoostPaths};

#[test]
fn config_roundtrip() {
    let dir = common::temp_roost_home();
    let paths = RoostPaths::for_test(dir.path());

    let mut domains = HashMap::new();
    domains.insert("api.example.local".to_string(), "default".to_string());
    domains.insert("app.test".to_string(), "custom".to_string());

    let config = Config {
        default_ca: "default".to_string(),
        domains,
    };

    config.save(&paths).unwrap();
    assert!(paths.config_file.is_file());

    let loaded = Config::load(&paths).unwrap();
    assert_eq!(loaded.default_ca, "default");
    assert_eq!(loaded.domains.get("api.example.local"), Some(&"default".to_string()));
    assert_eq!(loaded.domains.get("app.test"), Some(&"custom".to_string()));
}
