# CLI Rewrite Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rewrite doh-proxy as a polished single-binary Rust CLI (start/stop/config/status/logs subcommands) publishable to crates.io, removing all TypeScript/Tauri/TUI code.

**Architecture:** One `doh-proxy` binary with `clap` subcommands. `start` runs the proxy in the foreground, writing a PID file and a stats JSON to `~/.local/share/doh-proxy/` every second. `stop` reads the PID file and sends SIGTERM. `status` and `logs` read the persisted files — no IPC socket needed. `config` uses `dialoguer` interactive prompts to write `~/.config/doh-proxy/config.toml`. All terminal output styled with `console` + `indicatif`.

**Tech Stack:** Rust 1.77+, `clap 4` (derive), `indicatif 0.17`, `console 0.15`, `dialoguer 0.11`, `directories 5`, `serde_json 1`, `tracing-appender 0.2`, `nix 0.29` (Unix signals), existing `tokio`/`tracing`/`reqwest`/`moka`/`hickory-proto`.

---

## File Map

| Path | Action | Responsibility |
|------|--------|---------------|
| `Cargo.toml` | Modify | Remove workspace + GUI member; add crates.io metadata; swap ratatui/crossterm for new deps |
| `src/lib.rs` | Modify | Remove `pub mod tui`; add `pub mod runtime` |
| `src/config.rs` | Modify | Add `config_path()`, `load_or_create()`, `save()` using `directories` |
| `src/runtime.rs` | Create | PID file, stats JSON, log-file path helpers |
| `src/stats.rs` | Modify | Add `Deserialize` to `StatsSnapshot`; add `started_at` field to `RuntimeStats` |
| `src/main.rs` | Rewrite | `clap` CLI entry + dispatch to `cli::*` handlers |
| `src/cli/mod.rs` | Create | `pub mod` declarations for CLI subcommand modules |
| `src/cli/start.rs` | Create | `doh-proxy start` — spinner, pidfile, file logging, server loop, signal handling |
| `src/cli/config_cmd.rs` | Create | `doh-proxy config` — `dialoguer` prompts → save config.toml |
| `src/cli/status.rs` | Create | `doh-proxy status` — read pidfile + stats JSON, formatted output |
| `src/cli/logs.rs` | Create | `doh-proxy logs` — tail log file with color |
| `src/cli/stop.rs` | Create | `doh-proxy stop` — read pidfile, send SIGTERM, wait |
| `.github/workflows/ci.yml` | Create | CI: build + test on push/PR |
| `.github/workflows/release.yml` | Delete | Replaced — binary release workflow no longer needed |
| `README.md` | Rewrite | `cargo install` instructions, usage, examples |
| `src/tui/` | Delete | Entire directory (superseded by CLI) |
| `src/tui_main.rs` | Delete | Superseded by main.rs rewrite |
| `gui/` | Delete | All TypeScript/Tauri code |
| `tests/integration_test.rs` | Keep | Already passes; update import if needed |

---

## Task 1: Restructure Cargo.toml

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Write the failing test for XDG config path (will pass after Task 2, but write it now)**

Create `tests/config_path_test.rs`:

```rust
// This test verifies that Config::config_path() returns a path
// ending in "doh-proxy/config.toml" — the shape is what matters.
#[test]
fn config_path_ends_with_expected_suffix() {
    let path = doh_proxy::config::config_path();
    let s = path.to_string_lossy();
    assert!(s.ends_with("doh-proxy/config.toml"), "got: {s}");
}
```

Run: `cargo test config_path_ends_with_expected_suffix`
Expected: FAIL — `config_path` function does not exist yet.

- [ ] **Step 2: Replace Cargo.toml**

Replace the entire `Cargo.toml` with:

```toml
[package]
name = "doh-proxy"
version = "0.1.3"
edition = "2021"
description = "A DNS-over-HTTPS proxy with a polished terminal interface"
license = "MIT"
repository = "https://github.com/your-username/doh-proxy"
keywords = ["dns", "proxy", "doh", "cli", "terminal"]
categories = ["command-line-utilities", "network-programming"]
readme = "README.md"

[lib]
name = "doh_proxy"
path = "src/lib.rs"

[[bin]]
name = "doh-proxy"
path = "src/main.rs"

[dependencies]
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["rustls-tls"], default-features = false }
bytes = "1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
clap = { version = "4", features = ["derive"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
tracing-appender = "0.2"
thiserror = "2"
anyhow = "1"
hickory-proto = "0.25"
moka = { version = "0.12", features = ["future"] }
base64 = "0.22"
indicatif = "0.17"
console = "0.15"
dialoguer = "0.11"
directories = "5"

[target.'cfg(unix)'.dependencies]
nix = { version = "0.29", features = ["signal", "process"] }

[dev-dependencies]
tokio-test = "0.4"
```

- [ ] **Step 3: Verify the package compiles (it will fail on missing modules — that's OK)**

Run: `cargo check 2>&1 | head -30`
Expected: errors about missing `runtime` module and `tui` module — not linker errors.

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml tests/config_path_test.rs
git commit -m "chore: restructure Cargo.toml for crates.io; drop workspace and GUI member"
```

---

## Task 2: Update Config module for XDG paths

**Files:**
- Modify: `src/config.rs`

The test from Task 1 (`config_path_ends_with_expected_suffix`) should pass after this task.

- [ ] **Step 1: Replace `src/config.rs`**

```rust
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use crate::error::{ProxyError, Result};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    /// Address to listen for incoming DNS queries (UDP + TCP)
    #[serde(default = "default_listen_addr")]
    pub listen_addr: SocketAddr,

    /// Upstream DoH server URLs (tried in order, with fallback)
    #[serde(default = "default_upstreams")]
    pub upstreams: Vec<String>,

    /// DNS response cache settings
    #[serde(default)]
    pub cache: CacheConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CacheConfig {
    /// Maximum number of cached entries
    #[serde(default = "default_cache_capacity")]
    pub capacity: u64,

    /// Whether caching is enabled
    #[serde(default = "bool_true")]
    pub enabled: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            capacity: default_cache_capacity(),
            enabled: true,
        }
    }
}

