use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use sqlx::SqlitePool;
use tokio::io::{AsyncBufReadExt, AsyncSeekExt, BufReader};
use tokio::time::sleep;

use crate::db::{insert_query, NewQuery};

/// Parse a single log line. Returns `None` for lines that cannot be parsed.
///
/// Format: TIMESTAMP DOMAIN TYPE [BLOCKED] [latency=Xms] [resolver=URL]
pub fn parse_line(line: &str) -> Option<NewQuery> {
    let mut fields = line.split_whitespace();
    let timestamp = fields.next()?.to_string();
    let domain = fields.next()?.to_string();
    let query_type = fields.next()?.to_string();

    let mut blocked = false;
    let mut latency_ms: Option<i64> = None;
    let mut resolver: Option<String> = None;

    for field in fields {
        if field == "BLOCKED" {
            blocked = true;
        } else if let Some(rest) = field.strip_prefix("latency=") {
            let ms_str = rest.strip_suffix("ms").unwrap_or(rest);
            if let Ok(ms) = ms_str.parse::<i64>() {
                latency_ms = Some(ms);
            }
        } else if let Some(url) = field.strip_prefix("resolver=") {
            resolver = Some(url.to_string());
        }
        // unknown fields silently ignored
    }

    Some(NewQuery {
        timestamp,
        domain,
        query_type,
        latency_ms,
        blocked,
        resolver,
    })
}

/// Runs forever: tails `log_path`, inserting each new parseable line into the DB.
/// Seeks to end of file on first open so it only processes new lines.
pub async fn tail_log(pool: Arc<SqlitePool>, log_path: impl AsRef<Path>) {
    let log_path = log_path.as_ref().to_owned();

    loop {
        match try_tail_log(pool.clone(), &log_path).await {
            Ok(()) => {}
            Err(e) => {
                tracing::warn!(error = %e, path = ?log_path, "ingestor error, retrying in 5s");
                sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

async fn try_tail_log(pool: Arc<SqlitePool>, log_path: &Path) -> anyhow::Result<()> {
    use tokio::fs::File;

    // Wait for the file to appear
    while !log_path.exists() {
        tracing::info!(path = ?log_path, "waiting for log file to appear");
        sleep(Duration::from_secs(2)).await;
    }

    let file = File::open(log_path).await?;
    // Seek to end so we only read new lines
    let pos = file.metadata().await?.len();
    let mut reader = BufReader::new(file);
    reader.seek(std::io::SeekFrom::Start(pos)).await?;

    let mut line = String::new();
    loop {
        line.clear();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            // Check for file rotation: if current file is smaller than our position, re-open
            if let Ok(meta) = tokio::fs::metadata(log_path).await {
                if meta.len() < reader.stream_position().await.unwrap_or(0) {
                    tracing::info!(path = ?log_path, "log file rotated, re-opening");
                    return Ok(());
                }
            }
            // No new data — poll again shortly
            sleep(Duration::from_millis(200)).await;
            continue;
        }
        let trimmed = line.trim_end_matches('\n').trim_end_matches('\r');
        if let Some(query) = parse_line(trimmed) {
            if let Err(e) = insert_query(&pool, query).await {
                tracing::warn!(error = %e, "failed to insert query, skipping");
            }
        }
    }
}
