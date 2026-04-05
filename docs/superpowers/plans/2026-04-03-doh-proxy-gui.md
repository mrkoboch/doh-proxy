# DoH Proxy GUI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a native cross-platform GUI binary (`doh-proxy-gui`) that lets users start/stop the proxy, edit configuration, and watch live query stats in a clean, minimalistic window they can download and run locally.

**Architecture:** The GUI binary (`src/gui_main.rs`) uses `eframe`/`egui` (immediate-mode native Rust GUI) running on the main thread. A background `std::thread` owns a `tokio::runtime::Runtime` that runs the existing proxy `Server`. A shared `Arc<Stats>` struct feeds atomic counters and a bounded 200-entry ring-buffer log from the proxy to the GUI without blocking hot paths. An `Arc<AtomicBool>` stop flag lets the GUI signal the server to shut down. The four GUI tabs (Dashboard, Config, Stats, Query Log) share one `DohProxyApp` struct.

**Tech Stack:** Rust 2021, `eframe = "0.29"` / `egui = "0.29"` (verify latest on crates.io before adding), `tokio` (already present), `std::sync::atomic`, `std::sync::Mutex`, `hickory-proto` (already present for parsing query names in stats).

> **Version note:** Before Task 4, run `cargo search eframe` or check https://crates.io/crates/eframe to confirm the latest compatible 0.x version. The API described here tracks eframe 0.29; later minor versions are source-compatible.

---

## File Map

### New files
| File | Responsibility |
|---|---|
| `src/stats.rs` | `Stats` struct: atomic counters + 200-entry `LogEntry` ring-buffer |
| `src/gui_main.rs` | `fn main()` for the GUI binary — loads config, launches `DohProxyApp` |
| `src/gui/mod.rs` | Re-exports for the gui module |
| `src/gui/app.rs` | `DohProxyApp` struct + `eframe::App` impl: tab routing, server state machine |
| `src/gui/dashboard.rs` | Dashboard tab: status indicator, start/stop button, upstreams list |
| `src/gui/config_panel.rs` | Config tab: editable fields, "Save Config" writes TOML to disk |
| `src/gui/stats_panel.rs` | Stats tab: live counters with percentages |
| `src/gui/log_panel.rs` | Query Log tab: scrollable table of recent queries |

### Modified files
| File | What changes |
|---|---|
| `Cargo.toml` | Add `[[bin]] doh-proxy-gui`, add `eframe`/`egui` deps |
| `src/lib.rs` | `pub mod stats; pub mod gui;` |
| `src/resolver.rs` | Accept `Option<Arc<Stats>>`, record `LogEntry` per resolve |
| `src/server.rs` | Accept `Option<Arc<Stats>>` + `Arc<AtomicBool>` stop flag; use timeout recv loop |

---

## Task 1: Stats Module

**Files:**
- Create: `src/stats.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Create `src/stats.rs`**

```rust
// src/stats.rs
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

pub const LOG_CAPACITY: usize = 200;

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: SystemTime,
    pub query_name: String,
    pub query_type: String,
    pub status: QueryStatus,
    pub latency_ms: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum QueryStatus {
    CacheHit,
    Upstream,
    Error,
}

impl std::fmt::Display for QueryStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueryStatus::CacheHit => write!(f, "Cache Hit"),
            QueryStatus::Upstream => write!(f, "Upstream"),
            QueryStatus::Error    => write!(f, "Error"),
        }
    }
}

pub struct Stats {
    pub total_queries: AtomicU64,
    pub cache_hits:    AtomicU64,
    pub upstream_queries: AtomicU64,
    pub errors:        AtomicU64,
    log: Mutex<VecDeque<LogEntry>>,
}

