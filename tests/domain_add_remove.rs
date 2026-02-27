//! Full add/remove flow; config and cert files.

mod common;

use roost::ca;
use roost::config::RoostPaths;
use roost::domain;
use roost::platform::FileHostsEditor;
use roost::store;
use std::fs;

#[test]
fn add_remove_domain_full_flow() {
    let dir = common::temp_roost_home();
    let paths = RoostPaths::for_test(dir.path());
    let hosts_path = dir.path().join("hosts");
    fs::write(&hosts_path, "").unwrap();

    ca::create_ca(&paths, "default").unwrap();
    store::ensure_dirs(&paths).unwrap();

    let mut config = store::load_config(&paths).unwrap();
    let editor = FileHostsEditor::new(&hosts_path);

    domain::add_domain(&paths, &mut config, "api.test", false, Some(&editor)).unwrap();
    store::save_config(&paths, &config).unwrap();

    assert!(config.domains.contains_key("api.test"));
    assert!(paths.certs_dir.join("api.test.pem").is_file());
    assert!(paths.certs_dir.join("api.test-key.pem").is_file());
    let hosts_content = fs::read_to_string(&hosts_path).unwrap();
    assert!(hosts_content.contains("api.test"));

    domain::remove_domain(&paths, &mut config, "api.test", Some(&editor)).unwrap();
    store::save_config(&paths, &config).unwrap();

    assert!(!config.domains.contains_key("api.test"));
    assert!(!paths.certs_dir.join("api.test.pem").is_file());
    assert!(!paths.certs_dir.join("api.test-key.pem").is_file());
}
