use std::collections::HashMap;
use std::sync::Mutex;
use tokio::sync::broadcast;
use serde::Serialize;

use crate::discovery::DiscoveryState;

#[derive(Debug, Clone)]
pub struct StoredUser {
    pub username: String,
    pub nickname: String,
    pub avatar: Option<String>,
    pub password: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "reason")]
pub enum UploadStatus {
    Uploading,
    Merging,
    Completed,
    Failed(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct UploadProgress {
    pub upload_id: String,
    pub filename: String,
    pub total_chunks: usize,
    pub received_chunks: usize,
    pub status: UploadStatus,
}

pub struct UploadEntry {
    pub progress: UploadProgress,
    pub chunk_received: Vec<bool>,
}

pub struct AppState {
    pub uploads: Mutex<HashMap<String, UploadEntry>>,
    /// Broadcasts JSON-serialized `UploadProgress` to all WS clients.
    pub progress_tx: broadcast::Sender<String>,
    pub discovery: DiscoveryState,
    pub users: Mutex<HashMap<String, StoredUser>>,
}

impl AppState {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(256);
        Self {
            uploads: Mutex::new(HashMap::new()),
            progress_tx: tx,
            users: Mutex::new(HashMap::new()),
            discovery: DiscoveryState {
                self_info: std::sync::Arc::new(Mutex::new(crate::discovery::DeviceInfo {
                    id: String::new(),
                    name: String::new(),
                    avatar: None,
                    ip: String::new(),
                    port: 0,
                    last_seen: 0,
                    online: true,
                })),
                peers: std::sync::Arc::new(Mutex::new(HashMap::new())),
            },
        }
    }

    pub fn with_discovery(mut self, discovery: DiscoveryState) -> Self {
        self.discovery = discovery;
        self
    }
}
