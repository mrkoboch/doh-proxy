fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("DoH Proxy")
            .with_inner_size([700.0, 500.0])
            .with_min_inner_size([500.0, 400.0]),
        ..Default::default()
    };

    eframe::run_native(
        "DoH Proxy",
        native_options,
        Box::new(|cc| {
            Ok(Box::new(doh_proxy::gui::app::DohProxyApp::new(cc)))
        }),
    )
}
