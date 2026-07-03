use axum::{Json, http::StatusCode};
use serde::Serialize;

#[derive(Serialize)]
pub struct ServerInfo {
    pub ip: String,
    pub port: u16,
    pub download_url_prefix: String,
}

pub async fn server_info() -> Result<Json<ServerInfo>, StatusCode> {
    let ip = local_ip_address::local_ip()
        .map(|addr| addr.to_string())
        .unwrap_or_else(|_| "127.0.0.1".to_string());
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    Ok(Json(ServerInfo {
        download_url_prefix: format!("http://{}:{}/download/", ip, port),
        ip,
        port,
    }))
}
