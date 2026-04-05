# doh-proxy

A DNS-over-HTTPS proxy with a polished terminal interface. Forwards DNS queries to upstream DoH resolvers (Cloudflare, Google, Quad9, or any RFC 8484-compatible server). Caches responses locally to minimize upstream round-trips.

## Install

```sh
cargo install doh-proxy
```

Or build from source:

```sh
git clone https://github.com/your-username/doh-proxy
cd doh-proxy
cargo install --path .
```

Requires Rust 1.77+. Install Rust via [rustup.rs](https://rustup.rs).

## Quick start

```sh
# Set up configuration interactively
doh-proxy config

# Start the proxy (runs in foreground)
doh-proxy start

# In another terminal, check status
doh-proxy status

# Tail live logs
doh-proxy logs

# Stop the proxy
doh-proxy stop
```

## Commands

| Command | Description |
|---------|-------------|
| `doh-proxy start` | Start the proxy in the foreground. Ctrl+C or `stop` to quit. |
| `doh-proxy start --listen 0.0.0.0:53` | Override the listen address. |
| `doh-proxy start --upstream https://dns.quad9.net/dns-query` | Override upstream resolvers. |
| `doh-proxy config` | Interactive configuration setup. |
| `doh-proxy status` | Show proxy status, PID, uptime, and query stats. |
| `doh-proxy logs` | Tail the proxy log. Add `-n 50` for more history. |
| `doh-proxy stop` | Gracefully stop a running proxy. |

## Configuration

Config is stored at `~/.config/doh-proxy/config.toml`. Run `doh-proxy config` to edit interactively, or edit the file directly:

```toml
listen_addr = "0.0.0.0:5353"

upstreams = [
    "https://1.1.1.1/dns-query",
    "https://8.8.8.8/dns-query",
]

[cache]
enabled = true
capacity = 10000
```

Point your system DNS to the listen address. On Linux with systemd-resolved:

```sh
# /etc/systemd/resolved.conf
DNS=127.0.0.1
Ports=5353
```

## Runtime files

| Path | Purpose |
|------|---------|
| `~/.config/doh-proxy/config.toml` | Configuration |
| `~/.local/share/doh-proxy/doh-proxy.pid` | PID of running process |
| `~/.local/share/doh-proxy/stats.json` | Live stats (updated every second) |
| `~/.local/share/doh-proxy/doh-proxy.log` | Structured log output |

## License

MIT
