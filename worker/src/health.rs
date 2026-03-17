use std::sync::Arc;

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;

use crate::AppState;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    index_size: usize,
    stdb_url: String,
    stdb_db: String,
}

async fn handle_health(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    let index = state.index.load();
    Json(HealthResponse {
        status: "ok",
        index_size: index.len(),
        stdb_url: state.stdb_url.clone(),
        stdb_db: state.stdb_db.clone(),
    })
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/health", get(handle_health))
}
