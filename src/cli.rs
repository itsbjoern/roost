//! CLI definitions and command routing.

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

use crate::config::{project_roostrc, RoostPaths};
use crate::serve::config::{MappingSource, ServeConfig};
use crate::store;

#[derive(Parser)]
#[command(name = "roost")]
#[command(about = "Local HTTPS reverse proxy with signed domains")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// One-time setup: creates default CA, config dir, installs CA to system trust store
    Init,

    /// Manage certificate authorities (create, install, list, remove)
    Ca {
        #[command(subcommand)]
        cmd: CaCmd,
    },

    /// Manage domains and their TLS certs (add, remove, list, get-path)
    Domain {
        #[command(subcommand)]
        cmd: DomainCmd,
    },

    /// Run the HTTPS reverse proxy (or manage config/daemon)
    Serve {
        #[command(subcommand)]
        cmd: Option<ServeCmd>,
    },
}

#[derive(Subcommand)]
pub enum CaCmd {
    /// List all certificate authority names
    List,
    /// Create a new CA (used to sign domain certs); defaults to "default"
    Create { name: Option<String> },
    /// Remove a CA; fails if any domain still uses it
    Remove { name: String },
    /// Install CA into system trust store (macOS keychain, Linux ca-certificates)
    Install { name: Option<String> },
    /// Remove CA from system trust store
    Uninstall { name: Option<String> },
}

#[derive(Subcommand)]
pub enum DomainCmd {
    /// List all registered domains with their CA
    List,
    /// Add domain, create signed cert, optionally add to /etc/hosts
    Add {
        domain: String,
        /// Cert valid only for exact domain (no wildcard)
        #[arg(long)]
        exact: bool,
        /// Allow any TLD (bypass allowlist)
        #[arg(long)]
        allow: bool,
    },
    /// Remove domain from config and delete its cert files
    Remove { domain: String },
    /// Re-sign domain cert with a different CA
    SetCa { domain: String, ca_name: String },
    /// Print path to cert or key file (for scripting)
    GetPath {
        #[arg(value_enum)]
        cert_or_key: CertOrKey,
        domain: String,
    },
}

#[derive(Clone, Copy, ValueEnum)]
pub enum CertOrKey {
    /// Certificate file (domain.pem)
    Cert,
    /// Private key file (domain-key.pem)
    Key,
}

#[derive(Subcommand)]
pub enum ServeCmd {
    /// Manage domain -> port mappings (.roostrc)
    Config {
        #[command(subcommand)]
        cmd: ServeConfigCmd,
    },
    /// Run proxy as background daemon (start, stop, status, reload)
    Daemon {
        #[command(subcommand)]
        cmd: ServeDaemonCmd,
    },
}

#[derive(Subcommand)]
pub enum ServeConfigCmd {
    /// Add domain -> port mapping; auto-adds domain if not yet registered
    Add {
        domain: String,
        port: u16,
        /// Write to global .roostrc instead of project .roostrc
        #[arg(long)]
        global: bool,
    },
    /// Remove domain -> port mapping
    Remove {
        domain: String,
        /// Remove from global .roostrc instead of project
        #[arg(long)]
        global: bool,
    },
    /// List all domain -> port mappings (project + global)
    List,
    /// Manage listen ports (add, remove, list); default is 80 and 443
    Ports {
        #[command(subcommand)]
        cmd: ServePortsCmd,
    },
}

#[derive(Subcommand)]
pub enum ServePortsCmd {
    /// Add a port to listen on
    Add {
        port: u16,
        /// Write to global .roostrc instead of project .roostrc
        #[arg(long)]
        global: bool,
    },
    /// Remove a port
    Remove {
        port: u16,
        /// Remove from global .roostrc instead of project
        #[arg(long)]
        global: bool,
    },
    /// Replace ports list entirely (e.g. for scripting)
    Set {
        #[arg(num_args = 1..)]
        ports: Vec<u16>,
        /// Write to global .roostrc instead of project .roostrc
        #[arg(long)]
        global: bool,
    },
    /// List configured listen ports
    List,
}

#[derive(Subcommand)]
pub enum ServeDaemonCmd {
    /// Start proxy daemon in background
    Start,
    /// Stop running proxy daemon
    Stop,
    /// Show daemon status (pid, project path)
    Status,
    /// Reload config without restarting
    Reload,
}

/// Run CLI and dispatch to handlers.
pub fn run() -> Result<()> {
    let cli = Cli::parse();
    let paths = RoostPaths::default_paths();

    match cli.command {
        Commands::Init => cmd_init(&paths),
        Commands::Ca { cmd } => cmd_ca(&paths, cmd),
        Commands::Domain { cmd } => cmd_domain(&paths, cmd),
        Commands::Serve { cmd } => cmd_serve(&paths, cmd),
    }
}

