//! Domain validation: allowlist, --allow override, invalid hostnames, reject localhost.

mod common;

use roost::domain;

#[test]
fn allowlist_tld_passes() {
    domain::validate_domain("api.example.local", false).unwrap();
    domain::validate_domain("app.test", false).unwrap();
}

#[test]
fn non_allowlist_tld_fails() {
    let err = domain::validate_domain("api.example.com", false).unwrap_err();
    assert!(err.to_string().contains("allowlist"));
}

#[test]
fn allow_override_permits_any_tld() {
    domain::validate_domain("api.example.com", true).unwrap();
}

#[test]
fn invalid_hostname_fails() {
    assert!(domain::validate_hostname("").is_err());
    assert!(domain::validate_hostname("..").is_err());
    assert!(domain::validate_hostname("bad..domain").is_err());
}

#[test]
fn reject_bare_localhost() {
    let err = domain::validate_hostname("localhost").unwrap_err();
    assert!(err.to_string().to_lowercase().contains("localhost"));
}
