use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use doh_proxy::{config::Config, stats::Stats};

pub(crate) fn toml_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

enum ProxyState {
    Stopped,
    Running {
        stop_flag: Arc<AtomicBool>,
        _thread: std::thread::JoinHandle<()>,
        stats: Arc<Stats>,
    },
}

pub struct ProxyManager {
    state: ProxyState,
    config: Config,
    config_path: std::path::PathBuf,
}

impl ProxyManager {
    pub fn new() -> Self {
        let config_path = Self::locate_config();
        let config = Config::from_file(&config_path).unwrap_or_default();
        Self { state: ProxyState::Stopped, config, config_path }
    }

    pub fn new_for_test(config: Config) -> Self {
        Self {
            state: ProxyState::Stopped,
            config,
            config_path: std::path::PathBuf::from("/tmp/test-config.toml"),
        }
    }

    fn locate_config() -> std::path::PathBuf {
        if let Ok(exe) = std::env::current_exe() {
            let candidate = exe
                .parent()
                .unwrap_or(std::path::Path::new("."))
                .join("config/default.toml");
            if candidate.exists() {
                return candidate;
            }
        }
        std::path::PathBuf::from("config/default.toml")
    }

    pub fn is_running(&self) -> bool {
        matches!(self.state, ProxyState::Running { .. })
    }

    pub fn stats(&self) -> Option<Arc<Stats>> {
        match &self.state {
            ProxyState::Running { stats, .. } => Some(Arc::clone(stats)),
            ProxyState::Stopped => None,
        }
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn start(&mut self) -> Result<String, String> {
        if self.is_running() {
            return Err("proxy is already running".into());
        }
        let stop_flag = Arc::new(AtomicBool::new(false));
        let stats = Arc::new(Stats::new());
        let config = self.config.clone();
        let stop_clone = Arc::clone(&stop_flag);
        let stats_clone = Arc::clone(&stats);

        let thread = std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            rt.block_on(async move {
                match doh_proxy::server::Server::new(config, Some(stats_clone)).await {
                    Ok(server) => {
                        if let Err(e) = server.run_cancellable(stop_clone).await {
                            tracing::error!("server error: {e}");
                        }
                    }
                    Err(e) => tracing::error!("server init error: {e}"),
                }
            });
        });

        let addr = self.config.listen_addr.to_string();
        self.state = ProxyState::Running { stop_flag, _thread: thread, stats };
        Ok(addr)
    }

    pub fn stop(&mut self) {
        let old = std::mem::replace(&mut self.state, ProxyState::Stopped);
        if let ProxyState::Running { stop_flag, _thread, .. } = old {
            stop_flag.store(true, Ordering::Release);
            let _ = _thread.join();
        }
    }

    pub fn update_config(&mut self, config: Config) -> Result<(), String> {
        use std::io::Write;

        let upstreams_toml: String = config
            .upstreams
            .iter()
            .map(|u| format!("    \"{}\",\n", toml_escape(u)))
            .collect();

        let toml = format!(
            "listen_addr = \"{}\"\n\nupstreams = [\n{}]\n\n[cache]\nenabled = {}\ncapacity = {}\n",
            toml_escape(&config.listen_addr.to_string()),
            upstreams_toml,
            config.cache.enabled,
            config.cache.capacity,
        );

        if let Some(parent) = self.config_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        let tmp = self.config_path.with_extension("toml.tmp");
        let mut f = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
        f.write_all(toml.as_bytes()).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, &self.config_path).map_err(|e| e.to_string())?;

        self.config = config;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use doh_proxy::config::Config;

    #[test]
    fn new_manager_is_stopped() {
        let pm = ProxyManager::new_for_test(Config::default());
        assert!(!pm.is_running());
    }

    #[test]
    fn stopped_manager_has_no_stats() {
        let pm = ProxyManager::new_for_test(Config::default());
        assert!(pm.stats().is_none());
    }

    #[test]
    fn toml_escape_handles_quotes() {
        assert_eq!(toml_escape(r#"has "quotes""#), r#"has \"quotes\""#);
    }

    #[test]
    fn toml_escape_handles_backslash() {
        assert_eq!(toml_escape(r"back\slash"), r"back\\slash");
    }

    #[test]
    fn toml_escape_leaves_plain_strings_unchanged() {
        assert_eq!(toml_escape("https://1.1.1.1/dns-query"), "https://1.1.1.1/dns-query");
    }
}
