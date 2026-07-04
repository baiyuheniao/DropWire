use axum::{Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::net::UdpSocket;
use tokio::time::interval;

const DISCOVERY_PORT: u16 = 3001;
const BEACON_INTERVAL_SECS: u64 = 2;
const PEER_TIMEOUT_SECS: u64 = 8;
const MDNS_SERVICE_TYPE: &str = "_dropwire._tcp.local.";
fn device_info_path(port: u16) -> String {
    format!("./device_{}.json", port)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    pub ip: String,
    pub port: u16,
    pub last_seen: u64,
    pub online: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeaconPayload {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    pub ip: String,
    pub port: u16,
}

#[derive(Clone)]
pub struct DiscoveryState {
    pub self_info: Arc<Mutex<DeviceInfo>>,
    pub peers: Arc<Mutex<HashMap<String, DeviceInfo>>>,
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn generate_uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn hostname() -> String {
    whoami::fallible::hostname().unwrap_or_else(|_| "DropWire Device".to_string())
}

fn load_or_create_self_info(ip: String, port: u16) -> DeviceInfo {
    let path = device_info_path(port);
    if let Ok(data) = std::fs::read_to_string(&path) {
        if let Ok(mut info) = serde_json::from_str::<DeviceInfo>(&data) {
            info.ip = ip;
            info.port = port;
            info.last_seen = now_secs();
            info.online = true;
            return info;
        }
    }
    DeviceInfo {
        id: generate_uuid(),
        name: hostname(),
        avatar: None,
        ip,
        port,
        last_seen: now_secs(),
        online: true,
    }
}

fn save_self_info(info: &DeviceInfo) {
    let path = device_info_path(info.port);
    let _ = std::fs::write(path, serde_json::to_string_pretty(info).unwrap_or_default());
}

pub fn init_discovery(port: u16) -> DiscoveryState {
    let ip = local_ip_address::local_ip()
        .map(|a| a.to_string())
        .unwrap_or_else(|_| "127.0.0.1".to_string());
    let self_info = load_or_create_self_info(ip, port);
    save_self_info(&self_info);

    DiscoveryState {
        self_info: Arc::new(Mutex::new(self_info)),
        peers: Arc::new(Mutex::new(HashMap::new())),
    }
}

fn create_discovery_socket(bind_addr: SocketAddr) -> std::net::UdpSocket {
    let domain = if bind_addr.is_ipv4() {
        socket2::Domain::IPV4
    } else {
        socket2::Domain::IPV6
    };
    let socket = socket2::Socket::new(domain, socket2::Type::DGRAM, None).unwrap();
    socket.set_reuse_address(true).unwrap();
    #[cfg(unix)]
    socket.set_reuse_port(true).unwrap();
    socket.set_nonblocking(true).unwrap();
    socket.bind(&bind_addr.into()).unwrap();
    socket.into()
}

pub async fn run_discovery(state: DiscoveryState) {
    let bind_addr: SocketAddr = format!("0.0.0.0:{}", DISCOVERY_PORT).parse().unwrap();

    let std_socket = create_discovery_socket(bind_addr);
    std_socket.set_broadcast(true).unwrap();
    let socket = Arc::new(UdpSocket::from_std(std_socket).unwrap());

    let broadcast_addr: SocketAddr = format!("255.255.255.255:{}", DISCOVERY_PORT).parse().unwrap();

    // Beacon sender
    let sender_socket = socket.clone();
    let sender_state = state.clone();
    tokio::spawn(async move {
        let mut tick = interval(Duration::from_secs(BEACON_INTERVAL_SECS));
        loop {
            tick.tick().await;
            let info = {
                let info = sender_state.self_info.lock().unwrap();
                BeaconPayload {
                    id: info.id.clone(),
                    name: info.name.clone(),
                    avatar: info.avatar.clone(),
                    ip: info.ip.clone(),
                    port: info.port,
                }
            };
            if let Ok(payload) = serde_json::to_vec(&info) {
                let _ = sender_socket.send_to(&payload, broadcast_addr).await;
            }
        }
    });

    // Beacon receiver
    let mut buf = [0u8; 1024];
    loop {
        match socket.recv_from(&mut buf).await {
            Ok((len, src)) => {
                if let Ok(payload) = serde_json::from_slice::<BeaconPayload>(&buf[..len]) {
                    let self_id = state.self_info.lock().unwrap().id.clone();
                    if payload.id == self_id {
                        continue;
                    }
                    let mut peers = state.peers.lock().unwrap();
                    let ip = if payload.ip.is_empty() || payload.ip.starts_with("127.") {
                        src.ip().to_string()
                    } else {
                        payload.ip
                    };
                    peers.insert(
                        payload.id.clone(),
                        DeviceInfo {
                            id: payload.id,
                            name: payload.name,
                            avatar: payload.avatar,
                            ip,
                            port: payload.port,
                            last_seen: now_secs(),
                            online: true,
                        },
                    );
                }
            }
            Err(_) => continue,
        }
    }
}

pub fn cleanup_offline_peers(state: &DiscoveryState) {
    let now = now_secs();
    let mut peers = state.peers.lock().unwrap();
    peers.retain(|_, peer| {
        peer.online = now - peer.last_seen <= PEER_TIMEOUT_SECS;
        peer.online
    });
}

pub async fn list_devices(State(state): axum::extract::State<Arc<crate::state::AppState>>) -> Result<Json<Vec<DeviceInfo>>, StatusCode> {
    cleanup_offline_peers(&state.discovery);
    let peers: Vec<DeviceInfo> = state.discovery.peers.lock().unwrap().values().cloned().collect();
    Ok(Json(peers))
}

pub async fn get_self_device(State(state): axum::extract::State<Arc<crate::state::AppState>>) -> Result<Json<DeviceInfo>, StatusCode> {
    let info = state.discovery.self_info.lock().unwrap().clone();
    Ok(Json(info))
}

#[derive(Debug, Deserialize)]
pub struct UpdateDeviceRequest {
    pub name: Option<String>,
    pub avatar: Option<String>,
}

pub async fn update_self_device(
    State(state): axum::extract::State<Arc<crate::state::AppState>>,
    Json(req): Json<UpdateDeviceRequest>,
) -> Result<Json<DeviceInfo>, StatusCode> {
    let mut info = state.discovery.self_info.lock().unwrap();
    if let Some(name) = req.name {
        info.name = name;
    }
    if let Some(avatar) = req.avatar {
        info.avatar = Some(avatar);
    }
    save_self_info(&*info);
    Ok(Json(info.clone()))
}

// ------------------------------------------------------------------
// mDNS discovery (complements UDP broadcast on networks that support it)
// ------------------------------------------------------------------

fn parse_ip_or_unspecified(ip: &str) -> IpAddr {
    ip.parse().unwrap_or(IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED))
}

pub async fn run_mdns_discovery(state: DiscoveryState) {
    let mdns = match mdns_sd::ServiceDaemon::new() {
        Ok(d) => d,
        Err(e) => {
            tracing::warn!("Failed to create mDNS daemon: {}", e);
            return;
        }
    };

    // Register ourselves as a DropWire service.
    let (id, name, avatar, ip, port) = {
        let info = state.self_info.lock().unwrap();
        (
            info.id.clone(),
            info.name.clone(),
            info.avatar.clone(),
            parse_ip_or_unspecified(&info.ip),
            info.port,
        )
    };

    let instance_name = id.clone();
    let host_name = format!("dropwire-{}.local.", id);
    let mut props: Vec<(&str, String)> = vec![
        ("id", id.clone()),
        ("name", name),
        ("port", port.to_string()),
    ];
    if let Some(avatar) = avatar {
        props.push(("avatar", avatar));
    }
    let prop_refs: Vec<(&str, &str)> = props.iter().map(|(k, v)| (*k, v.as_str())).collect();

    match mdns_sd::ServiceInfo::new(
        MDNS_SERVICE_TYPE,
        &instance_name,
        &host_name,
        ip,
        port,
        &prop_refs[..],
    ) {
        Ok(service_info) => {
            if let Err(e) = mdns.register(service_info) {
                tracing::warn!("Failed to register mDNS service: {}", e);
                return;
            }
        }
        Err(e) => {
            tracing::warn!("Failed to create mDNS service info: {}", e);
            return;
        }
    }

    // Browse for other DropWire services.
    let receiver = match mdns.browse(MDNS_SERVICE_TYPE) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("Failed to browse mDNS services: {}", e);
            return;
        }
    };

    let self_id = state.self_info.lock().unwrap().id.clone();
    let peers = state.peers.clone();

    tokio::task::spawn_blocking(move || {
        while let Ok(event) = receiver.recv() {
            match event {
                mdns_sd::ServiceEvent::ServiceResolved(info) => {
                    let props = info.get_properties();
                    let peer_id = props
                        .get_property_val_str("id")
                        .unwrap_or_default()
                        .to_string();
                    if peer_id.is_empty() || peer_id == self_id {
                        continue;
                    }

                    let peer_name = props
                        .get_property_val_str("name")
                        .unwrap_or("DropWire Device")
                        .to_string();
                    let peer_avatar = props.get_property_val_str("avatar").map(|s| s.to_string());
                    let peer_port = props
                        .get_property_val_str("port")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or_else(|| info.get_port());

                    // Prefer resolved addresses; fall back to the advertised hostname.
                    let peer_ip = info
                        .get_addresses()
                        .iter()
                        .next()
                        .map(|addr| addr.to_string())
                        .unwrap_or_else(|| info.get_hostname().to_string());

                    let mut map = peers.lock().unwrap();
                    map.insert(
                        peer_id.clone(),
                        DeviceInfo {
                            id: peer_id,
                            name: peer_name,
                            avatar: peer_avatar,
                            ip: peer_ip,
                            port: peer_port,
                            last_seen: now_secs(),
                            online: true,
                        },
                    );
                }
                mdns_sd::ServiceEvent::ServiceRemoved(_, fullname) => {
                    // fullname format: "<instance>._dropwire._tcp.local."
                    let suffix = format!(".{}", MDNS_SERVICE_TYPE);
                    let instance = fullname.trim_end_matches(&suffix);
                    let mut map = peers.lock().unwrap();
                    map.remove(instance);
                }
                _ => {}
            }
        }
    });
}
