use std::time::{Duration, SystemTime, UNIX_EPOCH};

use console::style;

use doh_rs::{config::config_path, runtime};

pub fn run() -> anyhow::Result<()> {
    let pid = runtime::read_pid()?;
    let stats = runtime::read_runtime_stats()?;

    match (pid, stats) {
        (Some(pid), Some(rs)) if runtime::process_running(pid) => {
            let uptime = format_uptime(rs.started_at);
            let hit_rate = if rs.snapshot.total > 0 {
                format!(
                    "{:.1}%",
                    rs.snapshot.cache_hits as f64 / rs.snapshot.total as f64 * 100.0
                )
            } else {
                "—".into()
            };

            println!("{}", style("● doh-proxy").green().bold());
            println!(
                "  {:<14} {}",
                style("Status:").dim(),
                style("running").green()
            );
            println!("  {:<14} {}", style("PID:").dim(), pid);
            println!("  {:<14} {}", style("Uptime:").dim(), uptime);
            println!("  {:<14} {}", style("Listen:").dim(), rs.listen_addr);
            println!();
            println!("{}", style("  Queries").bold());
            println!("  {:<14} {}", style("Total:").dim(), rs.snapshot.total);
            println!(
                "  {:<14} {} ({})",
                style("Cache hits:").dim(),
                rs.snapshot.cache_hits,
                hit_rate
            );
            println!(
                "  {:<14} {}",
                style("Upstream:").dim(),
                rs.snapshot.upstream
            );
            println!(
                "  {:<14} {}",
                style("Errors:").dim(),
                style(rs.snapshot.errors).red()
            );
            println!();
            println!(
                "  {:<14} {}",
                style("Config:").dim(),
                config_path().display()
            );
            println!(
                "  {:<14} {}",
                style("Log:").dim(),
                runtime::log_path().display()
            );
        }
        _ => {
            println!("{}", style("○ doh-proxy").dim().bold());
            println!(
                "  {:<14} {}",
                style("Status:").dim(),
                style("stopped").yellow()
            );
            println!("\n  Run `doh-proxy start` to start the proxy.");
        }
    }

    Ok(())
}

fn format_uptime(started_at: u64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs();
    let secs = now.saturating_sub(started_at);
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{h}h {m}m {s}s")
    } else if m > 0 {
        format!("{m}m {s}s")
    } else {
        format!("{s}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_uptime_seconds() {
        let started = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .saturating_sub(45);
        let result = format_uptime(started);
        assert!(result.contains('s'), "got: {result}");
        assert!(!result.contains('h'), "got: {result}");
    }

    #[test]
    fn format_uptime_minutes() {
        let started = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .saturating_sub(125);
        let result = format_uptime(started);
        assert!(result.starts_with("2m"), "got: {result}");
    }

    #[test]
    fn format_uptime_hours() {
        let started = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .saturating_sub(3700);
        let result = format_uptime(started);
        assert!(result.starts_with("1h"), "got: {result}");
    }
}
