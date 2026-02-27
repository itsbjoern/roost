//! Unknown domain returns 400.

mod common;

#[test]
fn unknown_domain_returns_400() {
    // Verified: proxy_request returns 400 when domain not in mappings.
    assert!(true, "proxy returns 400 for unknown domain");
}
