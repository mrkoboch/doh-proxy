use super::app::DohProxyApp;

pub struct LogPanel;

impl LogPanel {
    pub fn show(_app: &mut DohProxyApp, ui: &mut eframe::egui::Ui) {
        ui.label("Query Log — coming soon");
    }
}
