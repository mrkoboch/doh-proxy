# DNS Dashboard Backend Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Rust HTTP backend that ingests a proxy log file and serves DNS query statistics via a JSON API.

**Architecture:** A new `dns-dashboard/` Cargo binary crate inside the workspace. A background tokio task tails a log file and writes rows into SQLite via sqlx. An Axum HTTP server exposes three read-only JSON endpoints. Both components share an `Arc<SqlitePool>`.

**Tech Stack:** Rust, axum 0.7, sqlx 0.8 (sqlite + runtime-tokio), tokio (full), serde/serde_json, tower-http (cors), chrono (serde)

---

## File Map

| Path | Purpose |
|---|---|
| `dns-dashboard/Cargo.toml` | Crate manifest with all dependencies |
| `dns-dashboard/src/main.rs` | Entry point: env config, shared pool, spawn ingestor, run axum |
| `dns-dashboard/src/db.rs` | sqlx pool setup, migration, all DB query functions |
| `dns-dashboard/src/ingestor.rs` | Tokio task that tails a file and calls `db::insert_query` |
| `dns-dashboard/src/api.rs` | Axum router and handlers for the three endpoints |
| `dns-dashboard/tests/db_test.rs` | Integration tests for all DB functions |
| `dns-dashboard/tests/api_test.rs` | Integration tests for all API endpoints |
| `dns-dashboard/tests/ingestor_test.rs` | Integration tests for log-line parsing |
| `Cargo.toml` (workspace root) | Add `dns-dashboard` to `[workspace]` members |

---

## Task 1: Workspace Setup

**Files:**
- Modify: `Cargo.toml` (root)
- Create: `dns-dashboard/Cargo.toml`
- Create: `dns-dashboard/src/main.rs` (stub)

- [ ] **Step 1: Convert root Cargo.toml to a workspace**

Open `/opt/doh_proxy/Cargo.toml`. Add a `[workspace]` table **before** `[package]` so that Cargo treats the repo as a workspace. The existing `[package]` entry covers `doh-rs`; add `dns-dashboard` as a second member.

```toml
[workspace]
members = [".", "dns-dashboard"]
resolver = "2"
```

- [ ] **Step 2: Verify the workspace parses**

```bash
cd /opt/doh_proxy && cargo metadata --no-deps --format-version 1 | grep '"name"'
```

Expected output includes both `"doh-rs"` and (not yet) — this will fail until dns-dashboard exists; proceed.

- [ ] **Step 3: Create the dns-dashboard crate directory and Cargo.toml**

```bash
mkdir -p /opt/doh_proxy/dns-dashboard/src
```

Create `/opt/doh_proxy/dns-dashboard/Cargo.toml`:

```toml
[package]
name = "dns-dashboard"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "dns-dashboard"
path = "src/main.rs"

[dependencies]
tokio = { version = "1", features = ["full"] }
axum = { version = "0.7", features = [] }
sqlx = { version = "0.8", features = ["sqlite", "runtime-tokio", "chrono", "migrate"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tower-http = { version = "0.5", features = ["cors"] }
chrono = { version = "0.4", features = ["serde"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
anyhow = "1"

[dev-dependencies]
tokio = { version = "1", features = ["full"] }
tempfile = "3"
```

- [ ] **Step 4: Create stub main.rs**

Create `/opt/doh_proxy/dns-dashboard/src/main.rs`:

```rust
fn main() {
    println!("dns-dashboard stub");
}
```

- [ ] **Step 5: Verify workspace compiles**

```bash
cd /opt/doh_proxy && cargo build -p dns-dashboard 2>&1 | tail -5
```

Expected: `Compiling dns-dashboard v0.1.0` … `Finished`

- [ ] **Step 6: Commit**

```bash
cd /opt/doh_proxy
git add Cargo.toml dns-dashboard/
git commit -m "chore: add dns-dashboard crate to workspace"
```

---

## Task 2: Database Layer (`db.rs`)

