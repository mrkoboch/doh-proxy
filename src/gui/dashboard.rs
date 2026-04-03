use eframe::egui::{self, Color32, RichText};

use super::app::DohProxyApp;

pub struct Dashboard;

impl Dashboard {
    pub fn show(app: &mut DohProxyApp, ui: &mut egui::Ui) {
        ui.add_space(16.0);

        // Status indicator
        ui.horizontal(|ui| {
            let (dot_color, status_text) = if app.is_running() {
                (Color32::from_rgb(72, 199, 116), "Running")
            } else {
                (Color32::from_rgb(180, 180, 180), "Stopped")
            };
            ui.label(RichText::new("●").color(dot_color));
            ui.label(RichText::new(status_text).strong().size(16.0));
        });

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        // Listen address
        ui.horizontal(|ui| {
            ui.label(RichText::new("Listen Address:").strong());
            ui.label(app.config.listen_addr.to_string());
        });

        ui.add_space(8.0);

        // Upstreams
        ui.label(RichText::new("Upstream Servers:").strong());
        for url in &app.config.upstreams {
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.label("▸");
                ui.label(url);
            });
        }

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(16.0);

        // Start / Stop button
        let (btn_label, btn_color) = if app.is_running() {
            ("■  Stop", Color32::from_rgb(220, 80, 80))
        } else {
            ("▶  Start", Color32::from_rgb(72, 199, 116))
        };

        let btn = egui::Button::new(RichText::new(btn_label).size(15.0))
            .fill(btn_color)
            .min_size(egui::vec2(120.0, 36.0));

        if ui.add(btn).clicked() {
            if app.is_running() {
                app.stop_server();
            } else {
                app.start_server();
            }
        }

        ui.add_space(12.0);

        // Quick stats row (only when running)
        if app.is_running() {
            ui.separator();
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(format!("Queries: {}", app.stats.total()));
                ui.add_space(20.0);
                ui.label(format!("Cache Hits: {}", app.stats.cache_hits()));
                ui.add_space(20.0);
                ui.label(format!("Errors: {}", app.stats.errors()));
            });
        }
    }
}
