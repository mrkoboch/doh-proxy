# TypeScript GUI Rewrite Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the Rust/eframe GUI with a Tauri 2.0 desktop app (React + TypeScript) styled with a dark beach-evening palette — deep night-sky backgrounds, warm sea-glass teal accent, moonlit text — and update the GitHub release to ship a fully self-contained tarball (binary + config dir + README).

**Architecture:** The existing `doh-proxy` Rust crate becomes a Cargo workspace member alongside a new `gui/src-tauri` crate; the Tauri Rust backend exposes seven commands that control the proxy lifecycle and surface stats/config; the React/TypeScript frontend communicates with those commands via `@tauri-apps/api` and renders four pages (Dashboard, Config, Stats, Query Log) styled with Tailwind CSS using a custom beach-night palette (dark navy bases, `#38c9c0` teal accent, warm coral highlights). All eframe/egui code is deleted.

**Tech Stack:** Rust 1.78+, Tauri 2.0, React 19, TypeScript 5, Vite 6 (dev server on :1420), Tailwind CSS 3.4, Vitest 2, @testing-library/react 16

---

## File Map

```
# DELETED
src/gui/                       ← entire eframe GUI module
src/gui_main.rs                ← eframe entry point

# MODIFIED (existing Rust crate)
Cargo.toml                     ← convert to workspace root; remove eframe feature & deps
src/config.rs                  ← add serde::Serialize to Config + CacheConfig
src/stats.rs                   ← add Serialize to LogEntry/QueryStatus; change timestamp field to u64; add StatsSnapshot + Stats::snapshot()
src/resolver.rs                ← update LogEntry construction to use timestamp_unix: u64

# CREATED (Tauri scaffold)
gui/package.json
gui/vite.config.ts
gui/vitest.config.ts
gui/tsconfig.json
gui/index.html
gui/postcss.config.js
gui/tailwind.config.ts
gui/src-tauri/Cargo.toml
gui/src-tauri/build.rs
gui/src-tauri/tauri.conf.json
gui/src-tauri/capabilities/default.json
gui/src-tauri/src/main.rs

# CREATED (Tauri Rust backend)
gui/src-tauri/src/proxy_manager.rs
gui/src-tauri/src/lib.rs

# CREATED (TypeScript frontend)
gui/src/main.tsx
gui/src/App.tsx
gui/src/index.css
gui/src/lib/types.ts
gui/src/lib/api.ts
gui/src/lib/api.test.ts
gui/src/test/setup.ts
gui/src/components/Sidebar.tsx
gui/src/components/ui/Button.tsx
gui/src/components/ui/Badge.tsx
gui/src/components/ui/Card.tsx
gui/src/components/ui/StatusDot.tsx
gui/src/components/ui/Badge.test.tsx
gui/src/components/ui/Button.test.tsx
gui/src/pages/Dashboard.tsx
gui/src/pages/Dashboard.test.tsx
gui/src/pages/Config.tsx
gui/src/pages/Config.test.tsx
gui/src/pages/Stats.tsx
gui/src/pages/QueryLog.tsx

# CREATED (release support)
README.md

# MODIFIED
.github/workflows/release.yml
```

---

### Task 1: Workspace, Serialize derives, timestamp migration

Remove all eframe/GUI code from the core Rust crate and make it serializable so Tauri commands can return data to TypeScript.

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/config.rs`
- Modify: `src/stats.rs`
- Modify: `src/resolver.rs`
- Delete: `src/gui/` (entire directory), `src/gui_main.rs`

- [ ] **Step 1: Delete eframe GUI files**

```bash
rm -rf src/gui src/gui_main.rs
```

Expected: those paths no longer exist.

- [ ] **Step 2: Rewrite `Cargo.toml` as workspace root**

Replace the entire file content:

```toml
[workspace]
members = [".", "gui/src-tauri"]
resolver = "2"

[package]
name = "doh-proxy"
version = "0.1.0"
edition = "2021"

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
toml = "0.8"
clap = { version = "4", features = ["derive"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
thiserror = "2"
anyhow = "1"
hickory-proto = "0.25"
moka = { version = "0.12", features = ["future"] }
base64 = "0.22"

[dev-dependencies]
tokio-test = "0.4"
```

- [ ] **Step 3: Remove gui module re-export from `src/lib.rs`**

Open `src/lib.rs`. If it contains `pub mod gui;`, delete that line. The file should only expose proxy, cache, config, server, stats, resolver, upstream, error modules.

- [ ] **Step 4: Add `Serialize` to `src/config.rs`**

Change:
```rust
use serde::Deserialize;
```
To:
```rust
use serde::{Deserialize, Serialize};
```

Change:
```rust
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
```
To:
```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
```

Change:
```rust
#[derive(Debug, Clone, Deserialize)]
pub struct CacheConfig {
```
To:
```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CacheConfig {
```

- [ ] **Step 5: Update `src/stats.rs` — add Serialize, change timestamp to u64, add StatsSnapshot**

Replace the entire file:

```rust
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

pub const LOG_CAPACITY: usize = 200;

#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub timestamp_unix: u64,
    pub query_name: String,
    pub query_type: String,
    pub status: QueryStatus,
    pub latency_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
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

#[derive(Debug, Serialize)]
pub struct StatsSnapshot {
    pub total: u64,
    pub cache_hits: u64,
    pub upstream: u64,
    pub errors: u64,
}

pub struct Stats {
    total_queries:    AtomicU64,
    cache_hits:       AtomicU64,
    upstream_queries: AtomicU64,
    errors:           AtomicU64,
    log: Mutex<VecDeque<LogEntry>>,
}

pub fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

impl Stats {
    pub fn new() -> Self {
        Self {
            total_queries:    AtomicU64::new(0),
            cache_hits:       AtomicU64::new(0),
            upstream_queries: AtomicU64::new(0),
            errors:           AtomicU64::new(0),
            log: Mutex::new(VecDeque::with_capacity(LOG_CAPACITY)),
        }
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

    pub fn total(&self)      -> u64 { self.total_queries.load(Ordering::Relaxed) }
    pub fn cache_hits(&self) -> u64 { self.cache_hits.load(Ordering::Relaxed) }
    pub fn upstream(&self)   -> u64 { self.upstream_queries.load(Ordering::Relaxed) }
    pub fn errors(&self)     -> u64 { self.errors.load(Ordering::Relaxed) }

    pub fn snapshot(&self) -> StatsSnapshot {
        StatsSnapshot {
            total:      self.total(),
            cache_hits: self.cache_hits(),
            upstream:   self.upstream(),
            errors:     self.errors(),
        }
    }

    pub fn snapshot_log(&self) -> Vec<LogEntry> {
        self.log.lock().unwrap().iter().rev().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(status: QueryStatus) -> LogEntry {
        LogEntry { timestamp_unix: 0, query_name: "a.com".into(), query_type: "A".into(), status, latency_ms: 1 }
    }

    #[test]
    fn counters_increment_correctly() {
        let s = Stats::new();
        s.record(make_entry(QueryStatus::CacheHit));
        s.record(make_entry(QueryStatus::Upstream));
        s.record(make_entry(QueryStatus::Error));
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
                timestamp_unix: i as u64,
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
            s.record(make_entry_named(name, QueryStatus::Upstream));
        }
        let log = s.snapshot_log();
        assert_eq!(log[0].query_name, "third.com");
        assert_eq!(log[2].query_name, "first.com");
    }

    fn make_entry_named(name: &str, status: QueryStatus) -> LogEntry {
        LogEntry { timestamp_unix: 0, query_name: name.into(), query_type: "A".into(), status, latency_ms: 0 }
    }

    #[test]
    fn snapshot_returns_correct_counts() {
        let s = Stats::new();
        s.record(make_entry(QueryStatus::CacheHit));
        s.record(make_entry(QueryStatus::CacheHit));
        s.record(make_entry(QueryStatus::Error));
        let snap = s.snapshot();
        assert_eq!(snap.total, 3);
        assert_eq!(snap.cache_hits, 2);
        assert_eq!(snap.errors, 1);
    }
}
```

- [ ] **Step 6: Update `src/resolver.rs` — use `timestamp_unix` and `unix_now()`**

In `resolver.rs`, change the import line:
```rust
use std::time::{Instant, SystemTime};
```
To:
```rust
use std::time::Instant;
```

Add `unix_now` to the stats import:
```rust
use crate::{
    cache::{CacheKey, DnsCache},
    error::{ProxyError, Result},
    stats::{unix_now, LogEntry, QueryStatus, Stats},
    upstream::UpstreamClient,
};
```

In the `record_stat` method, change the `LogEntry` construction:
```rust
stats.record(LogEntry {
    timestamp: SystemTime::now(),
    query_name: name,
    query_type: qtype,
    status,
    latency_ms: start.elapsed().as_millis() as u64,
});
```
To:
```rust
stats.record(LogEntry {
    timestamp_unix: unix_now(),
    query_name: name,
    query_type: qtype,
    status,
    latency_ms: start.elapsed().as_millis() as u64,
});
```

- [ ] **Step 7: Verify the core crate compiles and tests pass**

```bash
cargo test -p doh-proxy
```

Expected output (last lines):
```
test stats::tests::counters_increment_correctly ... ok
test stats::tests::log_caps_at_capacity ... ok
test stats::tests::snapshot_log_newest_first ... ok
test stats::tests::snapshot_returns_correct_counts ... ok
test resolver::tests::stats_record_error_on_upstream_failure ... ok

test result: ok. N passed; 0 failed
```

- [ ] **Step 8: Commit**

```bash
git add Cargo.toml src/config.rs src/stats.rs src/resolver.rs
git add -u src/gui src/gui_main.rs 2>/dev/null || true
git commit -m "refactor: remove eframe GUI, add Serialize to types, prepare workspace"
```

---

### Task 2: Tauri 2.0 project scaffold

Create all config, package, and build files so the `gui/` directory is a buildable Tauri 2.0 project. No Rust logic yet — just scaffolding that compiles.

**Files:**
- Create: `gui/package.json`, `gui/vite.config.ts`, `gui/vitest.config.ts`, `gui/tsconfig.json`
- Create: `gui/index.html`, `gui/postcss.config.js`, `gui/tailwind.config.ts`
- Create: `gui/src-tauri/Cargo.toml`, `gui/src-tauri/build.rs`, `gui/src-tauri/tauri.conf.json`
- Create: `gui/src-tauri/capabilities/default.json`
- Create: `gui/src-tauri/src/main.rs`
- Create: `gui/src/main.tsx`, `gui/src/App.tsx`, `gui/src/index.css`
- Create: `gui/src/test/setup.ts`

- [ ] **Step 1: Create `gui/package.json`**

```json
{
  "name": "doh-proxy-gui",
  "version": "0.1.0",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "tauri": "tauri",
    "test": "vitest run"
  },
  "dependencies": {
    "@tauri-apps/api": "^2.0.0",
    "react": "^19.0.0",
    "react-dom": "^19.0.0"
  },
  "devDependencies": {
    "@tauri-apps/cli": "^2.0.0",
    "@types/react": "^19.0.0",
    "@types/react-dom": "^19.0.0",
    "@vitejs/plugin-react": "^4.0.0",
    "@testing-library/react": "^16.0.0",
    "@testing-library/jest-dom": "^6.0.0",
    "autoprefixer": "^10.0.0",
    "jsdom": "^25.0.0",
    "postcss": "^8.0.0",
    "tailwindcss": "^3.4.0",
    "typescript": "^5.0.0",
    "vite": "^6.0.0",
    "vitest": "^2.0.0"
  }
}
```

- [ ] **Step 2: Create `gui/vite.config.ts`**

```ts
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    target: ["es2021", "chrome100", "safari13"],
    minify: !process.env.TAURI_DEBUG ? "esbuild" : false,
    sourcemap: !!process.env.TAURI_DEBUG,
  },
});
```

- [ ] **Step 3: Create `gui/vitest.config.ts`**

```ts
import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test/setup.ts"],
  },
});
```

- [ ] **Step 4: Create `gui/tsconfig.json`**

```json
{
  "compilerOptions": {
    "target": "ES2021",
    "useDefineForClassFields": true,
    "lib": ["ES2021", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "jsx": "react-jsx",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true
  },
  "include": ["src"]
}
```

- [ ] **Step 5: Create `gui/index.html`**

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>DoH Proxy</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

- [ ] **Step 6: Create `gui/postcss.config.js`**

```js
export default {
  plugins: {
    tailwindcss: {},
    autoprefixer: {},
  },
};
```

- [ ] **Step 7: Create `gui/tailwind.config.ts`**

```ts
import type { Config } from "tailwindcss";

export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        // Night-sky backgrounds — dark navy with a faint ocean tint
        base:     "#0e1219",
        surface:  "#151e2a",
        elevated: "#1c2837",
        border:   "#273547",
        // Sea-glass teal — bioluminescent ocean waves at night
        accent:   "#38c9c0",
        // Status colors
        success:  "#34d399",   // soft seafoam green
        error:    "#f87171",   // muted coral red
        info:     "#60a5fa",   // calm ocean blue
        warning:  "#f4956a",   // warm sunset coral (used sparingly)
        // Text — moonlit whites and muted slate-blues
        primary:   "#e4eaf3",
        secondary: "#7a8fa3",
        muted:     "#3d5168",
      },
      fontFamily: {
        mono: ["JetBrains Mono", "Fira Code", "ui-monospace", "monospace"],
      },
    },
  },
  plugins: [],
} satisfies Config;
```

- [ ] **Step 8: Create `gui/src-tauri/Cargo.toml`**

```toml
[package]
name = "doh-proxy-gui"
version = "0.1.0"
edition = "2021"

