mod health;
mod index;
mod search;
mod upload;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use arc_swap::ArcSwap;
use axum::Router;
use tokio::signal;
use tower_http::cors::CorsLayer;
use tracing::{info, warn};

use index::SearchIndex;

/// Shared application state available to all handlers.
pub struct AppState {
    pub index: ArcSwap<SearchIndex>,
    pub stdb_url: String,
    pub stdb_db: String,
    pub upload_dir: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "worker=info".into()),
        )
        .init();

    let stdb_url = std::env::var("STDB_URL").unwrap_or_else(|_| "http://localhost:3000".into());
    let stdb_db = std::env::var("STDB_DB").unwrap_or_else(|_| "app".into());
    let upload_dir = std::env::var("UPLOAD_DIR").unwrap_or_else(|_| "./uploads".into());
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    let sync_interval: u64 = std::env::var("SYNC_INTERVAL_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);

    // Ensure upload directory exists
    std::fs::create_dir_all(&upload_dir).expect("Failed to create upload directory");

    let state = Arc::new(AppState {
        index: ArcSwap::new(Arc::new(SearchIndex::new())),
        stdb_url: stdb_url.clone(),
        stdb_db: stdb_db.clone(),
        upload_dir: upload_dir.clone(),
    });

    // Spawn background sync task
    let sync_state = state.clone();
    tokio::spawn(async move {
        let mut cursor: u64 = 0;
        loop {
            match sync_index(&sync_state, cursor).await {
                Ok(new_cursor) => {
                    if new_cursor > cursor {
                        info!(
                            "Index synced: cursor {} -> {}, {} items",
                            cursor,
                            new_cursor,
                            sync_state.index.load().len()
                        );
                    }
                    cursor = new_cursor;
                }
                Err(e) => warn!("Sync failed: {}", e),
            }
            tokio::time::sleep(Duration::from_secs(sync_interval)).await;
        }
    });

    let app = Router::new()
        .merge(health::routes())
        .merge(search::routes())
        .merge(upload::routes())
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("Worker listening on {}", addr);
    info!("SpacetimeDB: {}/v1/database/{}", stdb_url, stdb_db);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

/// Sync index from SpacetimeDB. Returns the new cursor (max id seen).
async fn sync_index(state: &Arc<AppState>, cursor: u64) -> Result<u64, String> {
    let url = format!(
        "{}/v1/database/{}/sql",
        state.stdb_url, state.stdb_db
    );

    // Fetch rows with id > cursor (incremental sync)
    let sql = format!("SELECT * FROM post WHERE id > {}", cursor);

    let response = ureq::post(&url)
        .header("Content-Type", "text/plain")
        .send_string(&sql)
        .map_err(|e| format!("HTTP error: {}", e))?;

    let body: serde_json::Value = response
        .body_mut()
        .read_json()
        .map_err(|e| format!("JSON parse error: {}", e))?;

    let rows = body
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|r| r.get("rows"))
        .and_then(|r| r.as_array());

    let rows = match rows {
        Some(r) => r,
        None => return Ok(cursor), // No new rows
    };

    if rows.is_empty() {
        return Ok(cursor);
    }

    // Build new items from rows
    let mut new_items: Vec<index::IndexItem> = Vec::new();
    let mut max_id = cursor;

    for row in rows {
        let arr = match row.as_array() {
            Some(a) => a,
            None => continue,
        };
        // Post columns: id, owner, title, body, status, created_at, updated_at
        if arr.len() < 5 {
            continue;
        }
        let id = arr[0].as_u64().unwrap_or(0);
        let title = arr[2].as_str().unwrap_or("");
        let body = arr[3].as_str().unwrap_or("");
        let status = arr[4].as_u64().unwrap_or(0);

        // Only index published posts
        if status != 1 {
            continue;
        }

        if id > max_id {
            max_id = id;
        }

        new_items.push(index::IndexItem {
            id,
            text_fields: vec![title.to_string(), body.to_string()],
            numeric_fields: std::collections::HashMap::new(),
            lat: None,
            lon: None,
        });
    }

    if !new_items.is_empty() {
        // Load current index, merge, swap
        let current = state.index.load();
        let mut merged = (**current).clone();
        for item in new_items {
            merged.insert(item);
        }
        state.index.store(Arc::new(merged));
    }

    Ok(max_id)
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("Failed to listen for ctrl+c");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to listen for SIGTERM")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => info!("Received ctrl+c, shutting down"),
        _ = terminate => info!("Received SIGTERM, shutting down"),
    }
}
