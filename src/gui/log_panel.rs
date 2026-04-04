use std::time::{SystemTime, UNIX_EPOCH};

use eframe::egui::{self, Color32, RichText, ScrollArea};

use crate::stats::QueryStatus;

use super::app::DohProxyApp;

pub struct LogPanel;

impl LogPanel {
    pub fn show(app: &mut DohProxyApp, ui: &mut egui::Ui) {
        ui.add_space(8.0);

        let entries = app.stats.snapshot_log();

        if entries.is_empty() {
            ui.add_space(40.0);
            ui.centered_and_justified(|ui| {
                ui.label(
                    RichText::new(
                        "No queries yet. Start the server and route DNS traffic through it.",
                    )
                    .italics()
                    .color(Color32::GRAY),
                );
            });
            return;
        }

        // Header row
        egui::Grid::new("log_header")
            .num_columns(4)
            .min_col_width(80.0)
            .show(ui, |ui| {
                ui.label(RichText::new("Time").strong());
                ui.label(RichText::new("Query Name").strong());
                ui.label(RichText::new("Type").strong());
                ui.label(RichText::new("Status").strong());
                ui.end_row();
            });

        ui.separator();

        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("log_entries")
                    .num_columns(4)
                    .min_col_width(80.0)
                    .striped(true)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        for entry in &entries {
                            let time_str = format_time(entry.timestamp);
                            ui.label(RichText::new(&time_str).monospace().small());
                            ui.label(RichText::new(&entry.query_name).monospace().small());
                            ui.label(RichText::new(&entry.query_type).monospace().small());

                            let (color, label) = match entry.status {
                                QueryStatus::CacheHit => {
                                    (Color32::from_rgb(72, 199, 116), "Cache Hit")
                                }
                                QueryStatus::Upstream => {
                                    (Color32::from_rgb(100, 160, 220), "Upstream")
                                }
                                QueryStatus::Error => {
                                    (Color32::from_rgb(220, 80, 80), "Error")
                                }
                            };
                            ui.label(RichText::new(label).small().color(color));
                            ui.end_row();
                        }
                    });
            });
    }
}

fn format_time(t: SystemTime) -> String {
    let secs = t.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let h = (secs % 86400) / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    format!("{h:02}:{m:02}:{s:02}")
}
