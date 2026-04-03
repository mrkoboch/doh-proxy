use super::app::DohProxyApp;

pub struct Dashboard;

impl Dashboard {
    pub fn show(_app: &mut DohProxyApp, ui: &mut eframe::egui::Ui) {
        ui.label("Dashboard — coming soon");
    }
}
