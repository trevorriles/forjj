//! REST API handlers for Forjj.

use axum::{Json, Router, extract::Path, http::StatusCode, response::IntoResponse, routing::get};
use serde::{Deserialize, Serialize};
use tower_http::trace::TraceLayer;

/// Create the API router.
pub fn create_router() -> Router {
    Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/api/v1/repos", get(list_repos).post(create_repo))
        .route(
            "/api/v1/repos/{owner}/{name}",
            get(get_repo).delete(delete_repo),
        )
        .layer(TraceLayer::new_for_http())
}

/// Root handler - basic info.
async fn root() -> impl IntoResponse {
    Json(serde_json::json!({
        "name": "forjj",
        "version": "0.1.0-dev",
        "description": "A native jj forge"
    }))
}

/// Health check endpoint.
async fn health() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy"
    }))
}

/// Repository info response.
#[derive(Debug, Serialize)]
struct RepoResponse {
    owner: String,
    name: String,
    full_name: String,
    backend: String,
}

/// Create repository request.
#[derive(Debug, Deserialize)]
struct CreateRepoRequest {
    owner: String,
    name: String,
    #[allow(dead_code)]
    description: Option<String>,
}

/// List all repositories.
async fn list_repos() -> impl IntoResponse {
    // TODO: Implement actual repository listing
    Json(serde_json::json!({
        "repositories": []
    }))
}

/// Create a new repository.
async fn create_repo(Json(payload): Json<CreateRepoRequest>) -> impl IntoResponse {
    // TODO: Implement actual repository creation
    (
        StatusCode::CREATED,
        Json(RepoResponse {
            owner: payload.owner.clone(),
            name: payload.name.clone(),
            full_name: format!("{}/{}", payload.owner, payload.name),
            backend: "simple".to_string(),
        }),
    )
}

/// Get repository info.
async fn get_repo(Path((owner, name)): Path<(String, String)>) -> impl IntoResponse {
    // TODO: Implement actual repository lookup
    Json(RepoResponse {
        owner: owner.clone(),
        name: name.clone(),
        full_name: format!("{}/{}", owner, name),
        backend: "simple".to_string(),
    })
}

/// Delete a repository.
async fn delete_repo(Path((owner, name)): Path<(String, String)>) -> impl IntoResponse {
    // TODO: Implement actual repository deletion
    tracing::info!("Delete repository: {}/{}", owner, name);
    StatusCode::NO_CONTENT
}
