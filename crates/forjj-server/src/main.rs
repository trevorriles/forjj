//! Forjj Server
//!
//! A native jj forge server providing repository hosting, push/fetch over SSH,
//! and a REST API for repository management.

use anyhow::Result;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod api;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "forjj=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Forjj - A native jj forge");
    info!("Version: 0.1.0-dev");

    // Start HTTP server
    let app = api::create_router();

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    info!("Listening on http://0.0.0.0:3000");

    axum::serve(listener, app).await?;

    Ok(())
}