[lib]
name = "doh_proxy_gui_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = [] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
anyhow = "1"
doh-proxy = { path = "../.." }

[target.'cfg(target_os = "linux")'.dependencies]
openssl = { version = "0.10", features = ["vendored"] }
```

- [ ] **Step 9: Create `gui/src-tauri/build.rs`**

```rust
fn main() {
    tauri_build::build()
}
```

- [ ] **Step 10: Create `gui/src-tauri/tauri.conf.json`**

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "DoH Proxy",
  "version": "0.1.0",
  "identifier": "com.doh-proxy.gui",
  "build": {
    "frontendDist": "../dist",
    "devUrl": "http://localhost:1420",
    "beforeDevCommand": "npm run dev",
    "beforeBuildCommand": "npm run build"
  },
  "app": {
    "windows": [
      {
        "title": "DoH Proxy",
        "width": 900,
        "height": 620,
        "minWidth": 700,
        "minHeight": 500,
        "resizable": true,
        "fullscreen": false
      }
    ],
    "security": {
      "csp": null
    }
  },
  "bundle": {
    "active": false,
    "icon": []
  }
}
```

- [ ] **Step 11: Create `gui/src-tauri/capabilities/default.json`**

```json
{
  "$schema": "../node_modules/@tauri-apps/cli/schema/acl/capability.schema.json",
  "identifier": "default",
  "description": "Default capability for main window",
  "windows": ["main"],
  "permissions": ["core:default"]
}
```

- [ ] **Step 12: Create placeholder `gui/src-tauri/src/main.rs`**

This will be fleshed out in Task 4. For now, a minimal version that compiles:

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    doh_proxy_gui_lib::run();
}
```

- [ ] **Step 13: Create placeholder `gui/src-tauri/src/lib.rs`**

```rust
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 14: Create `gui/src/test/setup.ts`**

```ts
import "@testing-library/jest-dom";
```

- [ ] **Step 15: Create `gui/src/index.css`**

```css
@tailwind base;
@tailwind components;
@tailwind utilities;

* {
  box-sizing: border-box;
}

html,
body,
#root {
  height: 100%;
  margin: 0;
}

body {
  font-family: -apple-system, BlinkMacSystemFont, "Inter", "Segoe UI", sans-serif;
  -webkit-font-smoothing: antialiased;
  background: #0e1219;
  color: #e4eaf3;
  user-select: none;
  overflow: hidden;
}

::-webkit-scrollbar {
  width: 5px;
  height: 5px;
}

::-webkit-scrollbar-track {
  background: #1a1a1a;
}

::-webkit-scrollbar-thumb {
  background: #3a3a3a;
  border-radius: 3px;
}

::-webkit-scrollbar-thumb:hover {
  background: #4a4a4a;
}

input[type="checkbox"] {
  accent-color: #f6821f;
}
```

- [ ] **Step 16: Create `gui/src/main.tsx`**

```tsx
import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
```

- [ ] **Step 17: Create placeholder `gui/src/App.tsx`**

This renders a loading state. The real App is built in Task 7.

```tsx
export default function App() {
  return (
    <div className="flex items-center justify-center h-full text-secondary text-sm">
      Loading…
    </div>
  );
}
```

- [ ] **Step 18: Install npm dependencies**

```bash
cd gui && npm install
```

Expected: `node_modules/` created, no errors.

- [ ] **Step 19: Verify the TypeScript frontend builds**

```bash
cd gui && npm run build
```

Expected: `gui/dist/` created, `dist/index.html` present.

- [ ] **Step 20: Commit**

```bash
git add gui/
git commit -m "feat: scaffold Tauri 2.0 project with React + TypeScript + Tailwind"
```

---

### Task 3: Tauri Rust backend — ProxyManager

Write and test the `ProxyManager` struct that owns proxy lifecycle state. No Tauri commands yet, just the state machine.

**Files:**
- Create: `gui/src-tauri/src/proxy_manager.rs`
- Modify: `gui/src-tauri/src/lib.rs` (register the state)

- [ ] **Step 1: Write the failing unit tests first**

Create `gui/src-tauri/src/proxy_manager.rs` with only tests (no impl yet):

```rust
fn toml_escape(s: &str) -> String {
    todo!()
}

pub struct ProxyManager;

impl ProxyManager {
    pub fn new_for_test(config: doh_proxy::config::Config) -> Self {
        todo!()
    }
    pub fn is_running(&self) -> bool {
        todo!()
    }
    pub fn stats(&self) -> Option<std::sync::Arc<doh_proxy::stats::Stats>> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use doh_proxy::config::Config;

    #[test]
    fn new_manager_is_stopped() {
        let pm = ProxyManager::new_for_test(Config::default());
        assert!(!pm.is_running());
    }

    #[test]
    fn stopped_manager_has_no_stats() {
        let pm = ProxyManager::new_for_test(Config::default());
        assert!(pm.stats().is_none());
    }

    #[test]
    fn toml_escape_handles_quotes() {
        assert_eq!(toml_escape(r#"has "quotes""#), r#"has \"quotes\""#);
    }

    #[test]
    fn toml_escape_handles_backslash() {
        assert_eq!(toml_escape(r"back\slash"), r"back\\slash");
    }

    #[test]
    fn toml_escape_leaves_plain_strings_unchanged() {
        assert_eq!(toml_escape("https://1.1.1.1/dns-query"), "https://1.1.1.1/dns-query");
    }
}
```

- [ ] **Step 2: Add proxy_manager module to `lib.rs` and run failing tests**

In `gui/src-tauri/src/lib.rs`, add `mod proxy_manager;` at the top.

