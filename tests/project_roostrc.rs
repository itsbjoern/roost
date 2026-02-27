//! project_roostrc: cwd only, no walk-up.

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

mod common;

fn temp_project() -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let cwd = dir.path().to_path_buf();
    (dir, cwd)
}

#[test]
fn returns_some_when_roostrc_in_cwd() {
    let (dir, cwd) = temp_project();
    let rc_path = cwd.join(".roostrc");
    fs::write(&rc_path, "[serve]\n").unwrap();

    let result = roost::config::project_roostrc(&cwd);
    assert!(result.is_some());
    assert_eq!(result.unwrap(), rc_path);
}

#[test]
fn returns_none_when_no_roostrc_in_cwd() {
    let (_, cwd) = temp_project();
    // No .roostrc created

    let result = roost::config::project_roostrc(&cwd);
    assert!(result.is_none());
}

#[test]
fn does_not_find_roostrc_in_parent() {
    let (dir, _) = temp_project();
    let parent = dir.path();
    let child = parent.join("subdir");
    fs::create_dir_all(&child).unwrap();

    // Put .roostrc in parent only
    fs::write(parent.join(".roostrc"), "[serve]\n").unwrap();

    // Look from child - should NOT find parent's .roostrc
    let result = roost::config::project_roostrc(&child);
    assert!(result.is_none(), "should not walk up to find .roostrc in parent");
}