impl Stats {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            total_queries:    AtomicU64::new(0),
            cache_hits:       AtomicU64::new(0),
            upstream_queries: AtomicU64::new(0),
            errors:           AtomicU64::new(0),
            log: Mutex::new(VecDeque::with_capacity(LOG_CAPACITY)),
        })
    }

    pub fn record(&self, entry: LogEntry) {
        self.total_queries.fetch_add(1, Ordering::Relaxed);
        match entry.status {
            QueryStatus::CacheHit => { self.cache_hits.fetch_add(1, Ordering::Relaxed); }
            QueryStatus::Upstream => { self.upstream_queries.fetch_add(1, Ordering::Relaxed); }
            QueryStatus::Error    => { self.errors.fetch_add(1, Ordering::Relaxed); }
        }
        let mut log = self.log.lock().unwrap();
        if log.len() >= LOG_CAPACITY {
            log.pop_front();
        }
        log.push_back(entry);
    }

    pub fn total(&self)    -> u64 { self.total_queries.load(Ordering::Relaxed) }
    pub fn cache_hits(&self) -> u64 { self.cache_hits.load(Ordering::Relaxed) }
    pub fn upstream(&self) -> u64 { self.upstream_queries.load(Ordering::Relaxed) }
    pub fn errors(&self)   -> u64 { self.errors.load(Ordering::Relaxed) }

    pub fn snapshot_log(&self) -> Vec<LogEntry> {
        self.log.lock().unwrap().iter().rev().cloned().collect()
    }
}
```

- [ ] **Step 2: Add `pub mod stats;` to `src/lib.rs`**

Edit `src/lib.rs`:
```rust
pub mod cache;
pub mod config;
pub mod error;
pub mod proxy;
pub mod resolver;
pub mod server;
pub mod stats;
pub mod upstream;
```

- [ ] **Step 3: Write tests for Stats**

Add to end of `src/stats.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counters_increment_correctly() {
        let s = Stats::new();
        s.record(LogEntry {
            timestamp: SystemTime::now(),
            query_name: "example.com".into(),
            query_type: "A".into(),
            status: QueryStatus::CacheHit,
            latency_ms: 0,
        });
        s.record(LogEntry {
            timestamp: SystemTime::now(),
            query_name: "foo.com".into(),
            query_type: "AAAA".into(),
            status: QueryStatus::Upstream,
            latency_ms: 12,
        });
        s.record(LogEntry {
            timestamp: SystemTime::now(),
            query_name: "bad.com".into(),
            query_type: "A".into(),
            status: QueryStatus::Error,
            latency_ms: 5,
        });
        assert_eq!(s.total(), 3);
        assert_eq!(s.cache_hits(), 1);
        assert_eq!(s.upstream(), 1);
        assert_eq!(s.errors(), 1);
    }

    #[test]
    fn log_caps_at_capacity() {
        let s = Stats::new();
        for i in 0..=LOG_CAPACITY {
            s.record(LogEntry {
                timestamp: SystemTime::now(),
                query_name: format!("{i}.com"),
                query_type: "A".into(),
                status: QueryStatus::Upstream,
                latency_ms: 0,
            });
        }
        assert_eq!(s.snapshot_log().len(), LOG_CAPACITY);
    }

    #[test]
    fn snapshot_log_newest_first() {
        let s = Stats::new();
        for name in ["first.com", "second.com", "third.com"] {
            s.record(LogEntry {
                timestamp: SystemTime::now(),
                query_name: name.into(),
                query_type: "A".into(),
                status: QueryStatus::Upstream,
                latency_ms: 0,
            });
        }
        let log = s.snapshot_log();
        assert_eq!(log[0].query_name, "third.com");
        assert_eq!(log[2].query_name, "first.com");
    }
}
```

- [ ] **Step 4: Run tests**

```bash
cd /root/doh_proxy && cargo test stats
```

Expected: 3 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/stats.rs src/lib.rs
git commit -m "feat: add Stats module with atomic counters and query log ring-buffer"
```

---

## Task 2: Wire Stats into Resolver

**Files:**
- Modify: `src/resolver.rs`

The resolver is the right place to record stats because it knows whether the response came from cache or upstream, and it has the parsed DNS message for query name/type.

- [ ] **Step 1: Write failing test verifying stats are populated**

Add to end of `src/resolver.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::Stats;

    // Helper: build a minimal A-query for "test.example." as raw bytes.
    fn make_query() -> Vec<u8> {
        use hickory_proto::op::{Message, Query};
        use hickory_proto::rr::{Name, RecordType};
        use hickory_proto::serialize::binary::BinEncodable;
        use std::str::FromStr;

        let mut msg = Message::new();
        msg.set_id(1);
        let mut q = Query::new();
        q.set_name(Name::from_str("test.example.").unwrap());
        q.set_query_type(RecordType::A);
        msg.add_query(q);
        msg.to_bytes().unwrap()
    }

    #[tokio::test]
    async fn stats_record_error_on_upstream_failure() {
        // Use an invalid upstream to force an error path
        let upstream = UpstreamClient::new(vec!["http://127.0.0.1:1/dns-query".into()]).unwrap();
        let stats = Stats::new();
        let resolver = Resolver::new(upstream, None, Some(Arc::clone(&stats)));
        let query = make_query();
        let _ = resolver.resolve(&query).await; // will error
        assert_eq!(stats.errors(), 1);
        assert_eq!(stats.total(), 1);
    }
}
```

- [ ] **Step 2: Run test to confirm it fails to compile** (Stats not wired yet)

```bash
cd /root/doh_proxy && cargo test resolver::tests 2>&1 | head -20
```

Expected: compile error about `Resolver::new` signature mismatch.

- [ ] **Step 3: Update `src/resolver.rs` to accept and record stats**

