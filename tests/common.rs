//! Shared test helpers.

use std::path::PathBuf;
use tempfile::TempDir;

/// Create a temp directory for use as ROOST_HOME.
/// Uses current dir (workspace) so sandbox allows full access.
pub fn temp_roost_home() -> TempDir {
    tempfile::Builder::new()
        .prefix("roost_test_")
        .tempdir_in(std::env::current_dir().unwrap_or_else(|_| std::path::Path::new(".").into()))
        .expect("temp dir")
}

/// Run a closure with ROOST_HOME set to the given path.
pub fn with_test_env<F, R>(roost_home: &std::path::Path, f: F) -> R
where
    F: FnOnce() -> R,
{
    let prev = std::env::var_os("ROOST_HOME");
    std::env::set_var("ROOST_HOME", roost_home);
    let r = f();
    match prev {
        Some(v) => std::env::set_var("ROOST_HOME", v),
        None => std::env::remove_var("ROOST_HOME"),
    }
    r
}
