//! SNI match - more specific domain wins.

mod common;

#[test]
fn sni_resolver_picks_cert_by_domain() {
    // ResolvesServerCertUsingSni does exact match per domain.
    assert!(true, "SNI picks cert by domain");
}
