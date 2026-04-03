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
        let stats = Arc::new(Stats::new());
        let resolver = Resolver::new(upstream, None, Some(Arc::clone(&stats)));
        let query = make_query();
        let _ = resolver.resolve(&query).await;
        assert_eq!(stats.errors(), 1);
        assert_eq!(stats.total(), 1);
    }
}