Replace the entire file:
```rust
use std::sync::Arc;
use std::time::{Instant, SystemTime};

use bytes::Bytes;
use hickory_proto::{
    op::Message,
    rr::RecordType,
    serialize::binary::BinDecodable,
};
use tracing::debug;

use crate::{
    cache::{CacheKey, DnsCache},
    error::{ProxyError, Result},
    stats::{LogEntry, QueryStatus, Stats},
    upstream::UpstreamClient,
};

pub struct Resolver {
    upstream: UpstreamClient,
    cache: Option<DnsCache>,
    stats: Option<Arc<Stats>>,
}

impl Resolver {
    pub fn new(upstream: UpstreamClient, cache: Option<DnsCache>, stats: Option<Arc<Stats>>) -> Self {
        Self { upstream, cache, stats }
    }

    pub async fn resolve(&self, raw_query: &[u8]) -> Result<Bytes> {
        let start = Instant::now();
        let msg = Message::from_bytes(raw_query).map_err(ProxyError::DnsParse)?;

        let (query_name, query_type_str) = query_info(&msg);
        let cache_key = cache_key_for(&msg);

        if let Some(ref cache) = self.cache {
            if let Some(cached) = cache.get(&cache_key).await {
                debug!("cache hit");
                self.record_stat(query_name, query_type_str, QueryStatus::CacheHit, start);
                return Ok(cached);
            }
        }

        match self.upstream.resolve(raw_query).await {
            Ok(response_bytes) => {
                let min_ttl = extract_min_ttl(&response_bytes).unwrap_or(60);
                if let Some(ref cache) = self.cache {
                    cache.insert(cache_key, response_bytes.clone(), min_ttl).await;
                }
                self.record_stat(query_name, query_type_str, QueryStatus::Upstream, start);
                Ok(response_bytes)
            }
            Err(e) => {
                self.record_stat(query_name, query_type_str, QueryStatus::Error, start);
                Err(e)
            }
        }
    }

    fn record_stat(&self, name: String, qtype: String, status: QueryStatus, start: Instant) {
        if let Some(ref stats) = self.stats {
            stats.record(LogEntry {
                timestamp: SystemTime::now(),
                query_name: name,
                query_type: qtype,
                status,
                latency_ms: start.elapsed().as_millis() as u64,
            });
        }
    }
}

fn query_info(msg: &Message) -> (String, String) {
    if let Some(q) = msg.queries().first() {
        let name = q.name().to_lowercase().to_string();
        let qtype = format!("{}", RecordType::from(q.query_type()));
        (name, qtype)
    } else {
        (String::new(), "?".into())
    }
}

fn cache_key_for(msg: &Message) -> CacheKey {
    if let Some(q) = msg.queries().first() {
        (q.name().to_lowercase().to_string(), q.query_type().into())
    } else {
        (String::new(), 0)
    }
}

fn extract_min_ttl(raw: &[u8]) -> Option<u64> {
    let msg = Message::from_bytes(raw).ok()?;
    msg.answers().iter().map(|r| r.ttl() as u64).min()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::Stats;

    fn make_query() -> Vec<u8> {
        use hickory_proto::op::{Message, Query};
        use hickory_proto::rr::{Name, RecordType};
        use hickory_proto::serialize::binary::BinEncodable;
        use std::str::FromStr;

        let mut msg = Message::new();
        msg.set_id(1);
        let mut q = Query::new();
        q.set_name(Name::from_str("test.example.").unwrap());
        q.set_query_type(RecordType::A);
        msg.add_query(q);
        msg.to_bytes().unwrap()
    }

    #[tokio::test]
    async fn stats_record_error_on_upstream_failure() {
        let upstream = UpstreamClient::new(vec!["http://127.0.0.1:1/dns-query".into()]).unwrap();
        let stats = Stats::new();
        let resolver = Resolver::new(upstream, None, Some(Arc::clone(&stats)));
        let query = make_query();
        let _ = resolver.resolve(&query).await;
        assert_eq!(stats.errors(), 1);
        assert_eq!(stats.total(), 1);
    }
}
```

- [ ] **Step 4: Fix the call site in `src/server.rs`** — `Resolver::new` now takes 3 args

Edit `src/server.rs`, find the `Resolver::new(upstream, cache)` call and replace with `Resolver::new(upstream, cache, None)`. (Stats wiring into server happens in Task 3.)

- [ ] **Step 5: Run tests**

```bash
cd /root/doh_proxy && cargo test
```

Expected: all tests pass including `stats_record_error_on_upstream_failure`.

- [ ] **Step 6: Commit**

```bash
git add src/resolver.rs src/server.rs
git commit -m "feat: wire Stats into Resolver for per-query logging and counters"
```

---

## Task 3: Add Stop Signal to Server

**Files:**
- Modify: `src/server.rs`

The GUI needs to stop the running server. We add a `run_cancellable` method that polls an `Arc<AtomicBool>` stop flag using a 100ms timeout on `recv_from`.

- [ ] **Step 1: Update `src/server.rs`**

Replace the entire file:
```rust
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::net::UdpSocket;
use tracing::{error, info};

use crate::{
    cache::DnsCache,
    config::Config,
    error::Result,
    proxy::Proxy,
    resolver::Resolver,
    stats::Stats,
    upstream::UpstreamClient,
};

pub struct Server {
    proxy: Arc<Proxy>,
    config: Config,
}

impl Server {
    pub async fn new(config: Config, stats: Option<Arc<Stats>>) -> Result<Self> {
        let upstream = UpstreamClient::new(config.upstreams.clone())?;
        let cache = if config.cache.enabled {
            Some(DnsCache::new(config.cache.capacity))
        } else {
            None
        };
        let resolver = Resolver::new(upstream, cache, stats);
        let proxy = Arc::new(Proxy::new(resolver));
        Ok(Self { proxy, config })
    }

    /// Run until `stop` is set to true.
    pub async fn run_cancellable(self, stop: Arc<AtomicBool>) -> anyhow::Result<()> {
        let socket = Arc::new(UdpSocket::bind(self.config.listen_addr).await?);
        info!(addr = %self.config.listen_addr, "UDP listener ready");

        let mut buf = vec![0u8; 4096];
        loop {
            if stop.load(Ordering::Relaxed) {
                info!("stop signal received, shutting down");
                break;
            }
            match tokio::time::timeout(
                Duration::from_millis(100),
                socket.recv_from(&mut buf),
            )
            .await
            {
                Ok(Ok((len, peer))) => {
                    let query = buf[..len].to_vec();
                    let proxy = Arc::clone(&self.proxy);
                    let socket = Arc::clone(&socket);
                    tokio::spawn(async move {
                        let response = proxy.handle(&query).await;
                        if let Err(e) = socket.send_to(&response, peer).await {
                            error!(peer = %peer, error = %e, "failed to send response");
                        }
                    });
                }
                Ok(Err(e)) => {
                    error!(error = %e, "recv_from error");
                }
                Err(_) => {} // timeout — loop back and check stop flag
            }
        }
        Ok(())
    }

    /// Convenience: run forever (used by the CLI binary).
    pub async fn run(self) -> anyhow::Result<()> {
        let stop = Arc::new(AtomicBool::new(false));
        self.run_cancellable(stop).await
    }
}
```