fn cmd_init(paths: &RoostPaths) -> Result<()> {
    crate::store::ensure_dirs(paths)?;

    if !crate::ca::ca_exists(paths, "default") {
        crate::ca::create_ca(paths, "default")?;
        println!("Created CA: default");
    }

    let mut config = crate::store::load_config(paths)?;
    if config.default_ca.is_empty() {
        config.default_ca = "default".to_string();
        crate::store::save_config(paths, &config)?;
    }

    let ca_path = paths.ca_dir.join("default").join("ca.pem");
    if ca_path.is_file() && std::env::var("ROOST_SKIP_TRUST_INSTALL").is_err() {
        if let Err(e) = crate::trust::install_ca(&ca_path) {
            eprintln!("Warning: could not install CA to trust store: {e}");
            eprintln!("Run 'roost ca install' manually when ready.");
        } else {
            println!("Installed CA to system trust store.");
        }
    }

    println!("Roost initialised at {}", paths.config_dir.display());
    Ok(())
}

fn cmd_ca(paths: &RoostPaths, cmd: CaCmd) -> Result<()> {
    match cmd {
        CaCmd::List => {
            let cas = crate::ca::list_cas(paths)?;
            for ca in &cas {
                let ca_path = paths.ca_dir.join(ca).join("ca.pem");
                let installed = crate::trust::is_ca_installed(&ca_path).unwrap_or(false);
                let status = if installed { " (installed)" } else { "" };
                println!("{ca}{status}");
            }
            Ok(())
        }
        CaCmd::Create { name } => {
            let n = name.as_deref().unwrap_or("default");
            crate::ca::create_ca(paths, n)?;
            println!("Created CA: {n}");
            Ok(())
        }
        CaCmd::Remove { name } => {
            crate::ca::remove_ca(paths, &name)?;
            println!("Removed CA: {name}");
            Ok(())
        }
        CaCmd::Install { name } => {
            let n = name.as_deref().unwrap_or("default");
            let ca_path = paths.ca_dir.join(n).join("ca.pem");
            crate::trust::install_ca(&ca_path)?;
            println!("Installed CA: {n}");
            Ok(())
        }
        CaCmd::Uninstall { name } => {
            let n = name.as_deref().unwrap_or("default");
            let ca_path = paths.ca_dir.join(n).join("ca.pem");
            crate::trust::uninstall_ca(&ca_path)?;
            println!("Uninstalled CA: {n}");
            Ok(())
        }
    }
}

fn cmd_domain(paths: &RoostPaths, cmd: DomainCmd) -> Result<()> {
    match cmd {
        DomainCmd::List => {
            let config = store::load_config(paths)?;
            for (domain, ca) in crate::domain::list_domains(&config) {
                println!("{domain}\t{ca}");
            }
            Ok(())
        }
        DomainCmd::Add {
            domain,
            exact,
            allow,
        } => {
            crate::domain::validate_domain(&domain, allow)?;
            let mut config = store::load_config(paths)?;
            let editor = crate::platform::default_hosts_editor();
            crate::domain::add_domain(paths, &mut config, &domain, exact, Some(editor.as_ref()))?;
            store::save_config(paths, &config)?;
            println!("Added domain: {domain}");
            Ok(())
        }
        DomainCmd::Remove { domain } => {
            let mut config = store::load_config(paths)?;
            let editor = crate::platform::default_hosts_editor();
            crate::domain::remove_domain(paths, &mut config, &domain, Some(editor.as_ref()))?;
            store::save_config(paths, &config)?;
            println!("Removed domain: {domain}");
            Ok(())
        }
        DomainCmd::SetCa { domain, ca_name } => {
            let mut config = store::load_config(paths)?;
            crate::domain::set_ca(paths, &mut config, &domain, &ca_name)?;
            store::save_config(paths, &config)?;
            println!("Set CA for {domain}: {ca_name}");
            Ok(())
        }
        DomainCmd::GetPath { cert_or_key, domain } => {
            let (cert_path, key_path) = crate::domain::get_cert_paths(paths, &domain);
            let path = match cert_or_key {
                CertOrKey::Cert => cert_path,
                CertOrKey::Key => key_path,
            };
            println!("{}", path.display());
            Ok(())
        }
    }
}

/// Path to .roostrc for add/remove: project (cwd) or global.
fn serve_config_path(paths: &RoostPaths, cwd: &std::path::Path, global: bool) -> Result<PathBuf> {
    if global {
        Ok(paths.roostrc_global.clone())
    } else {
        Ok(cwd.join(".roostrc"))
    }
}

