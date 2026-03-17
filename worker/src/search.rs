use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::index::SearchQuery;
use crate::AppState;

#[derive(Deserialize)]
pub struct SearchRequest {
    /// Text query string.
    #[serde(default)]
    pub query: Option<String>,
    /// Numeric range filters: { "field": [min, max] }
    #[serde(default)]
    pub filters: HashMap<String, [f64; 2]>,
    /// Geographic bounding box: [min_lat, max_lat, min_lon, max_lon]
    #[serde(default)]
    pub bbox: Option<[f64; 4]>,
    /// Max results to return (default 20, max 100).
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Offset for pagination (default 0).
    #[serde(default)]
    pub offset: usize,
}

fn default_limit() -> usize {
    20
}

#[derive(Serialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResultItem>,
    pub total_scanned: usize,
}

#[derive(Serialize)]
pub struct SearchResultItem {
    pub id: u64,
    pub score: f64,
}

async fn handle_search(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SearchRequest>,
) -> Json<SearchResponse> {
    let index = state.index.load();
    let total_scanned = index.len();

    let numeric_filters: HashMap<String, (f64, f64)> = req
        .filters
        .into_iter()
        .map(|(k, [min, max])| (k, (min, max)))
        .collect();

    let bbox = req.bbox.map(|b| (b[0], b[1], b[2], b[3]));

    let query = SearchQuery {
        text: req.query,
        numeric_filters,
        bbox,
        limit: req.limit,
        offset: req.offset,
    };

    let results = index
        .search(&query)
        .into_iter()
        .map(|r| SearchResultItem {
            id: r.id,
            score: r.score,
        })
        .collect();

    Json(SearchResponse {
        results,
        total_scanned,
    })
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/search", post(handle_search))
}
