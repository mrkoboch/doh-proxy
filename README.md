# doh-proxy

A DNS-over-HTTPS proxy with a polished terminal interface. Forwards DNS queries to upstream DoH resolvers (Cloudflare, Google, Quad9, or any RFC 8484-compatible server) and caches responses locally to minimise upstream round-trips.

```
● Listening on 127.0.0.1:5300
  Upstreams: https://1.1.1.1/dns-query, https://8.8.8.8/dns-query
  Log: ~/.local/share/doh-proxy/doh-proxy.log
  Press Ctrl+C or run `doh-proxy stop` to quit.
```

---

## Table of contents

- [Prerequisites](#prerequisites)
- [Install](#install)
  - [Linux](#linux)
  - [macOS](#macos)
  - [Windows](#windows)
  - [Build from source](#build-from-source)
- [Quick start](#quick-start)
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
- [License](#license)

---

## Prerequisites

**Rust 1.77 or later** is the only build-time dependency. The binary uses [rustls](https://github.com/rustls/rustls) for TLS, so no OpenSSL or system SSL libraries are required on any platform.

### Install Rust

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Then follow the on-screen instructions and open a new terminal (or run `source ~/.cargo/env`) so that `cargo` is on your `PATH`.

> **Windows:** Download and run the [rustup installer](https://rustup.rs) instead.
> You will also need the [MSVC Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) (choose "Desktop development with C++" workload).

---

## Install

### Linux

```sh
cargo install doh-rs
```

The binary (`doh-proxy`) is placed in `~/.cargo/bin/`. Make sure that directory is on your `PATH` (the rustup installer adds it automatically).

### macOS

```sh
cargo install doh-rs
```

Works on both Apple Silicon (ARM) and Intel. No Homebrew or system dependencies needed.

### Windows

```sh
cargo install doh-rs
```

> **Note:** `doh-proxy stop` is not supported on Windows (it uses Unix signals). Use Ctrl+C in the terminal where `doh-proxy start` is running instead. All other commands work normally.

### Build from source

```sh
git clone https://github.com/mrkoboch/doh-proxy
cd doh-proxy
cargo build --release
# Binary is at ./target/release/doh-proxy
```

Or install directly from the cloned repository:

```sh
cargo install --path .
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

To change the listen address, upstream resolvers, or cache settings:

```sh
doh-proxy config
```

---

## Commands

| Command | Description |
|---------|-------------|
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
    "https://1.1.1.1/dns-query",         # Cloudflare
    "https://8.8.8.8/dns-query",         # Google
    # "https://dns.quad9.net/dns-query",  # Quad9
    # "https://dns.adguard.com/dns-query", # AdGuard
]

[cache]
enabled  = true
capacity = 10000   # maximum cached entries
```

### Platform config paths

| Platform | Path |
|----------|------|
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
|----------|------|---------|
| Linux | `~/.local/share/doh-proxy/doh-proxy.pid` | PID of the running process |
| Linux | `~/.local/share/doh-proxy/stats.json` | Live stats snapshot (updated every second) |
| Linux | `~/.local/share/doh-proxy/doh-proxy.log` | Structured log output |
| macOS | `~/Library/Application Support/doh-proxy/…` | Same files, different base path |
| Windows | `%LOCALAPPDATA%\doh-proxy\…` | Same files, different base path |

---

## Logging

```sh
doh-proxy logs            # last 20 lines + follow
doh-proxy logs -n 100     # last 100 lines + follow
doh-proxy logs --no-follow  # last 20 lines, then exit
```

Log lines are coloured by level — errors in red, warnings in yellow, debug output dimmed.

---

## License

MIT