fn default_listen_addr() -> SocketAddr {
    "0.0.0.0:5353".parse().unwrap()
}

fn default_upstreams() -> Vec<String> {
    vec![
        "https://1.1.1.1/dns-query".into(),
        "https://8.8.8.8/dns-query".into(),
    ]
}

fn default_cache_capacity() -> u64 {
    10_000
}

fn bool_true() -> bool {
    true
}

/// Returns `~/.config/doh-proxy/config.toml` (XDG on Linux, standard locations on macOS/Windows).
pub fn config_path() -> PathBuf {
    project_dirs().config_dir().join("config.toml")
}

fn project_dirs() -> ProjectDirs {
    ProjectDirs::from("", "", "doh-proxy")
        .expect("could not determine home directory")
}

impl Config {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| ProxyError::Config(e.to_string()))?;
        toml::from_str(&contents).map_err(|e| ProxyError::Config(e.to_string()))
    }

    /// Load from the XDG config path. If the file does not exist, create it with defaults.
    pub fn load_or_create() -> Result<Self> {
        let path = config_path();
        if path.exists() {
            return Self::from_file(&path);
        }
        let config = Self::default();
        config.save()?;
        Ok(config)
    }

    /// Write config to the XDG config path, creating parent directories as needed.
    pub fn save(&self) -> Result<()> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ProxyError::Config(e.to_string()))?;
        }
        let contents = toml::to_string_pretty(self)
            .map_err(|e| ProxyError::Config(e.to_string()))?;
        std::fs::write(&path, contents)
            .map_err(|e| ProxyError::Config(e.to_string()))?;
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            listen_addr: default_listen_addr(),
            upstreams: default_upstreams(),
            cache: CacheConfig::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_path_ends_with_expected_suffix() {
        let path = config_path();
        let s = path.to_string_lossy();
        assert!(s.ends_with("doh-proxy/config.toml"), "got: {s}");
    }

    #[test]
    fn config_defaults_parse() {
        let toml = r#"
            listen_addr = "127.0.0.1:5353"
            upstreams = ["https://1.1.1.1/dns-query"]
        "#;
        let config: Config = toml::from_str(toml).expect("config should parse");
        assert!(!config.upstreams.is_empty());
        assert!(config.cache.enabled);
    }

    #[test]
    fn save_and_load_roundtrip() {
        // Write to a temp file and reload using from_file.
        let dir = std::env::temp_dir().join("doh_proxy_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");

        let original = Config {
            listen_addr: "127.0.0.1:5353".parse().unwrap(),
            upstreams: vec!["https://dns.quad9.net/dns-query".into()],
            cache: CacheConfig { capacity: 500, enabled: false },
        };

        let contents = toml::to_string_pretty(&original).unwrap();
        std::fs::write(&path, contents).unwrap();

        let loaded = Config::from_file(&path).unwrap();
        assert_eq!(loaded.listen_addr.to_string(), "127.0.0.1:5353");
        assert_eq!(loaded.upstreams[0], "https://dns.quad9.net/dns-query");
        assert!(!loaded.cache.enabled);
        assert_eq!(loaded.cache.capacity, 500);
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test config`
Expected: all config tests PASS (including the one from `tests/config_path_test.rs`).

- [ ] **Step 3: Remove `tests/config_path_test.rs`** (the test now lives in `src/config.rs`)

```bash
rm tests/config_path_test.rs
```

- [ ] **Step 4: Commit**

```bash
git add src/config.rs tests/
git commit -m "feat: add XDG config path support (load_or_create, save)"
```

---

## Task 3: Create runtime module

**Files:**
- Create: `src/runtime.rs`
- Modify: `src/lib.rs`

The runtime module manages: PID file, stats JSON, log file path.

- [ ] **Step 1: Write failing tests first**

Add to `src/runtime.rs` (create the file):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tmp_dir() -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("doh_proxy_runtime_{}", std::process::id()));
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn write_and_read_pid() {
        let dir = tmp_dir();
        write_pid_to(&dir, 12345).unwrap();
        let pid = read_pid_from(&dir).unwrap();
        assert_eq!(pid, Some(12345));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn clear_pid() {
        let dir = tmp_dir();
        write_pid_to(&dir, 99).unwrap();
        clear_pid_in(&dir).unwrap();
        assert_eq!(read_pid_from(&dir).unwrap(), None);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn read_pid_when_file_absent_returns_none() {
        let dir = tmp_dir();
        assert_eq!(read_pid_from(&dir).unwrap(), None);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn write_and_read_stats() {
        use crate::stats::StatsSnapshot;
        let dir = tmp_dir();
        let rs = RuntimeStats {
            snapshot: StatsSnapshot { total: 10, cache_hits: 3, upstream: 6, errors: 1 },
            listen_addr: "0.0.0.0:5353".into(),
            started_at: 1000,
        };
        write_runtime_stats_to(&dir, &rs).unwrap();
        let loaded = read_runtime_stats_from(&dir).unwrap().unwrap();
        assert_eq!(loaded.snapshot.total, 10);
        assert_eq!(loaded.listen_addr, "0.0.0.0:5353");
        assert_eq!(loaded.started_at, 1000);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn read_stats_when_absent_returns_none() {
        let dir = tmp_dir();
        assert!(read_runtime_stats_from(&dir).unwrap().is_none());
        fs::remove_dir_all(&dir).unwrap();
    }
}
```

Run: `cargo test runtime`
Expected: FAIL — module doesn't exist yet.

- [ ] **Step 2: Add `pub mod runtime;` to `src/lib.rs`**

```rust
pub mod cache;
pub mod config;
pub mod error;
pub mod proxy;
pub mod resolver;
pub mod runtime;
pub mod server;
pub mod stats;
pub mod upstream;
```

- [ ] **Step 3: Create `src/runtime.rs` with implementations**

```rust
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::stats::StatsSnapshot;

#[derive(Debug, Serialize, Deserialize)]
pub struct RuntimeStats {
    pub snapshot: StatsSnapshot,
    pub listen_addr: String,
    pub started_at: u64,
}

/// Returns `~/.local/share/doh-proxy/` (XDG data dir).
pub fn runtime_dir() -> PathBuf {
    ProjectDirs::from("", "", "doh-proxy")
        .expect("could not determine home directory")
        .data_dir()
        .to_path_buf()
}

fn pid_path(dir: &Path) -> PathBuf {
    dir.join("doh-proxy.pid")
}

fn stats_path(dir: &Path) -> PathBuf {
    dir.join("stats.json")
}

pub fn log_path() -> PathBuf {
    runtime_dir().join("doh-proxy.log")
}

fn ensure_dir(dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)
}

// --- PID file ---

pub fn write_pid(pid: u32) -> anyhow::Result<()> {
    write_pid_to(&runtime_dir(), pid)
}

pub fn read_pid() -> anyhow::Result<Option<u32>> {
    read_pid_from(&runtime_dir())
}

pub fn clear_pid() -> anyhow::Result<()> {
    clear_pid_in(&runtime_dir())
}

pub(crate) fn write_pid_to(dir: &Path, pid: u32) -> anyhow::Result<()> {
    ensure_dir(dir)?;
    std::fs::write(pid_path(dir), pid.to_string())?;
    Ok(())
}

pub(crate) fn read_pid_from(dir: &Path) -> anyhow::Result<Option<u32>> {
    let path = pid_path(dir);
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)?;
    let pid: u32 = content.trim().parse()?;
    Ok(Some(pid))
}

pub(crate) fn clear_pid_in(dir: &Path) -> anyhow::Result<()> {
    let path = pid_path(dir);
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

// --- Stats JSON ---

pub fn write_runtime_stats(rs: &RuntimeStats) -> anyhow::Result<()> {
    write_runtime_stats_to(&runtime_dir(), rs)
}

pub fn read_runtime_stats() -> anyhow::Result<Option<RuntimeStats>> {
    read_runtime_stats_from(&runtime_dir())
}

pub(crate) fn write_runtime_stats_to(dir: &Path, rs: &RuntimeStats) -> anyhow::Result<()> {
    ensure_dir(dir)?;
    let json = serde_json::to_string(rs)?;
    std::fs::write(stats_path(dir), json)?;
    Ok(())
}

pub(crate) fn read_runtime_stats_from(dir: &Path) -> anyhow::Result<Option<RuntimeStats>> {
    let path = stats_path(dir);
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(path)?;
    let rs: RuntimeStats = serde_json::from_str(&content)?;
    Ok(Some(rs))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tmp_dir() -> PathBuf {
        let d = std::env::temp_dir().join(format!("doh_proxy_runtime_{}", std::process::id()));
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn write_and_read_pid() {
        let dir = tmp_dir();
        write_pid_to(&dir, 12345).unwrap();
        let pid = read_pid_from(&dir).unwrap();
        assert_eq!(pid, Some(12345));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn clear_pid() {
        let dir = tmp_dir();
        write_pid_to(&dir, 99).unwrap();
        clear_pid_in(&dir).unwrap();
        assert_eq!(read_pid_from(&dir).unwrap(), None);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn read_pid_when_file_absent_returns_none() {
        let dir = tmp_dir();
        assert_eq!(read_pid_from(&dir).unwrap(), None);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn write_and_read_stats() {
        let dir = tmp_dir();
        let rs = RuntimeStats {
            snapshot: StatsSnapshot { total: 10, cache_hits: 3, upstream: 6, errors: 1 },
            listen_addr: "0.0.0.0:5353".into(),
            started_at: 1000,
        };
        write_runtime_stats_to(&dir, &rs).unwrap();
        let loaded = read_runtime_stats_from(&dir).unwrap().unwrap();
        assert_eq!(loaded.snapshot.total, 10);
        assert_eq!(loaded.listen_addr, "0.0.0.0:5353");
        assert_eq!(loaded.started_at, 1000);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn read_stats_when_absent_returns_none() {
        let dir = tmp_dir();
        assert!(read_runtime_stats_from(&dir).unwrap().is_none());
        fs::remove_dir_all(&dir).unwrap();
    }
}
```

- [ ] **Step 4: Add `Deserialize` to `StatsSnapshot` in `src/stats.rs`**

Find this line:

```rust
#[derive(Debug, Clone, Serialize)]
pub struct StatsSnapshot {
```

Change it to:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsSnapshot {
```

Also add `use serde::Deserialize;` to the imports if needed (it's already in the serde derive).

- [ ] **Step 5: Run tests**

Run: `cargo test runtime`
Expected: all 5 runtime tests PASS.

- [ ] **Step 6: Commit**

```bash
git add src/runtime.rs src/lib.rs src/stats.rs
git commit -m "feat: add runtime module for pidfile and stats persistence"
```

---

## Task 4: Rewrite main.rs and scaffold CLI modules

**Files:**
- Rewrite: `src/main.rs`
- Create: `src/cli/mod.rs`
- Create: `src/cli/start.rs` (stub)
- Create: `src/cli/config_cmd.rs` (stub)
- Create: `src/cli/status.rs` (stub)
- Create: `src/cli/logs.rs` (stub)
- Create: `src/cli/stop.rs` (stub)

- [ ] **Step 1: Create `src/cli/mod.rs`**

```rust
pub mod config_cmd;
pub mod logs;
pub mod start;
pub mod status;
pub mod stop;
```

- [ ] **Step 2: Create stub files for each command**

`src/cli/start.rs`:
```rust
pub async fn run(_listen: Option<String>, _upstreams: Vec<String>) -> anyhow::Result<()> {
    todo!("start command — implemented in Task 5")
}
```

`src/cli/config_cmd.rs`:
```rust
pub fn run() -> anyhow::Result<()> {
    todo!("config command — implemented in Task 6")
}
```

`src/cli/status.rs`:
```rust
pub fn run() -> anyhow::Result<()> {
    todo!("status command — implemented in Task 7")
}
```

`src/cli/logs.rs`:
```rust
pub fn run(_follow: bool, _lines: usize) -> anyhow::Result<()> {
    todo!("logs command — implemented in Task 8")
}
```

`src/cli/stop.rs`:
```rust
pub fn run() -> anyhow::Result<()> {
    todo!("stop command — implemented in Task 9")
}
```

- [ ] **Step 3: Rewrite `src/main.rs`**

```rust
mod cli;

use clap::{Parser, Subcommand};

/// A DNS-over-HTTPS proxy with a polished terminal interface.
#[derive(Parser)]
#[command(
    name = "doh-proxy",
    version,
    about,
    long_about = "doh-proxy forwards DNS queries to upstream DoH resolvers.\n\
                  Config is stored at ~/.config/doh-proxy/config.toml.\n\
                  Run `doh-proxy config` to set up interactively."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the DoH proxy (runs in the foreground; Ctrl+C or `stop` to quit)
    Start {
        /// Override the listen address (e.g. 0.0.0.0:53)
        #[arg(short, long, value_name = "ADDR")]
        listen: Option<String>,

        /// Override upstream DoH resolvers (can be repeated)
        #[arg(short, long = "upstream", value_name = "URL")]
        upstreams: Vec<String>,
    },

    /// Interactive configuration setup
    Config,

    /// Show proxy status and statistics
    Status,

    /// Tail live proxy logs (Ctrl+C to quit)
    Logs {
        /// Number of historical lines to show on start
        #[arg(short = 'n', long, default_value = "20")]
        lines: usize,

        /// Keep following new log lines (like tail -f)
        #[arg(short, long, default_value = "true")]
        follow: bool,
    },

    /// Gracefully stop a running proxy
    Stop,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Start { listen, upstreams } => cli::start::run(listen, upstreams).await,
        Commands::Config => cli::config_cmd::run(),
        Commands::Status => cli::status::run(),
        Commands::Logs { lines, follow } => cli::logs::run(follow, lines),
        Commands::Stop => cli::stop::run(),
    }
}
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo build 2>&1 | grep -E "^error" | head -10`
Expected: no errors (the `todo!()` stubs compile fine).

- [ ] **Step 5: Test help output**

Run: `cargo run -- --help`
Expected: shows all subcommands with descriptions.

Run: `cargo run -- start --help`
Expected: shows `--listen` and `--upstream` flags.

- [ ] **Step 6: Commit**

```bash
git add src/main.rs src/cli/
git commit -m "feat: scaffold clap CLI with start/config/status/logs/stop subcommands"
```

---

## Task 5: Implement `start` command

**Files:**
- Modify: `src/cli/start.rs`

The `start` command:
1. Shows a spinner while initializing
2. Loads or creates config (merges CLI flag overrides)
3. Sets up layered logging (console + file)
4. Writes PID file
5. Starts the proxy server
6. Prints "listening" message
7. Spawns a task that writes stats to disk every second
8. Waits for SIGTERM or Ctrl+C, then gracefully stops

- [ ] **Step 1: Implement `src/cli/start.rs`**

```rust
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use tracing::info;