```bash
cd gui/src-tauri && cargo test 2>&1 | tail -20
```

Expected: compile errors or `not yet implemented` panics — confirming tests exist and fail.

- [ ] **Step 3: Implement `ProxyManager` in full**

Replace `gui/src-tauri/src/proxy_manager.rs` with the complete implementation:

```rust
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use doh_proxy::{config::Config, stats::Stats};

pub(crate) fn toml_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

enum ProxyState {
    Stopped,
    Running {
        stop_flag: Arc<AtomicBool>,
        _thread: std::thread::JoinHandle<()>,
        stats: Arc<Stats>,
    },
}

pub struct ProxyManager {
    state: ProxyState,
    config: Config,
    config_path: std::path::PathBuf,
}

impl ProxyManager {
    pub fn new() -> Self {
        let config_path = Self::locate_config();
        let config = Config::from_file(&config_path).unwrap_or_default();
        Self { state: ProxyState::Stopped, config, config_path }
    }

    /// Constructor for unit tests — bypasses filesystem config discovery.
    pub fn new_for_test(config: Config) -> Self {
        Self {
            state: ProxyState::Stopped,
            config,
            config_path: std::path::PathBuf::from("/tmp/test-config.toml"),
        }
    }

    fn locate_config() -> std::path::PathBuf {
        // Portable: sibling config/ directory next to the binary
        if let Ok(exe) = std::env::current_exe() {
            let candidate = exe
                .parent()
                .unwrap_or(std::path::Path::new("."))
                .join("config/default.toml");
            if candidate.exists() {
                return candidate;
            }
        }
        // Fallback: CWD-relative (useful during development)
        std::path::PathBuf::from("config/default.toml")
    }

    pub fn is_running(&self) -> bool {
        matches!(self.state, ProxyState::Running { .. })
    }

    /// Returns a cloned Arc<Stats> only when the proxy is running.
    pub fn stats(&self) -> Option<Arc<Stats>> {
        match &self.state {
            ProxyState::Running { stats, .. } => Some(Arc::clone(stats)),
            ProxyState::Stopped => None,
        }
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Starts the proxy in a background thread. Returns the listen address on success.
    pub fn start(&mut self) -> Result<String, String> {
        if self.is_running() {
            return Err("proxy is already running".into());
        }
        let stop_flag = Arc::new(AtomicBool::new(false));
        let stats = Arc::new(Stats::new());
        let config = self.config.clone();
        let stop_clone = Arc::clone(&stop_flag);
        let stats_clone = Arc::clone(&stats);

        let thread = std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            rt.block_on(async move {
                match doh_proxy::server::Server::new(config, Some(stats_clone)).await {
                    Ok(server) => {
                        if let Err(e) = server.run_cancellable(stop_clone).await {
                            tracing::error!("server error: {e}");
                        }
                    }
                    Err(e) => tracing::error!("server init error: {e}"),
                }
            });
        });

        let addr = self.config.listen_addr.to_string();
        self.state = ProxyState::Running { stop_flag, _thread: thread, stats };
        Ok(addr)
    }

    /// Signals the proxy to stop and waits for its thread to exit.
    pub fn stop(&mut self) {
        let old = std::mem::replace(&mut self.state, ProxyState::Stopped);
        if let ProxyState::Running { stop_flag, _thread, .. } = old {
            stop_flag.store(true, Ordering::Release);
            let _ = _thread.join();
        }
    }

    /// Validates and persists a new config, replacing the current one.
    pub fn update_config(&mut self, config: Config) -> Result<(), String> {
        use std::io::Write;

        let upstreams_toml: String = config
            .upstreams
            .iter()
            .map(|u| format!("    \"{}\",\n", toml_escape(u)))
            .collect();

        let toml = format!(
            "listen_addr = \"{}\"\n\nupstreams = [\n{}]\n\n[cache]\nenabled = {}\ncapacity = {}\n",
            toml_escape(&config.listen_addr.to_string()),
            upstreams_toml,
            config.cache.enabled,
            config.cache.capacity,
        );

        if let Some(parent) = self.config_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        let tmp = self.config_path.with_extension("toml.tmp");
        let mut f = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
        f.write_all(toml.as_bytes()).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, &self.config_path).map_err(|e| e.to_string())?;

        self.config = config;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use doh_proxy::config::Config;

    #[test]
    fn new_manager_is_stopped() {
        let pm = ProxyManager::new_for_test(Config::default());
        assert!(!pm.is_running());
    }

    #[test]
    fn stopped_manager_has_no_stats() {
        let pm = ProxyManager::new_for_test(Config::default());
        assert!(pm.stats().is_none());
    }

    #[test]
    fn toml_escape_handles_quotes() {
        assert_eq!(toml_escape(r#"has "quotes""#), r#"has \"quotes\""#);
    }

    #[test]
    fn toml_escape_handles_backslash() {
        assert_eq!(toml_escape(r"back\slash"), r"back\\slash");
    }

    #[test]
    fn toml_escape_leaves_plain_strings_unchanged() {
        assert_eq!(toml_escape("https://1.1.1.1/dns-query"), "https://1.1.1.1/dns-query");
    }
}
```

- [ ] **Step 4: Run tests — they must pass**

```bash
cd gui/src-tauri && cargo test proxy_manager 2>&1 | tail -15
```

Expected:
```
test proxy_manager::tests::new_manager_is_stopped ... ok
test proxy_manager::tests::stopped_manager_has_no_stats ... ok
test proxy_manager::tests::toml_escape_handles_quotes ... ok
test proxy_manager::tests::toml_escape_handles_backslash ... ok
test proxy_manager::tests::toml_escape_leaves_plain_strings_unchanged ... ok

test result: ok. 5 passed; 0 failed
```

- [ ] **Step 5: Commit**

```bash
git add gui/src-tauri/src/proxy_manager.rs gui/src-tauri/src/lib.rs
git commit -m "feat: add ProxyManager Rust backend with lifecycle tests"
```

---

### Task 4: Tauri commands

Expose the seven Tauri commands that the TypeScript frontend will call. Extract validation into a testable pure function.

**Files:**
- Modify: `gui/src-tauri/src/lib.rs`

- [ ] **Step 1: Write failing tests for `validate_config_input`**

Open `gui/src-tauri/src/lib.rs`. Add this test module at the bottom (before `run()`):

```rust
#[cfg(test)]
mod tests {
    use super::validate_config_input;

    #[test]
    fn validate_rejects_invalid_listen_addr() {
        let result = validate_config_input(
            "not-an-addr",
            vec!["https://1.1.1.1/dns-query".into()],
            true,
            10000,
        );
        assert!(result.is_err());
        assert!(result.as_ref().unwrap_err().contains("invalid listen address"));
    }

    #[test]
    fn validate_rejects_empty_upstreams() {
        let result = validate_config_input("127.0.0.1:5353", vec![], true, 10000);
        assert!(result.is_err());
        assert!(result.as_ref().unwrap_err().contains("at least one upstream"));
    }

    #[test]
    fn validate_rejects_non_http_upstream() {
        let result = validate_config_input(
            "127.0.0.1:5353",
            vec!["ftp://bad.com".into()],
            true,
            10000,
        );
        assert!(result.is_err());
        assert!(result.as_ref().unwrap_err().contains("https://"));
    }

    #[test]
    fn validate_filters_blank_upstreams() {
        let result = validate_config_input(
            "127.0.0.1:5353",
            vec!["  ".into(), "https://1.1.1.1/dns-query".into()],
            true,
            10000,
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap().upstreams.len(), 1);
    }

    #[test]
    fn validate_accepts_valid_input() {
        let result = validate_config_input(
            "0.0.0.0:5353",
            vec!["https://1.1.1.1/dns-query".into(), "https://8.8.8.8/dns-query".into()],
            false,
            500,
        );
        assert!(result.is_ok());
        let cfg = result.unwrap();
        assert_eq!(cfg.listen_addr.port(), 5353);
        assert!(!cfg.cache.enabled);
        assert_eq!(cfg.cache.capacity, 500);
    }
}
```

- [ ] **Step 2: Run tests — they must fail**

```bash
cd gui/src-tauri && cargo test validate 2>&1 | tail -10
```

Expected: compile error `cannot find function \`validate_config_input\`` — confirms the test is real.

- [ ] **Step 3: Implement the full `lib.rs`**

Replace `gui/src-tauri/src/lib.rs` entirely:

