# Roost

A local HTTPS reverse proxy that manages certificate authorities, signed domain certificates, and hosts file entries. Use real HTTPS for local development without manual cert setup.

## What it does

- **CA management**: Create and install a root CA into your system trust store so browsers accept local certs
- **Domain management**: Add domains (e.g. `api.example.local`); Roost creates certs, updates `/etc/hosts`, and handles renewal
- **Reverse proxy**: Terminates TLS and forwards `https://api.example.local` to `http://127.0.0.1:5001`
- **Daemon mode**: Run the proxy in the background; reload config without restarting

## Requirements

- Rust 1.70+
- macOS, Linux (Ubuntu/Debian), or Windows
- Admin/sudo for trust store and hosts file edits (one-time prompts)

## Setup

```bash
cargo build --release
./target/release/roost init
```

`roost init` creates a default CA, installs it into the system trust store, and sets up `~/.roost/`. You will be prompted for your password to install the CA and edit the hosts file.

## Quick start

```bash
# Add a domain (creates cert, updates hosts)
roost domain add api.example.local

# Map it to your app and start the proxy
roost serve config add api.example.local 5001
roost serve --port 8443
```

Visit `https://api.example.local:8443` (or use port 443 with elevated privileges).

## Permissions

| Action | Required |
|--------|----------|
| CA install / uninstall | Admin (macOS: osascript; Linux: sudo) |
| Hosts file edit | Admin (same escalation) |
| Port 443 | Root or `CAP_NET_BIND_SERVICE` |
| Port 8443+ | None (use `--port 8443`) |

## Features

- **TLD allowlist**: Only `.test`, `.local`, `.dev`, etc. by default; use `--allow` to override
- **Wildcard certs**: `domain add foo.local` covers `foo.local` and `*.foo.local`; use `--exact` to disable
- **Config merge**: Project `.roostrc` in cwd plus global `~/.roost/.roostrc`; project overrides on conflict
- **Daemon**: `roost serve daemon start|stop|status|reload`; config add/remove triggers reload when running
- **Auto renewal**: Certs expiring within 30 days are regenerated automatically
- **Parseable output**: `roost domain get-path cert <domain>` and `get-path key <domain>` print single paths for scripting

## Environment variables

| Variable | Purpose |
|----------|---------|
| `ROOST_HOME` | Override data directory (default: `~/.roost` or `%APPDATA%\roost`) |
| `ROOST_SKIP_TRUST_INSTALL` | Skip trust store install in `roost init` (CI/testing) |
| `ROOST_HOSTS_FILE` | Override hosts file path (testing) |

## Data layout

```
~/.roost/
  config.toml    # domain -> CA mapping
  ca/            # CAs (ca.pem, ca-key.pem per CA)
  certs/         # Domain certs (domain.pem, domain-key.pem)
  daemon.json    # Daemon state when running
```

Project `.roostrc` defines serve mappings; global `~/.roost/.roostrc` holds shared mappings.

## Commands

- `roost init` – One-time setup
- `roost ca list|create|remove|install|uninstall` – CA management
- `roost domain list|add|remove|set-ca|get-path` – Domain management
- `roost serve [--port N]` – Start proxy (foreground)
- `roost serve config add|remove|list` – Mappings
- `roost serve daemon start|stop|status|reload` – Daemon control

Run `roost --help` or `roost <cmd> --help` for full usage.
