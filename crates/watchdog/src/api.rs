use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use sqlx::Row;
use tower_http::trace::{DefaultMakeSpan, TraceLayer};

use crate::db::Db;

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
}

type AppResult<T> = Result<T, AppError>;

async fn handler(State(state): State<AppState>) -> AppResult<String> {
    let two: i32 = sqlx::query("SELECT 1 + 1")
        .fetch_one(&state.db.pool)
        .await?
        .get(0);

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

pub fn create_router() -> Router<AppState> {
    let api_v1_routes = Router::new()
        .route("/ping", get(|| async { "\"pong\"" }))
        .route("/ping/database", get(handler));

    let trace_layer =
        TraceLayer::new_for_http().make_span_with(DefaultMakeSpan::default().include_headers(true));

    Router::new()
        .layer(trace_layer)
        .nest("/api/v1", api_v1_routes)
}
