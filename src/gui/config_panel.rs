use crate::config::Config;
use super::app::DohProxyApp;

pub struct ConfigPanel {
    pub listen_addr: String,
    pub upstreams: Vec<String>,
    pub cache_enabled: bool,
    pub cache_capacity: String,
}

impl ConfigPanel {
    pub fn from_config(config: &Config) -> Self {
        Self {
            listen_addr: config.listen_addr.to_string(),
            upstreams: config.upstreams.clone(),
            cache_enabled: config.cache.enabled,
            cache_capacity: config.cache.capacity.to_string(),
        }
    }

    pub fn show(_app: &mut DohProxyApp, ui: &mut eframe::egui::Ui) {
        ui.label("Config — coming soon");
    }
}
