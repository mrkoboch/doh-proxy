use std::env;
use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use axum::http::{header, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use rust_embed::RustEmbed;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;
use std::str::FromStr;
use tower_http::cors::{AllowOrigin, CorsLayer};

use dns_dashboard::{api, db, ingestor};

/// The compiled frontend, embedded into the binary at build time.
#[derive(RustEmbed)]
#[folder = "../dns-dashboard-ui/dist"]
struct Frontend;

/// Serve a file from the embedded frontend. Falls back to index.html for
/// unknown paths so client-side routing works correctly.
async fn frontend_handler(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    match Frontend::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            (
                [(header::CONTENT_TYPE, mime.as_ref().to_string())],
                content.data,
            )
                .into_response()
        }
        None => {
            // SPA fallback: any unknown path serves index.html
            match Frontend::get("index.html") {
                Some(content) => (
                    StatusCode::OK,
                    [(
                        header::CONTENT_TYPE,
                        "text/html; charset=utf-8".to_string(),
                    )],
                    content.data,
                )
                    .into_response(),
                None => StatusCode::NOT_FOUND.into_response(),
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let log_file = env::var("LOG_FILE").unwrap_or_else(|_| "./proxy.log".to_string());
    let db_url =
        env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://dashboard.db".to_string());

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

    // CORS: keep for API clients calling from other origins (e.g. curl, Postman)
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::any())
        .allow_methods(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any);

    // API routes + static frontend fallback
    let app = api::router(pool)
        .fallback(frontend_handler)
        .layer(cors);

    let addr: SocketAddr = env::var("LISTEN_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:4000".to_string())
        .parse()
        .expect("LISTEN_ADDR must be a valid socket address (e.g. 127.0.0.1:4000)");
    tracing::info!(%addr, "listening — open http://{addr} in your browser");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