**Files:**
- Create: `dns-dashboard/src/db.rs`
- Create: `dns-dashboard/tests/db_test.rs`
- Modify: `dns-dashboard/src/main.rs` (add `mod db;`)

The `db` module owns the schema migration and all query functions. No SQL lives anywhere else.

### Data types

```rust
// QueryRow — used for insert and for recent-query responses
pub struct QueryRow {
    pub id: i64,
    pub timestamp: String,   // ISO8601 e.g. "2026-04-06T12:00:00Z"
    pub domain: String,
    pub query_type: String,  // "A", "AAAA", etc.
    pub latency_ms: Option<i64>,
    pub blocked: bool,
    pub resolver: Option<String>,
}

// NewQuery — what the ingestor passes in (no id yet)
pub struct NewQuery {
    pub timestamp: String,
    pub domain: String,
    pub query_type: String,
    pub latency_ms: Option<i64>,
    pub blocked: bool,
    pub resolver: Option<String>,
}

pub struct DomainCount {
    pub domain: String,
    pub count: i64,
}

pub struct StatsResult {
    pub total: i64,
    pub blocked: i64,
    pub avg_latency_ms: Option<f64>,
}
```

- [ ] **Step 1: Write the failing db tests**

Create `/opt/doh_proxy/dns-dashboard/tests/db_test.rs`:

```rust
use dns_dashboard::db::{self, DomainCount, NewQuery, StatsResult};
use sqlx::SqlitePool;
use tempfile::tempdir;

async fn test_pool() -> SqlitePool {
    // Use in-memory DB so tests are isolated and fast
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("in-memory pool");
    db::run_migrations(&pool).await.expect("migrations");
    pool
}

fn sample_query(domain: &str, blocked: bool, latency: Option<i64>) -> NewQuery {
    NewQuery {
        timestamp: "2026-04-06T12:00:00Z".to_string(),
        domain: domain.to_string(),
        query_type: "A".to_string(),
        latency_ms: latency,
        blocked,
        resolver: Some("https://dns.example/dns-query".to_string()),
    }
}

#[tokio::test]
async fn insert_and_get_recent() {
    let pool = test_pool().await;
    db::insert_query(&pool, sample_query("example.com", false, Some(42)))
        .await
        .unwrap();
    db::insert_query(&pool, sample_query("other.com", true, None))
        .await
        .unwrap();

    let rows = db::get_recent_queries(&pool, 10).await.unwrap();
    assert_eq!(rows.len(), 2);
    // newest first
    assert_eq!(rows[0].domain, "other.com");
    assert_eq!(rows[0].blocked, true);
    assert_eq!(rows[1].domain, "example.com");
    assert_eq!(rows[1].latency_ms, Some(42));
}

#[tokio::test]
async fn get_recent_respects_limit() {
    let pool = test_pool().await;
    for i in 0..5 {
        db::insert_query(&pool, sample_query(&format!("d{i}.com"), false, None))
            .await
            .unwrap();
    }
    let rows = db::get_recent_queries(&pool, 3).await.unwrap();
    assert_eq!(rows.len(), 3);
}

#[tokio::test]
async fn get_top_domains() {
    let pool = test_pool().await;
    for _ in 0..3 {
        db::insert_query(&pool, sample_query("popular.com", false, None))
            .await
            .unwrap();
    }
    db::insert_query(&pool, sample_query("rare.com", false, None))
        .await
        .unwrap();

    let top: Vec<DomainCount> = db::get_top_domains(&pool, 10).await.unwrap();
    assert_eq!(top[0].domain, "popular.com");
    assert_eq!(top[0].count, 3);
    assert_eq!(top[1].domain, "rare.com");
    assert_eq!(top[1].count, 1);
}

#[tokio::test]
async fn get_stats_empty() {
    let pool = test_pool().await;
    let stats: StatsResult = db::get_stats(&pool).await.unwrap();
    assert_eq!(stats.total, 0);
    assert_eq!(stats.blocked, 0);
    assert!(stats.avg_latency_ms.is_none());
}

#[tokio::test]
async fn get_stats_with_data() {
    let pool = test_pool().await;
    db::insert_query(&pool, sample_query("a.com", false, Some(10)))
        .await
        .unwrap();
    db::insert_query(&pool, sample_query("b.com", true, Some(30)))
        .await
        .unwrap();
    db::insert_query(&pool, sample_query("c.com", false, None))
        .await
        .unwrap();

    let stats = db::get_stats(&pool).await.unwrap();
    assert_eq!(stats.total, 3);
    assert_eq!(stats.blocked, 1);
    // avg over rows with latency: (10+30)/2 = 20.0
    assert_eq!(stats.avg_latency_ms, Some(20.0));
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd /opt/doh_proxy && cargo test -p dns-dashboard --test db_test 2>&1 | tail -10
```

