use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{get, post},
};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{CorsLayer, Any};

mod models;
mod routes;
mod state;

use state::AppState;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let state = Arc::new(AppState::new());

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route(
            "/upload/chunk",
            post(routes::upload::upload_chunk).layer(DefaultBodyLimit::max(8 * 1024 * 1024)),
        )
        .route("/upload/merge", post(routes::upload::merge_chunks))
        .route("/files", get(routes::upload::list_files))
        .route("/download/:filename", get(routes::upload::download_file))
        .route("/server-info", get(routes::info::server_info))
        .route("/ws", get(routes::ws::ws_handler))
        .layer(cors)
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!("DropWire backend listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
