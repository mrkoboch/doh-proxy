use eframe::egui::{self, Color32, RichText};

use super::app::DohProxyApp;

pub struct StatsPanel;

impl StatsPanel {
    pub fn show(app: &mut DohProxyApp, ui: &mut egui::Ui) {
        ui.add_space(16.0);
        ui.heading("Live Statistics");
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(12.0);

        let total = app.stats.total();
        let hits  = app.stats.cache_hits();
        let ups   = app.stats.upstream();
        let errs  = app.stats.errors();

        let hit_pct = if total > 0 { hits * 100 / total } else { 0 };
        let ups_pct = if total > 0 { ups  * 100 / total } else { 0 };
        let err_pct = if total > 0 { errs * 100 / total } else { 0 };

        egui::Grid::new("stats_grid")
            .num_columns(3)
            .spacing([24.0, 8.0])
            .show(ui, |ui| {
                ui.label(RichText::new("Metric").strong());
                ui.label(RichText::new("Count").strong());
                ui.label(RichText::new("%").strong());
                ui.end_row();

                ui.separator(); ui.separator(); ui.separator();
                ui.end_row();

                ui.label("Total Queries");
                ui.label(RichText::new(total.to_string()).monospace());
                ui.label("—");
                ui.end_row();

                ui.label("Cache Hits");
                ui.colored_label(
                    Color32::from_rgb(72, 199, 116),
                    RichText::new(hits.to_string()).monospace(),
                );
                ui.label(format!("{hit_pct}%"));
                ui.end_row();

                ui.label("Upstream Queries");
                ui.label(RichText::new(ups.to_string()).monospace());
                ui.label(format!("{ups_pct}%"));
                ui.end_row();

                ui.label("Errors");
                ui.colored_label(
                    if errs > 0 {
                        Color32::from_rgb(220, 80, 80)
                    } else {
                        Color32::GRAY
                    },
                    RichText::new(errs.to_string()).monospace(),
                );
                ui.label(format!("{err_pct}%"));
                ui.end_row();
            });

        ui.add_space(16.0);

        if total > 0 {
            let filled = (hit_pct as usize).min(50);
            let bar: String = "█".repeat(filled) + &"░".repeat(50 - filled);
            ui.label(
                RichText::new(format!("Cache hit rate: {bar} {hit_pct}%"))
                    .monospace()
                    .small()
                    .color(Color32::from_rgb(72, 199, 116)),
            );
        }
    }
}