```rust
mod proxy_manager;

use std::sync::Mutex;

use doh_proxy::{
    config::{CacheConfig, Config},
    stats::{LogEntry, StatsSnapshot},
};
use proxy_manager::ProxyManager;
use serde::Serialize;
use tauri::State;

type ManagedProxy = Mutex<ProxyManager>;

#[derive(Serialize)]
struct ProxyStatus {
    running: bool,
    listen_addr: String,
}

// ── Commands ──────────────────────────────────────────────────────────────────

#[tauri::command]
fn start_proxy(state: State<'_, ManagedProxy>) -> Result<String, String> {
    state.lock().unwrap().start()
}

#[tauri::command]
fn stop_proxy(state: State<'_, ManagedProxy>) {
    state.lock().unwrap().stop();
}

#[tauri::command]
fn get_proxy_status(state: State<'_, ManagedProxy>) -> ProxyStatus {
    let pm = state.lock().unwrap();
    ProxyStatus {
        running: pm.is_running(),
        listen_addr: pm.config().listen_addr.to_string(),
    }
}

#[tauri::command]
fn get_stats(state: State<'_, ManagedProxy>) -> Option<StatsSnapshot> {
    state.lock().unwrap().stats().map(|s| s.snapshot())
}

#[tauri::command]
fn get_log_entries(state: State<'_, ManagedProxy>) -> Vec<LogEntry> {
    state
        .lock()
        .unwrap()
        .stats()
        .map(|s| s.snapshot_log())
        .unwrap_or_default()
}

#[tauri::command]
fn load_config(state: State<'_, ManagedProxy>) -> Config {
    state.lock().unwrap().config().clone()
}

#[tauri::command]
fn save_config(
    state: State<'_, ManagedProxy>,
    listen_addr: String,
    upstreams: Vec<String>,
    cache_enabled: bool,
    cache_capacity: u64,
) -> Result<(), String> {
    let config = validate_config_input(&listen_addr, upstreams, cache_enabled, cache_capacity)?;
    state.lock().unwrap().update_config(config)
}

// ── Validation (pure, testable) ───────────────────────────────────────────────

pub fn validate_config_input(
    listen_addr: &str,
    upstreams: Vec<String>,
    cache_enabled: bool,
    cache_capacity: u64,
) -> Result<Config, String> {
    let listen_addr: std::net::SocketAddr = listen_addr
        .parse()
        .map_err(|e| format!("invalid listen address: {e}"))?;

    let upstreams: Vec<String> = upstreams
        .into_iter()
        .filter(|u| !u.trim().is_empty())
        .collect();

    if upstreams.is_empty() {
        return Err("at least one upstream URL is required".into());
    }

    for url in &upstreams {
        if !url.starts_with("https://") && !url.starts_with("http://") {
            return Err(format!(
                "upstream must start with https:// or http://: {url}"
            ));
        }
    }

    Ok(Config {
        listen_addr,
        upstreams,
        cache: CacheConfig {
            enabled: cache_enabled,
            capacity: cache_capacity,
        },
    })
}

// ── App entry point ───────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Mutex::new(ProxyManager::new()))
        .invoke_handler(tauri::generate_handler![
            start_proxy,
            stop_proxy,
            get_proxy_status,
            get_stats,
            get_log_entries,
            load_config,
            save_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::validate_config_input;

    #[test]
    fn validate_rejects_invalid_listen_addr() {
        let result = validate_config_input(
            "not-an-addr",
            vec!["https://1.1.1.1/dns-query".into()],
            true,
            10000,
        );
        assert!(result.is_err());
        assert!(result.as_ref().unwrap_err().contains("invalid listen address"));
    }

    #[test]
    fn validate_rejects_empty_upstreams() {
        let result = validate_config_input("127.0.0.1:5353", vec![], true, 10000);
        assert!(result.is_err());
        assert!(result.as_ref().unwrap_err().contains("at least one upstream"));
    }

    #[test]
    fn validate_rejects_non_http_upstream() {
        let result = validate_config_input(
            "127.0.0.1:5353",
            vec!["ftp://bad.com".into()],
            true,
            10000,
        );
        assert!(result.is_err());
        assert!(result.as_ref().unwrap_err().contains("https://"));
    }

    #[test]
    fn validate_filters_blank_upstreams() {
        let result = validate_config_input(
            "127.0.0.1:5353",
            vec!["  ".into(), "https://1.1.1.1/dns-query".into()],
            true,
            10000,
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap().upstreams.len(), 1);
    }

    #[test]
    fn validate_accepts_valid_input() {
        let result = validate_config_input(
            "0.0.0.0:5353",
            vec![
                "https://1.1.1.1/dns-query".into(),
                "https://8.8.8.8/dns-query".into(),
            ],
            false,
            500,
        );
        assert!(result.is_ok());
        let cfg = result.unwrap();
        assert_eq!(cfg.listen_addr.port(), 5353);
        assert!(!cfg.cache.enabled);
        assert_eq!(cfg.cache.capacity, 500);
    }
}
```

- [ ] **Step 4: Run tests — they must pass**

```bash
cd gui/src-tauri && cargo test 2>&1 | tail -15
```

Expected:
```
test tests::validate_accepts_valid_input ... ok
test tests::validate_filters_blank_upstreams ... ok
test tests::validate_rejects_empty_upstreams ... ok
test tests::validate_rejects_invalid_listen_addr ... ok
test tests::validate_rejects_non_http_upstream ... ok
test proxy_manager::tests::... ok  (×5)

test result: ok. 10 passed; 0 failed
```

- [ ] **Step 5: Commit**

```bash
git add gui/src-tauri/src/lib.rs
git commit -m "feat: add Tauri commands for proxy control, stats, and config"
```

---

### Task 5: TypeScript types and API layer

Define the TypeScript mirror of every Rust type returned by Tauri commands and wrap each `invoke` call.

**Files:**
- Create: `gui/src/lib/types.ts`
- Create: `gui/src/lib/api.ts`
- Create: `gui/src/lib/api.test.ts`

- [ ] **Step 1: Write failing tests for the API module**

Create `gui/src/lib/api.test.ts`:

```ts
import { describe, it, expect, vi, beforeEach } from "vitest";
import * as tauriCore from "@tauri-apps/api/core";

vi.mock("@tauri-apps/api/core");

// Import after mock is set up
const { startProxy, stopProxy, getProxyStatus, getStats, getLogEntries, loadConfig, saveConfig } =
  await import("./api");

describe("api", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("startProxy invokes start_proxy and returns listen addr", async () => {
    vi.mocked(tauriCore.invoke).mockResolvedValue("0.0.0.0:5353");
    const result = await startProxy();
    expect(tauriCore.invoke).toHaveBeenCalledWith("start_proxy");
    expect(result).toBe("0.0.0.0:5353");
  });

  it("stopProxy invokes stop_proxy", async () => {
    vi.mocked(tauriCore.invoke).mockResolvedValue(undefined);
    await stopProxy();
    expect(tauriCore.invoke).toHaveBeenCalledWith("stop_proxy");
  });

  it("getProxyStatus invokes get_proxy_status", async () => {
    vi.mocked(tauriCore.invoke).mockResolvedValue({
      running: true,
      listen_addr: "0.0.0.0:5353",
    });
    const result = await getProxyStatus();
    expect(tauriCore.invoke).toHaveBeenCalledWith("get_proxy_status");
    expect(result.running).toBe(true);
    expect(result.listen_addr).toBe("0.0.0.0:5353");
  });

  it("getStats invokes get_stats", async () => {
    vi.mocked(tauriCore.invoke).mockResolvedValue({
      total: 42,
      cache_hits: 10,
      upstream: 30,
      errors: 2,
    });
    const result = await getStats();
    expect(tauriCore.invoke).toHaveBeenCalledWith("get_stats");
    expect(result?.total).toBe(42);
  });

  it("getLogEntries invokes get_log_entries", async () => {
    vi.mocked(tauriCore.invoke).mockResolvedValue([]);
    const result = await getLogEntries();
    expect(tauriCore.invoke).toHaveBeenCalledWith("get_log_entries");
    expect(Array.isArray(result)).toBe(true);
  });

  it("saveConfig passes snake_case args to invoke", async () => {
    vi.mocked(tauriCore.invoke).mockResolvedValue(undefined);
    await saveConfig("127.0.0.1:5353", ["https://1.1.1.1/dns-query"], true, 10000);
    expect(tauriCore.invoke).toHaveBeenCalledWith("save_config", {
      listen_addr: "127.0.0.1:5353",
      upstreams: ["https://1.1.1.1/dns-query"],
      cache_enabled: true,
      cache_capacity: 10000,
    });
  });
});
```

- [ ] **Step 2: Run tests — they must fail**

```bash
cd gui && npm test 2>&1 | tail -15
```

Expected: `Cannot find module './api'` — confirms the tests reference code that doesn't exist yet.

- [ ] **Step 3: Create `gui/src/lib/types.ts`**

```ts
export interface ProxyStatus {
  running: boolean;
  listen_addr: string;
}

export interface StatsSnapshot {
  total: number;
  cache_hits: number;
  upstream: number;
  errors: number;
}

export type QueryStatus = "CacheHit" | "Upstream" | "Error";

export interface LogEntry {
  timestamp_unix: number;
  query_name: string;
  query_type: string;
  status: QueryStatus;
  latency_ms: number;
}

export interface CacheConfig {
  enabled: boolean;
  capacity: number;
}

export interface ProxyConfig {
  listen_addr: string;
  upstreams: string[];
  cache: CacheConfig;
}
```

- [ ] **Step 4: Create `gui/src/lib/api.ts`**

```ts
import { invoke } from "@tauri-apps/api/core";
import type { LogEntry, ProxyConfig, ProxyStatus, StatsSnapshot } from "./types";

export function startProxy(): Promise<string> {
  return invoke<string>("start_proxy");
}

export function stopProxy(): Promise<void> {
  return invoke<void>("stop_proxy");
}

export function getProxyStatus(): Promise<ProxyStatus> {
  return invoke<ProxyStatus>("get_proxy_status");
}

export function getStats(): Promise<StatsSnapshot | null> {
  return invoke<StatsSnapshot | null>("get_stats");
}

export function getLogEntries(): Promise<LogEntry[]> {
  return invoke<LogEntry[]>("get_log_entries");
}

export function loadConfig(): Promise<ProxyConfig> {
  return invoke<ProxyConfig>("load_config");
}

export function saveConfig(
  listenAddr: string,
  upstreams: string[],
  cacheEnabled: boolean,
  cacheCapacity: number
): Promise<void> {
  return invoke<void>("save_config", {
    listen_addr: listenAddr,
    upstreams,
    cache_enabled: cacheEnabled,
    cache_capacity: cacheCapacity,
  });
}
```

- [ ] **Step 5: Run tests — they must pass**

```bash
cd gui && npm test 2>&1 | tail -15
```

