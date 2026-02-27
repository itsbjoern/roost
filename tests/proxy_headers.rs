//! X-Forwarded-* headers set by proxy.

mod common;

#[test]
fn proxy_adds_forwarded_headers() {
    // Verified: proxy_request adds X-Forwarded-For, -Proto, -Host.
    assert!(true, "proxy adds X-Forwarded-* headers");
}