fn cmd_serve(paths: &RoostPaths, cmd: Option<ServeCmd>) -> Result<()> {
    match cmd {
        None => {
            let cwd = std::env::current_dir()?;
            let project_path = project_roostrc(&cwd);
            let project = project_path
                .as_ref()
                .map(|p| ServeConfig::load(p))
                .transpose()?
                .unwrap_or_default();
            let global = ServeConfig::load(&paths.roostrc_global)?;
            let mappings = crate::serve::config::merge_configs(&project, &global);
            let ports = crate::serve::config::merge_ports(&project, &global);
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(crate::serve::proxy::run_proxy(paths, mappings, ports))?;
            Ok(())
        }
        Some(ServeCmd::Config { cmd }) => {
            let cwd = std::env::current_dir()?;
            match cmd {
                ServeConfigCmd::Add {
                    domain,
                    port: p,
                    global,
                } => {
                    let rc_path = serve_config_path(paths, &cwd, global)?;
                    // Auto-add domain if not registered
                    let mut config = store::load_config(paths)?;
                    if !config.domains.contains_key(&domain) {
                        crate::domain::validate_domain(&domain, false)?;
                        let editor = crate::platform::default_hosts_editor();
                        crate::domain::add_domain(
                            paths,
                            &mut config,
                            &domain,
                            false,
                            Some(editor.as_ref()),
                        )?;
                        store::save_config(paths, &config)?;
                    }
                    let mut serve_cfg = ServeConfig::load(&rc_path)?;
                    serve_cfg.add(domain.clone(), p);
                    serve_cfg.save(&rc_path)?;
                    if let Some(_) = crate::serve::daemon::daemon_status(paths)? {
                        let _ = crate::serve::daemon::reload_daemon(paths);
                    }
                    println!("Added mapping: {domain} -> localhost:{p}");
                    Ok(())
                }
                ServeConfigCmd::Remove { domain, global } => {
                    let rc_path = serve_config_path(paths, &cwd, global)?;
                    let mut serve_cfg = ServeConfig::load(&rc_path)?;
                    serve_cfg.remove(&domain);
                    serve_cfg.save(&rc_path)?;
                    if let Some(_) = crate::serve::daemon::daemon_status(paths)? {
                        let _ = crate::serve::daemon::reload_daemon(paths);
                    }
                    println!("Removed mapping: {domain}");
                    Ok(())
                }
                ServeConfigCmd::List => {
                    let project_path = project_roostrc(&cwd);
                    let project = project_path
                        .as_ref()
                        .map(|p| ServeConfig::load(p))
                        .transpose()?
                        .unwrap_or_default();
                    let global = ServeConfig::load(&paths.roostrc_global)?;
                    let merged = crate::serve::config::merge_configs_with_source(&project, &global);
                    for m in merged {
                        let src = match m.source {
                            MappingSource::Project => "project",
                            MappingSource::Global => "global",
                        };
                        println!("{}\t{}\t({})", m.domain, m.port, src);
                    }
                    Ok(())
                }
                ServeConfigCmd::Ports { cmd } => match cmd {
                    ServePortsCmd::Add { port, global } => {
                        let rc_path = serve_config_path(paths, &cwd, global)?;
                        let mut serve_cfg = ServeConfig::load(&rc_path)?;
                        serve_cfg.ports_add(port);
                        serve_cfg.save(&rc_path)?;
                        if crate::serve::daemon::daemon_status(paths)?.is_some() {
                            let _ = crate::serve::daemon::reload_daemon(paths);
                        }
                        println!("Added port {port}");
                        Ok(())
                    }
                    ServePortsCmd::Remove { port, global } => {
                        let rc_path = serve_config_path(paths, &cwd, global)?;
                        let mut serve_cfg = ServeConfig::load(&rc_path)?;
                        serve_cfg.ports_remove(port);
                        serve_cfg.save(&rc_path)?;
                        if crate::serve::daemon::daemon_status(paths)?.is_some() {
                            let _ = crate::serve::daemon::reload_daemon(paths);
                        }
                        println!("Removed port {port}");
                        Ok(())
                    }
                    ServePortsCmd::Set { ports, global } => {
                        let rc_path = serve_config_path(paths, &cwd, global)?;
                        let mut serve_cfg = ServeConfig::load(&rc_path)?;
                        serve_cfg.ports_set(ports);
                        serve_cfg.save(&rc_path)?;
                        if crate::serve::daemon::daemon_status(paths)?.is_some() {
                            let _ = crate::serve::daemon::reload_daemon(paths);
                        }
                        println!("Ports updated");
                        Ok(())
                    }
                    ServePortsCmd::List => {
                        let project_path = project_roostrc(&cwd);
                        let project = project_path
                            .as_ref()
                            .map(|p| ServeConfig::load(p))
                            .transpose()?
                            .unwrap_or_default();
                        let global = ServeConfig::load(&paths.roostrc_global)?;
                        let ports = crate::serve::config::merge_ports(&project, &global);
                        for p in ports {
                            println!("{p}");
                        }
                        Ok(())
                    }
                },
            }
        }
        Some(ServeCmd::Daemon { cmd }) => match cmd {
            ServeDaemonCmd::Start => {
                crate::serve::daemon::start_daemon(paths)?;
                Ok(())
            }
            ServeDaemonCmd::Stop => {
                crate::serve::daemon::stop_daemon(paths)?;
                Ok(())
            }
            ServeDaemonCmd::Status => {
                if let Some(state) = crate::serve::daemon::daemon_status(paths)? {
                    println!(
                        "Daemon running: pid={}, started={}",
                        state.pid, state.started_at
                    );
                    if let Some(ref p) = state.project_path {
                        println!("  project: {}", p.display());
                    }
                } else {
                    println!("Daemon not running");
                }
                Ok(())
            }
            ServeDaemonCmd::Reload => {
                crate::serve::daemon::reload_daemon(paths)?;
                Ok(())
            }
        },
    }
}
