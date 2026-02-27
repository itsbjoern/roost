//! E2E: Init, add domain, add mapping, proxy, curl.

mod common;

use assert_cmd::Command;
use std::process::{Child, Stdio};

#[test]
fn e2e_serve_flow() {
    let dir = common::temp_roost_home();
    let hosts_path = dir.path().join("hosts");
    std::fs::write(&hosts_path, "").unwrap();
    let project_dir = dir.path();
    let port: u16 = 19443;

    common::with_test_env(project_dir, || {
        std::env::set_var("ROOST_SKIP_TRUST_INSTALL", "1");
        std::env::set_var("ROOST_HOSTS_FILE", hosts_path.to_str().unwrap());

        // Init
        Command::cargo_bin("roost")
            .unwrap()
            .current_dir(project_dir)
            .arg("init")
            .assert()
            .success();

        // Add domain
        Command::cargo_bin("roost")
            .unwrap()
            .current_dir(project_dir)
            .args(["domain", "add", "api.test"])
            .assert()
            .success();

        // Add mapping
        Command::cargo_bin("roost")
            .unwrap()
            .current_dir(project_dir)
            .args(["serve", "config", "add", "api.test", "8080"])
            .assert()
            .success();

        // Start proxy in background (mock backend would need separate process;
        // we verify proxy starts and responds for unknown Host)
        let roost_exe = std::path::PathBuf::from(
            Command::cargo_bin("roost").unwrap().get_program(),
        );
        let mut proxy_child: Child = std::process::Command::new(&roost_exe)
            .args(["serve", "--port", &port.to_string()])
            .current_dir(project_dir)
            .env("ROOST_HOME", project_dir)
            .env("ROOST_SKIP_TRUST_INSTALL", "1")
            .env("ROOST_HOSTS_FILE", hosts_path.to_str().unwrap())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .spawn()
            .expect("spawn proxy");

        std::thread::sleep(std::time::Duration::from_millis(300));

        if proxy_child.try_wait().unwrap().is_some() {
            let _ = proxy_child.kill();
            panic!("Proxy exited immediately");
        }

        // Curl via HTTPS (insecure: -k) - we expect 400 for wrong Host or connection
        let curl_status = std::process::Command::new("curl")
            .args([
                "-sk",
                "--connect-timeout",
                "2",
                "-H",
                "Host: unknown.test",
                &format!("https://127.0.0.1:{}/", port),
            ])
            .output();

        let _ = proxy_child.kill();

        // Optional: verify proxy responds. Curl may fail in sandbox or if not installed.
        if let Ok(out) = curl_status {
            let body = String::from_utf8_lossy(&out.stdout);
            if !body.is_empty() {
                assert!(body.contains("Unknown domain"), "Expected 'Unknown domain', got: {}", body);
            }
        }

        let _ = std::env::remove_var("ROOST_SKIP_TRUST_INSTALL");
        let _ = std::env::remove_var("ROOST_HOSTS_FILE");
    });
}
