# DoH Proxy

A DNS-over-HTTPS proxy with a desktop GUI built with Tauri.

## Installation

Extract the archive for your platform:

```sh
tar -xzf doh-proxy-gui-<platform>.tar.gz
cd doh-proxy-gui-<platform>/
```

Edit `config/default.toml` to set your listen address and upstream resolvers, then run:

```sh
./doh-proxy-gui
```

The GUI manages the proxy server. Point your system DNS to the configured listen address (default `0.0.0.0:5353`).

## Configuration

| Field | Default | Description |
|-------|---------|-------------|
| `listen_addr` | `0.0.0.0:5353` | UDP address the proxy listens on |
| `upstreams` | Cloudflare + Google | DoH resolver URLs |
| `cache.enabled` | `true` | Enable DNS response caching |
| `cache.capacity` | `10000` | Max cached entries |

## CLI usage (headless)

```sh
./doh-proxy --config config/default.toml
```
