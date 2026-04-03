use super::app::DohProxyApp;

pub struct StatsPanel;

impl StatsPanel {
    pub fn show(_app: &mut DohProxyApp, ui: &mut eframe::egui::Ui) {
        ui.label("Stats — coming soon");
    }
}
