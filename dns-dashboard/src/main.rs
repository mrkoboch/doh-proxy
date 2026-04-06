use std::env;
use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;
use std::str::FromStr;
use tower_http::cors::{AllowOrigin, CorsLayer};

use dns_dashboard::{api, db, ingestor};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let log_file = env::var("LOG_FILE").unwrap_or_else(|_| "./proxy.log".to_string());
    let db_url = env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://dashboard.db".to_string());

    tracing::info!(log_file = %log_file, db = %db_url, "starting dns-dashboard");

    let opts = SqliteConnectOptions::from_str(&db_url)?.create_if_missing(true);
    let pool = SqlitePool::connect_with(opts).await?;
    db::run_migrations(&pool).await?;
    let pool = Arc::new(pool);

    // Spawn ingestor in the background
    let ingestor_pool = pool.clone();
    tokio::spawn(async move {
        ingestor::tail_log(ingestor_pool, &log_file).await;
    });

    // Build Axum router with CORS
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::exact(
            "http://localhost:5173".parse().unwrap(),
        ))
        .allow_methods(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any);

    let app = api::router(pool).layer(cors);

    let addr: SocketAddr = "127.0.0.1:4000".parse().unwrap();
    tracing::info!(%addr, "listening");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