- [ ] **Step 2: Fix `src/main.rs`** — `Server::new` now requires a second arg

Edit `src/main.rs`: change `Server::new(config).await?` to `Server::new(config, None).await?`.

- [ ] **Step 3: Verify it compiles and existing tests pass**

```bash
cd /root/doh_proxy && cargo test
```

Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/server.rs src/main.rs
git commit -m "feat: add cancellable server run loop for GUI start/stop control"
```

---

## Task 4: Add GUI Dependencies and Binary Scaffold

**Files:**
- Modify: `Cargo.toml`
- Create: `src/gui_main.rs`

- [ ] **Step 1: Check the latest eframe version**

```bash
cargo search eframe 2>&1 | head -5
```

Note the latest `0.x` version. Use it in the next step (e.g., `"0.29"` or `"0.31"`).

- [ ] **Step 2: Update `Cargo.toml`**

Add after the existing `[[bin]]` block and before `[lib]`:
```toml
[[bin]]
name = "doh-proxy-gui"
path = "src/gui_main.rs"
```

Add to `[dependencies]` (replace `"0.29"` with the version found in Step 1):
```toml
eframe = { version = "0.29", default-features = false, features = ["default_fonts", "glow"] }
egui = "0.29"
```

- [ ] **Step 3: Create `src/gui_main.rs`**

```rust
// src/gui_main.rs
fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("DoH Proxy")
            .with_inner_size([700.0, 500.0])
            .with_min_inner_size([500.0, 400.0]),
        ..Default::default()
    };

    eframe::run_native(
        "DoH Proxy",
        native_options,
        Box::new(|cc| {
            Ok(Box::new(doh_proxy::gui::app::DohProxyApp::new(cc)))
        }),
    )
}
```

- [ ] **Step 4: Create stub module files so it compiles**

Create `src/gui/mod.rs`:
```rust
pub mod app;
pub mod dashboard;
pub mod config_panel;
pub mod stats_panel;
pub mod log_panel;
```

Create `src/gui/app.rs` (stub):
```rust
pub struct DohProxyApp;

impl DohProxyApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self
    }
}

impl eframe::App for DohProxyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("DoH Proxy GUI - coming soon");
        });
    }
}
```

Create empty stubs:
```rust
// src/gui/dashboard.rs
// src/gui/config_panel.rs
// src/gui/stats_panel.rs
// src/gui/log_panel.rs
```
(Each is an empty file for now.)

- [ ] **Step 5: Add `pub mod gui;` to `src/lib.rs`**

```rust
pub mod cache;
pub mod config;
pub mod error;
pub mod gui;
pub mod proxy;
pub mod resolver;
pub mod server;
pub mod stats;
pub mod upstream;
```

- [ ] **Step 6: Verify both binaries compile**

```bash
cd /root/doh_proxy && cargo build 2>&1 | tail -5
```

Expected: `Compiling doh-proxy ...` and `Compiling doh-proxy-gui ...` with no errors.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml src/gui_main.rs src/gui/mod.rs src/gui/app.rs src/gui/dashboard.rs src/gui/config_panel.rs src/gui/stats_panel.rs src/gui/log_panel.rs src/lib.rs
git commit -m "feat: scaffold doh-proxy-gui binary with eframe stub"
```

---

## Task 5: Full App State and Tab Skeleton

**Files:**
- Modify: `src/gui/app.rs`

- [ ] **Step 1: Replace stub `src/gui/app.rs` with full state struct**