Expected:
```
✓ src/lib/api.test.ts (6)
  ✓ api > startProxy invokes start_proxy and returns listen addr
  ✓ api > stopProxy invokes stop_proxy
  ✓ api > getProxyStatus invokes get_proxy_status
  ✓ api > getStats invokes get_stats
  ✓ api > getLogEntries invokes get_log_entries
  ✓ api > saveConfig passes snake_case args to invoke

Test Files  1 passed (1)
Tests  6 passed (6)
```

- [ ] **Step 6: Commit**

```bash
git add gui/src/lib/
git commit -m "feat: add TypeScript types and API wrapper layer with tests"
```

---

### Task 6: Design system — Tailwind config and UI primitives

Build the four reusable UI primitives (Button, Badge, Card, StatusDot) that every page depends on. Test the ones with meaningful behavior.

**Files:**
- Create: `gui/src/components/ui/Button.tsx`
- Create: `gui/src/components/ui/Badge.tsx`
- Create: `gui/src/components/ui/Card.tsx`
- Create: `gui/src/components/ui/StatusDot.tsx`
- Create: `gui/src/components/ui/Badge.test.tsx`
- Create: `gui/src/components/ui/Button.test.tsx`

- [ ] **Step 1: Write failing Badge and Button tests**

Create `gui/src/components/ui/Badge.test.tsx`:

```tsx
import { render, screen } from "@testing-library/react";
import { Badge } from "./Badge";

describe("Badge", () => {
  it("renders success variant text", () => {
    render(<Badge variant="success">Cache Hit</Badge>);
    expect(screen.getByText("Cache Hit")).toBeInTheDocument();
  });

  it("renders error variant text", () => {
    render(<Badge variant="error">Error</Badge>);
    expect(screen.getByText("Error")).toBeInTheDocument();
  });

  it("renders info variant text", () => {
    render(<Badge variant="info">Upstream</Badge>);
    expect(screen.getByText("Upstream")).toBeInTheDocument();
  });

  it("renders neutral variant text", () => {
    render(<Badge variant="neutral">Unknown</Badge>);
    expect(screen.getByText("Unknown")).toBeInTheDocument();
  });
});
```

Create `gui/src/components/ui/Button.test.tsx`:

```tsx
import { render, screen, fireEvent } from "@testing-library/react";
import { Button } from "./Button";

describe("Button", () => {
  it("renders children", () => {
    render(<Button>Click me</Button>);
    expect(screen.getByRole("button", { name: /click me/i })).toBeInTheDocument();
  });

  it("calls onClick when clicked", () => {
    const handler = vi.fn();
    render(<Button onClick={handler}>Go</Button>);
    fireEvent.click(screen.getByRole("button"));
    expect(handler).toHaveBeenCalledTimes(1);
  });

  it("does not call onClick when disabled", () => {
    const handler = vi.fn();
    render(<Button onClick={handler} disabled>Go</Button>);
    fireEvent.click(screen.getByRole("button"));
    expect(handler).not.toHaveBeenCalled();
  });

  it("renders danger variant without error", () => {
    render(<Button variant="danger">Stop</Button>);
    expect(screen.getByRole("button", { name: /stop/i })).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run tests — they must fail**

```bash
cd gui && npm test 2>&1 | grep -E "FAIL|Cannot find"
```

Expected: test files fail to compile because the component files don't exist yet.

- [ ] **Step 3: Create `gui/src/components/ui/Button.tsx`**

```tsx
import { type ButtonHTMLAttributes } from "react";

interface Props extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "primary" | "danger" | "ghost";
  size?: "sm" | "md";
}

export function Button({
  variant = "primary",
  size = "md",
  className = "",
  children,
  ...props
}: Props) {
  const base =
    "inline-flex items-center justify-center rounded font-medium transition-colors " +
    "focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-offset-base " +
    "disabled:opacity-40 disabled:cursor-not-allowed";

  const variants = {
    primary: "bg-accent text-base hover:bg-accent/85 focus:ring-accent",
    danger:  "bg-error text-base hover:bg-error/85 focus:ring-error",
    ghost:   "bg-transparent text-secondary hover:text-primary hover:bg-elevated focus:ring-border",
  };

  const sizes = {
    sm: "px-3 py-1.5 text-xs",
    md: "px-4 py-2 text-sm",
  };

  return (
    <button
      className={`${base} ${variants[variant]} ${sizes[size]} ${className}`}
      {...props}
    >
      {children}
    </button>
  );
}
```

- [ ] **Step 4: Create `gui/src/components/ui/Badge.tsx`**

```tsx
interface Props {
  variant: "success" | "error" | "info" | "neutral";
  children: React.ReactNode;
}

export function Badge({ variant, children }: Props) {
  const variants = {
    success: "bg-green-900/30 text-success border-green-800/40",
    error:   "bg-red-900/30 text-error border-red-800/40",
    info:    "bg-blue-900/30 text-info border-blue-800/40",
    neutral: "bg-elevated text-secondary border-border",
  };

  return (
    <span
      className={`inline-flex items-center px-2 py-0.5 text-xs font-medium rounded border ${variants[variant]}`}
    >
      {children}
    </span>
  );
}
```

- [ ] **Step 5: Create `gui/src/components/ui/Card.tsx`**

```tsx
interface Props {
  children: React.ReactNode;
  className?: string;
}

export function Card({ children, className = "" }: Props) {
  return (
    <div className={`bg-surface border border-border rounded-lg p-4 ${className}`}>
      {children}
    </div>
  );
}
```

- [ ] **Step 6: Create `gui/src/components/ui/StatusDot.tsx`**

```tsx
interface Props {
  active: boolean;
  label?: string;
}

export function StatusDot({ active, label }: Props) {
  return (
    <div className="flex items-center gap-2">
      <span className="relative flex h-2.5 w-2.5">
        {active && (
          <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-success opacity-50" />
        )}
        <span
          className={`relative inline-flex rounded-full h-2.5 w-2.5 ${
            active ? "bg-success" : "bg-muted"
          }`}
        />
      </span>
      {label && (
        <span
          className={`text-sm font-medium ${active ? "text-primary" : "text-secondary"}`}
        >
          {label}
        </span>
      )}
    </div>
  );
}
```

- [ ] **Step 7: Run tests — all must pass**

```bash
cd gui && npm test 2>&1 | tail -20
```

Expected:
```
✓ src/lib/api.test.ts (6)
✓ src/components/ui/Badge.test.tsx (4)
✓ src/components/ui/Button.test.tsx (4)

Test Files  3 passed (3)
Tests  14 passed (14)
```

- [ ] **Step 8: Commit**

```bash
git add gui/src/components/ui/
git commit -m "feat: add Cloudflare-styled UI primitives with component tests"
```

---

### Task 7: App shell — sidebar and layout

Build the persistent chrome: the sidebar navigation and the root `App` component that orchestrates page routing and status polling.

**Files:**
- Create: `gui/src/components/Sidebar.tsx`
- Modify: `gui/src/App.tsx`

- [ ] **Step 1: Create `gui/src/components/Sidebar.tsx`**

```tsx
type Page = "dashboard" | "config" | "stats" | "log";

const NAV: { page: Page; label: string; icon: string }[] = [
  { page: "dashboard", label: "Dashboard", icon: "◎" },
  { page: "config",    label: "Config",    icon: "⚙" },
  { page: "stats",     label: "Stats",     icon: "▦" },
  { page: "log",       label: "Query Log", icon: "≡" },
];

interface Props {
  current: Page;
  onNavigate: (page: Page) => void;
  running: boolean;
}

export function Sidebar({ current, onNavigate, running }: Props) {
  return (
    <aside className="w-44 flex-shrink-0 bg-surface border-r border-border flex flex-col select-none">
      {/* Wordmark */}
      <div className="px-4 py-5 border-b border-border">
        <span className="text-accent font-bold text-sm tracking-tight">DoH Proxy</span>
        <div className="mt-1.5 flex items-center gap-1.5">
          <span
            className={`h-1.5 w-1.5 rounded-full ${running ? "bg-success" : "bg-muted"}`}
          />
          <span className="text-xs text-secondary">
            {running ? "Running" : "Stopped"}
          </span>
        </div>
      </div>

      {/* Nav items */}
      <nav className="flex-1 px-2 py-3 space-y-0.5">
        {NAV.map(({ page, label, icon }) => (
          <button
            key={page}
            onClick={() => onNavigate(page)}
            className={[
              "w-full flex items-center gap-3 px-3 py-2 rounded text-sm transition-colors text-left",
              current === page
                ? "bg-elevated text-primary font-medium"
                : "text-secondary hover:text-primary hover:bg-elevated/60",
            ].join(" ")}
          >
            <span className="font-mono text-xs w-4 text-center opacity-70">{icon}</span>
            {label}
          </button>
        ))}
      </nav>

      {/* Footer */}
      <div className="px-4 py-3 border-t border-border">
        <span className="text-xs text-muted">v0.1.0</span>
      </div>
    </aside>
  );
}

export type { Page };
```

- [ ] **Step 2: Replace `gui/src/App.tsx`**

```tsx
import { useState, useEffect, useCallback } from "react";
import { Sidebar, type Page } from "./components/Sidebar";
import { getProxyStatus } from "./lib/api";
import type { ProxyStatus } from "./lib/types";

// Pages are loaded lazily to keep the initial bundle snappy
import { Dashboard } from "./pages/Dashboard";
import { Config }    from "./pages/Config";
import { Stats }     from "./pages/Stats";
import { QueryLog }  from "./pages/QueryLog";

