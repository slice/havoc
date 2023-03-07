use std::sync::Arc;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Extension, Router,
};
use tower_http::trace::{DefaultMakeSpan, TraceLayer};

use crate::db::Db;

pub struct State {
    pub db: Db,
}

type AppResult<T> = Result<T, AppError>;

async fn handler(Extension(state): Extension<Arc<State>>) -> AppResult<String> {
    let two = state
        .db
        .call(|conn| conn.query_row("SELECT 1 + 1", [], |row| row.get::<_, i32>(0)))
        .await??;

    Ok(format!("1 + 1 = {}", two))
}

struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("a fatal error occurred: {}", self.0),
        )
            .into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

pub fn create_router(state: Arc<State>) -> Router<axum::body::Body> {
    let api_v1_routes = Router::new()
        .route("/ping", get(|| async { "\"pong\"" }))
        .route("/ping/database", get(handler));

    let trace_layer =
        TraceLayer::new_for_http().make_span_with(DefaultMakeSpan::default().include_headers(true));
    Router::new()
        .nest("/api/v1", api_v1_routes)
        .layer(Extension(state))
        .layer(trace_layer)
}