```rust
// src/gui/app.rs
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use doh_proxy::{config::Config, stats::Stats};

use crate::gui::{
    config_panel::ConfigPanel,
    dashboard::Dashboard,
    log_panel::LogPanel,
    stats_panel::StatsPanel,
};

#[derive(PartialEq)]
pub enum Tab {
    Dashboard,
    Config,
    Stats,
    QueryLog,
}

pub enum ServerState {
    Stopped,
    Running {
        stop_flag: Arc<AtomicBool>,
        _thread: std::thread::JoinHandle<()>,
    },
}

pub struct DohProxyApp {
    pub config: Config,
    pub config_path: String,
    pub stats: Arc<Stats>,
    pub server_state: ServerState,
    pub active_tab: Tab,
    pub status_message: Option<String>,
    pub config_panel: ConfigPanel,
}

impl DohProxyApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let config_path = "config/default.toml".to_string();
        let config = Config::from_file(&config_path)
            .unwrap_or_else(|_| Config::default());
        let config_panel = ConfigPanel::from_config(&config);
        Self {
            config: config.clone(),
            config_path,
            stats: Stats::new(),
            server_state: ServerState::Stopped,
            active_tab: Tab::Dashboard,
            status_message: None,
            config_panel,
        }
    }

    pub fn is_running(&self) -> bool {
        matches!(self.server_state, ServerState::Running { .. })
    }
}

impl eframe::App for DohProxyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Request repaint every 500ms so stats refresh while running
        ctx.request_repaint_after(std::time::Duration::from_millis(500));

        egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.active_tab, Tab::Dashboard, "Dashboard");
                ui.selectable_value(&mut self.active_tab, Tab::Config, "Config");
                ui.selectable_value(&mut self.active_tab, Tab::Stats, "Stats");
                ui.selectable_value(&mut self.active_tab, Tab::QueryLog, "Query Log");
            });
        });

        if let Some(ref msg) = self.status_message.clone() {
            egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
                ui.label(egui::RichText::new(msg).small());
            });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            match self.active_tab {
                Tab::Dashboard => Dashboard::show(self, ui),
                Tab::Config    => ConfigPanel::show(self, ui),
                Tab::Stats     => StatsPanel::show(self, ui),
                Tab::QueryLog  => LogPanel::show(self, ui),
            }
        });
    }
}
```

- [ ] **Step 2: Add `Config::default()` to `src/config.rs`**

Add after the existing `impl Config` block:
```rust
impl Default for Config {
    fn default() -> Self {
        Self {
            listen_addr: default_listen_addr(),
            upstreams: default_upstreams(),
            cache: CacheConfig::default(),
        }
    }
}
```

- [ ] **Step 3: Verify it compiles (stubs still used for tab panels)**

```bash
cd /root/doh_proxy && cargo build --bin doh-proxy-gui 2>&1 | tail -10
```

Expected: compile errors only about missing functions in stub files (Dashboard::show, etc.) — fix by adding empty impls to each stub:

`src/gui/dashboard.rs`:
```rust
use super::app::DohProxyApp;

pub struct Dashboard;
impl Dashboard {
    pub fn show(_app: &mut DohProxyApp, ui: &mut egui::Ui) {
        ui.label("Dashboard");
    }
}
```

`src/gui/config_panel.rs`:
```rust
use doh_proxy::config::Config;
use super::app::DohProxyApp;

pub struct ConfigPanel {
    pub listen_addr: String,
    pub upstreams: Vec<String>,
    pub cache_enabled: bool,
    pub cache_capacity: String,
}

impl ConfigPanel {
    pub fn from_config(config: &Config) -> Self {
        Self {
            listen_addr: config.listen_addr.to_string(),
            upstreams: config.upstreams.clone(),
            cache_enabled: config.cache.enabled,
            cache_capacity: config.cache.capacity.to_string(),
        }
    }
    pub fn show(_app: &mut DohProxyApp, _ui: &mut egui::Ui) {}
}
```

`src/gui/stats_panel.rs`:
```rust
use super::app::DohProxyApp;

pub struct StatsPanel;
impl StatsPanel {
    pub fn show(_app: &mut DohProxyApp, _ui: &mut egui::Ui) {}
}
```

`src/gui/log_panel.rs`:
```rust
use super::app::DohProxyApp;

pub struct LogPanel;
impl LogPanel {
    pub fn show(_app: &mut DohProxyApp, _ui: &mut egui::Ui) {}
}
```

- [ ] **Step 4: Confirm clean build**

```bash
cd /root/doh_proxy && cargo build --bin doh-proxy-gui 2>&1 | tail -5
```

Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add src/gui/app.rs src/gui/dashboard.rs src/gui/config_panel.rs src/gui/stats_panel.rs src/gui/log_panel.rs src/config.rs
git commit -m "feat: full app state struct with tab routing skeleton"
```

---

## Task 6: Dashboard Tab — Start / Stop Server

**Files:**
- Modify: `src/gui/dashboard.rs`
- Modify: `src/gui/app.rs` (start/stop helpers)

- [ ] **Step 1: Add `start_server` and `stop_server` methods to `DohProxyApp` in `src/gui/app.rs`**

Add these methods inside `impl DohProxyApp`:
```rust
pub fn start_server(&mut self) {
    if self.is_running() {
        return;
    }
    let stop_flag = Arc::new(AtomicBool::new(false));
    let config = self.config.clone();
    let stats = Arc::clone(&self.stats);
    let stop_clone = Arc::clone(&stop_flag);

    let thread = std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        rt.block_on(async {
            match doh_proxy::server::Server::new(config, Some(stats)).await {
                Ok(server) => {
                    if let Err(e) = server.run_cancellable(stop_clone).await {
                        tracing::error!("server error: {}", e);
                    }
                }
                Err(e) => {
                    tracing::error!("server init error: {}", e);
                }
            }
        });
    });

    self.server_state = ServerState::Running { stop_flag, _thread: thread };
    self.status_message = Some(format!("Server started on {}", self.config.listen_addr));
}