use doh_proxy::{
    config::Config,
    runtime::{self, RuntimeStats},
    server::Server,
    stats::{unix_now, Stats},
};

pub async fn run(listen: Option<String>, upstreams: Vec<String>) -> anyhow::Result<()> {
    // Check if already running
    if let Some(pid) = runtime::read_pid()? {
        if process_running(pid) {
            eprintln!(
                "{} doh-proxy is already running (PID {pid}). Run `doh-proxy stop` first.",
                style("✗").red()
            );
            std::process::exit(1);
        }
        // Stale pidfile — clean it up
        runtime::clear_pid()?;
    }

    // Load config and apply CLI overrides
    let mut config = Config::load_or_create()?;
    if let Some(addr) = listen {
        config.listen_addr = addr
            .parse()
            .map_err(|e| anyhow::anyhow!("invalid listen address: {e}"))?;
    }
    if !upstreams.is_empty() {
        config.upstreams = upstreams;
    }

    // Set up file logging
    let log_path = runtime::log_path();
    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file_appender = tracing_appender::rolling::never(
        log_path.parent().unwrap(),
        log_path.file_name().unwrap(),
    );
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_level(true),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
                .with_target(false),
        )
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    // Spinner during server init
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    spinner.set_message("Starting proxy...");
    spinner.enable_steady_tick(Duration::from_millis(80));

    let stats = Arc::new(Stats::new());
    let stop_flag = Arc::new(AtomicBool::new(false));

    let server = match Server::new(config.clone(), Some(Arc::clone(&stats))).await {
        Ok(s) => s,
        Err(e) => {
            spinner.finish_and_clear();
            eprintln!("{} Failed to start server: {e}", style("✗").red());
            return Err(e.into());
        }
    };

    spinner.finish_and_clear();

    // Write PID file
    runtime::write_pid(std::process::id())?;

    let started_at = unix_now();
    println!(
        "{} Listening on {}",
        style("●").green().bold(),
        style(&config.listen_addr).cyan()
    );
    println!(
        "  {} {}\n  {} {}",
        style("Upstreams:").dim(),
        config.upstreams.join(", "),
        style("Log:").dim(),
        runtime::log_path().display()
    );
    println!("{}", style("  Press Ctrl+C or run `doh-proxy stop` to quit.").dim());

    // Spawn stats writer task
    {
        let stats = Arc::clone(&stats);
        let stop = Arc::clone(&stop_flag);
        let listen_addr = config.listen_addr.to_string();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            loop {
                interval.tick().await;
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                let rs = RuntimeStats {
                    snapshot: stats.snapshot(),
                    listen_addr: listen_addr.clone(),
                    started_at,
                };
                runtime::write_runtime_stats(&rs).ok();
            }
        });
    }

    // Spawn server on a background thread (it needs its own runtime)
    let stop_server = Arc::clone(&stop_flag);
    let server_thread = {
        let stop = Arc::clone(&stop_server);
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            rt.block_on(async move {
                if let Err(e) = server.run_cancellable(stop).await {
                    tracing::error!("server error: {e}");
                }
            });
        })
    };

    // Wait for shutdown signal
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigterm = signal(SignalKind::terminate())?;
        tokio::select! {
            _ = sigterm.recv() => { info!("received SIGTERM, shutting down"); }
            _ = tokio::signal::ctrl_c() => { info!("received SIGINT, shutting down"); }
        }
    }
    #[cfg(not(unix))]
    tokio::signal::ctrl_c().await?;

    println!("\n{} Shutting down...", style("◼").yellow());
    stop_flag.store(true, Ordering::Release);
    let _ = server_thread.join();

    // Clean up runtime files
    runtime::clear_pid().ok();
    println!("{} Stopped.", style("●").dim());

    Ok(())
}

