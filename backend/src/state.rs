use std::collections::HashMap;
use tokio::sync::Mutex;
use tokio::sync::broadcast;
use serde::{Deserialize, Serialize};

use crate::discovery::DiscoveryState;

const USERS_FILE: &str = "./users.json";
const SESSIONS_FILE: &str = "./sessions.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredUser {
    pub username: String,
    pub nickname: String,
    pub avatar: Option<String>,
    pub password_hash: String,
}

/// Load a `HashMap<String, T>` previously written by `save_json_map`. Missing
/// or unparsable files just mean "nothing persisted yet" (e.g. first run).
fn load_json_map<T: serde::de::DeserializeOwned>(path: &str) -> HashMap<String, T> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|data| serde_json::from_str(&data).ok())
        .unwrap_or_default()
}

async fn save_json_map<T: Serialize>(path: &str, map: &HashMap<String, T>) {
    if let Ok(data) = serde_json::to_vec_pretty(map) {
        if let Err(e) = tokio::fs::write(path, data).await {
            tracing::warn!("failed to persist {}: {}", path, e);
        }
    }
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
    /// SHA-256 hex of each chunk, validated on merge.
    pub chunk_hashes: Vec<Option<String>>,
}

pub struct AppState {
    pub uploads: Mutex<HashMap<String, UploadEntry>>,
    /// Broadcasts JSON-serialized `UploadProgress` to all WS clients.
    pub progress_tx: broadcast::Sender<String>,
    pub discovery: DiscoveryState,
    pub users: Mutex<HashMap<String, StoredUser>>,
    /// Maps bearer token to username.
    pub sessions: Mutex<HashMap<String, String>>,
}

impl AppState {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(256);
        Self {
            uploads: Mutex::new(HashMap::new()),
            progress_tx: tx,
            users: Mutex::new(load_json_map(USERS_FILE)),
            sessions: Mutex::new(load_json_map(SESSIONS_FILE)),
            discovery: DiscoveryState {
                self_info: std::sync::Arc::new(std::sync::Mutex::new(crate::discovery::DeviceInfo {
                    id: String::new(),
                    name: String::new(),
                    avatar: None,
                    ip: String::new(),
                    port: 0,
                    last_seen: 0,
                    online: true,
                })),
                peers: std::sync::Arc::new(std::sync::Mutex::new(HashMap::new())),
            },
        }
    }

    pub fn with_discovery(mut self, discovery: DiscoveryState) -> Self {
        self.discovery = discovery;
        self
    }

    /// Load users from an arbitrary path, bypassing the fixed `USERS_FILE`
    /// constant. Used by tests to point at a scratch file instead of the
    /// real one.
    #[cfg(test)]
    pub fn load_users_from(path: &str) -> HashMap<String, StoredUser> {
        load_json_map(path)
    }

    #[cfg(test)]
    pub async fn save_users_to(path: &str, users: &HashMap<String, StoredUser>) {
        save_json_map(path, users).await;
    }

    /// Persist the current user map so accounts survive a restart.
    ///
    /// No-op under `cargo test`: unit tests construct throwaway `AppState`s
    /// via `register`/`login`, and writing those to the real `USERS_FILE`
    /// path would race across parallel test threads and pollute the working
    /// directory with test data.
    pub async fn save_users(&self) {
        if cfg!(test) {
            return;
        }
        let users = self.users.lock().await;
        save_json_map(USERS_FILE, &*users).await;
    }

    /// Persist the current session map so logins survive a restart. See
    /// `save_users` for why this is a no-op under `cargo test`.
    pub async fn save_sessions(&self) {
        if cfg!(test) {
            return;
        }
        let sessions = self.sessions.lock().await;
        save_json_map(SESSIONS_FILE, &*sessions).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn users_round_trip_through_disk() {
        let path = std::env::temp_dir().join(format!(
            "dropwire-test-users-{}.json",
            uuid::Uuid::new_v4()
        ));
        let path = path.to_str().unwrap();

        let mut users = HashMap::new();
        users.insert(
            "alice".to_string(),
            StoredUser {
                username: "alice".to_string(),
                nickname: "Alice".to_string(),
                avatar: None,
                password_hash: "hash".to_string(),
            },
        );

        AppState::save_users_to(path, &users).await;
        let loaded = AppState::load_users_from(path);

        assert_eq!(loaded.get("alice").unwrap().nickname, "Alice");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn missing_file_loads_as_empty() {
        let loaded: HashMap<String, StoredUser> =
            load_json_map("./this-file-does-not-exist.json");
        assert!(loaded.is_empty());
    }
}
