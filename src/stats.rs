use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

    fn make_entry_named(name: &str, status: QueryStatus) -> LogEntry {
        LogEntry { timestamp_unix: 0, query_name: name.into(), query_type: "A".into(), status, latency_ms: 0 }
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