pub fn stop_server(&mut self) {
    if let ServerState::Running { ref stop_flag, .. } = self.server_state {
        stop_flag.store(true, Ordering::Relaxed);
    }
    self.server_state = ServerState::Stopped;
    self.status_message = Some("Server stopped.".into());
}
```

- [ ] **Step 2: Implement Dashboard::show in `src/gui/dashboard.rs`**

```rust
// src/gui/dashboard.rs
use egui::{Color32, RichText};

use super::app::DohProxyApp;

pub struct Dashboard;

impl Dashboard {
    pub fn show(app: &mut DohProxyApp, ui: &mut egui::Ui) {
        ui.add_space(16.0);

        // Status indicator
        ui.horizontal(|ui| {
            let (dot_color, status_text) = if app.is_running() {
                (Color32::from_rgb(72, 199, 116), "Running")
            } else {
                (Color32::from_rgb(180, 180, 180), "Stopped")
            };
            ui.colored_label(dot_color, "●");
            ui.label(RichText::new(status_text).strong().size(16.0));
        });

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        // Listen address
        ui.horizontal(|ui| {
            ui.label(RichText::new("Listen Address:").strong());
            ui.label(app.config.listen_addr.to_string());
        });

        ui.add_space(8.0);

        // Upstreams
        ui.label(RichText::new("Upstream Servers:").strong());
        for url in &app.config.upstreams {
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.label("▸");
                ui.label(url);
            });
        }

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(16.0);

        // Start / Stop button
        let (btn_label, btn_color) = if app.is_running() {
            ("■  Stop", Color32::from_rgb(220, 80, 80))
        } else {
            ("▶  Start", Color32::from_rgb(72, 199, 116))
        };

        let btn = egui::Button::new(RichText::new(btn_label).size(15.0))
            .fill(btn_color)
            .min_size(egui::vec2(120.0, 36.0));

        if ui.add(btn).clicked() {
            if app.is_running() {
                app.stop_server();
            } else {
                app.start_server();
            }
        }

        ui.add_space(12.0);

        // Quick stats row
        if app.is_running() {
            ui.separator();
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(format!("Queries: {}", app.stats.total()));
                ui.add_space(20.0);
                ui.label(format!("Cache Hits: {}", app.stats.cache_hits()));
                ui.add_space(20.0);
                ui.label(format!("Errors: {}", app.stats.errors()));
            });
        }
    }
}
```

- [ ] **Step 3: Build and quick visual check**

```bash
cd /root/doh_proxy && cargo build --bin doh-proxy-gui && ./target/debug/doh-proxy-gui
```

Expected: a window opens with a Dashboard tab showing "● Stopped", a "▶ Start" button, and the configured listen address. The button should toggle (you can try pressing Start — it will attempt to bind `0.0.0.0:5353`).

Close the window after verifying.

- [ ] **Step 4: Commit**

```bash
git add src/gui/app.rs src/gui/dashboard.rs
git commit -m "feat: Dashboard tab with start/stop server and live status indicator"
```

---

## Task 7: Config Tab

**Files:**
- Modify: `src/gui/config_panel.rs`

The config tab lets the user edit listen address, upstream URLs, and cache settings, then saves to the TOML file.

- [ ] **Step 1: Replace `src/gui/config_panel.rs` with full implementation**

```rust
// src/gui/config_panel.rs
use egui::RichText;

use doh_proxy::config::{CacheConfig, Config};

use super::app::DohProxyApp;

pub struct ConfigPanel {
    pub listen_addr: String,
    pub upstreams: Vec<String>,
    pub cache_enabled: bool,
    pub cache_capacity: String,
}

impl ConfigPanel {
    pub fn from_config(config: &Config) -> Self {
        Self {
            listen_addr: config.listen_addr.to_string(),
            upstreams: config.upstreams.clone(),
            cache_enabled: config.cache.enabled,
            cache_capacity: config.cache.capacity.to_string(),
        }
    }

