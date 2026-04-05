use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::stats::StatsSnapshot;

#[derive(Debug, Serialize, Deserialize)]
pub struct RuntimeStats {
    pub snapshot: StatsSnapshot,
    pub listen_addr: String,
    pub started_at: u64,
}

/// Returns `~/.local/share/doh-proxy/` (XDG data dir).
pub fn runtime_dir() -> PathBuf {
    ProjectDirs::from("", "", "doh-proxy")
        .map(|d| d.data_dir().to_path_buf())
        .unwrap_or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".local/share/doh-proxy")
        })
}

pub fn log_path() -> PathBuf {
    runtime_dir().join("doh-proxy.log")
}

fn pid_path(dir: &Path) -> PathBuf {
    dir.join("doh-proxy.pid")
}

fn stats_path(dir: &Path) -> PathBuf {
    dir.join("stats.json")
}

fn ensure_dir(dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)
}

// --- Public API (delegates to internal helpers with runtime_dir()) ---

pub fn write_pid(pid: u32) -> anyhow::Result<()> {
    write_pid_to(&runtime_dir(), pid)
}

pub fn read_pid() -> anyhow::Result<Option<u32>> {
    read_pid_from(&runtime_dir())
}

pub fn clear_pid() -> anyhow::Result<()> {
    clear_pid_in(&runtime_dir())
}

pub fn write_runtime_stats(rs: &RuntimeStats) -> anyhow::Result<()> {
    write_runtime_stats_to(&runtime_dir(), rs)
}

pub fn read_runtime_stats() -> anyhow::Result<Option<RuntimeStats>> {
    read_runtime_stats_from(&runtime_dir())
}

// --- Internal helpers (accept &Path for testability) ---

pub(crate) fn write_pid_to(dir: &Path, pid: u32) -> anyhow::Result<()> {
    ensure_dir(dir)?;
    std::fs::write(pid_path(dir), pid.to_string())?;
    Ok(())
}

pub(crate) fn read_pid_from(dir: &Path) -> anyhow::Result<Option<u32>> {
    let path = pid_path(dir);
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)?;
    let pid: u32 = content.trim().parse()?;
    Ok(Some(pid))
}

pub(crate) fn clear_pid_in(dir: &Path) -> anyhow::Result<()> {
    let path = pid_path(dir);
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

pub(crate) fn write_runtime_stats_to(dir: &Path, rs: &RuntimeStats) -> anyhow::Result<()> {
    ensure_dir(dir)?;
    let json = serde_json::to_string(rs)?;
    std::fs::write(stats_path(dir), json)?;
    Ok(())
}

pub(crate) fn read_runtime_stats_from(dir: &Path) -> anyhow::Result<Option<RuntimeStats>> {
    let path = stats_path(dir);
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(path)?;
    let rs: RuntimeStats = serde_json::from_str(&content)?;
    Ok(Some(rs))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::StatsSnapshot;
    use std::fs;

    fn tmp_dir() -> PathBuf {
        let d = std::env::temp_dir().join(format!("doh_proxy_runtime_{}", std::process::id()));
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn write_and_read_pid() {
        let dir = tmp_dir();
        write_pid_to(&dir, 12345).unwrap();
        let pid = read_pid_from(&dir).unwrap();
        assert_eq!(pid, Some(12345));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn clear_pid() {
        let dir = tmp_dir();
        write_pid_to(&dir, 99).unwrap();
        clear_pid_in(&dir).unwrap();
        assert_eq!(read_pid_from(&dir).unwrap(), None);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn read_pid_when_file_absent_returns_none() {
        let dir = tmp_dir();
        assert_eq!(read_pid_from(&dir).unwrap(), None);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn write_and_read_stats() {
        let dir = tmp_dir();
        let rs = RuntimeStats {
            snapshot: StatsSnapshot { total: 10, cache_hits: 3, upstream: 6, errors: 1 },
            listen_addr: "0.0.0.0:5353".into(),
            started_at: 1000,
        };
        write_runtime_stats_to(&dir, &rs).unwrap();
        let loaded = read_runtime_stats_from(&dir).unwrap().unwrap();
        assert_eq!(loaded.snapshot.total, 10);
        assert_eq!(loaded.listen_addr, "0.0.0.0:5353");
        assert_eq!(loaded.started_at, 1000);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn read_stats_when_absent_returns_none() {
        let dir = tmp_dir();
        assert!(read_runtime_stats_from(&dir).unwrap().is_none());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn runtime_dir_returns_valid_path() {
        let dir = runtime_dir();
        assert!(dir.to_string_lossy().contains("doh-proxy"));
    }
}
