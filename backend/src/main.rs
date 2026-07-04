use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{get, post},
};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{CorsLayer, Any};

mod discovery;
mod models;
mod routes;
mod state;

use discovery::{init_discovery, run_discovery, run_mdns_discovery};
use state::AppState;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    let discovery = init_discovery(port);

    let discovery_for_udp = discovery.clone();
    tokio::spawn(async move {
        run_discovery(discovery_for_udp).await;
    });

    let discovery_for_mdns = discovery.clone();
    tokio::spawn(async move {
        run_mdns_discovery(discovery_for_mdns).await;
    });

    let state = Arc::new(AppState::new().with_discovery(discovery));

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route(
            "/upload/chunk",
            post(routes::upload::upload_chunk).layer(DefaultBodyLimit::max(8 * 1024 * 1024)),
        )
        .route("/upload/status/:upload_id", get(routes::upload::get_upload_status))
        .route("/upload/merge", post(routes::upload::merge_chunks))
        .route("/files", get(routes::upload::list_files))
        .route("/files/received", post(routes::upload::mark_file_received))
        .route("/download/*path", get(routes::upload::download_file))
        .route("/server-info", get(routes::info::server_info))
        .route("/auth/register", post(routes::auth::register))
        .route("/auth/login", post(routes::auth::login))
        .route("/auth/profile", post(routes::auth::update_profile))
        .route("/devices", get(discovery::list_devices))
        .route("/device", get(discovery::get_self_device).post(discovery::update_self_device))
        .route("/ws", get(routes::ws::ws_handler))
        .layer(cors)
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("DropWire backend listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