Expected: compile error — `dns_dashboard::db` does not exist.

- [ ] **Step 3: Create `db.rs`**

Create `/opt/doh_proxy/dns-dashboard/src/db.rs`:

```rust
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct QueryRow {
    pub id: i64,
    pub timestamp: String,
    pub domain: String,
    pub query_type: String,
    pub latency_ms: Option<i64>,
    pub blocked: bool,
    pub resolver: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NewQuery {
    pub timestamp: String,
    pub domain: String,
    pub query_type: String,
    pub latency_ms: Option<i64>,
    pub blocked: bool,
    pub resolver: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct DomainCount {
    pub domain: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct StatsResult {
    pub total: i64,
    pub blocked: i64,
    pub avg_latency_ms: Option<f64>,
}

pub async fn run_migrations(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS queries (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp   TEXT    NOT NULL,
            domain      TEXT    NOT NULL,
            query_type  TEXT    NOT NULL,
            latency_ms  INTEGER,
            blocked     BOOLEAN NOT NULL DEFAULT 0,
            resolver    TEXT
        )",
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn insert_query(pool: &SqlitePool, q: NewQuery) -> Result<()> {
    sqlx::query(
        "INSERT INTO queries (timestamp, domain, query_type, latency_ms, blocked, resolver)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&q.timestamp)
    .bind(&q.domain)
    .bind(&q.query_type)
    .bind(q.latency_ms)
    .bind(q.blocked)
    .bind(&q.resolver)
    .execute(pool)
    .await?;
    Ok(())
}

/// Returns rows ordered newest-first.
pub async fn get_recent_queries(pool: &SqlitePool, limit: i64) -> Result<Vec<QueryRow>> {
    let rows = sqlx::query_as::<_, QueryRow>(
        "SELECT id, timestamp, domain, query_type, latency_ms, blocked, resolver
         FROM queries
         ORDER BY id DESC
         LIMIT ?",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn get_top_domains(pool: &SqlitePool, limit: i64) -> Result<Vec<DomainCount>> {
    let rows = sqlx::query_as::<_, DomainCount>(
        "SELECT domain, COUNT(*) AS count
         FROM queries
         GROUP BY domain
         ORDER BY count DESC
         LIMIT ?",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn get_stats(pool: &SqlitePool) -> Result<StatsResult> {
    let row = sqlx::query!(
        "SELECT
            COUNT(*) AS total,
            SUM(CASE WHEN blocked THEN 1 ELSE 0 END) AS blocked,
            AVG(CAST(latency_ms AS REAL)) AS avg_latency_ms
         FROM queries"
    )
    .fetch_one(pool)
    .await?;

    Ok(StatsResult {
        total: row.total,
        blocked: row.blocked.unwrap_or(0),
        avg_latency_ms: row.avg_latency_ms,
    })
}
```

- [ ] **Step 4: Expose `db` from the crate and add a lib target**

Add `/opt/doh_proxy/dns-dashboard/src/lib.rs`:

