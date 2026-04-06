use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::db;

type Pool = Arc<SqlitePool>;

pub fn router(pool: Pool) -> Router {
    Router::new()
        .route("/api/queries/recent", get(recent_queries))
        .route("/api/queries/top-domains", get(top_domains))
        .route("/api/stats", get(stats))
        .with_state(pool)
}

#[derive(Deserialize)]
struct LimitParam {
    limit: Option<i64>,
}

async fn recent_queries(
    State(pool): State<Pool>,
    Query(params): Query<LimitParam>,
) -> Response {
    let limit = params.limit.unwrap_or(50);
    match db::get_recent_queries(&pool, limit).await {
        Ok(rows) => Json(rows).into_response(),
        Err(e) => {
            tracing::error!(error = %e, "recent_queries failed");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn top_domains(
    State(pool): State<Pool>,
    Query(params): Query<LimitParam>,
) -> Response {
    let limit = params.limit.unwrap_or(10);
    match db::get_top_domains(&pool, limit).await {
        Ok(rows) => Json(rows).into_response(),
        Err(e) => {
            tracing::error!(error = %e, "top_domains failed");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn stats(State(pool): State<Pool>) -> Response {
    match db::get_stats(&pool).await {
        Ok(s) => Json(s).into_response(),
        Err(e) => {
            tracing::error!(error = %e, "stats failed");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