/// Returns true if a process with this PID is currently running.
#[cfg(unix)]
fn process_running(pid: u32) -> bool {
    use nix::sys::signal::{kill, Signal};
    use nix::unistd::Pid;
    // Sending signal 0 checks if the process exists without actually signaling it
    kill(Pid::from_raw(pid as i32), None).is_ok()
}

#[cfg(not(unix))]
fn process_running(_pid: u32) -> bool {
    false
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build 2>&1 | grep "^error" | head -10`
Expected: no errors.

- [ ] **Step 3: Smoke test**

Run: `cargo run -- start --help`
Expected: shows `--listen` and `--upstream` options with descriptions.

- [ ] **Step 4: Commit**

```bash
git add src/cli/start.rs
git commit -m "feat: implement doh-proxy start command with spinner, pidfile, file logging"
```

---

## Task 6: Implement `config` command

**Files:**
- Modify: `src/cli/config_cmd.rs`

The `config` command shows current values as defaults and uses `dialoguer` interactive prompts to collect new values, then saves `~/.config/doh-proxy/config.toml`.

- [ ] **Step 1: Implement `src/cli/config_cmd.rs`**

```rust
use console::style;
use dialoguer::{theme::ColorfulTheme, Confirm, Input};

use doh_proxy::config::{CacheConfig, Config, config_path};

pub fn run() -> anyhow::Result<()> {
    let theme = ColorfulTheme::default();
    let current = Config::load_or_create()?;

    println!(
        "{} Interactive config setup",
        style("◆").cyan().bold()
    );
    println!(
        "  Config path: {}\n",
        style(config_path().display().to_string()).dim()
    );

    // Listen address
    let listen_addr: String = Input::with_theme(&theme)
        .with_prompt("Listen address")
        .default(current.listen_addr.to_string())
        .validate_with(|s: &String| {
            s.parse::<std::net::SocketAddr>()
                .map(|_| ())
                .map_err(|e| format!("invalid socket address: {e}"))
        })
        .interact_text()?;

    // Upstream resolvers — collect until empty line
    println!(
        "\n  {}",
        style("Upstream DoH resolvers (enter one per line; empty line to finish):").dim()
    );
    let mut upstreams: Vec<String> = Vec::new();
    for (i, default) in current.upstreams.iter().enumerate() {
        let prompt = format!("Upstream {}", i + 1);
        let val: String = Input::with_theme(&theme)
            .with_prompt(&prompt)
            .default(default.clone())
            .allow_empty(true)
            .interact_text()?;
        if val.is_empty() {
            break;
        }
        upstreams.push(val);
    }
    // Allow adding more upstreams beyond current list
    let mut idx = current.upstreams.len() + 1;
    loop {
        let val: String = Input::with_theme(&theme)
            .with_prompt(format!("Upstream {idx} (empty to finish)"))
            .allow_empty(true)
            .interact_text()?;
        if val.is_empty() {
            break;
        }
        upstreams.push(val);
        idx += 1;
    }
    if upstreams.is_empty() {
        upstreams = doh_proxy::config::Config::default().upstreams;
        println!(
            "  {} No upstreams entered — using defaults.",
            style("!").yellow()
        );
    }

    // Cache settings
    let cache_enabled: bool = Confirm::with_theme(&theme)
        .with_prompt("Enable DNS response cache?")
        .default(current.cache.enabled)
        .interact()?;

    let cache_capacity: u64 = Input::with_theme(&theme)
        .with_prompt("Cache capacity (max entries)")
        .default(current.cache.capacity)
        .validate_with(|s: &u64| {
            if *s == 0 {
                Err("capacity must be > 0".to_string())
            } else {
                Ok(())
            }
        })
        .interact_text()?;

    let config = Config {
        listen_addr: listen_addr.parse()?,
        upstreams,
        cache: CacheConfig {
            enabled: cache_enabled,
            capacity: cache_capacity,
        },
    };

    config.save()?;

    println!(
        "\n{} Config saved to {}",
        style("✓").green(),
        style(config_path().display().to_string()).cyan()
    );
    println!("  Run `doh-proxy start` to apply the new configuration.");

    Ok(())
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build 2>&1 | grep "^error" | head -10`
Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/cli/config_cmd.rs
git commit -m "feat: implement doh-proxy config command with dialoguer interactive prompts"
```

---

## Task 7: Implement `status` command

**Files:**
- Modify: `src/cli/status.rs`

`status` reads the pidfile and stats JSON. If the server is not running it says so clearly.

- [ ] **Step 1: Implement `src/cli/status.rs`**

```rust
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use console::style;

use doh_proxy::{config::config_path, runtime};

pub fn run() -> anyhow::Result<()> {
    let pid = runtime::read_pid()?;
    let stats = runtime::read_runtime_stats()?;

    match (pid, stats) {
        (Some(pid), Some(rs)) if process_running(pid) => {
            let uptime = format_uptime(rs.started_at);
            let hit_rate = if rs.snapshot.total > 0 {
                format!("{:.1}%", rs.snapshot.cache_hits as f64 / rs.snapshot.total as f64 * 100.0)
            } else {
                "—".into()
            };

            println!("{}", style("● doh-proxy").green().bold());
            println!("  {:<14} {}", style("Status:").dim(), style("running").green());
            println!("  {:<14} {}", style("PID:").dim(), pid);
            println!("  {:<14} {}", style("Uptime:").dim(), uptime);
            println!("  {:<14} {}", style("Listen:").dim(), rs.listen_addr);
            println!();
            println!("{}", style("  Queries").bold());
            println!(
                "  {:<14} {}",
                style("Total:").dim(),
                rs.snapshot.total
            );
            println!(
                "  {:<14} {} ({})",
                style("Cache hits:").dim(),
                rs.snapshot.cache_hits,
                hit_rate
            );
            println!(
                "  {:<14} {}",
                style("Upstream:").dim(),
                rs.snapshot.upstream
            );
            println!(
                "  {:<14} {}",
                style("Errors:").dim(),
                style(rs.snapshot.errors).red()
            );
            println!();
            println!(
                "  {:<14} {}",
                style("Config:").dim(),
                config_path().display()
            );
            println!(
                "  {:<14} {}",
                style("Log:").dim(),
                runtime::log_path().display()
            );
        }
        _ => {
            println!("{}", style("○ doh-proxy").dim().bold());
            println!("  {:<14} {}", style("Status:").dim(), style("stopped").yellow());
            println!("\n  Run `doh-proxy start` to start the proxy.");
        }
    }

    Ok(())
}

fn format_uptime(started_at: u64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs();
    let secs = now.saturating_sub(started_at);
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{h}h {m}m {s}s")
    } else if m > 0 {
        format!("{m}m {s}s")
    } else {
        format!("{s}s")
    }
}

#[cfg(unix)]
fn process_running(pid: u32) -> bool {
    use nix::sys::signal::kill;
    use nix::unistd::Pid;
    kill(Pid::from_raw(pid as i32), None).is_ok()
}

#[cfg(not(unix))]
fn process_running(_pid: u32) -> bool {
    true // assume running on non-Unix if pidfile exists
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_uptime_seconds() {
        // Use current time minus 45 seconds
        let started = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .saturating_sub(45);
        let result = format_uptime(started);
        assert!(result.contains('s'), "got: {result}");
        assert!(!result.contains('h'), "got: {result}");
    }

    #[test]
    fn format_uptime_minutes() {
        let started = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .saturating_sub(125);
        let result = format_uptime(started);
        assert!(result.starts_with("2m"), "got: {result}");
    }

    #[test]
    fn format_uptime_hours() {
        let started = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .saturating_sub(3700);
        let result = format_uptime(started);
        assert!(result.starts_with("1h"), "got: {result}");
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test status`
Expected: 3 tests PASS.

- [ ] **Step 3: Commit**

```bash
git add src/cli/status.rs
git commit -m "feat: implement doh-proxy status command"
```

---

## Task 8: Implement `logs` command

**Files:**
- Modify: `src/cli/logs.rs`

The `logs` command reads the log file, shows the last N lines, then optionally polls for new content. Uses `console` for colored output based on log level.

- [ ] **Step 1: Implement `src/cli/logs.rs`**

```rust
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::thread;
use std::time::Duration;

use console::style;

use doh_proxy::runtime;

pub fn run(follow: bool, lines: usize) -> anyhow::Result<()> {
    let log_path = runtime::log_path();

    if !log_path.exists() {
        println!(
            "{} No log file found at {}",
            style("!").yellow(),
            log_path.display()
        );
        println!("  The proxy has not been started yet, or logging hasn't written yet.");
        return Ok(());
    }

    let file = File::open(&log_path)?;
    let mut reader = BufReader::new(file);

    // Seek to show only the last N lines
    let last_n_lines = collect_last_lines(&mut reader, lines)?;
    for line in &last_n_lines {
        print_log_line(line);
    }

    if !follow {
        return Ok(());
    }

    // Seek to end and follow new content
    reader.seek(SeekFrom::End(0))?;

    println!(
        "\n{} Following {} — Ctrl+C to quit",
        style("↓").cyan(),
        log_path.display()
    );

    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => {
                // No new content — wait and retry
                thread::sleep(Duration::from_millis(100));
            }
            Ok(_) => {
                let trimmed = line.trim_end();
                if !trimmed.is_empty() {
                    print_log_line(trimmed);
                }
            }
            Err(e) => return Err(e.into()),
        }
    }
}

/// Read the entire file line-by-line and return the last `n` lines.
fn collect_last_lines(reader: &mut BufReader<File>, n: usize) -> anyhow::Result<Vec<String>> {
    reader.seek(SeekFrom::Start(0))?;
    let lines: Vec<String> = reader
        .lines()
        .filter_map(|l| l.ok())
        .collect();
    let start = lines.len().saturating_sub(n);
    Ok(lines[start..].to_vec())
}

/// Apply color based on log level prefix found in the line.
fn print_log_line(line: &str) {
    if line.contains(" ERROR ") || line.contains(" error ") {
        println!("{}", style(line).red());
    } else if line.contains(" WARN ") || line.contains(" warn ") {
        println!("{}", style(line).yellow());
    } else if line.contains(" DEBUG ") || line.contains(" debug ") {
        println!("{}", style(line).dim());
    } else {
        println!("{line}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    // helper to create a temp log file with content
    fn make_log(lines: &[&str]) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        for l in lines {
            writeln!(f, "{l}").unwrap();
        }
        f.flush().unwrap();
        f
    }

    #[test]
    fn collect_last_lines_returns_at_most_n() {
        let content: Vec<&str> = (0..30).map(|i| if i == 0 { "line 0" } else { "line N" }).collect();
        // Use a temp file to avoid the borrow issue
        let log = make_log(&content);
        let file = File::open(log.path()).unwrap();
        let mut reader = BufReader::new(file);
        let result = collect_last_lines(&mut reader, 10).unwrap();
        assert_eq!(result.len(), 10);
    }

    #[test]
    fn collect_last_lines_when_fewer_than_n() {
        let log = make_log(&["a", "b", "c"]);
        let file = File::open(log.path()).unwrap();
        let mut reader = BufReader::new(file);
        let result = collect_last_lines(&mut reader, 20).unwrap();
        assert_eq!(result.len(), 3);
    }
}
```

The `tempfile` crate is needed for the tests. Add it to `Cargo.toml` dev-dependencies:

```toml
[dev-dependencies]
tokio-test = "0.4"
tempfile = "3"
```

- [ ] **Step 2: Run tests**

Run: `cargo test logs`
Expected: 2 tests PASS.

- [ ] **Step 3: Commit**

```bash
git add src/cli/logs.rs Cargo.toml
git commit -m "feat: implement doh-proxy logs command with tail and colorized output"
```

---

## Task 9: Implement `stop` command

**Files:**
- Modify: `src/cli/stop.rs`

`stop` reads the PID file, verifies the process is running, sends SIGTERM, and waits for it to exit (polling the PID file with a 5-second timeout).

- [ ] **Step 1: Implement `src/cli/stop.rs`**

```rust
use std::thread;
use std::time::{Duration, Instant};

use console::style;

use doh_proxy::runtime;

pub fn run() -> anyhow::Result<()> {
    let pid = match runtime::read_pid()? {
        Some(p) => p,
        None => {
            println!("{} doh-proxy is not running.", style("○").dim());
            return Ok(());
        }
    };

    if !process_running(pid) {
        // Stale pidfile
        runtime::clear_pid().ok();
        println!("{} doh-proxy is not running (stale PID file removed).", style("○").dim());
        return Ok(());
    }

    println!(
        "{} Stopping doh-proxy (PID {pid})...",
        style("◼").yellow()
    );

    send_sigterm(pid)?;

    // Wait up to 5 seconds for the process to exit
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        thread::sleep(Duration::from_millis(100));
        if !process_running(pid) {
            println!("{} Stopped.", style("○").dim());
            return Ok(());
        }
        if Instant::now() > deadline {
            eprintln!(
                "{} Process did not exit within 5 seconds. Send SIGKILL manually: kill -9 {pid}",
                style("✗").red()
            );
            return Err(anyhow::anyhow!("timeout waiting for process {pid} to exit"));
        }
    }
}

#[cfg(unix)]
fn process_running(pid: u32) -> bool {
    use nix::sys::signal::kill;
    use nix::unistd::Pid;
    kill(Pid::from_raw(pid as i32), None).is_ok()
}

#[cfg(unix)]
fn send_sigterm(pid: u32) -> anyhow::Result<()> {
    use nix::sys::signal::{kill, Signal};
    use nix::unistd::Pid;
    kill(Pid::from_raw(pid as i32), Signal::SIGTERM)
        .map_err(|e| anyhow::anyhow!("failed to send SIGTERM to {pid}: {e}"))
}

#[cfg(not(unix))]
fn process_running(_pid: u32) -> bool {
    false
}

#[cfg(not(unix))]
fn send_sigterm(pid: u32) -> anyhow::Result<()> {
    Err(anyhow::anyhow!(
        "`doh-proxy stop` is not supported on this platform. Kill PID {pid} manually."
    ))
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build 2>&1 | grep "^error" | head -10`
Expected: no errors.

- [ ] **Step 3: Run all tests to ensure nothing is broken**

Run: `cargo test`
Expected: all existing tests PASS.

- [ ] **Step 4: Commit**

```bash
git add src/cli/stop.rs
git commit -m "feat: implement doh-proxy stop command with SIGTERM and timeout"
```

---

## Task 10: Remove dead code (GUI, TUI, old binary)

**Files:**
- Delete: `gui/` directory
- Delete: `src/tui/` directory
- Delete: `src/tui_main.rs`
- Modify: `src/lib.rs` (remove `pub mod tui`)
- Delete: `.github/workflows/release.yml`

- [ ] **Step 1: Remove GUI directory**

```bash
git rm -r gui/
```

- [ ] **Step 2: Remove TUI source**

```bash
git rm -r src/tui/ src/tui_main.rs
```

- [ ] **Step 3: Remove `pub mod tui;` from `src/lib.rs`**

Edit `src/lib.rs` to be:

```rust
pub mod cache;
pub mod config;
pub mod error;
pub mod proxy;
pub mod resolver;
pub mod runtime;
pub mod server;
pub mod stats;
pub mod upstream;
```

- [ ] **Step 4: Remove old release workflow**

```bash
git rm .github/workflows/release.yml
```

- [ ] **Step 5: Update workspace in Cargo.toml**

The `Cargo.toml` written in Task 1 does not declare a workspace, so there's nothing to change. Verify:

Run: `grep -n workspace Cargo.toml`
Expected: no output (no workspace key).

- [ ] **Step 6: Build and test everything**

Run: `cargo build`
Expected: clean build, no errors, no warnings about unused modules.

Run: `cargo test`
Expected: all tests PASS.

- [ ] **Step 7: Verify binary help**

Run: `cargo run -- --help`
Expected: clean help output with all 5 subcommands listed.

- [ ] **Step 8: Commit**

```bash
git add src/lib.rs
git commit -m "chore: remove GUI, TUI, and old release workflow; single doh-proxy binary"
```

---

## Task 11: CI workflow and README rewrite

**Files:**
- Create: `.github/workflows/ci.yml`
- Rewrite: `README.md`

- [ ] **Step 1: Create `.github/workflows/ci.yml`**

```yaml
name: CI

on:
  push:
    branches: ["**"]
  pull_request:
    branches: ["**"]

env:
  CARGO_TERM_COLOR: always

jobs:
  test:
    name: Test (${{ matrix.os }})
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest]

    steps:
      - uses: actions/checkout@v4

      - name: Install Rust stable
        uses: dtolnay/rust-toolchain@stable

      - name: Cache cargo registry
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
          restore-keys: ${{ runner.os }}-cargo-

      - name: Build
        run: cargo build --locked

      - name: Run tests
        run: cargo test --locked

  lint:
    name: Lint
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust stable
        uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt

      - name: Check formatting
        run: cargo fmt --check

      - name: Clippy
        run: cargo clippy -- -D warnings
```

- [ ] **Step 2: Rewrite `README.md`**

```markdown
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
```

- [ ] **Step 3: Verify CI file is valid YAML**

Run: `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))" && echo "OK"`
Expected: `OK`

- [ ] **Step 4: Run final tests**

Run: `cargo test`
Expected: all tests PASS.

Run: `cargo build --release`
Expected: produces a single `target/release/doh-proxy` binary.

- [ ] **Step 5: Commit**

```bash
git add .github/workflows/ci.yml README.md
git commit -m "docs: rewrite README for CLI install; add GitHub Actions CI workflow"
```

---

## Self-Review

### Spec coverage

| Requirement | Task |
|------------|------|
| `doh-proxy start` | Task 5 |
| `doh-proxy config` | Task 6 |
| `doh-proxy status` | Task 7 |
| `doh-proxy logs` | Task 8 |
| `doh-proxy stop` | Task 9 |
| `clap` with rich help text | Task 4 |
| `indicatif` + `console` spinners/colors | Tasks 1, 5, 6, 7, 8, 9 |
| Config at `~/.config/doh-proxy/config.toml` | Task 2 |
| Structured logging with `tracing` | Task 5 |
| Single binary output | Tasks 1, 10 |
| Publishable to crates.io (metadata) | Task 1 |
| `cargo install doh-proxy` in README | Task 11 |
| GitHub Actions CI | Task 11 |
| Remove TypeScript/GUI | Task 10 |
| Keep existing proxy/DNS logic | All tasks (lib unchanged) |

All spec requirements are covered. No gaps found.

### Placeholder scan

No TBDs, TODOs, or "implement later" markers — all steps contain complete, runnable code.

### Type consistency

- `StatsSnapshot` is modified once in Task 3 (add `Deserialize`) and used consistently in `RuntimeStats` throughout Tasks 3–7.
- `RuntimeStats` is defined in `src/runtime.rs` (Task 3) and consumed by `start.rs` (Task 5), `status.rs` (Task 7).
- `runtime::read_pid()` / `write_pid()` / `clear_pid()` signatures defined in Task 3 are called identically in Tasks 5 and 9.
- `process_running(pid: u32)` is defined independently in both `start.rs` and `status.rs` / `stop.rs` — intentional duplication to keep modules self-contained.