```rust
pub mod db;
```

Add to `/opt/doh_proxy/dns-dashboard/Cargo.toml` under `[package]`:

```toml
[lib]
name = "dns_dashboard"
path = "src/lib.rs"
```

Update `dns-dashboard/src/main.rs` to reference the lib:

```rust
use dns_dashboard::db;

#[tokio::main]
async fn main() {
    println!("dns-dashboard stub");
}
```

- [ ] **Step 5: Set `DATABASE_URL` for sqlx compile-time checks and run tests**

```bash
cd /opt/doh_proxy && export DATABASE_URL="sqlite::memory:" && cargo test -p dns-dashboard --test db_test 2>&1 | tail -20
```

Expected: all 5 tests pass.

- [ ] **Step 6: Commit**

```bash
cd /opt/doh_proxy
git add dns-dashboard/
git commit -m "feat(dns-dashboard): db layer with schema migration and query functions"
```

---

## Task 3: Log Ingestor (`ingestor.rs`)

**Files:**
- Create: `dns-dashboard/src/ingestor.rs`
- Create: `dns-dashboard/tests/ingestor_test.rs`
- Modify: `dns-dashboard/src/lib.rs` (add `pub mod ingestor;`)

The ingestor is a single async function `tail_log(pool, path)` that runs forever in a background task. It opens the file, seeks to the end, then polls for new lines using `tokio::fs`. On each new line it calls `parse_line` and if successful calls `db::insert_query`.

### Log format

Lines the proxy writes look like:

```
2026-04-06T12:00:00Z example.com A latency=42ms resolver=https://dns.example/dns-query
2026-04-06T12:00:01Z ads.tracker.io AAAA BLOCKED latency=5ms
2026-04-06T12:00:02Z malware.com A BLOCKED
2026-04-06T12:00:03Z plain.com A
```

Field rules:
- Field 0: timestamp (ISO8601)
- Field 1: domain
- Field 2: query type
- Remaining fields (optional, any order): `BLOCKED` (bare word), `latency=<N>ms`, `resolver=<URL>`

- [ ] **Step 1: Write the failing ingestor tests**

Create `/opt/doh_proxy/dns-dashboard/tests/ingestor_test.rs`:

```rust
use dns_dashboard::ingestor::parse_line;

#[test]
fn parses_full_line() {
    let line = "2026-04-06T12:00:00Z example.com A latency=42ms resolver=https://dns.example/dns-query";
    let q = parse_line(line).expect("should parse");
    assert_eq!(q.timestamp, "2026-04-06T12:00:00Z");
    assert_eq!(q.domain, "example.com");
    assert_eq!(q.query_type, "A");
    assert_eq!(q.latency_ms, Some(42));
    assert_eq!(q.blocked, false);
    assert_eq!(q.resolver.as_deref(), Some("https://dns.example/dns-query"));
}

#[test]
fn parses_blocked_with_latency() {
    let line = "2026-04-06T12:00:01Z ads.tracker.io AAAA BLOCKED latency=5ms";
    let q = parse_line(line).expect("should parse");
    assert_eq!(q.domain, "ads.tracker.io");
    assert_eq!(q.query_type, "AAAA");
    assert_eq!(q.blocked, true);
    assert_eq!(q.latency_ms, Some(5));
    assert!(q.resolver.is_none());
}

#[test]
fn parses_blocked_no_extras() {
    let line = "2026-04-06T12:00:02Z malware.com A BLOCKED";
    let q = parse_line(line).expect("should parse");
    assert_eq!(q.blocked, true);
    assert!(q.latency_ms.is_none());
    assert!(q.resolver.is_none());
}

#[test]
fn parses_minimal_line() {
    let line = "2026-04-06T12:00:03Z plain.com A";
    let q = parse_line(line).expect("should parse");
    assert_eq!(q.domain, "plain.com");
    assert_eq!(q.blocked, false);
    assert!(q.latency_ms.is_none());
    assert!(q.resolver.is_none());
}

#[test]
fn returns_none_for_short_lines() {
    assert!(parse_line("").is_none());
    assert!(parse_line("only_one_field").is_none());
    assert!(parse_line("ts domain").is_none()); // missing query_type
}

#[test]
fn ignores_unknown_fields() {
    let line = "2026-04-06T12:00:04Z x.com A unknown_field=foo latency=10ms";
    let q = parse_line(line).expect("should parse despite unknown field");
    assert_eq!(q.latency_ms, Some(10));
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd /opt/doh_proxy && cargo test -p dns-dashboard --test ingestor_test 2>&1 | tail -10
```

