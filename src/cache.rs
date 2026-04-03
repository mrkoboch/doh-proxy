use bytes::Bytes;
use moka::future::Cache;
use std::time::Duration;

/// Cache key: raw DNS query name + qtype (lowercased).
pub type CacheKey = (String, u16);

#[derive(Clone)]
pub struct DnsCache {
    inner: Cache<CacheKey, Bytes>,
}

impl DnsCache {
    pub fn new(capacity: u64) -> Self {
        let inner = Cache::builder()
            .max_capacity(capacity)
            .time_to_live(Duration::from_secs(300))
            .build();
        Self { inner }
    }

    pub async fn get(&self, key: &CacheKey) -> Option<Bytes> {
        self.inner.get(key).await
    }

    pub async fn insert(&self, key: CacheKey, value: Bytes, ttl_secs: u64) {
        // Moka supports per-entry TTL via insert_with_expiry; use a simple insert
        // here and rely on the global TTL for now.
        let _ = ttl_secs; // used in future per-entry TTL support
        self.inner.insert(key, value).await;
    }
}
