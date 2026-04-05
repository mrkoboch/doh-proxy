use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use tracing::info;

use doh_proxy::{
    config::Config,
    runtime::{self, RuntimeStats},
    server::Server,
    stats::{unix_now, Stats},
};

pub async fn run(listen: Option<String>, upstreams: Vec<String>) -> anyhow::Result<()> {
    // Check if already running
    if let Some(pid) = runtime::read_pid()? {
        if process_running(pid) {
            eprintln!(
                "{} doh-proxy is already running (PID {pid}). Run `doh-proxy stop` first.",
                style("✗").red()
            );
            std::process::exit(1);
        }
        // Stale pidfile — clean it up
        runtime::clear_pid()?;
    }

    // Load config and apply CLI overrides
    let mut config = Config::load_or_create()?;
    if let Some(addr) = listen {
        config.listen_addr = addr
            .parse()
            .map_err(|e| anyhow::anyhow!("invalid listen address: {e}"))?;
    }
    if !upstreams.is_empty() {
        config.upstreams = upstreams;
    }

    // Set up file logging
    let log_path = runtime::log_path();
    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file_appender = tracing_appender::rolling::never(
        log_path.parent().unwrap(),
        log_path.file_name().unwrap(),
    );
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_level(true),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
                .with_target(false),
        )
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    // Spinner during server init
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .map_err(|e| anyhow::anyhow!("spinner template error: {e}"))?
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    spinner.set_message("Starting proxy...");
    spinner.enable_steady_tick(Duration::from_millis(80));

    let stats = Arc::new(Stats::new());
    let stop_flag = Arc::new(AtomicBool::new(false));

    let server = match Server::new(config.clone(), Some(Arc::clone(&stats))).await {
        Ok(s) => s,
        Err(e) => {
            spinner.finish_and_clear();
            eprintln!("{} Failed to start server: {e}", style("✗").red());
            return Err(e.into());
        }
    };

    spinner.finish_and_clear();

    // Write PID file
    runtime::write_pid(std::process::id())?;

    let started_at = unix_now();
    println!(
        "{} Listening on {}",
        style("●").green().bold(),
        style(&config.listen_addr).cyan()
    );
    println!(
        "  {} {}\n  {} {}",
        style("Upstreams:").dim(),
        config.upstreams.join(", "),
        style("Log:").dim(),
        log_path.display()
    );
    println!("{}", style("  Press Ctrl+C or run `doh-proxy stop` to quit.").dim());

    // Spawn stats writer task
    let stats_handle = {
        let stats = Arc::clone(&stats);
        let stop = Arc::clone(&stop_flag);
        let listen_addr = config.listen_addr.to_string();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            loop {
                interval.tick().await;
                if stop.load(Ordering::Acquire) {
                    break;
                }
                let rs = RuntimeStats {
                    snapshot: stats.snapshot(),
                    listen_addr: listen_addr.clone(),
                    started_at,
                };
                runtime::write_runtime_stats(&rs).ok();
            }
        })
    };

    // Spawn server on a background thread (it needs its own Tokio runtime)
    let server_thread = {
        let stop = Arc::clone(&stop_flag);
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            rt.block_on(async move {
                if let Err(e) = server.run_cancellable(stop).await {
                    tracing::error!("server error: {e}");
                }
            });
        })
    };

    // Wait for shutdown signal
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigterm = signal(SignalKind::terminate())?;
        tokio::select! {
            _ = sigterm.recv() => { info!("received SIGTERM, shutting down"); }
            _ = tokio::signal::ctrl_c() => { info!("received SIGINT, shutting down"); }
        }
    }
    #[cfg(not(unix))]
    tokio::signal::ctrl_c().await?;

    println!("\n{} Shutting down...", style("◼").yellow());
    stop_flag.store(true, Ordering::Release);
    let _ = server_thread.join();
    let _ = stats_handle.await;   // flush final stats snapshot
    runtime::clear_pid().ok();
    println!("{} Stopped.", style("●").dim());

    Ok(())
}

/// Returns true if a process with this PID is currently running.
/// Sending signal 0 checks process existence without actually signaling.
#[cfg(unix)]
fn process_running(pid: u32) -> bool {
    use nix::sys::signal::kill;
    use nix::unistd::Pid;
    kill(Pid::from_raw(pid as i32), None).is_ok()
}

#[cfg(not(unix))]
fn process_running(_pid: u32) -> bool {
    false
}
