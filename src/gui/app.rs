use eframe::egui;

pub struct DohProxyApp;

impl DohProxyApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self
    }
}

impl eframe::App for DohProxyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.label("DoH Proxy GUI - coming soon");
    }
}
