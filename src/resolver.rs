use bytes::Bytes;
use hickory_proto::{
    op::Message,
    serialize::binary::BinDecodable,
};
use tracing::debug;

use crate::{
    cache::{CacheKey, DnsCache},
    error::{ProxyError, Result},
    upstream::UpstreamClient,
};

/// Resolves DNS queries: cache-first, then upstream DoH.
pub struct Resolver {
    upstream: UpstreamClient,
    cache: Option<DnsCache>,
}

impl Resolver {
    pub fn new(upstream: UpstreamClient, cache: Option<DnsCache>) -> Self {
        Self { upstream, cache }
    }

    pub async fn resolve(&self, raw_query: &[u8]) -> Result<Bytes> {
        let msg = Message::from_bytes(raw_query).map_err(ProxyError::DnsParse)?;

        let cache_key = cache_key_for(&msg);

        if let Some(ref cache) = self.cache {
            if let Some(cached) = cache.get(&cache_key).await {
                debug!("cache hit");
                return Ok(cached);
            }
        }

        let response_bytes = self.upstream.resolve(raw_query).await?;

        let min_ttl = extract_min_ttl(&response_bytes).unwrap_or(60);

        if let Some(ref cache) = self.cache {
            cache.insert(cache_key, response_bytes.clone(), min_ttl).await;
        }

        Ok(response_bytes)
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
    msg.answers()
        .iter()
        .map(|r| r.ttl() as u64)
        .min()
}
