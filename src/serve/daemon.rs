//! Daemon start/stop/status/reload.

use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

use crate::config::{project_roostrc, RoostPaths};

/// Daemon state stored in daemon.json.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DaemonState {
    pub pid: u32,
    pub project_path: Option<PathBuf>,
    pub started_at: String,
}

fn daemon_json_path(paths: &RoostPaths) -> PathBuf {
    paths.config_dir.join("daemon.json")
}

/// Check if PID is alive (Unix: kill -0).
fn is_pid_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        false
    }
}

fn read_state(paths: &RoostPaths) -> Result<Option<DaemonState>> {
    let path = daemon_json_path(paths);
    if !path.is_file() {
        return Ok(None);
    }
    let s = fs::read_to_string(&path).context("read daemon.json")?;
    let state: DaemonState = serde_json::from_str(&s).context("parse daemon.json")?;
    Ok(Some(state))
}

fn write_state(paths: &RoostPaths, state: &DaemonState) -> Result<()> {
    let path = daemon_json_path(paths);
    if let Some(p) = path.parent() {
        fs::create_dir_all(p)?;
    }
    let s = serde_json::to_string_pretty(state)?;
    fs::write(&path, s)?;
    Ok(())
}

fn clear_state(paths: &RoostPaths) -> Result<()> {
    let path = daemon_json_path(paths);
    if path.is_file() {
        fs::remove_file(&path)?;
    }
    Ok(())
}

/// Start daemon: spawn proxy in background, write daemon.json.
/// Ports are read from config (project + global .roostrc) when serve starts.
pub fn start_daemon(paths: &RoostPaths) -> Result<()> {
    if let Some(state) = read_state(paths)? {
        if is_pid_alive(state.pid) {
            anyhow::bail!(
                "Daemon already running (pid={}). Use 'roost serve daemon stop' first.",
                state.pid
            );
        }
        clear_state(paths)?;
    }

    let cwd = std::env::current_dir()?;
    let project_path = project_roostrc(&cwd).map(|p| p.parent().unwrap_or(&cwd).to_path_buf());

    let exe = std::env::current_exe().context("current exe")?;
    let mut cmd = Command::new(&exe);
    cmd.args(["serve"])
        .current_dir(&cwd)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null());

    // Pass through ROOST_* env vars so child has same config (critical for tests)
    for (k, v) in std::env::vars_os() {
        if let Some(s) = k.to_str() {
            if s.starts_with("ROOST_") {
                cmd.env(k, v);
            }
        }
    }

    let child: Child = cmd.spawn().context("spawn daemon")?;
    let pid = child.id();

    let state = DaemonState {
        pid,
        project_path,
        started_at: chrono::Utc::now().to_rfc3339(),
    };
    write_state(paths, &state)?;

    println!("Daemon started (pid={pid})");
    Ok(())
}

/// Stop daemon: send SIGTERM, clear state.
pub fn stop_daemon(paths: &RoostPaths) -> Result<()> {
    let state = match read_state(paths)? {
        Some(s) => s,
        None => {
            println!("Daemon not running");
            return Ok(());
        }
    };

    if !is_pid_alive(state.pid) {
        clear_state(paths)?;
        println!("Daemon not running (stale state cleared)");
        return Ok(());
    }

    #[cfg(unix)]
    {
        unsafe {
            libc::kill(state.pid as i32, libc::SIGTERM);
        }
        clear_state(paths)?;
        println!("Daemon stopped (pid={})", state.pid);
        return Ok(());
    }
    #[cfg(not(unix))]
    {
        let _ = state;
        anyhow::bail!("daemon stop not implemented on this platform");
    }
}

/// Get daemon status. Returns None if not running or state is stale.
pub fn daemon_status(paths: &RoostPaths) -> Result<Option<DaemonState>> {
    let state = match read_state(paths)? {
        Some(s) => s,
        None => return Ok(None),
    };

    if !is_pid_alive(state.pid) {
        clear_state(paths)?;
        return Ok(None);
    }

    Ok(Some(state))
}

/// Reload daemon config by sending SIGHUP.
pub fn reload_daemon(paths: &RoostPaths) -> Result<()> {
    let state = match read_state(paths)? {
        Some(s) => s,
        None => anyhow::bail!("Daemon not running"),
    };

    if !is_pid_alive(state.pid) {
        clear_state(paths)?;
        anyhow::bail!("Daemon not running (stale state cleared)");
    }

    #[cfg(unix)]
    {
        unsafe {
            if libc::kill(state.pid as i32, libc::SIGHUP) != 0 {
                anyhow::bail!("Failed to send SIGHUP to daemon");
            }
        }
        println!("Reload signal sent to daemon (pid={})", state.pid);
        return Ok(());
    }
    #[cfg(not(unix))]
    {
        let _ = state;
        anyhow::bail!("daemon reload not implemented on this platform");
    }
}
