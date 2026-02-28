# Roost

[![Release](https://img.shields.io/github/v/release/itsbjoern/roost?label=release)](https://github.com/itsbjoern/roost/releases)
[![CI](https://img.shields.io/github/actions/workflow/status/itsbjoern/roost/ci.yml?branch=main&label=ci)](https://github.com/itsbjoern/roost/actions)

A local HTTPS reverse proxy that manages certificate authorities, signed domain certificates, and hosts file entries. Use real HTTPS for local development without manual cert setup.

## Install

**Download a release** (recommended):

1. Go to [Releases](https://github.com/itsbjoern/roost/releases) and download the archive for your platform (Linux, macOS, or Windows).
2. Extract the `roost` binary to a directory in your `PATH`.
3. Run `roost init` to complete setup.

**Build from source** (requires [Rust](https://rustup.rs)):

```bash
git clone https://github.com/itsbjoern/roost.git && cd roost
cargo build --release
./target/release/roost init
```

## Quick start

```bash
roost init                    # One-time setup (creates CA, installs to trust store)
roost domain add api.local    # Add a domain (creates cert, updates /etc/hosts)
roost serve config add api.local 5001
roost serve                   # Start the proxy
```

Visit `https://api.local` — it proxies to `http://localhost:5001`. Port 80 redirects to HTTPS.

**Without root/sudo?** Use a non-privileged port:

```bash
roost serve config ports add 8443
roost serve
# Visit https://api.local:8443
```

### Use certs directly in a local server (no proxy)

You can add a domain and use its certs in your own dev server—no proxy needed. Roost creates the cert, updates hosts, and you get the paths:

```bash
roost domain add api.local
# Get paths for your server config:
roost domain get-path cert api.local   # → ~/.roost/certs/api.local.pem
roost domain get-path key api.local    # → ~/.roost/certs/api.local-key.pem
```

Point your local server at those paths. Example with Vite:

```ts
// vite.config.ts
import { defineConfig } from 'vite';
import { execSync } from 'child_process';
import fs from 'fs';

const certPath = execSync('roost domain get-path cert api.local').toString().trim();
const keyPath = execSync('roost domain get-path key api.local').toString().trim();

export default defineConfig({
  server: {
    https: {
      cert: fs.readFileSync(certPath),
      key: fs.readFileSync(keyPath),
    },
  },
});
```

Or with Node's `https.createServer`:

```js
const https = require('https');
const { execSync } = require('child_process');
const fs = require('fs');

const cert = fs.readFileSync(execSync('roost domain get-path cert api.local').toString().trim());
const key = fs.readFileSync(execSync('roost domain get-path key api.local').toString().trim());
const server = https.createServer({ cert, key }, (req, res) => { /* ... */ });
server.listen(5173);
```

Your dev server then serves HTTPS directly at `https://api.local:5173` using Roost's trusted certs.

## What it does

- **CA management**: Create and install a root CA into your system trust store so browsers accept local certs
- **Domain management**: Add domains (e.g. `api.example.local`); Roost creates certs, updates `/etc/hosts`, and handles renewal
- **Reverse proxy**: Terminates TLS and forwards `https://api.example.local` to `http://localhost:5001`; explicit ports in the URL (e.g. `https://api.example.local:5173`) forward to that backend port
- **Daemon mode**: Run the proxy in the background; reload config without restarting

## Port configuration

Ports are configured in `.roostrc` and default to **80** and **443**:

- **Port 80** (when 443 is configured): Plain HTTP redirect to HTTPS
- **Port 443** and others: TLS proxy to your backends

```bash
roost serve config ports add 5173     # Add Vite, etc.
roost serve config ports list        # Show configured ports
```

When you use an explicit port in the URL (e.g. `https://api.local:5173`), the proxy forwards directly to that backend port.

## Permissions

| Action | Required |
|--------|----------|
| CA install / uninstall | Admin (macOS: osascript; Linux: sudo) |
| Hosts file edit | Admin (same escalation) |
| Port 80 / 443 | Root or `CAP_NET_BIND_SERVICE` |
| Port 8443+ | None |

## Commands

| Command | Description |
|---------|-------------|
| `roost init` | One-time setup |
| `roost doctor` | Check configuration health (CA, hosts, certs, trust store) |
| `roost ca list` | List CAs (shows installed status) |
| `roost ca create <name>` | Create a new CA |
| `roost ca install [name]` | Install CA into system trust store |
| `roost ca uninstall [name]` | Remove CA from trust store |
| `roost domain add <domain>` | Add domain, create cert, update hosts. Use `--exact` for no wildcard; `--allow` to bypass TLD allowlist |
| `roost domain list` | List registered domains |
| `roost domain get-path cert <domain>`, `key <domain>` | Print path to cert or key file (for scripting, e.g. with local HTTPS servers) |
| `roost serve` | Start proxy (foreground) |
| `roost serve config add <domain> <port>` | Map domain to port. Use `--global` to write to user config instead of project |
| `roost serve config remove <domain>` | Remove mapping. Use `--global` for user config |
| `roost serve config list` | List mappings (shows project or global source per mapping) |
| `roost serve config ports add/remove/set` | Manage listen ports. Use `--global` for user config |
| `roost serve daemon start` | Run proxy in background |

Run `roost --help` or `roost <cmd> --help` for full usage.

### Global vs project config

Domain→port mappings and listen ports live in `.roostrc` in one of two places:

- **Project** (default): `.roostrc` in the current working directory. Best for per-project config you’ll commit to the repo. Use when adding mappings from within a project.
- **Global**: `~/.roost/.roostrc` (or `%APPDATA%\roost\.roostrc` on Windows). User-wide config that applies in any directory. Use for personal defaults or domains you use across many projects.

When you run `roost serve`, both configs are merged. For mappings, project overrides global when the same domain exists in both. For ports, the sets are combined. Use `roost serve config list` to see which file each mapping comes from. Add `--global` to `config add`, `config remove`, and `ports add/remove/set` to target the global file instead of the project.

## Features

- **TLD allowlist**: Only `.test`, `.local`, `.dev`, etc. by default; use `--allow` to override
- **Wildcard certs**: `domain add foo.local` covers `foo.local` and `*.foo.local`; use `--exact` to disable
- **Config merge**: Project and global `.roostrc` merge when you serve; see [Global vs project config](#global-vs-project-config)
- **Daemon**: `roost serve daemon start|stop|status|reload`; add/remove mappings triggers reload when running
- **Auto renewal**: Certs expiring within 30 days are regenerated automatically

## Configuration

**Data directory** (`~/.roost` or `%APPDATA%\roost` on Windows):

```
~/.roost/
  config.toml    # domain -> CA mapping
  ca/            # CAs (ca.pem, ca-key.pem per CA)
  certs/         # Domain certs (domain.pem, domain-key.pem)
  daemon.json    # Daemon state when running
```

**`.roostrc`** (project or global) defines domain→port mappings and listen ports. Project: `<cwd>/.roostrc`. Global: `~/.roost/.roostrc`.

```toml
[serve]
[[serve.mappings]]
domain = "api.example.local"
port = 5001

ports = [80, 443]   # optional; defaults to [80, 443]
```

**Environment variables:**

| Variable | Purpose |
|----------|---------|
| `ROOST_HOME` | Override data directory |
| `ROOST_SKIP_TRUST_INSTALL` | Skip trust store install in `roost init` (CI/testing) |
| `ROOST_HOSTS_FILE` | Override hosts file path (testing) |

## Releasing

```bash
./scripts/release.sh patch    # 0.0.X – bug fixes
./scripts/release.sh minor    # 0.X.0 – new features
./scripts/release.sh major    # X.0.0 – breaking changes
```

The script bumps the version, commits, tags, and optionally pushes. Add `-y` to push without prompting. Pushing a tag triggers the release workflow to build binaries for Linux, macOS, and Windows.
