use axum::body::to_bytes;
use axum::http::{Request, StatusCode};
use dns_dashboard::api::router;
use dns_dashboard::db::{self, NewQuery};
use sqlx::SqlitePool;
use std::sync::Arc;
use tower::ServiceExt; // for .oneshot()

async fn test_pool() -> Arc<SqlitePool> {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("in-memory pool");
    db::run_migrations(&pool).await.expect("migrations");
    Arc::new(pool)
}

fn insert_q(domain: &str, blocked: bool, latency: Option<i64>) -> NewQuery {
    NewQuery {
        timestamp: "2026-04-06T12:00:00Z".to_string(),
        domain: domain.to_string(),
        query_type: "A".to_string(),
        latency_ms: latency,
        blocked,
        resolver: None,
    }
}

#[tokio::test]
async fn recent_returns_json_array() {
    let pool = test_pool().await;
    db::insert_query(&pool, insert_q("a.com", false, Some(10)))
        .await
        .unwrap();

    let app = router(pool);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/queries/recent?limit=10")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.is_array());
    assert_eq!(json.as_array().unwrap().len(), 1);
    assert_eq!(json[0]["domain"], "a.com");
}

#[tokio::test]
async fn recent_default_limit() {
    let pool = test_pool().await;
    for i in 0..60 {
        db::insert_query(&pool, insert_q(&format!("d{i}.com"), false, None))
            .await
            .unwrap();
    }
    let app = router(pool);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/queries/recent")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json.as_array().unwrap().len(), 50);
}

#[tokio::test]
async fn top_domains_ordered() {
    let pool = test_pool().await;
    for _ in 0..3 {
        db::insert_query(&pool, insert_q("popular.com", false, None))
            .await
            .unwrap();
    }
    db::insert_query(&pool, insert_q("rare.com", false, None))
        .await
        .unwrap();

    let app = router(pool);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/queries/top-domains?limit=5")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json[0]["domain"], "popular.com");
    assert_eq!(json[0]["count"], 3);
}

#[tokio::test]
async fn stats_endpoint() {
    let pool = test_pool().await;
    db::insert_query(&pool, insert_q("a.com", true, Some(20)))
        .await
        .unwrap();
    db::insert_query(&pool, insert_q("b.com", false, Some(40)))
        .await
        .unwrap();

    let app = router(pool);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/stats")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["total"], 2);
    assert_eq!(json["blocked"], 1);
    assert_eq!(json["avg_latency_ms"], 30.0);
}
