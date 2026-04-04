use base64::Engine;
use bytes::Bytes;
use reqwest::Client;
use tracing::debug;

use crate::error::{ProxyError, Result};

/// Sends a DNS wire-format query to an upstream DoH server using
/// the `application/dns-message` wire format (RFC 8484).
pub struct UpstreamClient {
    client: Client,
    urls: Vec<String>,
}

impl UpstreamClient {
    pub fn new(urls: Vec<String>) -> Result<Self> {
        let client = Client::builder()
            .use_rustls_tls()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(ProxyError::Upstream)?;
        Ok(Self { client, urls })
    }

    /// Send `query` (raw DNS message bytes) and return the raw DNS response.
    /// Tries each upstream URL in order, returning the first success.
    pub async fn resolve(&self, query: &[u8]) -> Result<Bytes> {
        let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(query);

        let mut last_err = None;
        for url in &self.urls {
            debug!(url, "querying upstream");
            match self.query_upstream(url, &encoded).await {
                Ok(bytes) => return Ok(bytes),
                Err(e) => last_err = Some(e),
            }
        }

        Err(last_err.unwrap_or(ProxyError::InvalidUpstreamResponse))
    }

    async fn query_upstream(&self, url: &str, encoded: &str) -> Result<Bytes> {
        let response = self
            .client
            .get(url)
            .query(&[("dns", encoded)])
            .header("accept", "application/dns-message")
            .send()
            .await
            .map_err(ProxyError::Upstream)?;

        if !response.status().is_success() {
            return Err(ProxyError::InvalidUpstreamResponse);
        }

        Ok(response.bytes().await.map_err(ProxyError::Upstream)?)
    }
}
