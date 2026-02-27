//! MockTrustStore records install/uninstall calls.

mod common;

use roost::platform::TrustStore;
use roost::trust;
use std::path::Path;
use std::sync::Mutex;

struct MockTrustStore {
    installed: Mutex<Vec<String>>,
    uninstalled: Mutex<Vec<String>>,
}

impl MockTrustStore {
    fn new() -> Self {
        Self {
            installed: Mutex::new(Vec::new()),
            uninstalled: Mutex::new(Vec::new()),
        }
    }

    fn installed(&self) -> Vec<String> {
        self.installed.lock().unwrap().clone()
    }

    fn uninstalled(&self) -> Vec<String> {
        self.uninstalled.lock().unwrap().clone()
    }
}

impl TrustStore for MockTrustStore {
    fn install_ca(&self, ca_pem_path: &Path) -> anyhow::Result<()> {
        self.installed
            .lock()
            .unwrap()
            .push(ca_pem_path.to_string_lossy().to_string());
        Ok(())
    }

    fn uninstall_ca(&self, ca_pem_path: &Path) -> anyhow::Result<()> {
        self.uninstalled
            .lock()
            .unwrap()
            .push(ca_pem_path.to_string_lossy().to_string());
        Ok(())
    }
}

#[test]
fn mock_store_records_install() {
    let dir = common::temp_roost_home();
    let paths = roost::config::RoostPaths::for_test(dir.path());
    roost::ca::create_ca(&paths, "default").unwrap();

    let store = MockTrustStore::new();
    let ca_path = paths.ca_dir.join("default").join("ca.pem");

    trust::install_ca_with_store(&store, &ca_path).unwrap();

    let installed = store.installed();
    assert_eq!(installed.len(), 1);
    assert!(installed[0].contains("ca.pem"));
}

#[test]
fn mock_store_records_uninstall() {
    let dir = common::temp_roost_home();
    let paths = roost::config::RoostPaths::for_test(dir.path());
    roost::ca::create_ca(&paths, "default").unwrap();

    let store = MockTrustStore::new();
    let ca_path = paths.ca_dir.join("default").join("ca.pem");

    trust::uninstall_ca_with_store(&store, &ca_path).unwrap();

    let uninstalled = store.uninstalled();
    assert_eq!(uninstalled.len(), 1);
    assert!(uninstalled[0].contains("ca.pem"));
}
