# Roost

A local HTTPS reverse proxy that manages certificate authorities, signed domain certificates, and hosts file entries. Use real HTTPS for local development without manual cert setup.

## What it does

- **CA management**: Create and install a root CA into your system trust store so browsers accept local certs
- **Domain management**: Add domains (e.g. `api.example.local`); Roost creates certs, updates `/etc/hosts`, and handles renewal
- **Reverse proxy**: Terminates TLS and forwards `https://api.example.local` to `http://localhost:5001`; explicit ports in the URL (e.g. `https://api.example.local:5173`) forward to that backend port
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
roost serve
```

By default the proxy listens on **80** and **443**. Port 80 redirects to HTTPS. Visit `https://api.example.local` (or `http://api.example.local` to be redirected).

For development without elevated privileges, add a non-privileged port:

```bash
roost serve config ports add 8443
roost serve
```

Visit `https://api.example.local:8443`.

## Port configuration

Ports are configured in `.roostrc` and default to **80** and **443**:

- **Port 80** (when 443 is also configured): Plain HTTP redirect to HTTPS
- **Port 443** and other ports: TLS proxy to your backends

```bash
roost serve config ports add 5173    # Add Vite, etc.
roost serve config ports remove 80   # HTTPS only
roost serve config ports set 8443    # Replace list (e.g. for scripting)
roost serve config ports list        # Show configured ports
```

When you visit a URL with an explicit port (e.g. `https://api.example.local:5173`), the proxy forwards to that backend port directly instead of using the domain mapping.

## Permissions

| Action | Required |
|--------|----------|
| CA install / uninstall | Admin (macOS: osascript; Linux: sudo) |
| Hosts file edit | Admin (same escalation) |
| Port 80 / 443 | Root or `CAP_NET_BIND_SERVICE` |
| Port 8443+ | None (add with `ports add 8443`) |

## Features

- **TLD allowlist**: Only `.test`, `.local`, `.dev`, etc. by default; use `--allow` to override
- **Wildcard certs**: `domain add foo.local` covers `foo.local` and `*.foo.local`; use `--exact` to disable
- **Config merge**: Project `.roostrc` in cwd plus global `~/.roost/.roostrc`; project overrides on conflict; ports are merged (union)
- **Daemon**: `roost serve daemon start|stop|status|reload`; ports and mappings from config; add/remove triggers reload when running
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

Project `.roostrc` defines serve mappings and ports; global `~/.roost/.roostrc` holds shared config.

```toml
[serve]
[[serve.mappings]]
domain = "api.example.local"
port = 5001

ports = [80, 443]   # optional; defaults to [80, 443]
```

## Commands

- `roost init` – One-time setup
- `roost doctor` – Check configuration health (CA, hosts, certs, trust store)
- `roost ca list|create|remove|install|uninstall` – CA management
- `roost domain list|add|remove|set-ca|get-path` – Domain management
- `roost serve` – Start proxy (foreground; ports from config)
- `roost serve config add|remove|list` – Domain → port mappings
- `roost serve config ports add|remove|set|list` – Listen ports (default: 80, 443)
- `roost serve daemon start|stop|status|reload` – Daemon control

Run `roost --help` or `roost <cmd> --help` for full usage.