Expected: compile error — `dns_dashboard::ingestor` does not exist.

- [ ] **Step 3: Create `ingestor.rs`**

Create `/opt/doh_proxy/dns-dashboard/src/ingestor.rs`:

```rust
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use sqlx::SqlitePool;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::time::sleep;

use crate::db::{insert_query, NewQuery};

/// Parse a single log line. Returns `None` for lines that cannot be parsed.
///
/// Format: TIMESTAMP DOMAIN TYPE [BLOCKED] [latency=Xms] [resolver=URL]
pub fn parse_line(line: &str) -> Option<NewQuery> {
    let mut fields = line.split_whitespace();
    let timestamp = fields.next()?.to_string();
    let domain = fields.next()?.to_string();
    let query_type = fields.next()?.to_string();

    let mut blocked = false;
    let mut latency_ms: Option<i64> = None;
    let mut resolver: Option<String> = None;

    for field in fields {
        if field == "BLOCKED" {
            blocked = true;
        } else if let Some(rest) = field.strip_prefix("latency=") {
            let ms_str = rest.strip_suffix("ms").unwrap_or(rest);
            if let Ok(ms) = ms_str.parse::<i64>() {
                latency_ms = Some(ms);
            }
        } else if let Some(url) = field.strip_prefix("resolver=") {
            resolver = Some(url.to_string());
        }
        // unknown fields are silently ignored
    }

    Some(NewQuery {
        timestamp,
        domain,
        query_type,
        latency_ms,
        blocked,
        resolver,
    })
}

/// Runs forever: tails `log_path`, inserting each new parseable line into the DB.
/// Seeks to end of file on first open so it only processes new lines.
pub async fn tail_log(pool: Arc<SqlitePool>, log_path: impl AsRef<Path>) {
    let log_path = log_path.as_ref().to_owned();

    loop {
        match try_tail_log(pool.clone(), &log_path).await {
            Ok(()) => {}
            Err(e) => {
                tracing::warn!(error = %e, path = ?log_path, "ingestor error, retrying in 5s");
                sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

async fn try_tail_log(pool: Arc<SqlitePool>, log_path: &Path) -> anyhow::Result<()> {
    use tokio::fs::File;

    // Wait for the file to appear
    while !log_path.exists() {
        tracing::info!(path = ?log_path, "waiting for log file to appear");
        sleep(Duration::from_secs(2)).await;
    }

    let file = File::open(log_path).await?;
    // Seek to end so we only read new lines
    let pos = file.metadata().await?.len();
    let mut reader = BufReader::new(file);
    use tokio::io::AsyncSeekExt;
    reader.seek(std::io::SeekFrom::Start(pos)).await?;

    let mut line = String::new();
    loop {
        line.clear();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            // No new data — poll again shortly
            sleep(Duration::from_millis(200)).await;
            continue;
        }
        let trimmed = line.trim_end_matches('\n').trim_end_matches('\r');
        if let Some(query) = parse_line(trimmed) {
            if let Err(e) = insert_query(&pool, query).await {
                tracing::warn!(error = %e, "failed to insert query, skipping");
            }
        }
    }
}
```

- [ ] **Step 4: Register `ingestor` in `lib.rs`**

Edit `/opt/doh_proxy/dns-dashboard/src/lib.rs`:

