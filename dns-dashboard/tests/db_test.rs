use dns_dashboard::db::{self, DomainCount, NewQuery, StatsResult};
use sqlx::SqlitePool;

async fn test_pool() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("in-memory pool");
    db::run_migrations(&pool).await.expect("migrations");
    pool
}

fn sample_query(domain: &str, blocked: bool, latency: Option<i64>) -> NewQuery {
    NewQuery {
        timestamp: "2026-04-06T12:00:00Z".to_string(),
        domain: domain.to_string(),
        query_type: "A".to_string(),
        latency_ms: latency,
        blocked,
        resolver: Some("https://dns.example/dns-query".to_string()),
    }
}

#[tokio::test]
async fn insert_and_get_recent() {
    let pool = test_pool().await;
    db::insert_query(&pool, sample_query("example.com", false, Some(42)))
        .await
        .unwrap();
    db::insert_query(&pool, sample_query("other.com", true, None))
        .await
        .unwrap();

    let rows = db::get_recent_queries(&pool, 10).await.unwrap();
    assert_eq!(rows.len(), 2);
    // newest first
    assert_eq!(rows[0].domain, "other.com");
    assert_eq!(rows[0].blocked, true);
    assert_eq!(rows[1].domain, "example.com");
    assert_eq!(rows[1].latency_ms, Some(42));
}

#[tokio::test]
async fn get_recent_respects_limit() {
    let pool = test_pool().await;
    for i in 0..5 {
        db::insert_query(&pool, sample_query(&format!("d{i}.com"), false, None))
            .await
            .unwrap();
    }
    let rows = db::get_recent_queries(&pool, 3).await.unwrap();
    assert_eq!(rows.len(), 3);
}

#[tokio::test]
async fn get_top_domains() {
    let pool = test_pool().await;
    for _ in 0..3 {
        db::insert_query(&pool, sample_query("popular.com", false, None))
            .await
            .unwrap();
    }
    db::insert_query(&pool, sample_query("rare.com", false, None))
        .await
        .unwrap();

    let top: Vec<DomainCount> = db::get_top_domains(&pool, 10).await.unwrap();
    assert_eq!(top[0].domain, "popular.com");
    assert_eq!(top[0].count, 3);
    assert_eq!(top[1].domain, "rare.com");
    assert_eq!(top[1].count, 1);
}

#[tokio::test]
async fn get_stats_empty() {
    let pool = test_pool().await;
    let stats: StatsResult = db::get_stats(&pool).await.unwrap();
    assert_eq!(stats.total, 0);
    assert_eq!(stats.blocked, 0);
    assert!(stats.avg_latency_ms.is_none());
}

#[tokio::test]
async fn get_stats_with_data() {
    let pool = test_pool().await;
    db::insert_query(&pool, sample_query("a.com", false, Some(10)))
        .await
        .unwrap();
    db::insert_query(&pool, sample_query("b.com", true, Some(30)))
        .await
        .unwrap();
    db::insert_query(&pool, sample_query("c.com", false, None))
        .await
        .unwrap();

    let stats = db::get_stats(&pool).await.unwrap();
    assert_eq!(stats.total, 3);
    assert_eq!(stats.blocked, 1);
    // avg over rows with latency: (10+30)/2 = 20.0
    assert_eq!(stats.avg_latency_ms, Some(20.0));
}
