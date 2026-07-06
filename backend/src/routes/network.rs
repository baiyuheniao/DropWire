use axum::{
    body::Body,
    extract::{Query, State},
    http::{header, StatusCode},
    response::Response,
    Json,
};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::state::AppState;

const PUBLIC_SPEED_URL: &str = "https://speed.cloudflare.com/__down";

#[derive(Serialize)]
pub struct SpeedResult {
    pub bytes: usize,
    pub duration_ms: u64,
    pub speed_mbps: f64,
}

#[derive(Serialize)]
pub struct NetworkStatus {
    pub self_id: String,
    pub self_name: String,
    pub self_ip: String,
    pub port: u16,
    pub peer_count: usize,
    pub peers: Vec<crate::discovery::DeviceInfo>,
    pub public_ip: Option<String>,
    pub has_public_internet: bool,
    pub summary: String,
}

#[derive(Deserialize)]
pub struct DownloadSizeQuery {
    #[serde(default = "default_size_mb")]
    pub size_mb: usize,
}

fn default_size_mb() -> usize {
    10
}

/// Receive arbitrary bytes and report how long it took.
/// Used by the frontend to measure upload speed to the backend.
pub async fn speed_test_upload(body: Body) -> Result<Json<SpeedResult>, StatusCode> {
    let start = Instant::now();
    let mut stream = body.into_data_stream();
    let mut bytes = 0usize;

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(data) => bytes += data.len(),
            Err(_) => return Err(StatusCode::BAD_REQUEST),
        }
    }

    let duration = start.elapsed();
    let duration_ms = duration.as_millis().max(1) as u64;
    let speed_mbps = bytes as f64 * 8.0 / 1_000_000.0 / (duration_ms as f64 / 1000.0);

    Ok(Json(SpeedResult {
        bytes,
        duration_ms,
        speed_mbps,
    }))
}