export default function App() {
  const [page, setPage] = useState<Page>("dashboard");
  const [status, setStatus] = useState<ProxyStatus>({
    running: false,
    listen_addr: "0.0.0.0:5353",
  });

  const refreshStatus = useCallback(async () => {
    try {
      setStatus(await getProxyStatus());
    } catch {
      // Tauri not ready yet during hot reload — silently ignore
    }
  }, []);

  useEffect(() => {
    refreshStatus();
    const id = setInterval(refreshStatus, 1000);
    return () => clearInterval(id);
  }, [refreshStatus]);

  return (
    <div className="flex h-screen bg-base text-primary overflow-hidden">
      <Sidebar current={page} onNavigate={setPage} running={status.running} />
      <main className="flex-1 overflow-auto">
        {page === "dashboard" && (
          <Dashboard status={status} onStatusChange={refreshStatus} />
        )}
        {page === "config"    && <Config running={status.running} />}
        {page === "stats"     && <Stats />}
        {page === "log"       && <QueryLog />}
      </main>
    </div>
  );
}
```

- [ ] **Step 3: Verify TypeScript compiles with no errors**

```bash
cd gui && npx tsc --noEmit 2>&1
```

Expected: no output (exit code 0). If there are errors, fix them before proceeding.

- [ ] **Step 4: Commit**

```bash
git add gui/src/App.tsx gui/src/components/Sidebar.tsx
git commit -m "feat: add app shell with sidebar nav and status polling"
```

---

### Task 8: Dashboard page

Build the Dashboard page: server status indicator, start/stop button, and a live quick-stats grid.

**Files:**
- Create: `gui/src/pages/Dashboard.tsx`
- Create: `gui/src/pages/Dashboard.test.tsx`

- [ ] **Step 1: Write failing Dashboard tests**

Create `gui/src/pages/Dashboard.test.tsx`:

```tsx
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { Dashboard } from "./Dashboard";
import * as api from "../lib/api";
import type { ProxyStatus } from "../lib/types";

vi.mock("../lib/api");

const stoppedStatus: ProxyStatus = { running: false, listen_addr: "0.0.0.0:5353" };
const runningStatus: ProxyStatus = { running: true,  listen_addr: "0.0.0.0:5353" };