```rust
pub mod db;
pub mod ingestor;
```

- [ ] **Step 5: Run ingestor tests**

```bash
cd /opt/doh_proxy && cargo test -p dns-dashboard --test ingestor_test 2>&1 | tail -15
```

Expected: all 6 tests pass.

- [ ] **Step 6: Commit**

```bash
cd /opt/doh_proxy
git add dns-dashboard/src/ingestor.rs dns-dashboard/src/lib.rs dns-dashboard/tests/ingestor_test.rs
git commit -m "feat(dns-dashboard): log ingestor with parse_line and tail_log"
```

---

## Task 4: HTTP API (`api.rs`)

**Files:**
- Create: `dns-dashboard/src/api.rs`
- Create: `dns-dashboard/tests/api_test.rs`
- Modify: `dns-dashboard/src/lib.rs` (add `pub mod api;`)

The API module exports a single `router(pool)` function that returns an `axum::Router`. Three handlers call the matching `db::*` functions and serialize results as JSON.

### Route table

| Method | Path | Query params | Response |
|---|---|---|---|
| GET | `/api/queries/recent` | `limit` (default 50) | `Vec<QueryRow>` |
| GET | `/api/queries/top-domains` | `limit` (default 10) | `Vec<DomainCount>` |
| GET | `/api/stats` | — | `StatsResult` |

- [ ] **Step 1: Write the failing API tests**

Create `/opt/doh_proxy/dns-dashboard/tests/api_test.rs`:

```rust
use axum::body::to_bytes;
use axum::http::{Request, StatusCode};
use dns_dashboard::api::router;
use dns_dashboard::db::{self, NewQuery};
use sqlx::SqlitePool;
use std::sync::Arc;
use tower::ServiceExt; // for .oneshot()

async fn test_pool() -> Arc<SqlitePool> {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("in-memory pool");
    db::run_migrations(&pool).await.expect("migrations");
    Arc::new(pool)
}

fn insert_q(domain: &str, blocked: bool, latency: Option<i64>) -> NewQuery {
    NewQuery {
        timestamp: "2026-04-06T12:00:00Z".to_string(),
        domain: domain.to_string(),
        query_type: "A".to_string(),
        latency_ms: latency,
        blocked,
        resolver: None,
    }
}

#[tokio::test]
async fn recent_returns_json_array() {
    let pool = test_pool().await;
    db::insert_query(&pool, insert_q("a.com", false, Some(10)))
        .await
        .unwrap();

    let app = router(pool);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/queries/recent?limit=10")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.is_array());
    assert_eq!(json.as_array().unwrap().len(), 1);
    assert_eq!(json[0]["domain"], "a.com");
}

#[tokio::test]
async fn recent_default_limit() {
    let pool = test_pool().await;
    // Insert 60 entries — default limit is 50 so we should get 50 back
    for i in 0..60 {
        db::insert_query(&pool, insert_q(&format!("d{i}.com"), false, None))
            .await
            .unwrap();
    }
    let app = router(pool);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/queries/recent")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json.as_array().unwrap().len(), 50);
}

#[tokio::test]
async fn top_domains_ordered() {
    let pool = test_pool().await;
    for _ in 0..3 {
        db::insert_query(&pool, insert_q("popular.com", false, None))
            .await
            .unwrap();
    }
    db::insert_query(&pool, insert_q("rare.com", false, None))
        .await
        .unwrap();

    let app = router(pool);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/queries/top-domains?limit=5")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json[0]["domain"], "popular.com");
    assert_eq!(json[0]["count"], 3);
}

#[tokio::test]
async fn stats_endpoint() {
    let pool = test_pool().await;
    db::insert_query(&pool, insert_q("a.com", true, Some(20)))
        .await
        .unwrap();
    db::insert_query(&pool, insert_q("b.com", false, Some(40)))
        .await
        .unwrap();

    let app = router(pool);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/stats")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["total"], 2);
    assert_eq!(json["blocked"], 1);
    assert_eq!(json["avg_latency_ms"], 30.0);
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd /opt/doh_proxy && cargo test -p dns-dashboard --test api_test 2>&1 | tail -10
```

