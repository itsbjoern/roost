//! All allowlisted TLDs pass.

mod common;

use roost::domain;

#[test]
fn all_tlds_pass() {
    for tld in domain::TLD_ALLOWLIST {
        let domain = format!("api.example.{}", tld);
        domain::validate_domain(&domain, false).unwrap();
    }
}
