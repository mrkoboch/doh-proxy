use serde::Deserialize;
use std::net::SocketAddr;
use std::path::Path;

use crate::error::{ProxyError, Result};

#[derive(Debug, Clone, Deserialize)]
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

#[derive(Debug, Clone, Deserialize)]
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
    "0.0.0.0:53".parse().unwrap()
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

impl Config {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| ProxyError::Config(e.to_string()))?;
        toml::from_str(&contents)
            .map_err(|e| ProxyError::Config(e.to_string()))
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
