pub mod proxy_manager;

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
