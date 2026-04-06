use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct QueryRow {
    pub id: i64,
    pub timestamp: String,
    pub domain: String,
    pub query_type: String,
    pub latency_ms: Option<i64>,
    pub blocked: bool,
    pub resolver: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NewQuery {
    pub timestamp: String,
    pub domain: String,
    pub query_type: String,
    pub latency_ms: Option<i64>,
    pub blocked: bool,
    pub resolver: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct DomainCount {
    pub domain: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct StatsResult {
    pub total: i64,
    pub blocked: i64,
    pub avg_latency_ms: Option<f64>,
}

pub async fn run_migrations(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS queries (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp   TEXT    NOT NULL,
            domain      TEXT    NOT NULL,
            query_type  TEXT    NOT NULL,
            latency_ms  INTEGER,
            blocked     BOOLEAN NOT NULL DEFAULT 0,
            resolver    TEXT
        )",
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn insert_query(pool: &SqlitePool, q: NewQuery) -> Result<()> {
    sqlx::query(
        "INSERT INTO queries (timestamp, domain, query_type, latency_ms, blocked, resolver)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&q.timestamp)
    .bind(&q.domain)
    .bind(&q.query_type)
    .bind(q.latency_ms)
    .bind(q.blocked)
    .bind(&q.resolver)
    .execute(pool)
    .await?;
    Ok(())
}

/// Returns rows ordered newest-first.
pub async fn get_recent_queries(pool: &SqlitePool, limit: i64) -> Result<Vec<QueryRow>> {
    let rows = sqlx::query_as::<_, QueryRow>(
        "SELECT id, timestamp, domain, query_type, latency_ms, blocked, resolver
         FROM queries
         ORDER BY id DESC
         LIMIT ?",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn get_top_domains(pool: &SqlitePool, limit: i64) -> Result<Vec<DomainCount>> {
    let rows = sqlx::query_as::<_, DomainCount>(
        "SELECT domain, COUNT(*) AS count
         FROM queries
         GROUP BY domain
         ORDER BY count DESC
         LIMIT ?",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn get_stats(pool: &SqlitePool) -> Result<StatsResult> {
    #[derive(sqlx::FromRow)]
    struct StatsRow {
        total: i64,
        blocked: i64,
        avg_latency_ms: Option<f64>,
    }
    let row = sqlx::query_as::<_, StatsRow>(
        "SELECT
            COUNT(*) AS total,
            COALESCE(SUM(CASE WHEN blocked THEN 1 ELSE 0 END), 0) AS blocked,
            AVG(CAST(latency_ms AS REAL)) AS avg_latency_ms
         FROM queries",
    )
    .fetch_one(pool)
    .await?;

    Ok(StatsResult {
        total: row.total,
        blocked: row.blocked,
        avg_latency_ms: row.avg_latency_ms,
    })
}
