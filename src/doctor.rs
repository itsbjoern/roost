//! Doctor command: health checks for roost configuration.

use anyhow::Result;

use crate::config::{project_roostrc, RoostPaths};
use crate::serve::config::{merge_configs_with_source, ServeConfig};

/// Result of a single check.
#[derive(Debug, Clone)]
pub struct CheckResult {
    pub ok: bool,
    pub message: String,
}

/// Run all doctor checks.
pub fn run_checks(paths: &RoostPaths, cwd: &std::path::Path) -> Result<Vec<CheckResult>> {
    let mut results = Vec::new();

    // 1. At least one CA exists
    let cas = crate::ca::list_cas(paths)?;
    if cas.is_empty() {
        results.push(CheckResult {
            ok: false,
            message: "No CA found. Run 'roost init' or 'roost ca create <name>'.".to_string(),
        });
    } else {
        results.push(CheckResult {
            ok: true,
            message: format!("Found {} CA(s): {}", cas.len(), cas.join(", ")),
        });
    }

    // 2. Get merged domains from project + global .roostrc
    let project_path = project_roostrc(cwd);
    let project = project_path
        .as_ref()
        .map(|p| ServeConfig::load(p))
        .transpose()?
        .unwrap_or_default();
    let global = ServeConfig::load(&paths.roostrc_global)?;
    let merged = merge_configs_with_source(&project, &global);

    if merged.is_empty() {
        results.push(CheckResult {
            ok: true,
            message: "No domain mappings configured (project or global .roostrc).".to_string(),
        });
        return Ok(results);
    }

    let config = crate::store::load_config(paths)?;
    let hosts_editor = crate::platform::default_hosts_editor();

    for m in &merged {
        let domain = &m.domain;
        let source = match m.source {
            crate::serve::config::MappingSource::Project => "project",
            crate::serve::config::MappingSource::Global => "global",
        };

        // 2a. Domain in hosts file
        match crate::hosts::domain_in_hosts(hosts_editor.as_ref(), domain) {
            Ok(true) => {
                results.push(CheckResult {
                    ok: true,
                    message: format!("[{domain}] ({source}) in hosts file"),
                });
            }
            Ok(false) => {
                results.push(CheckResult {
                    ok: false,
                    message: format!(
                        "[{domain}] ({source}) not in hosts file. Run 'roost domain add {domain}'."
                    ),
                });
            }
            Err(e) => {
                results.push(CheckResult {
                    ok: false,
                    message: format!("[{domain}] ({source}) cannot read hosts file: {e}"),
                });
            }
        }

        // 2b. Domain registered and has valid cert/key
        let ca_name = match config.domains.get(domain) {
            Some(ca) => ca.clone(),
            None => {
                results.push(CheckResult {
                    ok: false,
                    message: format!(
                        "[{domain}] ({source}) mapped but not registered. Run 'roost domain add {domain}'."
                    ),
                });
                continue;
            }
        };

        let (cert_path, key_path) = crate::domain::get_cert_paths(paths, domain);
        match (cert_path.is_file(), key_path.is_file()) {
            (false, _) => {
                results.push(CheckResult {
                    ok: false,
                    message: format!(
                        "[{domain}] ({source}) missing cert. Run 'roost domain add {domain}'."
                    ),
                });
            }
            (_, false) => {
                results.push(CheckResult {
                    ok: false,
                    message: format!(
                        "[{domain}] ({source}) missing key. Run 'roost domain add {domain}'."
                    ),
                });
            }
            (true, true) => {
                match crate::cert::load_domain_cert(paths, domain) {
                    Ok(_) => {
                        let expired = crate::cert::cert_expires_within_days(&cert_path, 0)
                            .unwrap_or(true);
                        if expired {
                            results.push(CheckResult {
                                ok: false,
                                message: format!(
                                    "[{domain}] ({source}) cert expired. Run 'roost domain add {domain}' to regenerate."
                                ),
                            });
                        } else {
                            results.push(CheckResult {
                                ok: true,
                                message: format!("[{domain}] ({source}) cert and key valid"),
                            });
                        }
                    }
                    Err(e) => {
                        results.push(CheckResult {
                            ok: false,
                            message: format!("[{domain}] ({source}) invalid cert/key: {e}"),
                        });
                    }
                }
            }
        }

        // 2c. Domain's CA is installed in system trust store
        let ca_path = paths.ca_dir.join(&ca_name).join("ca.pem");
        match crate::trust::is_ca_installed(&ca_path) {
            Ok(true) => {
                results.push(CheckResult {
                    ok: true,
                    message: format!("[{domain}] ({source}) CA '{ca_name}' installed"),
                });
            }
            Ok(false) => {
                results.push(CheckResult {
                    ok: false,
                    message: format!(
                        "[{domain}] ({source}) CA '{ca_name}' not installed. Run 'roost ca install {ca_name}'."
                    ),
                });
            }
            Err(e) => {
                results.push(CheckResult {
                    ok: false,
                    message: format!("[{domain}] ({source}) cannot check CA install status: {e}"),
                });
            }
        }
    }

    Ok(results)
}
