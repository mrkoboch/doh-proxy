# doh-proxy

A DNS-over-HTTPS proxy with a polished terminal interface. Forwards DNS queries to upstream DoH resolvers (Cloudflare, Google, Quad9, or any RFC 8484-compatible server) and caches responses locally to minimise upstream round-trips.

**v0.1.1** adds the DNS Stats Dashboard — a self-contained web UI for visualising query activity in real time. The dashboard is a single binary that embeds the full frontend; no Node.js or separate web server is required at runtime.

```
● Listening on 127.0.0.1:5300
  Upstreams: https://1.1.1.1/dns-query, https://8.8.8.8/dns-query
  Log: ~/.local/share/doh-proxy/doh-proxy.log
  Press Ctrl+C or run `doh-proxy stop` to quit.
```

---

## Table of contents

- [What's new in v0.1.1](#whats-new-in-v011)
- [Prerequisites](#prerequisites)
- [Install](#install)
  - [Linux / macOS / Windows](#linux--macos--windows)
  - [Build from source](#build-from-source)
- [Quick start](#quick-start)
- [DNS Stats Dashboard](#dns-stats-dashboard)
  - [Running the dashboard](#running-the-dashboard)
  - [Environment variables](#environment-variables)
  - [Expected log format](#expected-log-format)
  - [API endpoints](#api-endpoints)
- [Commands](#commands)
- [Configuration](#configuration)
- [Pointing your system DNS at the proxy](#pointing-your-system-dns-at-the-proxy)
  - [Linux (systemd-resolved)](#linux-systemd-resolved)
  - [Linux (NetworkManager)](#linux-networkmanager)
  - [macOS](#macos-1)
  - [Windows](#windows-1)
- [Running on port 53](#running-on-port-53)
- [Runtime files](#runtime-files)
- [Logging](#logging)
- [Upgrading from v0.1.0](#upgrading-from-v010)
- [License](#license)

---

## What's new in v0.1.1

| Component | Change |
|---|---|
| `dns-dashboard` binary | New — real-time DNS stats dashboard with embedded React frontend |
| Workspace | Repository is now a Cargo workspace (`doh-rs` + `dns-dashboard`) |
| `.gitignore` | SQLite database files (`*.db`) excluded |

The `doh-proxy` binary itself is unchanged from v0.1.0. Upgrading only adds the new `dns-dashboard` binary alongside it.

---

## Prerequisites

### For running pre-built binaries

None. Both binaries are statically linked with [rustls](https://github.com/rustls/rustls) — no OpenSSL, no system SSL libraries, no runtime dependencies.

### For building from source

| Dependency | Version | Required for |
|---|---|---|
| Rust | 1.77 or later | Both binaries |
| Node.js | 18 or later | `dns-dashboard` only (embedded at compile time) |

**Install Rust:**

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

> **Windows:** Download and run the [rustup installer](https://rustup.rs) and install the [MSVC Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) ("Desktop development with C++" workload).

**Install Node.js** (required only when building `dns-dashboard` from source):

```sh
# macOS — Homebrew
brew install node

# Linux — NodeSource
curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
sudo apt-get install -y nodejs

# Any platform — NVM
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.7/install.sh | bash
nvm install 20
```

Node.js is only needed at build time. The resulting `dns-dashboard` binary contains the full frontend and has no runtime dependency on Node.

---

## Install

### Linux / macOS / Windows

Install the DNS proxy from crates.io:

```sh
cargo install doh-rs
```

The `dns-dashboard` binary is not published to crates.io. Build it from source (see below).

### Build from source

```sh
git clone https://github.com/mrkoboch/doh-proxy
cd doh-proxy

# Build the proxy
cargo build --release -p doh-rs

# Build the dashboard (requires Node.js — see Prerequisites)
cargo build --release -p dns-dashboard
```

Binaries are placed in `./target/release/`:

```
./target/release/doh-proxy        # DNS-over-HTTPS proxy
./target/release/dns-dashboard    # Stats dashboard (self-contained)
```

To install both to `~/.cargo/bin/` (must be run from inside the cloned repo):

```sh
cd doh-proxy                                          # if not already inside
cargo install --path . --bin doh-proxy
cargo install --path dns-dashboard --bin dns-dashboard
```

---

## Quick start

```sh
# 1. Start the proxy
doh-proxy start

# 2. In a second terminal — check that it is running
doh-proxy status

# 3. Tail the live query log
doh-proxy logs

# 4. Graceful shutdown
doh-proxy stop
```

To also run the dashboard alongside the proxy:

```sh
# Terminal 1
doh-proxy start

# Terminal 2 — point the dashboard at the proxy log
LOG_FILE=~/.local/share/doh-proxy/doh-proxy.log dns-dashboard

# Open http://127.0.0.1:4000 in your browser
```

---

## DNS Stats Dashboard

`dns-dashboard` is a standalone HTTP server that:

- Tails a log file produced by `doh-proxy` and stores parsed entries in a local SQLite database
- Serves a React dashboard at `http://127.0.0.1:4000` with live stats, a query feed, and charts
- Refreshes all data in the browser every 5 seconds

The dashboard and the proxy are fully decoupled — they share nothing at runtime except the log file.

### Running the dashboard

```sh
dns-dashboard
```

Then open `http://127.0.0.1:4000` in a browser on the same machine.

For remote access (e.g. from a laptop to a server), port-forward over SSH:

```sh
ssh -L 4000:127.0.0.1:4000 user@your-server
# then open http://127.0.0.1:4000 locally
```

### Environment variables

| Variable | Default | Description |
|---|---|---|
| `LOG_FILE` | `./proxy.log` | Path to the log file to tail |
| `DATABASE_URL` | `sqlite://dashboard.db` | SQLite database path (created automatically) |
| `LISTEN_ADDR` | `127.0.0.1:4000` | Address and port to serve the dashboard on |

Example — run on a custom port with an absolute log path:

```sh
LOG_FILE=/var/log/doh-proxy.log LISTEN_ADDR=127.0.0.1:8080 dns-dashboard
```

### Expected log format

The dashboard ingestor parses lines in the following space-separated format:

```
TIMESTAMP DOMAIN TYPE [BLOCKED] [latency=Xms] [resolver=URL]
```

| Field | Required | Description |
|---|---|---|
| `TIMESTAMP` | Yes | ISO 8601 UTC, e.g. `2026-04-06T12:00:00Z` |
| `DOMAIN` | Yes | Queried hostname, e.g. `example.com` |
| `TYPE` | Yes | DNS record type, e.g. `A`, `AAAA`, `MX` |
| `BLOCKED` | No | Literal word `BLOCKED` if the query was blocked |
| `latency=Xms` | No | Round-trip latency in milliseconds, e.g. `latency=42ms` |
| `resolver=URL` | No | Upstream resolver used, e.g. `resolver=https://1.1.1.1/dns-query` |

Example lines:

```
2026-04-06T12:00:00Z example.com A latency=42ms resolver=https://1.1.1.1/dns-query
2026-04-06T12:00:01Z ads.tracker.io AAAA BLOCKED latency=5ms
2026-04-06T12:00:02Z plain.com A
```

Lines that do not match this format are skipped silently. The ingestor seeks to the end of the file on startup and only processes new lines — it will not re-import historical entries.

### API endpoints

The dashboard exposes a JSON API that can be queried directly:

| Method | Path | Query params | Description |
|---|---|---|---|
| GET | `/api/stats` | — | Total queries, blocked count, average latency |
| GET | `/api/queries/recent` | `limit` (default 50) | Most recent queries, newest first |
| GET | `/api/queries/top-domains` | `limit` (default 10) | Most queried domains with counts |

Example:

```sh
curl http://127.0.0.1:4000/api/stats
# {"total":1042,"blocked":87,"avg_latency_ms":23.4}
```

---

## Commands

| Command | Description |
|---|---|
| `doh-proxy start` | Start the proxy in the foreground. Ctrl+C or `doh-proxy stop` to quit. |
| `doh-proxy start --listen 127.0.0.1:5300` | Override the listen address for this run. |
| `doh-proxy start --upstream URL` | Override upstream resolvers (repeat for multiple). |
| `doh-proxy config` | Interactive setup — saves to the config file. |
| `doh-proxy status` | Show running status, PID, uptime, and query statistics. |
| `doh-proxy logs` | Print the last 20 log lines and follow new entries. |
| `doh-proxy logs -n 50` | Print the last 50 lines instead. |
| `doh-proxy logs --no-follow` | Print history without following. |
| `doh-proxy stop` | Send SIGTERM to the running proxy and wait for it to exit. |

### Logging verbosity

```sh
RUST_LOG=debug doh-proxy start   # verbose
RUST_LOG=warn  doh-proxy start   # quiet
```

---

## Configuration

Config is stored at the platform path below and created automatically with defaults on first run. Edit it directly or run `doh-proxy config` for an interactive prompt.

```toml
# Address and port to listen on for DNS queries (UDP)
# Binding to 127.0.0.1 restricts access to this machine only (recommended)
listen_addr = "127.0.0.1:5300"

# Upstream DoH resolvers — tried in order, first success wins
upstreams = [
    "https://1.1.1.1/dns-query",          # Cloudflare
    "https://8.8.8.8/dns-query",          # Google
    # "https://dns.quad9.net/dns-query",  # Quad9
    # "https://dns.adguard.com/dns-query", # AdGuard
]

[cache]
enabled  = true
capacity = 10000   # maximum cached entries
```

### Platform config paths

| Platform | Path |
|---|---|
| Linux | `~/.config/doh-proxy/config.toml` |
| macOS | `~/Library/Application Support/doh-proxy/config.toml` |
| Windows | `%APPDATA%\doh-proxy\config\config.toml` |

---

## Pointing your system DNS at the proxy

Once `doh-proxy start` is running on `127.0.0.1:5300`, point your system resolver at it. Most system resolvers only query port 53, so the easiest path is to run on port 53 — see [Running on port 53](#running-on-port-53).

If you prefer to keep the proxy on a high port, use a redirect:

```sh
# Linux — redirect port 53 UDP to 5300 (no root required for the proxy itself)
sudo iptables -t nat -A OUTPUT -p udp --dport 53 -j REDIRECT --to-ports 5300
```

### Linux (systemd-resolved)

Edit `/etc/systemd/resolved.conf`:

```ini
[Resolve]
DNS=127.0.0.1
DNSStubListener=no
```

Then restart:

```sh
sudo systemctl restart systemd-resolved
```

### Linux (NetworkManager)

```sh
# Replace "Wired connection 1" with your connection name (nmcli con show)
nmcli con mod "Wired connection 1" ipv4.dns "127.0.0.1"
nmcli con mod "Wired connection 1" ipv4.ignore-auto-dns yes
nmcli con up "Wired connection 1"
```

### macOS

**System Settings → Wi-Fi / Ethernet → Details → DNS**

Add `127.0.0.1` and remove existing entries. Apply.

Or via the command line (replace `Wi-Fi` with your interface name):

```sh
sudo networksetup -setdnsservers Wi-Fi 127.0.0.1
# Restore with:
sudo networksetup -setdnsservers Wi-Fi "Empty"
```

### Windows

**Settings → Network & Internet → [your adapter] → DNS server assignment → Manual**

Set IPv4 preferred DNS to `127.0.0.1`.

Or via PowerShell (replace `Ethernet` with your adapter name):

```powershell
Set-DnsClientServerAddress -InterfaceAlias "Ethernet" -ServerAddresses ("127.0.0.1")
```

---

## Running on port 53

Port 53 is the standard DNS port. Binding below 1024 requires elevated privileges on most systems.

### Linux — grant capability without running as root

```sh
sudo setcap cap_net_bind_service=+ep ~/.cargo/bin/doh-proxy
doh-proxy start --listen 127.0.0.1:53
```

### Linux — systemd service (recommended for always-on use)

Create `/etc/systemd/system/doh-proxy.service`:

```ini
[Unit]
Description=DNS-over-HTTPS proxy
After=network-online.target
Wants=network-online.target

[Service]
ExecStart=/home/YOUR_USER/.cargo/bin/doh-proxy start
Restart=on-failure
RestartSec=5s
User=YOUR_USER

[Install]
WantedBy=multi-user.target
```

```sh
sudo systemctl daemon-reload
sudo systemctl enable --now doh-proxy
```

To also run the dashboard as a service, create `/etc/systemd/system/dns-dashboard.service`:

```ini
[Unit]
Description=DNS Stats Dashboard
After=network-online.target

[Service]
ExecStart=/home/YOUR_USER/.cargo/bin/dns-dashboard
Environment=LOG_FILE=/home/YOUR_USER/.local/share/doh-proxy/doh-proxy.log
Environment=DATABASE_URL=sqlite:///home/YOUR_USER/.local/share/doh-proxy/dashboard.db
Restart=on-failure
RestartSec=5s
User=YOUR_USER

[Install]
WantedBy=multi-user.target
```

```sh
sudo systemctl daemon-reload
sudo systemctl enable --now dns-dashboard
```

### macOS — LaunchAgent (always-on without root)

Create `~/Library/LaunchAgents/com.doh-proxy.plist`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>             <string>com.doh-proxy</string>
  <key>ProgramArguments</key>
  <array>
    <string>/Users/YOUR_USER/.cargo/bin/doh-proxy</string>
    <string>start</string>
  </array>
  <key>RunAtLoad</key>         <true/>
  <key>KeepAlive</key>         <true/>
  <key>StandardOutPath</key>   <string>/tmp/doh-proxy.log</string>
  <key>StandardErrorPath</key> <string>/tmp/doh-proxy.err</string>
</dict>
</plist>
```

```sh
launchctl load ~/Library/LaunchAgents/com.doh-proxy.plist
```

---

## Runtime files

| Platform | Path | Purpose |
|---|---|---|
| Linux | `~/.local/share/doh-proxy/doh-proxy.pid` | PID of the running process |
| Linux | `~/.local/share/doh-proxy/stats.json` | Live stats snapshot (updated every second) |
| Linux | `~/.local/share/doh-proxy/doh-proxy.log` | Structured log output |
| macOS | `~/Library/Application Support/doh-proxy/…` | Same files, different base path |
| Windows | `%LOCALAPPDATA%\doh-proxy\…` | Same files, different base path |

The `dns-dashboard` binary writes one additional file:

| File | Default path | Purpose |
|---|---|---|
| `dashboard.db` | `./dashboard.db` (or `DATABASE_URL`) | SQLite database of parsed query history |

---

## Logging

```sh
doh-proxy logs            # last 20 lines + follow
doh-proxy logs -n 100     # last 100 lines + follow
doh-proxy logs --no-follow  # last 20 lines, then exit
```

Log lines are coloured by level — errors in red, warnings in yellow, debug output dimmed.

---

## Upgrading from v0.1.0

The `doh-proxy` binary is unchanged. Upgrading adds the `dns-dashboard` binary and converts the repository to a Cargo workspace — no breaking changes to the proxy itself.

### From crates.io

```sh
cargo install doh-rs
```

`dns-dashboard` is not on crates.io. Build it from source (see [Build from source](#build-from-source)).

### From source

```sh
git -C doh-proxy pull
git -C doh-proxy checkout v0.1.1

# Rebuild the proxy (unchanged, rebuilds quickly)
cargo install --path doh-proxy --bin doh-proxy

# Build and install the dashboard (requires Node.js the first time)
cargo install --path doh-proxy/dns-dashboard --bin dns-dashboard
```

### Configuration

No configuration changes are required. All existing `doh-proxy` config files and runtime files are fully compatible with v0.1.1.

### Data migration

v0.1.1 introduces a new `dashboard.db` SQLite database. It is created automatically on first run of `dns-dashboard` and has no relation to any v0.1.0 files. No migration is needed.

---

## License

MIT
