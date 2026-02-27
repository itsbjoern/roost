//! set-ca re-signs cert.

mod common;

use roost::ca;
use roost::config::RoostPaths;
use roost::domain;
use roost::store;

#[test]
fn set_ca_re_signs_cert() {
    let dir = common::temp_roost_home();
    let paths = RoostPaths::for_test(dir.path());

    ca::create_ca(&paths, "default").unwrap();
    ca::create_ca(&paths, "custom").unwrap();
    store::ensure_dirs(&paths).unwrap();

    let mut config = store::load_config(&paths).unwrap();
    config.default_ca = "default".to_string();

    domain::add_domain(&paths, &mut config, "api.test", false, None).unwrap();
    store::save_config(&paths, &config).unwrap();

    let cert_before = std::fs::read(paths.certs_dir.join("api.test.pem")).unwrap();

    domain::set_ca(&paths, &mut config, "api.test", "custom").unwrap();
    store::save_config(&paths, &config).unwrap();

    let cert_after = std::fs::read(paths.certs_dir.join("api.test.pem")).unwrap();
    assert_ne!(cert_before, cert_after, "cert should change when CA changes");
    assert_eq!(config.domains.get("api.test"), Some(&"custom".to_string()));
}
