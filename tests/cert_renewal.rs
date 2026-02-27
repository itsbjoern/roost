//! Cert near expiry regenerated on domain command.

mod common;

use roost::ca;
use roost::cert;
use roost::config::RoostPaths;
use roost::domain;
use roost::platform::FileHostsEditor;
use roost::store;
use std::fs;

#[test]
fn cert_near_expiry_regenerated_on_add() {
    let dir = common::temp_roost_home();
    let paths = RoostPaths::for_test(dir.path());
    let hosts_path = dir.path().join("hosts");
    fs::write(&hosts_path, "").unwrap();

    ca::create_ca(&paths, "default").unwrap();
    store::ensure_dirs(&paths).unwrap();

    let mut config = store::load_config(&paths).unwrap();
    config.default_ca = "default".to_string();
    store::save_config(&paths, &config).unwrap();

    let editor = FileHostsEditor::new(&hosts_path);

    // Add domain (creates cert with long validity)
    domain::add_domain(&paths, &mut config, "api.test", false, Some(&editor)).unwrap();
    store::save_config(&paths, &config).unwrap();

    let cert_path = paths.certs_dir.join("api.test.pem");
    let cert_before = fs::read(&cert_path).unwrap();

    // Overwrite with cert that expires in 5 days (< 30 day threshold)
    let (ca_pem, ca_key_pem) = ca::load_ca(&paths, "default").unwrap();
    let (cert_pem, key_pem) = cert::generate_domain_cert_with_validity(
        "api.test",
        &ca_pem,
        &ca_key_pem,
        false,
        5,
    )
    .unwrap();
    cert::save_domain_cert(&paths, "api.test", &cert_pem, &key_pem).unwrap();

    // Trigger ensure_cert_valid via add_domain (idempotent - domain already exists)
    // add_domain always calls ensure_cert_valid
    domain::add_domain(&paths, &mut config, "api.test", false, Some(&editor)).unwrap();

    let cert_after = fs::read(&cert_path).unwrap();

    // Cert should have been regenerated (different content, longer validity)
    assert_ne!(
        cert_before, cert_after,
        "Cert should have been regenerated when near expiry"
    );
}