    pub fn show(app: &mut DohProxyApp, ui: &mut egui::Ui) {
        if app.is_running() {
            ui.add_space(8.0);
            ui.colored_label(
                egui::Color32::from_rgb(255, 200, 0),
                "⚠  Stop the server before editing config.",
            );
            ui.add_space(8.0);
        }

        ui.add_space(8.0);
        let enabled = !app.is_running();

        // Listen address
        ui.horizontal(|ui| {
            ui.label(RichText::new("Listen Address:").strong().monospace());
            ui.add_enabled(
                enabled,
                egui::TextEdit::singleline(&mut app.config_panel.listen_addr).desired_width(200.0),
            );
        });

        ui.add_space(8.0);

        // Upstream URLs
        ui.label(RichText::new("Upstream URLs:").strong());
        let mut to_remove: Option<usize> = None;
        for (i, url) in app.config_panel.upstreams.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.add_enabled(
                    enabled,
                    egui::TextEdit::singleline(url).desired_width(360.0),
                );
                if enabled && ui.small_button("✕").clicked() {
                    to_remove = Some(i);
                }
            });
        }
        if let Some(idx) = to_remove {
            app.config_panel.upstreams.remove(idx);
        }
        if enabled && ui.small_button("+ Add upstream").clicked() {
            app.config_panel.upstreams.push("https://".into());
        }

        ui.add_space(8.0);

        // Cache settings
        ui.horizontal(|ui| {
            ui.label(RichText::new("Cache:").strong());
            ui.add_enabled(enabled, egui::Checkbox::new(&mut app.config_panel.cache_enabled, "Enabled"));
            ui.add_space(16.0);
            ui.label("Capacity:");
            ui.add_enabled(
                enabled,
                egui::TextEdit::singleline(&mut app.config_panel.cache_capacity).desired_width(80.0),
            );
        });

        ui.add_space(16.0);

        // Save button
        if enabled && ui.button(RichText::new("💾  Save Config").size(14.0)).clicked() {
            match Self::apply_and_save(app) {
                Ok(()) => {
                    app.status_message = Some("Config saved.".into());
                }
                Err(e) => {
                    app.status_message = Some(format!("Save failed: {e}"));
                }
            }
        }
    }

    fn apply_and_save(app: &mut DohProxyApp) -> anyhow::Result<()> {
        use std::io::Write;

        let listen_addr = app.config_panel.listen_addr.parse()
            .map_err(|e| anyhow::anyhow!("invalid listen address: {e}"))?;
        let capacity: u64 = app.config_panel.cache_capacity.trim().parse()
            .map_err(|e| anyhow::anyhow!("invalid cache capacity: {e}"))?;

        let upstreams: Vec<String> = app.config_panel.upstreams
            .iter()
            .filter(|u| !u.trim().is_empty())
            .cloned()
            .collect();
        if upstreams.is_empty() {
            return Err(anyhow::anyhow!("at least one upstream URL is required"));
        }

        // Build TOML manually to keep it readable
        let toml = format!(
            r#"listen_addr = "{listen_addr}"

upstreams = [
{upstream_lines}]

[cache]
enabled = {cache_enabled}
capacity = {capacity}
"#,
            upstream_lines = upstreams.iter()
                .map(|u| format!("    \"{u}\",\n"))
                .collect::<String>(),
            cache_enabled = app.config_panel.cache_enabled,
        );

        let mut f = std::fs::File::create(&app.config_path)
            .map_err(|e| anyhow::anyhow!("cannot write config: {e}"))?;
        f.write_all(toml.as_bytes())?;

        // Update live config
        app.config.listen_addr = listen_addr;
        app.config.upstreams = upstreams;
        app.config.cache.enabled = app.config_panel.cache_enabled;
        app.config.cache.capacity = capacity;

        Ok(())
    }
}
```

- [ ] **Step 2: Build**

```bash
cd /root/doh_proxy && cargo build --bin doh-proxy-gui 2>&1 | tail -5
```

Expected: no errors.

- [ ] **Step 3: Quick visual verify**

```bash
./target/debug/doh-proxy-gui
```

Click "Config" tab. Edit an upstream URL, click "Save Config". Confirm `config/default.toml` was updated:

```bash
cat /root/doh_proxy/config/default.toml
```

- [ ] **Step 4: Commit**

```bash
git add src/gui/config_panel.rs
git commit -m "feat: Config tab with editable fields and TOML save"
```

---

## Task 8: Stats Tab

**Files:**
- Modify: `src/gui/stats_panel.rs`

- [ ] **Step 1: Replace `src/gui/stats_panel.rs`**

```rust
// src/gui/stats_panel.rs
use egui::{Color32, RichText};

use super::app::DohProxyApp;

pub struct StatsPanel;

impl StatsPanel {
    pub fn show(app: &mut DohProxyApp, ui: &mut egui::Ui) {
        ui.add_space(16.0);
        ui.heading("Live Statistics");
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(12.0);

        let total = app.stats.total();
        let hits  = app.stats.cache_hits();
        let ups   = app.stats.upstream();
        let errs  = app.stats.errors();

        let hit_pct = if total > 0 { hits * 100 / total } else { 0 };
        let ups_pct = if total > 0 { ups  * 100 / total } else { 0 };
        let err_pct = if total > 0 { errs * 100 / total } else { 0 };

        egui::Grid::new("stats_grid")
            .num_columns(3)
            .spacing([24.0, 8.0])
            .show(ui, |ui| {
                ui.label(RichText::new("Metric").strong());
                ui.label(RichText::new("Count").strong());
                ui.label(RichText::new("%").strong());
                ui.end_row();

                ui.separator(); ui.separator(); ui.separator();
                ui.end_row();

                ui.label("Total Queries");
                ui.label(RichText::new(total.to_string()).monospace());
                ui.label("—");
                ui.end_row();

                ui.label("Cache Hits");
                ui.colored_label(Color32::from_rgb(72, 199, 116),
                    RichText::new(hits.to_string()).monospace());
                ui.label(format!("{hit_pct}%"));
                ui.end_row();

                ui.label("Upstream Queries");
                ui.label(RichText::new(ups.to_string()).monospace());
                ui.label(format!("{ups_pct}%"));
                ui.end_row();

                ui.label("Errors");
                ui.colored_label(
                    if errs > 0 { Color32::from_rgb(220, 80, 80) } else { Color32::GRAY },
                    RichText::new(errs.to_string()).monospace(),
                );
                ui.label(format!("{err_pct}%"));
                ui.end_row();
            });

        ui.add_space(16.0);

        if total > 0 {
            // Simple text progress bar for cache hit rate
            let filled = (hit_pct as usize).min(50);
            let bar: String = "█".repeat(filled) + &"░".repeat(50 - filled);
            ui.label(RichText::new(format!("Cache hit rate: {bar} {hit_pct}%"))
                .monospace()
                .small()
                .color(Color32::from_rgb(72, 199, 116)));
        }
    }
}
```

- [ ] **Step 2: Build**

```bash
cd /root/doh_proxy && cargo build --bin doh-proxy-gui 2>&1 | tail -5
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/gui/stats_panel.rs
git commit -m "feat: Stats tab with live counters, percentages, and cache-hit bar"
```

---

## Task 9: Query Log Tab

**Files:**
- Modify: `src/gui/log_panel.rs`

- [ ] **Step 1: Replace `src/gui/log_panel.rs`**

```rust
// src/gui/log_panel.rs
use std::time::{SystemTime, UNIX_EPOCH};