/// Generate a payload of `size_mb` megabytes for download speed tests.
pub async fn speed_test_download(
    Query(query): Query<DownloadSizeQuery>,
) -> Result<Response, StatusCode> {
    let size_mb = query.size_mb.clamp(1, 100);
    let total = (size_mb * 1024 * 1024) as u64;

    // Stream the payload in 64 KiB chunks rather than allocating the whole
    // thing (up to 100 MB) in memory at once.
    const BUF_SIZE: u64 = 64 * 1024;
    let stream = futures::stream::unfold(total, |remaining| async move {
        if remaining == 0 {
            return None;
        }
        let n = remaining.min(BUF_SIZE);
        let chunk = bytes::Bytes::from(vec![0u8; n as usize]);
        Some((Ok::<_, std::io::Error>(chunk), remaining - n))
    });

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(header::CONTENT_LENGTH, total.to_string())
        .body(Body::from_stream(stream))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

/// Proxy a request to a public speed-test endpoint so the client can measure
/// "public" internet speed without worrying about CORS.
pub async fn speed_test_public(
    Query(query): Query<DownloadSizeQuery>,
) -> Result<Response, StatusCode> {
    let size = query.size_mb.clamp(1, 100) * 1024 * 1024;
    let url = format!("{}?bytes={}", PUBLIC_SPEED_URL, size);

    let client = match reqwest::Client::builder().timeout(Duration::from_secs(30)).build() {
        Ok(c) => c,
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    let res = match client.get(&url).send().await {
        Ok(r) => r,
        Err(_) => return Err(StatusCode::BAD_GATEWAY),
    };

    let status = StatusCode::from_u16(res.status().as_u16()).unwrap_or(StatusCode::OK);
    let bytes = match res.bytes().await {
        Ok(b) => b,
        Err(_) => return Err(StatusCode::BAD_GATEWAY),
    };

    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(header::CONTENT_LENGTH, bytes.len().to_string())
        .body(Body::from(bytes))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[derive(Serialize)]
pub struct LatencyResult {
    pub target: String,
    pub latency_ms: u64,
}

#[derive(Deserialize)]
pub struct LatencyQuery {
    pub target: String,
}

/// Extract the host portion of a `target` (with or without a scheme/port),
/// e.g. "http://192.168.1.5:3000" or "192.168.1.5:3000" -> "192.168.1.5".
fn extract_host(target: &str) -> &str {
    let without_scheme = target
        .trim_start_matches("http://")
        .trim_start_matches("https://");
    let authority = without_scheme.split('/').next().unwrap_or(without_scheme);
    authority.rsplit_once(':').map_or(authority, |(host, _)| host)
}

/// Look up a known device (ourselves or a discovered peer) whose IP matches
/// `host`, returning its trusted `(ip, port)` from server-side state.
///
/// Without this, `target` would be attacker-controlled input to a
/// server-side HTTP request — an unauthenticated SSRF primitive that lets any
/// caller make this backend probe arbitrary hosts (internal services, cloud
/// metadata endpoints, etc.) and read back whether/how fast they responded.
/// Critically, the request URL below is built entirely from this trusted
/// `(ip, port)` pair, never from the raw client-supplied `target` string —
/// otherwise a value like "&lt;known-ip&gt;:1@evil.example.com" would pass a
/// naive host check (the part before the last `:`) while a URL parser still
/// sends the actual request to `evil.example.com` (everything after `@` is
/// the authority's host, per URL syntax).
fn resolve_known_peer(state: &AppState, host: &str) -> Option<(String, u16)> {
    let self_info = state.discovery.self_info.lock().unwrap();
    if self_info.ip == host {
        return Some((self_info.ip.clone(), self_info.port));
    }
    drop(self_info);

    state
        .discovery
        .peers
        .lock()
        .unwrap()
        .values()
        .find(|peer| peer.ip == host)
        .map(|peer| (peer.ip.clone(), peer.port))
}

/// Measure latency to a known peer by hitting its `/server-info` endpoint.
pub async fn measure_latency(
    State(state): State<Arc<AppState>>,
    Query(query): Query<LatencyQuery>,
) -> Result<Json<LatencyResult>, StatusCode> {
    let host = extract_host(&query.target);
    let (ip, port) = match resolve_known_peer(&state, host) {
        Some(v) => v,
        None => return Err(StatusCode::FORBIDDEN),
    };
    let url = format!("http://{}:{}/server-info", ip, port);

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    let start = Instant::now();
    match client.get(&url).send().await {
        Ok(_) => Ok(Json(LatencyResult {
            target: query.target,
            latency_ms: start.elapsed().as_millis() as u64,
        })),
        Err(_) => Err(StatusCode::BAD_GATEWAY),
    }
}

/// Return a summary of the current network environment.
pub async fn network_status(
    State(state): State<Arc<AppState>>,
) -> Result<Json<NetworkStatus>, StatusCode> {
    let self_info = state.discovery.self_info.lock().unwrap().clone();

    crate::discovery::cleanup_offline_peers(&state.discovery);
    let peers: Vec<crate::discovery::DeviceInfo> = state
        .discovery
        .peers
        .lock()
        .unwrap()
        .values()
        .cloned()
        .collect();

    let public_ip = fetch_public_ip().await.ok();
    let has_public_internet = public_ip.is_some();

    let summary = if peers.is_empty() {
        if has_public_internet {
            "当前未在同一局域网发现其他 DropWire 设备，但公网可用。".to_string()
        } else {
            "当前未发现局域网设备，且无法访问公网，请检查网络连接。".to_string()
        }
    } else {
        format!(
            "局域网内发现 {} 台 DropWire 设备，{}。",
            peers.len(),
            if has_public_internet {
                "公网访问正常"
            } else {
                "公网访问受限"
            }
        )
    };

    Ok(Json(NetworkStatus {
        self_id: self_info.id,
        self_name: self_info.name,
        self_ip: self_info.ip,
        port: self_info.port,
        peer_count: peers.len(),
        peers,
        public_ip,
        has_public_internet,
        summary,
    }))
}

async fn fetch_public_ip() -> Result<String, reqwest::Error> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;
    let text = client
        .get("https://api.ipify.org?format=text")
        .send()
        .await?
        .text()
        .await?;
    Ok(text.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::DeviceInfo;

    #[test]
    fn extract_host_strips_scheme_port_and_path() {
        assert_eq!(extract_host("192.168.1.5"), "192.168.1.5");
        assert_eq!(extract_host("192.168.1.5:3000"), "192.168.1.5");
        assert_eq!(extract_host("http://192.168.1.5:3000"), "192.168.1.5");
        assert_eq!(extract_host("https://192.168.1.5:3000/"), "192.168.1.5");
        assert_eq!(extract_host("http://evil.example.com/attack"), "evil.example.com");
    }

    #[test]
    fn extract_host_can_be_fooled_by_userinfo_syntax() {
        // A naive rsplit_once(':') host extraction is fooled by URL userinfo
        // syntax ("user:pass@host") into treating a peer's IP as the host,
        // even though a URL parser sends the real request to the host after
        // '@'. This is exactly why resolve_known_peer must never let this
        // extracted string leak into the outgoing request URL - see below.
        assert_eq!(extract_host("192.168.1.5:1@evil.example.com"), "192.168.1.5");
    }

    fn state_with_peer(peer_ip: &str) -> Arc<AppState> {
        let state = Arc::new(AppState::new());
        state.discovery.peers.lock().unwrap().insert(
            "peer-1".to_string(),
            DeviceInfo {
                id: "peer-1".to_string(),
                name: "Peer".to_string(),
                avatar: None,
                ip: peer_ip.to_string(),
                port: 3000,
                last_seen: 0,
                online: true,
            },
        );
        state
    }

    #[test]
    fn known_peer_host_resolves_to_its_trusted_address() {
        let state = state_with_peer("192.168.1.5");
        assert_eq!(
            resolve_known_peer(&state, "192.168.1.5"),
            Some(("192.168.1.5".to_string(), 3000))
        );
    }

    #[test]
    fn arbitrary_host_is_rejected() {
        let state = state_with_peer("192.168.1.5");
        assert_eq!(resolve_known_peer(&state, "169.254.169.254"), None);
        assert_eq!(resolve_known_peer(&state, "evil.example.com"), None);
    }

    #[test]
    fn self_host_resolves_to_its_trusted_address() {
        let state = Arc::new(AppState::new());
        {
            let mut self_info = state.discovery.self_info.lock().unwrap();
            self_info.ip = "10.0.0.9".to_string();
            self_info.port = 4000;
        }
        assert_eq!(
            resolve_known_peer(&state, "10.0.0.9"),
            Some(("10.0.0.9".to_string(), 4000))
        );
    }

    #[test]
    fn resolved_address_is_never_influenced_by_the_raw_target_string() {
        // Even though extract_host mis-parses "192.168.1.5:1@evil.example.com"
        // as host "192.168.1.5" (see extract_host_can_be_fooled_by_userinfo_syntax),
        // resolve_known_peer only ever returns the peer's own stored ip/port -
        // it has no way to leak "evil.example.com" into the outgoing request,
        // because it never looks at the raw target string, only the matched
        // peer's trusted record.
        let state = state_with_peer("192.168.1.5");
        let host = extract_host("192.168.1.5:1@evil.example.com");
        let resolved = resolve_known_peer(&state, host);
        assert_eq!(resolved, Some(("192.168.1.5".to_string(), 3000)));
    }
}

