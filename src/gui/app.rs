use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use eframe::egui;

use crate::{config::Config, stats::Stats};

use crate::gui::{
    config_panel::ConfigPanel,
    dashboard::Dashboard,
    log_panel::LogPanel,
    stats_panel::StatsPanel,
};

#[derive(PartialEq)]
pub enum Tab {
    Dashboard,
    Config,
    Stats,
    QueryLog,
}

pub enum ServerState {
    Stopped,
    Running {
        stop_flag: Arc<AtomicBool>,
        _thread: std::thread::JoinHandle<()>,
    },
}

pub struct DohProxyApp {
    pub config: Config,
    pub config_path: String,
    pub stats: Arc<Stats>,
    pub server_state: ServerState,
    pub active_tab: Tab,
    pub status_message: Option<String>,
    pub config_panel: ConfigPanel,
}

impl DohProxyApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let config_path = "config/default.toml".to_string();
        let config = Config::from_file(&config_path)
            .unwrap_or_else(|_| Config::default());
        let config_panel = ConfigPanel::from_config(&config);
        Self {
            config: config.clone(),
            config_path,
            stats: Arc::new(Stats::new()),
            server_state: ServerState::Stopped,
            active_tab: Tab::Dashboard,
            status_message: None,
            config_panel,
        }
    }

    pub fn is_running(&self) -> bool {
        matches!(self.server_state, ServerState::Running { .. })
    }

    pub fn start_server(&mut self) {
        if self.is_running() {
            return;
        }
        let stop_flag = Arc::new(AtomicBool::new(false));
        let config = self.config.clone();
        let stats = Arc::clone(&self.stats);
        let stop_clone = Arc::clone(&stop_flag);

        let thread = std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            rt.block_on(async move {
                match crate::server::Server::new(config, Some(stats)).await {
                    Ok(server) => {
                        if let Err(e) = server.run_cancellable(stop_clone).await {
                            tracing::error!("server error: {}", e);
                        }
                    }
                    Err(e) => {
                        tracing::error!("server init error: {}", e);
                    }
                }
            });
        });

        self.server_state = ServerState::Running { stop_flag, _thread: thread };
        self.status_message = Some(format!("Server started on {}", self.config.listen_addr));
    }

    pub fn stop_server(&mut self) {
        let old_state = std::mem::replace(&mut self.server_state, ServerState::Stopped);
        if let ServerState::Running { stop_flag, _thread } = old_state {
            stop_flag.store(true, Ordering::Release);
            if let Err(e) = _thread.join() {
                tracing::error!("server thread panicked: {:?}", e);
            }
        }
        self.status_message = Some("Server stopped.".into());
    }
}

impl eframe::App for DohProxyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Request repaint every 500ms so stats refresh while running
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(500));

        // Tab bar
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.active_tab, Tab::Dashboard, "Dashboard");
            ui.selectable_value(&mut self.active_tab, Tab::Config,    "Config");
            ui.selectable_value(&mut self.active_tab, Tab::Stats,     "Stats");
            ui.selectable_value(&mut self.active_tab, Tab::QueryLog,  "Query Log");
        });

        ui.separator();

        // Status bar at bottom (render before tab content so layout is stable)
        if let Some(ref msg) = self.status_message {
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.small(msg);
            });
        }

        // Tab content
        match self.active_tab {
            Tab::Dashboard => Dashboard::show(self, ui),
            Tab::Config    => ConfigPanel::show(self, ui),
            Tab::Stats     => StatsPanel::show(self, ui),
            Tab::QueryLog  => LogPanel::show(self, ui),
        }
    }
}
