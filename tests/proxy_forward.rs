//! Proxy tests.

mod common;

use roost::config::RoostPaths;
use std::collections::HashMap;

#[tokio::test]
async fn proxy_fails_with_no_mappings() {
    let dir = common::temp_roost_home();
    let paths = RoostPaths::for_test(dir.path());
    let result = roost::serve::proxy::run_proxy(&paths, HashMap::new(), vec![17444]).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("no mappings"));
}
