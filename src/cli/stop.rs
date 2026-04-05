use std::thread;
use std::time::{Duration, Instant};

use console::style;

use doh_proxy::runtime;

pub fn run() -> anyhow::Result<()> {
    let pid = match runtime::read_pid()? {
        Some(p) => p,
        None => {
            println!("{} doh-proxy is not running.", style("○").dim());
            return Ok(());
        }
    };

    if !runtime::process_running(pid) {
        // Stale pidfile
        runtime::clear_pid().ok();
        println!(
            "{} doh-proxy is not running (stale PID file removed).",
            style("○").dim()
        );
        return Ok(());
    }

    println!("{} Stopping doh-proxy (PID {pid})...", style("◼").yellow());

    send_sigterm(pid)?;

    // Wait up to 5 seconds for the process to exit
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        thread::sleep(Duration::from_millis(100));
        if !runtime::process_running(pid) {
            println!("{} Stopped.", style("○").dim());
            return Ok(());
        }
        if Instant::now() > deadline {
            eprintln!(
                "{} Process did not exit within 5 seconds. Send SIGKILL manually: kill -9 {pid}",
                style("✗").red()
            );
            return Err(anyhow::anyhow!("timeout waiting for process {pid} to exit"));
        }
    }
}

#[cfg(unix)]
fn send_sigterm(pid: u32) -> anyhow::Result<()> {
    use nix::sys::signal::{kill, Signal};
    use nix::unistd::Pid;
    kill(Pid::from_raw(pid as i32), Signal::SIGTERM)
        .map_err(|e| anyhow::anyhow!("failed to send SIGTERM to {pid}: {e}"))
}

#[cfg(not(unix))]
fn send_sigterm(pid: u32) -> anyhow::Result<()> {
    Err(anyhow::anyhow!(
        "`doh-proxy stop` is not supported on this platform. Kill PID {pid} manually."
    ))
}
