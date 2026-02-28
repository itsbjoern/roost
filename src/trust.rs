//! Trust store install/uninstall (platform abstraction).

use anyhow::Result;
use std::path::Path;

use crate::platform::{default_trust_store, TrustStore};

/// Install CA into system trust store.
pub fn install_ca(ca_pem_path: &Path) -> Result<()> {
    default_trust_store().install_ca(ca_pem_path)
}

/// Install CA using provided store (for testing).
pub fn install_ca_with_store(store: &dyn TrustStore, ca_pem_path: &Path) -> Result<()> {
    store.install_ca(ca_pem_path)
}

/// Remove CA from system trust store.
pub fn uninstall_ca(ca_pem_path: &Path) -> Result<()> {
    default_trust_store().uninstall_ca(ca_pem_path)
}

/// Remove CA using provided store (for testing).
pub fn uninstall_ca_with_store(store: &dyn TrustStore, ca_pem_path: &Path) -> Result<()> {
    store.uninstall_ca(ca_pem_path)
}

/// Check if CA is installed in system trust store.
pub fn is_ca_installed(ca_pem_path: &Path) -> Result<bool> {
    default_trust_store().is_ca_installed(ca_pem_path)
}
