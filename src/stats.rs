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
