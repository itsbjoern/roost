//! Hosts file read/write (platform abstraction).

use anyhow::Result;

use crate::platform::HostsEditor;

/// Add domain to hosts (127.0.0.1 and ::1).
pub fn add_domain_to_hosts(editor: &dyn HostsEditor, domain: &str) -> Result<()> {
    editor.add_domain(domain)
}

/// Remove domain from hosts.
pub fn remove_domain_from_hosts(editor: &dyn HostsEditor, domain: &str) -> Result<()> {
    editor.remove_domain(domain)
}

/// Check if domain is in hosts file.
pub fn domain_in_hosts(editor: &dyn HostsEditor, domain: &str) -> Result<bool> {
    editor.has_domain(domain)
}