Expected: compile error — `dns_dashboard::api` does not exist.

- [ ] **Step 3: Create `api.rs`**

Create `/opt/doh_proxy/dns-dashboard/src/api.rs`:

```rust
use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::db;

type Pool = Arc<SqlitePool>;

pub fn router(pool: Pool) -> Router {
    Router::new()
        .route("/api/queries/recent", get(recent_queries))
        .route("/api/queries/top-domains", get(top_domains))
        .route("/api/stats", get(stats))
        .with_state(pool)
}

#[derive(Deserialize)]
struct LimitParam {
    limit: Option<i64>,
}

async fn recent_queries(
    State(pool): State<Pool>,
    Query(params): Query<LimitParam>,
) -> Response {
    let limit = params.limit.unwrap_or(50);
    match db::get_recent_queries(&pool, limit).await {
        Ok(rows) => Json(rows).into_response(),
        Err(e) => {
            tracing::error!(error = %e, "recent_queries failed");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn top_domains(
    State(pool): State<Pool>,
    Query(params): Query<LimitParam>,
) -> Response {
    let limit = params.limit.unwrap_or(10);
    match db::get_top_domains(&pool, limit).await {
        Ok(rows) => Json(rows).into_response(),
        Err(e) => {
            tracing::error!(error = %e, "top_domains failed");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn stats(State(pool): State<Pool>) -> Response {
    match db::get_stats(&pool).await {
        Ok(s) => Json(s).into_response(),
        Err(e) => {
            tracing::error!(error = %e, "stats failed");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
```

- [ ] **Step 4: Register `api` in `lib.rs`**

Edit `/opt/doh_proxy/dns-dashboard/src/lib.rs`:

```rust
pub mod api;
pub mod db;
pub mod ingestor;
```

- [ ] **Step 5: Add `tower` as a dev-dependency** (needed for `.oneshot()` in tests)

Edit `/opt/doh_proxy/dns-dashboard/Cargo.toml`, add to `[dev-dependencies]`:

```toml
tower = { version = "0.4", features = ["util"] }
axum = { version = "0.7", features = ["macros"] }
```

- [ ] **Step 6: Run API tests**

```bash
cd /opt/doh_proxy && cargo test -p dns-dashboard --test api_test 2>&1 | tail -20
```

Expected: all 4 tests pass.

- [ ] **Step 7: Commit**

```bash
cd /opt/doh_proxy
git add dns-dashboard/src/api.rs dns-dashboard/src/lib.rs dns-dashboard/tests/api_test.rs dns-dashboard/Cargo.toml
git commit -m "feat(dns-dashboard): axum API router with recent, top-domains, and stats endpoints"
```

---

## Task 5: Main Entry Point (`main.rs`)

**Files:**
- Modify: `dns-dashboard/src/main.rs`

Wire everything together: read `LOG_FILE` env var, open the SQLite pool, run migrations, spawn the ingestor, start the Axum server with CORS.

- [ ] **Step 1: Write `main.rs`**

Replace `/opt/doh_proxy/dns-dashboard/src/main.rs`:

```rust
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use sqlx::SqlitePool;
use tower_http::cors::{AllowOrigin, CorsLayer};

use dns_dashboard::{api, db, ingestor};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let log_file = env::var("LOG_FILE").unwrap_or_else(|_| "./proxy.log".to_string());
    let db_url = env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://dashboard.db".to_string());

    tracing::info!(log_file = %log_file, db = %db_url, "starting dns-dashboard");

    let pool = SqlitePool::connect(&db_url).await?;
    db::run_migrations(&pool).await?;
    let pool = Arc::new(pool);

    // Spawn ingestor in the background
    let ingestor_pool = pool.clone();
    tokio::spawn(async move {
        ingestor::tail_log(ingestor_pool, &log_file).await;
    });

    // Build Axum router
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::exact(
            "http://localhost:5173".parse().unwrap(),
        ))
        .allow_methods(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any);

    let app = api::router(pool).layer(cors);

    let addr: SocketAddr = "127.0.0.1:4000".parse().unwrap();
    tracing::info!(%addr, "listening");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
```

