use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use crate::error::{ProxyError, Result};

#[derive(Debug, Clone, Deserialize, Serialize)]
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

#[derive(Debug, Clone, Deserialize, Serialize)]
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
    "0.0.0.0:5353".parse().unwrap()
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

/// Returns `~/.config/doh-proxy/config.toml` (XDG on Linux, standard locations on macOS/Windows).
pub fn config_path() -> PathBuf {
    project_dirs().config_dir().join("config.toml")
}

fn project_dirs() -> ProjectDirs {
    ProjectDirs::from("", "", "doh-proxy")
        .expect("could not determine home directory")
}

impl Config {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| ProxyError::Config(e.to_string()))?;
        toml::from_str(&contents).map_err(|e| ProxyError::Config(e.to_string()))
    }

    /// Load from the XDG config path. If the file does not exist, create it with defaults.
    pub fn load_or_create() -> Result<Self> {
        let path = config_path();
        if path.exists() {
            return Self::from_file(&path);
        }
        let config = Self::default();
        config.save()?;
        Ok(config)
    }

    /// Write config to the XDG config path, creating parent directories as needed.
    pub fn save(&self) -> Result<()> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ProxyError::Config(e.to_string()))?;
        }
        let contents = toml::to_string_pretty(self)
            .map_err(|e| ProxyError::Config(e.to_string()))?;
        std::fs::write(&path, contents)
            .map_err(|e| ProxyError::Config(e.to_string()))?;
        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_path_ends_with_expected_suffix() {
        let path = config_path();
        let s = path.to_string_lossy();
        assert!(s.ends_with("doh-proxy/config.toml"), "got: {s}");
    }

    #[test]
    fn config_defaults_parse() {
        let toml = r#"
            listen_addr = "127.0.0.1:5353"
            upstreams = ["https://1.1.1.1/dns-query"]
        "#;
        let config: Config = toml::from_str(toml).expect("config should parse");
        assert!(!config.upstreams.is_empty());
        assert!(config.cache.enabled);
    }

    #[test]
    fn save_and_load_roundtrip() {
        // Write to a temp file and reload using from_file.
        let dir = std::env::temp_dir().join("doh_proxy_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");

        let original = Config {
            listen_addr: "127.0.0.1:5353".parse().unwrap(),
            upstreams: vec!["https://dns.quad9.net/dns-query".into()],
            cache: CacheConfig { capacity: 500, enabled: false },
        };

        let contents = toml::to_string_pretty(&original).unwrap();
        std::fs::write(&path, contents).unwrap();

        let loaded = Config::from_file(&path).unwrap();
        assert_eq!(loaded.listen_addr.to_string(), "127.0.0.1:5353");
        assert_eq!(loaded.upstreams[0], "https://dns.quad9.net/dns-query");
        assert!(!loaded.cache.enabled);
        assert_eq!(loaded.cache.capacity, 500);
    }
}
