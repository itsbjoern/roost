//! List with project/global source.

mod common;

use assert_cmd::Command;
use std::fs;

#[test]
fn list_shows_source() {
    let dir = common::temp_roost_home();
    common::with_test_env(dir.path(), || {
        // Create roost home with global .roostrc
        let global_rc = dir.path().join(".roostrc");
        fs::write(
            &global_rc,
            r#"[serve]
mappings = [
  { domain = "global.test", port = 4000 },
]
"#,
        )
        .unwrap();

        // Run from dir.path() - no project .roostrc, so only global
        let mut cmd = Command::cargo_bin("roost").unwrap();
        cmd.current_dir(dir.path())
            .args(["serve", "config", "list"])
            .assert()
            .success()
            .stdout(predicates::str::contains("global.test"))
            .stdout(predicates::str::contains("4000"))
            .stdout(predicates::str::contains("global"));
    });
}

#[test]
fn list_shows_project_when_both() {
    let dir = common::temp_roost_home();
    common::with_test_env(dir.path(), || {
        // Global .roostrc
        let global_rc = dir.path().join(".roostrc");
        fs::write(
            &global_rc,
            r#"[serve]
mappings = [
  { domain = "global.test", port = 4000 },
]
"#,
        )
        .unwrap();

        // Project .roostrc in subdir
        let project_dir = dir.path().join("proj");
        std::fs::create_dir_all(&project_dir).unwrap();
        let project_rc = project_dir.join(".roostrc");
        fs::write(
            &project_rc,
            r#"[serve]
mappings = [
  { domain = "project.test", port = 3000 },
]
"#,
        )
        .unwrap();

        let mut cmd = Command::cargo_bin("roost").unwrap();
        cmd.current_dir(&project_dir)
            .args(["serve", "config", "list"])
            .assert()
            .success()
            .stdout(predicates::str::contains("project.test\t3000\t(project)"))
            .stdout(predicates::str::contains("global.test\t4000\t(global)"));
    });
}