- [ ] **Step 2: Build the binary**

```bash
cd /opt/doh_proxy && cargo build -p dns-dashboard 2>&1 | tail -10
```

Expected: `Finished` with no errors.

- [ ] **Step 3: Smoke-test the server**

In one terminal start the server:

```bash
cd /opt/doh_proxy && DATABASE_URL="sqlite://test-dash.db" cargo run -p dns-dashboard &
sleep 1
```

Check the stats endpoint:

```bash
curl -s http://127.0.0.1:4000/api/stats
```

Expected:

```json
{"total":0,"blocked":0,"avg_latency_ms":null}
```

Check CORS header:

```bash
curl -si -H "Origin: http://localhost:5173" http://127.0.0.1:4000/api/stats | grep -i access-control
```

Expected: `access-control-allow-origin: http://localhost:5173`

Kill the background process:

```bash
kill %1 && rm -f test-dash.db
```

- [ ] **Step 4: Run all tests one final time**

```bash
cd /opt/doh_proxy && DATABASE_URL="sqlite::memory:" cargo test -p dns-dashboard 2>&1 | tail -20
```

Expected: all tests pass, no warnings.

- [ ] **Step 5: Commit**

```bash
cd /opt/doh_proxy
git add dns-dashboard/src/main.rs
git commit -m "feat(dns-dashboard): wire main.rs with ingestor, axum server, and CORS"
```

---

## Self-Review

**Spec coverage:**

| Spec requirement | Task |
|---|---|
| `dns-dashboard/` directory as workspace member | Task 1 |
| `db.rs` — queries table with all columns | Task 2 |
| `db::insert_query` | Task 2 |
| `db::get_recent_queries(limit)` | Task 2 |
| `db::get_top_domains(limit)` | Task 2 |
| `db::get_stats` (total, blocked, avg latency) | Task 2 |
| `ingestor.rs` — tail file, parse format, insert, skip bad lines | Task 3 |
| `api.rs` — GET /api/queries/recent?limit | Task 4 |
| `api.rs` — GET /api/queries/top-domains?limit | Task 4 |
| `api.rs` — GET /api/stats | Task 4 |
| `main.rs` — background ingestor task | Task 5 |
| `main.rs` — Axum on 127.0.0.1:4000 | Task 5 |
| CORS allow http://localhost:5173 | Task 5 |
| `LOG_FILE` env var, default `./proxy.log` | Task 5 |
| `Arc<SqlitePool>` shared state | Tasks 2, 4, 5 |

All requirements covered. No gaps found.

**Placeholder scan:** No TBD / TODO / "similar to" language present. All code steps are complete.

**Type consistency:**
- `NewQuery` defined in Task 2 `db.rs`, used in Task 3 `ingestor.rs` via `use crate::db::NewQuery` ✓
- `QueryRow`, `DomainCount`, `StatsResult` defined in Task 2, used in Task 4 handlers (returned via `Json(rows)`) ✓
- `db::run_migrations`, `db::insert_query`, `db::get_recent_queries`, `db::get_top_domains`, `db::get_stats` — named consistently across all tasks ✓
- `ingestor::tail_log` called in Task 5 `main.rs` with `(Arc<SqlitePool>, &log_file)` — matches Task 3 signature `tail_log(pool: Arc<SqlitePool>, log_path: impl AsRef<Path>)` ✓
- `api::router(pool: Arc<SqlitePool>) -> Router` — called in Task 5 with `.layer(cors)` applied to the returned `Router` ✓
