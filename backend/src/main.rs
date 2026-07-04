use axum::{
    Router,
    extract::DefaultBodyLimit,
    http::Method,
    middleware,
    routing::{get, post},
};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{AllowOrigin, CorsLayer};

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

    let allowed_origins: Vec<String> = std::env::var("ALLOWED_ORIGINS")
        .ok()
        .map(|s| s.split(',').map(|o| o.trim().to_string()).collect())
        .unwrap_or_default();

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

    let allow_origin = if allowed_origins.is_empty() {
        AllowOrigin::any()
    } else {
        let origins: Vec<_> = allowed_origins
            .iter()
            .map(|o| o.parse().expect("invalid ALLOWED_ORIGINS"))
            .collect();
        AllowOrigin::list(origins)
    };

    let cors = CorsLayer::new()
        .allow_origin(allow_origin)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
        ]);

    let protected = Router::new()
        .route(
            "/upload/chunk",
            post(routes::upload::upload_chunk).layer(DefaultBodyLimit::max(8 * 1024 * 1024)),
        )
        .route("/upload/status/:upload_id", get(routes::upload::get_upload_status))
        .route("/upload/merge", post(routes::upload::merge_chunks))
        .route("/files", get(routes::upload::list_files))
        .route("/files/received", post(routes::upload::mark_file_received))
        .route("/download/*path", get(routes::upload::download_file))
        .route("/device", get(discovery::get_self_device).post(discovery::update_self_device))
        .route("/devices", get(discovery::list_devices))
        .route("/auth/profile", post(routes::auth::update_profile))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            routes::auth_middleware::require_auth,
        ));

    let public = Router::new()
        .route("/server-info", get(routes::info::server_info))
        .route("/auth/register", post(routes::auth::register))
        .route("/auth/login", post(routes::auth::login))
        .route("/ws", get(routes::ws::ws_handler));

    let app = protected
        .merge(public)
        .layer(cors)
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("DropWire backend listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