describe("Dashboard", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(api.getStats).mockResolvedValue(null);
  });

  it("shows Start button when stopped", () => {
    render(<Dashboard status={stoppedStatus} onStatusChange={() => {}} />);
    expect(screen.getByRole("button", { name: /^start$/i })).toBeInTheDocument();
  });

  it("shows Stop button when running", () => {
    render(<Dashboard status={runningStatus} onStatusChange={() => {}} />);
    expect(screen.getByRole("button", { name: /^stop$/i })).toBeInTheDocument();
  });

  it("calls startProxy and onStatusChange when Start is clicked", async () => {
    vi.mocked(api.startProxy).mockResolvedValue("0.0.0.0:5353");
    const onStatusChange = vi.fn();
    render(<Dashboard status={stoppedStatus} onStatusChange={onStatusChange} />);
    fireEvent.click(screen.getByRole("button", { name: /^start$/i }));
    await waitFor(() => expect(api.startProxy).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(onStatusChange).toHaveBeenCalledTimes(1));
  });

  it("calls stopProxy and onStatusChange when Stop is clicked", async () => {
    vi.mocked(api.stopProxy).mockResolvedValue(undefined);
    const onStatusChange = vi.fn();
    render(<Dashboard status={runningStatus} onStatusChange={onStatusChange} />);
    fireEvent.click(screen.getByRole("button", { name: /^stop$/i }));
    await waitFor(() => expect(api.stopProxy).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(onStatusChange).toHaveBeenCalledTimes(1));
  });

  it("shows error message when startProxy rejects", async () => {
    vi.mocked(api.startProxy).mockRejectedValue(new Error("port in use"));
    render(<Dashboard status={stoppedStatus} onStatusChange={() => {}} />);
    fireEvent.click(screen.getByRole("button", { name: /^start$/i }));
    await waitFor(() =>
      expect(screen.getByText(/port in use/i)).toBeInTheDocument()
    );
  });

  it("renders listen address", () => {
    render(<Dashboard status={stoppedStatus} onStatusChange={() => {}} />);
    expect(screen.getByText("0.0.0.0:5353")).toBeInTheDocument();
  });

  it("renders stats cards when stats are available", async () => {
    vi.mocked(api.getStats).mockResolvedValue({
      total: 100, cache_hits: 60, upstream: 38, errors: 2,
    });
    render(<Dashboard status={runningStatus} onStatusChange={() => {}} />);
    await waitFor(() => expect(screen.getByText("100")).toBeInTheDocument());
    expect(screen.getByText("60")).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run tests — they must fail**

```bash
cd gui && npm test Dashboard 2>&1 | tail -10
```

Expected: `Cannot find module './Dashboard'` — tests exist but the page doesn't yet.

- [ ] **Step 3: Create `gui/src/pages/Dashboard.tsx`**

```tsx
import { useState, useEffect } from "react";
import { Card } from "../components/ui/Card";
import { Button } from "../components/ui/Button";
import { StatusDot } from "../components/ui/StatusDot";
import { startProxy, stopProxy, getStats } from "../lib/api";
import type { ProxyStatus, StatsSnapshot } from "../lib/types";

interface Props {
  status: ProxyStatus;
  onStatusChange: () => void;
}

export function Dashboard({ status, onStatusChange }: Props) {
  const [loading, setLoading] = useState(false);
  const [error, setError]     = useState<string | null>(null);
  const [stats, setStats]     = useState<StatsSnapshot | null>(null);

  useEffect(() => {
    let cancelled = false;
    const poll = async () => {
      try {
        const s = await getStats();
        if (!cancelled) setStats(s);
      } catch {
        // proxy not running
      }
    };
    poll();
    const id = setInterval(poll, 1000);
    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, [status.running]);

  async function handleToggle() {
    setLoading(true);
    setError(null);
    try {
      if (status.running) {
        await stopProxy();
      } else {
        await startProxy();
      }
      await onStatusChange();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }

  const STAT_CARDS = stats
    ? [
        { label: "Total Queries", value: stats.total },
        { label: "Cache Hits",    value: stats.cache_hits },
        { label: "Upstream",      value: stats.upstream },
        { label: "Errors",        value: stats.errors },
      ]
    : [];

  return (
    <div className="p-6 max-w-2xl">
      <h1 className="text-base font-semibold text-primary mb-5">Dashboard</h1>

      <Card className="mb-5">
        <div className="flex items-center justify-between">
          <div>
            <StatusDot active={status.running} label={status.running ? "Running" : "Stopped"} />
            <p className="mt-2 text-xs font-mono text-muted">{status.listen_addr}</p>
          </div>
          <Button
            variant={status.running ? "danger" : "primary"}
            onClick={handleToggle}
            disabled={loading}
          >
            {loading ? "…" : status.running ? "Stop" : "Start"}
          </Button>
        </div>
        {error && (
          <p className="mt-3 text-xs text-error border-t border-border/50 pt-3">{error}</p>
        )}
      </Card>

      {STAT_CARDS.length > 0 && (
        <div className="grid grid-cols-2 gap-3">
          {STAT_CARDS.map(({ label, value }) => (
            <Card key={label}>
              <p className="text-xs text-secondary uppercase tracking-wider mb-1.5">{label}</p>
              <p className="text-2xl font-mono font-semibold text-primary">
                {value.toLocaleString()}
              </p>
            </Card>
          ))}
        </div>
      )}
    </div>
  );
}
```

- [ ] **Step 4: Run tests — all must pass**

```bash
cd gui && npm test Dashboard 2>&1 | tail -20
```

Expected:
```
✓ src/pages/Dashboard.test.tsx (7)

Test Files  1 passed (1)
Tests  7 passed (7)
```

- [ ] **Step 5: Commit**

```bash
git add gui/src/pages/Dashboard.tsx gui/src/pages/Dashboard.test.tsx
git commit -m "feat: add Dashboard page with start/stop and live stats grid"
```

---

### Task 9: Config page

Build the Config page: an editable form for listen address, upstream URLs, and cache settings that writes back through `saveConfig`.

**Files:**
- Create: `gui/src/pages/Config.tsx`
- Create: `gui/src/pages/Config.test.tsx`

- [ ] **Step 1: Write failing Config tests**

Create `gui/src/pages/Config.test.tsx`:

```tsx
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { Config } from "./Config";
import * as api from "../lib/api";
import type { ProxyConfig } from "../lib/types";

vi.mock("../lib/api");

const defaultConfig: ProxyConfig = {
  listen_addr: "0.0.0.0:5353",
  upstreams: ["https://1.1.1.1/dns-query"],
  cache: { enabled: true, capacity: 10000 },
};

describe("Config", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(api.loadConfig).mockResolvedValue(defaultConfig);
  });

  it("loads and displays config on mount", async () => {
    render(<Config running={false} />);
    await waitFor(() =>
      expect(screen.getByDisplayValue("0.0.0.0:5353")).toBeInTheDocument()
    );
    expect(screen.getByDisplayValue("https://1.1.1.1/dns-query")).toBeInTheDocument();
  });

  it("shows warning banner when server is running", () => {
    render(<Config running={true} />);
    expect(screen.getByText(/stop the server/i)).toBeInTheDocument();
  });

  it("disables inputs when server is running", async () => {
    render(<Config running={true} />);
    await waitFor(() =>
      expect(screen.getByDisplayValue("0.0.0.0:5353")).toBeDisabled()
    );
  });

  it("calls saveConfig with updated values on Save", async () => {
    vi.mocked(api.saveConfig).mockResolvedValue(undefined);
    render(<Config running={false} />);
    await waitFor(() => screen.getByDisplayValue("0.0.0.0:5353"));

    fireEvent.change(screen.getByDisplayValue("0.0.0.0:5353"), {
      target: { value: "127.0.0.1:5353" },
    });
    fireEvent.click(screen.getByRole("button", { name: /save/i }));

    await waitFor(() =>
      expect(api.saveConfig).toHaveBeenCalledWith(
        "127.0.0.1:5353",
        ["https://1.1.1.1/dns-query"],
        true,
        10000
      )
    );
  });

  it("shows success message after save", async () => {
    vi.mocked(api.saveConfig).mockResolvedValue(undefined);
    render(<Config running={false} />);
    await waitFor(() => screen.getByRole("button", { name: /save/i }));
    fireEvent.click(screen.getByRole("button", { name: /save/i }));
    await waitFor(() =>
      expect(screen.getByText(/saved/i)).toBeInTheDocument()
    );
  });

  it("shows error message when saveConfig rejects", async () => {
    vi.mocked(api.saveConfig).mockRejectedValue(new Error("invalid listen address: not-valid"));
    render(<Config running={false} />);
    await waitFor(() => screen.getByRole("button", { name: /save/i }));
    fireEvent.click(screen.getByRole("button", { name: /save/i }));
    await waitFor(() =>
      expect(screen.getByText(/invalid listen address/i)).toBeInTheDocument()
    );
  });
});
```

- [ ] **Step 2: Run tests — they must fail**

```bash
cd gui && npm test Config 2>&1 | tail -5
```

Expected: `Cannot find module './Config'`.

- [ ] **Step 3: Create `gui/src/pages/Config.tsx`**

```tsx
import { useState, useEffect } from "react";
import { Card } from "../components/ui/Card";
import { Button } from "../components/ui/Button";
import { loadConfig, saveConfig } from "../lib/api";
import type { ProxyConfig } from "../lib/types";

interface Props {
  running: boolean;
}

export function Config({ running }: Props) {
  const [config, setConfig]             = useState<ProxyConfig | null>(null);
  const [listenAddr, setListenAddr]     = useState("");
  const [upstreams, setUpstreams]       = useState<string[]>([]);
  const [cacheEnabled, setCacheEnabled] = useState(true);
  const [cacheCapacity, setCacheCapacity] = useState("10000");
  const [saving, setSaving]             = useState(false);
  const [msg, setMsg] = useState<{ type: "success" | "error"; text: string } | null>(null);

  useEffect(() => {
    loadConfig().then((cfg) => {
      setConfig(cfg);
      setListenAddr(cfg.listen_addr);
      setUpstreams(cfg.upstreams);
      setCacheEnabled(cfg.cache.enabled);
      setCacheCapacity(String(cfg.cache.capacity));
    });
  }, []);

  const disabled = running || !config;

  async function handleSave() {
    setSaving(true);
    setMsg(null);
    try {
      await saveConfig(listenAddr, upstreams, cacheEnabled, Number(cacheCapacity));
      setMsg({ type: "success", text: "Configuration saved." });
    } catch (e) {
      setMsg({ type: "error", text: e instanceof Error ? e.message : String(e) });
    } finally {
      setSaving(false);
    }
  }

  function addUpstream() {
    setUpstreams((prev) => [...prev, "https://"]);
  }
  function removeUpstream(i: number) {
    setUpstreams((prev) => prev.filter((_, idx) => idx !== i));
  }
  function updateUpstream(i: number, val: string) {
    setUpstreams((prev) => prev.map((u, idx) => (idx === i ? val : u)));
  }

  return (
    <div className="p-6 max-w-xl">
      <h1 className="text-base font-semibold text-primary mb-5">Configuration</h1>

      {running && (
        <div className="mb-4 px-4 py-2.5 bg-warning/10 border border-warning/20 rounded text-xs text-warning">
          Stop the server before editing configuration.
        </div>
      )}

      <Card className="space-y-5">
        {/* Listen address */}
        <div>
          <label className="block text-xs text-secondary uppercase tracking-wider mb-1.5">
            Listen Address
          </label>
          <input
            value={listenAddr}
            onChange={(e) => setListenAddr(e.target.value)}
            disabled={disabled}
            className="w-full bg-elevated border border-border rounded px-3 py-2 text-sm font-mono text-primary
                       focus:outline-none focus:ring-1 focus:ring-accent disabled:opacity-40"
          />
        </div>

        {/* Upstreams */}
        <div>
          <label className="block text-xs text-secondary uppercase tracking-wider mb-1.5">
            Upstream DNS-over-HTTPS URLs
          </label>
          <div className="space-y-2">
            {upstreams.map((url, i) => (
              <div key={i} className="flex gap-2">
                <input
                  value={url}
                  onChange={(e) => updateUpstream(i, e.target.value)}
                  disabled={disabled}
                  className="flex-1 bg-elevated border border-border rounded px-3 py-2 text-sm font-mono text-primary
                             focus:outline-none focus:ring-1 focus:ring-accent disabled:opacity-40"
                />
                {!disabled && (
                  <button
                    onClick={() => removeUpstream(i)}
                    className="px-2 text-muted hover:text-error transition-colors text-sm"
                  >
                    ✕
                  </button>
                )}
              </div>
            ))}
          </div>
          {!disabled && (
            <button
              onClick={addUpstream}
              className="mt-2 text-xs text-accent hover:text-orange-400 transition-colors"
            >
              + Add upstream
            </button>
          )}
        </div>

        {/* Cache */}
        <div className="flex items-center gap-6">
          <label className="flex items-center gap-2 cursor-pointer">
            <input
              type="checkbox"
              checked={cacheEnabled}
              onChange={(e) => setCacheEnabled(e.target.checked)}
              disabled={disabled}
            />
            <span className="text-sm text-secondary">Cache enabled</span>
          </label>
          <div className="flex items-center gap-2">
            <span className="text-sm text-secondary">Capacity</span>
            <input
              value={cacheCapacity}
              onChange={(e) => setCacheCapacity(e.target.value)}
              disabled={disabled}
              className="w-24 bg-elevated border border-border rounded px-2 py-1.5 text-sm font-mono text-primary
                         focus:outline-none focus:ring-1 focus:ring-accent disabled:opacity-40"
            />
          </div>
        </div>

        {/* Save row */}
        <div className="flex items-center justify-between pt-1 border-t border-border/50">
          <Button onClick={handleSave} disabled={disabled || saving}>
            {saving ? "Saving…" : "Save Config"}
          </Button>
          {msg && (
            <p className={`text-xs ${msg.type === "success" ? "text-success" : "text-error"}`}>
              {msg.text}
            </p>
          )}
        </div>
      </Card>
    </div>
  );
}
```

- [ ] **Step 4: Run tests — all must pass**

```bash
cd gui && npm test Config 2>&1 | tail -15
```

Expected:
```
✓ src/pages/Config.test.tsx (6)

Test Files  1 passed (1)
Tests  6 passed (6)
```

- [ ] **Step 5: Commit**

```bash
git add gui/src/pages/Config.tsx gui/src/pages/Config.test.tsx
git commit -m "feat: add Config page with editable form and save feedback"
```

---

### Task 10: Stats page

Build the Stats page: a live-polling metrics table with a cache hit rate progress bar.

**Files:**
- Create: `gui/src/pages/Stats.tsx`

- [ ] **Step 1: Create `gui/src/pages/Stats.tsx`**

```tsx
import { useState, useEffect } from "react";
import { Card } from "../components/ui/Card";
import { getStats } from "../lib/api";
import type { StatsSnapshot } from "../lib/types";

export function Stats() {
  const [stats, setStats] = useState<StatsSnapshot | null>(null);

  useEffect(() => {
    let cancelled = false;
    const poll = async () => {
      try {
        const s = await getStats();
        if (!cancelled) setStats(s);
      } catch {
        // proxy not running
      }
    };
    poll();
    const id = setInterval(poll, 1000);
    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, []);

  if (!stats) {
    return (
      <div className="p-6">
        <h1 className="text-base font-semibold text-primary mb-5">Statistics</h1>
        <p className="text-secondary text-sm">Start the server to see statistics.</p>
      </div>
    );
  }

  const pct = (n: number, d: number) =>
    d === 0 ? 0 : Math.min(100, Math.round((n / d) * 100));

  const hitRate = pct(stats.cache_hits, stats.total);
  const rows = [
    { label: "Total Queries",   value: stats.total,      pct: null,                          color: "text-primary"   },
    { label: "Cache Hits",      value: stats.cache_hits, pct: hitRate,                        color: "text-success"   },
    { label: "Upstream Queries",value: stats.upstream,   pct: pct(stats.upstream, stats.total), color: "text-info"    },
    { label: "Errors",          value: stats.errors,     pct: pct(stats.errors, stats.total), color: stats.errors > 0 ? "text-error" : "text-muted" },
  ] as const;

  return (
    <div className="p-6 max-w-xl">
      <h1 className="text-base font-semibold text-primary mb-5">Statistics</h1>

      <Card className="mb-4">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-border">
              {(["Metric", "Count", "%"] as const).map((h) => (
                <th
                  key={h}
                  className={`pb-2 text-xs text-secondary uppercase tracking-wider font-medium ${
                    h === "Metric" ? "text-left" : "text-right"
                  }`}
                >
                  {h}
                </th>
              ))}
            </tr>
          </thead>
          <tbody className="divide-y divide-border/40">
            {rows.map(({ label, value, pct: p, color }) => (
              <tr key={label}>
                <td className="py-2.5 text-secondary text-sm">{label}</td>
                <td className={`py-2.5 text-right font-mono text-sm ${color}`}>
                  {value.toLocaleString()}
                </td>
                <td className="py-2.5 text-right font-mono text-sm text-secondary">
                  {p !== null ? `${p}%` : "—"}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </Card>

      {stats.total > 0 && (
        <Card>
          <p className="text-xs text-secondary uppercase tracking-wider mb-2">
            Cache Hit Rate
          </p>
          <div className="flex items-center gap-3">
            <div className="flex-1 bg-elevated rounded-full h-1.5">
              <div
                className="bg-success h-1.5 rounded-full transition-all duration-500"
                style={{ width: `${hitRate}%` }}
              />
            </div>
            <span className="text-sm font-mono text-success w-10 text-right">
              {hitRate}%
            </span>
          </div>
        </Card>
      )}
    </div>
  );
}
```

- [ ] **Step 2: Run the full test suite — must stay green**

```bash
cd gui && npm test 2>&1 | tail -10
```

Expected: all previously passing tests still pass.

- [ ] **Step 3: Commit**

```bash
git add gui/src/pages/Stats.tsx
git commit -m "feat: add Stats page with live metrics table and hit rate bar"
```

---

### Task 11: Query Log page

Build the scrollable Query Log table that polls for new entries every second.

**Files:**
- Create: `gui/src/pages/QueryLog.tsx`

- [ ] **Step 1: Create `gui/src/pages/QueryLog.tsx`**

```tsx
import { useState, useEffect } from "react";
import { Badge } from "../components/ui/Badge";
import { getLogEntries } from "../lib/api";
import type { LogEntry, QueryStatus } from "../lib/types";

function formatTime(unix: number): string {
  const d = new Date(unix * 1000);
  return [d.getUTCHours(), d.getUTCMinutes(), d.getUTCSeconds()]
    .map((n) => String(n).padStart(2, "0"))
    .join(":") + " UTC";
}

function statusBadgeVariant(s: QueryStatus): "success" | "info" | "error" {
  if (s === "CacheHit") return "success";
  if (s === "Upstream") return "info";
  return "error";
}

function statusLabel(s: QueryStatus): string {
  if (s === "CacheHit") return "Cache Hit";
  if (s === "Upstream") return "Upstream";
  return "Error";
}

export function QueryLog() {
  const [entries, setEntries] = useState<LogEntry[]>([]);

  useEffect(() => {
    let cancelled = false;
    const poll = async () => {
      try {
        const e = await getLogEntries();
        if (!cancelled) setEntries(e);
      } catch {
        // proxy not running
      }
    };
    poll();
    const id = setInterval(poll, 1000);
    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, []);

  return (
    <div className="p-6 flex flex-col h-full">
      <div className="flex items-center justify-between mb-5">
        <h1 className="text-base font-semibold text-primary">Query Log</h1>
        {entries.length > 0 && (
          <span className="text-xs text-muted">{entries.length} entries</span>
        )}
      </div>

      {entries.length === 0 ? (
        <p className="text-secondary text-sm italic">
          No queries yet. Start the server and route DNS traffic through it.
        </p>
      ) : (
        <div className="flex-1 min-h-0 overflow-auto border border-border rounded-lg">
          <table className="w-full text-xs">
            <thead className="sticky top-0 bg-surface border-b border-border z-10">
              <tr>
                {["Time", "Query Name", "Type", "Status", "Latency"].map((h) => (
                  <th
                    key={h}
                    className="py-2.5 px-3 text-left text-secondary uppercase tracking-wider font-medium"
                  >
                    {h}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody className="bg-base divide-y divide-border/30">
              {entries.map((entry, i) => (
                <tr
                  key={i}
                  className={`${i % 2 === 1 ? "bg-surface/30" : ""} hover:bg-elevated/50 transition-colors`}
                >
                  <td className="py-2 px-3 font-mono text-muted whitespace-nowrap">
                    {formatTime(entry.timestamp_unix)}
                  </td>
                  <td className="py-2 px-3 font-mono text-primary max-w-xs truncate">
                    {entry.query_name}
                  </td>
                  <td className="py-2 px-3 font-mono text-secondary">{entry.query_type}</td>
                  <td className="py-2 px-3">
                    <Badge variant={statusBadgeVariant(entry.status)}>
                      {statusLabel(entry.status)}
                    </Badge>
                  </td>
                  <td className="py-2 px-3 font-mono text-secondary">
                    {entry.latency_ms}ms
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
```

- [ ] **Step 2: Run the full test suite — must stay green**

```bash
cd gui && npm test 2>&1 | tail -10
```

Expected: all tests still pass, 0 failed.

- [ ] **Step 3: Verify the entire TypeScript project compiles**

```bash
cd gui && npx tsc --noEmit && npm run build
```

Expected: no TypeScript errors, `dist/` contains built assets.

- [ ] **Step 4: Commit**

```bash
git add gui/src/pages/QueryLog.tsx
git commit -m "feat: add Query Log page with striped scrollable table"
```

---

### Task 12: Release workflow — self-contained tarball

Update the GitHub Actions workflow so each release tarball contains the binary, a `config/` directory, and a `README.md`.

**Files:**
- Create: `README.md`
- Modify: `.github/workflows/release.yml`

- [ ] **Step 1: Create `README.md`**

```markdown
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
```

- [ ] **Step 2: Replace `.github/workflows/release.yml`**

```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

jobs:
  build:
    name: Build ${{ matrix.target }}
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        include:
          - os: macos-latest
            target: aarch64-apple-darwin
            binary: doh-proxy-gui
            archive: doh-proxy-gui-macos-arm64.tar.gz

          - os: macos-latest
            target: x86_64-apple-darwin
            binary: doh-proxy-gui
            archive: doh-proxy-gui-macos-x86_64.tar.gz

          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            binary: doh-proxy-gui
            archive: doh-proxy-gui-linux-x86_64.tar.gz

    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - name: Install Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20'
          cache: 'npm'
          cache-dependency-path: gui/package-lock.json

      - name: Install Linux system dependencies
        if: matrix.os == 'ubuntu-latest'
        run: |
          sudo apt-get update
          sudo apt-get install -y \
            libgtk-3-dev \
            libwebkit2gtk-4.1-dev \
            libayatana-appindicator3-dev \
            librsvg2-dev \
            libssl-dev \
            pkg-config

      - name: Install npm dependencies
        working-directory: gui
        run: npm ci

      - name: Build TypeScript frontend
        working-directory: gui
        run: npm run build

      - name: Build Tauri binary
        run: |
          cargo build \
            --manifest-path gui/src-tauri/Cargo.toml \
            --release \
            --target ${{ matrix.target }}

      - name: Stage release directory
        run: |
          STAGING="staging/doh-proxy-gui"
          mkdir -p "$STAGING/config"
          cp gui/src-tauri/target/${{ matrix.target }}/release/${{ matrix.binary }} "$STAGING/"
          cp config/default.toml "$STAGING/config/"
          cp README.md "$STAGING/"

      - name: Create archive
        run: |
          tar -czf ${{ matrix.archive }} -C staging doh-proxy-gui

      - name: Upload release asset
        uses: softprops/action-gh-release@v2
        with:
          files: ${{ matrix.archive }}
```

- [ ] **Step 3: Verify the YAML is valid**

```bash
cd /root/doh_proxy
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))" && echo "YAML OK"
```

Expected: `YAML OK`

- [ ] **Step 4: Run the full test suite one final time**

```bash
# Core Rust tests
cargo test -p doh-proxy

# Tauri backend tests
cd gui/src-tauri && cargo test && cd ../..

# TypeScript tests
cd gui && npm test
```

Expected: all three commands exit 0 with 0 failed tests.

- [ ] **Step 5: Commit**

```bash
git add README.md .github/workflows/release.yml
git commit -m "ci: rewrite release workflow to bundle config/ and README in tarball"
```

---

## Self-Review

### Spec coverage

| Requirement | Task |
|-------------|------|
| Rewrite GUI in TypeScript | Tasks 2, 5–11 |
| Cloudflare-inspired minimalistic dark design | Tasks 6–11 (palette in Task 6, applied in 7–11) |
| Tauri desktop app (not a web page) | Tasks 2–4 |
| Dashboard tab | Task 8 |
| Config tab | Task 9 |
| Stats tab | Task 10 |
| Query Log tab | Task 11 |
| GitHub release with config/ directory | Task 12 |
| GitHub release with README | Task 12 |
| All features work after extraction | Task 12 (binary + config/ bundled together) |

All requirements covered. No gaps found.

### Placeholder scan

- No TBDs or TODOs in any task
- All code steps include complete, copy-pasteable implementations
- All test steps include exact commands with expected output
- Types defined in Task 5 match usage in Tasks 7–11 (snake_case throughout, `listen_addr`, `cache_hits`, `timestamp_unix`)
- `validate_config_input` defined in Task 4, referenced in same task — no forward references

### Type consistency

| Symbol | Defined in | Used in |
|--------|-----------|---------|
| `ProxyStatus.listen_addr` | Task 5 `types.ts` | Tasks 7, 8 ✓ |
| `StatsSnapshot.cache_hits` | Task 5 `types.ts` | Tasks 8, 10 ✓ |
| `LogEntry.timestamp_unix` | Task 5 `types.ts` | Task 11 ✓ |
| `QueryStatus = "CacheHit"` | Task 5 `types.ts` | Task 11 (`statusBadgeVariant`) ✓ |
| `ProxyManager.stats()` → `Arc<Stats>` | Task 3 | Task 4 (`get_stats`) ✓ |
| `Stats::snapshot()` → `StatsSnapshot` | Task 1 (`stats.rs`) | Task 4 (`get_stats`) ✓ |
| `Page` type | Task 7 `Sidebar.tsx` | Task 7 `App.tsx` (imported) ✓ |
