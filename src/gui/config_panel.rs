use std::io::Write;

use eframe::egui::{self, RichText};

use crate::config::{CacheConfig, Config};

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

    pub fn show(app: &mut DohProxyApp, ui: &mut egui::Ui) {
        if app.is_running() {
            ui.add_space(8.0);
            ui.label(
                RichText::new("⚠  Stop the server before editing config.")
                    .color(egui::Color32::from_rgb(255, 200, 0)),
            );
            ui.add_space(8.0);
        }

        ui.add_space(8.0);
        let enabled = !app.is_running();

        // Listen address
        ui.horizontal(|ui| {
            ui.label(RichText::new("Listen Address:").strong());
            ui.add_enabled(
                enabled,
                egui::TextEdit::singleline(&mut app.config_panel.listen_addr)
                    .desired_width(200.0),
            );
        });

        ui.add_space(8.0);

        // Upstream URLs
        ui.label(RichText::new("Upstream URLs:").strong());
        let mut to_remove: Option<usize> = None;
        let upstream_count = app.config_panel.upstreams.len();
        for i in 0..upstream_count {
            ui.horizontal(|ui| {
                ui.add_enabled(
                    enabled,
                    egui::TextEdit::singleline(&mut app.config_panel.upstreams[i])
                        .desired_width(360.0),
                );
                if enabled && ui.small_button("✕").clicked() {
                    to_remove = Some(i);
                }
            });
        }
        if let Some(idx) = to_remove {
            app.config_panel.upstreams.remove(idx);
        }
        if enabled && ui.small_button("+ Add upstream").clicked() {
            app.config_panel.upstreams.push("https://".into());
        }

        ui.add_space(8.0);

        // Cache settings
        ui.horizontal(|ui| {
            ui.label(RichText::new("Cache:").strong());
            ui.add_enabled(
                enabled,
                egui::Checkbox::new(&mut app.config_panel.cache_enabled, "Enabled"),
            );
            ui.add_space(16.0);
            ui.label("Capacity:");
            ui.add_enabled(
                enabled,
                egui::TextEdit::singleline(&mut app.config_panel.cache_capacity)
                    .desired_width(80.0),
            );
        });

        ui.add_space(16.0);

        // Save button
        if enabled && ui.button(RichText::new("💾  Save Config").size(14.0)).clicked() {
            match Self::apply_and_save(app) {
                Ok(()) => {
                    app.status_message = Some("Config saved.".into());
                }
                Err(e) => {
                    app.status_message = Some(format!("Save failed: {e}"));
                }
            }
        }
    }

    fn apply_and_save(app: &mut DohProxyApp) -> anyhow::Result<()> {
        let listen_addr: std::net::SocketAddr = app
            .config_panel
            .listen_addr
            .parse()
            .map_err(|e| anyhow::anyhow!("invalid listen address: {e}"))?;

        let capacity: u64 = app
            .config_panel
            .cache_capacity
            .trim()
            .parse()
            .map_err(|e| anyhow::anyhow!("invalid cache capacity: {e}"))?;

        let upstreams: Vec<String> = app
            .config_panel
            .upstreams
            .iter()
            .filter(|u| !u.trim().is_empty())
            .cloned()
            .collect();

        if upstreams.is_empty() {
            return Err(anyhow::anyhow!("at least one upstream URL is required"));
        }

        for url in &upstreams {
            if !url.starts_with("https://") && !url.starts_with("http://") {
                return Err(anyhow::anyhow!(
                    "invalid upstream URL (must start with https:// or http://): {url}"
                ));
            }
        }

        // Build TOML — keep it human-readable, escape strings to prevent injection
        let toml = format!(
            "listen_addr = \"{}\"\n\nupstreams = [\n{}]\n\n[cache]\nenabled = {}\ncapacity = {}\n",
            toml_escape(&listen_addr.to_string()),
            upstreams.iter().map(|u| format!("    \"{}\",\n", toml_escape(u))).collect::<String>(),
            app.config_panel.cache_enabled,
            capacity,
        );

        let tmp_path = format!("{}.tmp", app.config_path);
        {
            let mut f = std::fs::File::create(&tmp_path)
                .map_err(|e| anyhow::anyhow!("cannot write config: {e}"))?;
            f.write_all(toml.as_bytes())
                .map_err(|e| anyhow::anyhow!("cannot write config: {e}"))?;
        }
        std::fs::rename(&tmp_path, &app.config_path)
            .map_err(|e| anyhow::anyhow!("cannot replace config file: {e}"))?;

        // Update live config
        app.config.listen_addr = listen_addr;
        app.config_panel.listen_addr = app.config.listen_addr.to_string();
        app.config.upstreams = upstreams;
        app.config.cache = CacheConfig {
            enabled: app.config_panel.cache_enabled,
            capacity,
        };

        Ok(())
    }
}

fn toml_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
