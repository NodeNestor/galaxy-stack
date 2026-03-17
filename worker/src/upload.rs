use std::path::PathBuf;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Multipart, Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Serialize;
use tracing::{info, warn};

use crate::AppState;

const MAX_FILE_SIZE: usize = 10 * 1024 * 1024; // 10 MB

#[derive(Serialize)]
struct UploadResponse {
    path: String,
    size: usize,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

async fn handle_upload(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Response {
    while let Ok(Some(field)) = multipart.next_field().await {
        let filename = match field.file_name() {
            Some(name) => sanitize_filename(name),
            None => continue,
        };

        if filename.is_empty() {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "Invalid filename".into(),
                }),
            )
                .into_response();
        }

        let data = match field.bytes().await {
            Ok(d) => d,
            Err(e) => {
                warn!("Failed to read upload: {}", e);
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: "Failed to read file data".into(),
                    }),
                )
                    .into_response();
            }
        };

        if data.len() > MAX_FILE_SIZE {
            return (
                StatusCode::PAYLOAD_TOO_LARGE,
                Json(ErrorResponse {
                    error: format!("File too large (max {} MB)", MAX_FILE_SIZE / 1024 / 1024),
                }),
            )
                .into_response();
        }

        let path = PathBuf::from(&state.upload_dir).join(&filename);

        if let Err(e) = tokio::fs::write(&path, &data).await {
            warn!("Failed to write file: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to save file".into(),
                }),
            )
                .into_response();
        }

        info!("Uploaded: {} ({} bytes)", filename, data.len());

        return (
            StatusCode::OK,
            Json(UploadResponse {
                path: filename,
                size: data.len(),
            }),
        )
            .into_response();
    }

    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: "No file in request".into(),
        }),
    )
        .into_response()
}

async fn handle_serve_file(
    State(state): State<Arc<AppState>>,
    Path(filename): Path<String>,
) -> Response {
    let filename = sanitize_filename(&filename);
    if filename.is_empty() {
        return StatusCode::BAD_REQUEST.into_response();
    }

    let path = PathBuf::from(&state.upload_dir).join(&filename);

    match tokio::fs::read(&path).await {
        Ok(data) => {
            let content_type = mime_from_ext(&filename);
            Response::builder()
                .header(header::CONTENT_TYPE, content_type)
                .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
                .body(Body::from(data))
                .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
        }
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

/// Remove path traversal characters and unsafe characters from filenames.
fn sanitize_filename(name: &str) -> String {
    let name = name
        .replace('\\', "/")
        .split('/')
        .last()
        .unwrap_or("")
        .to_string();

    name.chars()
        .filter(|c| c.is_alphanumeric() || *c == '.' || *c == '-' || *c == '_')
        .collect::<String>()
}

/// Basic MIME type detection from file extension.
fn mime_from_ext(filename: &str) -> &'static str {
    match filename.rsplit('.').next().map(|e| e.to_lowercase()) {
        Some(ref ext) => match ext.as_str() {
            "jpg" | "jpeg" => "image/jpeg",
            "png" => "image/png",
            "gif" => "image/gif",
            "webp" => "image/webp",
            "svg" => "image/svg+xml",
            "pdf" => "application/pdf",
            "json" => "application/json",
            "txt" => "text/plain",
            "html" => "text/html",
            "css" => "text/css",
            "js" => "application/javascript",
            _ => "application/octet-stream",
        },
        None => "application/octet-stream",
    }
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/upload", post(handle_upload))
        .route("/files/{filename}", get(handle_serve_file))
}