use egui::{Color32, RichText, ScrollArea};

use doh_proxy::stats::QueryStatus;

use super::app::DohProxyApp;

pub struct LogPanel;

impl LogPanel {
    pub fn show(app: &mut DohProxyApp, ui: &mut egui::Ui) {
        ui.add_space(8.0);

        let entries = app.stats.snapshot_log();

        if entries.is_empty() {
            ui.add_space(40.0);
            ui.centered_and_justified(|ui| {
                ui.label(RichText::new("No queries yet. Start the server and route DNS traffic through it.")
                    .italics()
                    .color(Color32::GRAY));
            });
            return;
        }

        // Header row
        egui::Grid::new("log_header")
            .num_columns(4)
            .min_col_width(80.0)
            .show(ui, |ui| {
                ui.label(RichText::new("Time").strong());
                ui.label(RichText::new("Query Name").strong());
                ui.label(RichText::new("Type").strong());
                ui.label(RichText::new("Status").strong());
                ui.end_row();
            });

        ui.separator();

        ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                egui::Grid::new("log_entries")
                    .num_columns(4)
                    .min_col_width(80.0)
                    .striped(true)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        for entry in &entries {
                            let time_str = format_time(entry.timestamp);
                            ui.label(RichText::new(&time_str).monospace().small());
                            ui.label(RichText::new(&entry.query_name).monospace().small());
                            ui.label(RichText::new(&entry.query_type).monospace().small());

                            let (color, label) = match entry.status {
                                QueryStatus::CacheHit => (Color32::from_rgb(72, 199, 116), "Cache Hit"),
                                QueryStatus::Upstream => (Color32::from_rgb(100, 160, 220), "Upstream"),
                                QueryStatus::Error    => (Color32::from_rgb(220, 80, 80), "Error"),
                            };
                            ui.colored_label(color, RichText::new(label).small());
                            ui.end_row();
                        }
                    });
            });
    }
}

fn format_time(t: SystemTime) -> String {
    let secs = t.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let h = (secs % 86400) / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    format!("{h:02}:{m:02}:{s:02}")
}
```

- [ ] **Step 2: Build**

```bash
cd /root/doh_proxy && cargo build --bin doh-proxy-gui 2>&1 | tail -5
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/gui/log_panel.rs
git commit -m "feat: Query Log tab with scrollable striped table of recent DNS queries"
```

---

## Task 10: Build Release Binary

- [ ] **Step 1: Build release binary**

```bash
cd /root/doh_proxy && cargo build --release --bin doh-proxy-gui 2>&1 | tail -5
```

Expected: `Finished release [optimized] target(s)`.

- [ ] **Step 2: Confirm binary location and size**

```bash
ls -lh /root/doh_proxy/target/release/doh-proxy-gui
```

Expected: a binary ~5-15 MB (depending on dynamic linking).

- [ ] **Step 3: Run release binary once to confirm it launches**

```bash
/root/doh_proxy/target/release/doh-proxy-gui &
sleep 2 && kill %1
```

Expected: window opens and closes cleanly.

- [ ] **Step 4: (Optional) Strip binary for smaller size**

```bash
strip /root/doh_proxy/target/release/doh-proxy-gui
ls -lh /root/doh_proxy/target/release/doh-proxy-gui
```

- [ ] **Step 5: Final commit**

```bash
git add Cargo.toml
git commit -m "feat: doh-proxy-gui release build confirmed working"
```

---

## Self-Review

### Spec Coverage Check
| Requirement | Task |
|---|---|
| Modern minimalistic native GUI | Task 4 (eframe/egui) |
| Start/stop proxy from GUI | Task 6 |
| Live stats | Task 8 |
| Config editing + save | Task 7 |
| Query log | Task 9 |
| Downloadable/runnable binary | Task 10 |
| Stats wired into proxy | Tasks 1-2 |
| Stop signal | Task 3 |

### Type Consistency
- `Stats::new()` → `Arc<Stats>` — used consistently in Tasks 1, 2, 3, 6
- `Resolver::new(upstream, cache, stats)` — 3-arg signature defined in Task 2, fixed in Task 2 Step 4
- `Server::new(config, stats)` — 2-arg signature defined in Task 3, fixed in Task 3 Step 2
- `ConfigPanel::from_config` — defined in Task 5, used in Task 5 Step 1
- `Dashboard::show(app, ui)` / `ConfigPanel::show(app, ui)` etc. — all 2-arg `(app, ui)` consistently

### Placeholder scan
No TBDs, TODOs, or incomplete steps found. All code blocks are complete.
