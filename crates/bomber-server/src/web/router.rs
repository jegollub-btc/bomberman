use std::net::SocketAddr;
use std::path::Path;

use anyhow::Result;
use axum::routing::get;
use axum::Router;
use tower_http::services::ServeDir;

use crate::runtime::Shared;

use super::{admin, placeholder, spectate};

pub async fn serve(addr: SocketAddr, static_dir: &str, shared: Shared) -> Result<()> {
    let mut app = Router::new()
        .route("/ws/spectate", get(spectate::handler))
        .route("/ws/admin", get(admin::handler));

    // Serve the built visualizer if somebody has built one. If not, say so on
    // the page rather than returning a bare 404 that looks like a broken
    // server.
    if Path::new(static_dir).is_dir() {
        tracing::info!(dir = static_dir, "serving visualizer");
        app = app.fallback_service(
            ServeDir::new(static_dir).append_index_html_on_directories(true),
        );
    } else {
        tracing::info!(
            dir = static_dir,
            "no visualizer build found; serving the placeholder page"
        );
        app = app.fallback(placeholder::page);
    }

    let app = app.with_state(shared);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, "web interface on http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}
