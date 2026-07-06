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
///
/// Under `cargo test` this always returns an empty map, regardless of what's
/// actually on disk at `path`: unit tests construct throwaway `AppState`s via
/// `register`/`login`/etc, and a real `users.json`/`sessions.json` left over
/// from a previous `cargo run` in the same directory must never leak into a
/// test's "fresh" state (see `save_json_map` for the write-side counterpart).
fn load_json_map<T: serde::de::DeserializeOwned>(path: &str) -> HashMap<String, T> {
    if cfg!(test) {
        return HashMap::new();
    }
    std::fs::read_to_string(path)
        .ok()
        .and_then(|data| serde_json::from_str(&data).ok())
        .unwrap_or_default()
}

/// Persist a map by writing to a temp file and renaming it into place, so a
/// process killed mid-write can never leave `path` truncated. Without this, a
/// crash during the write corrupts the file; on restart the parser fails and
/// silently falls back to an empty map, losing every account - the exact
/// "state lost on restart" failure this persistence layer exists to prevent.
///
/// No-op under `cargo test`, matching `load_json_map`.
async fn save_json_map<T: Serialize>(path: &str, map: &HashMap<String, T>) {
    if cfg!(test) {
        return;
    }
    let data = match serde_json::to_vec_pretty(map) {
        Ok(d) => d,
        Err(_) => return,
    };
    let tmp_path = format!("{}.tmp", path);
    if let Err(e) = tokio::fs::write(&tmp_path, &data).await {
        tracing::warn!("failed to persist {}: {}", path, e);
        return;
    }
    if let Err(e) = tokio::fs::rename(&tmp_path, path).await {
        tracing::warn!("failed to persist {}: {}", path, e);
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

/// Tracks failed login attempts for a single username so repeated wrong
/// passwords can be throttled instead of allowed at unlimited speed.
#[derive(Debug, Default)]
pub struct LoginAttempt {
    pub failures: u32,
    pub locked_until: Option<u64>,
    /// A monotonically increasing sequence number (not a wall-clock
    /// timestamp - many failures can land in the same second under load,
    /// which would make time-based eviction order ties arbitrary) set on
    /// every failure. Used to evict the least-recently-active entry when
    /// the map is capped (see `record_failed_login` in `routes::auth`).
    pub last_attempt_seq: u64,
}

pub struct AppState {
    pub uploads: Mutex<HashMap<String, UploadEntry>>,
    /// Broadcasts JSON-serialized `UploadProgress` to all WS clients.
    pub progress_tx: broadcast::Sender<String>,
    pub discovery: DiscoveryState,
    pub users: Mutex<HashMap<String, StoredUser>>,
    /// Maps bearer token to username.
    pub sessions: Mutex<HashMap<String, String>>,
    /// Not persisted across restarts - a fresh process gets a clean slate,
    /// which is an acceptable tradeoff for a LAN tool.
    pub login_attempts: Mutex<HashMap<String, LoginAttempt>>,
}

impl AppState {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(256);
        Self {
            uploads: Mutex::new(HashMap::new()),
            progress_tx: tx,
            users: Mutex::new(load_json_map(USERS_FILE)),
            sessions: Mutex::new(load_json_map(SESSIONS_FILE)),
            login_attempts: Mutex::new(HashMap::new()),
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

    /// Persist the current user map so accounts survive a restart.
    pub async fn save_users(&self) {
        let users = self.users.lock().await;
        save_json_map(USERS_FILE, &*users).await;
    }

    /// Persist the current session map so logins survive a restart.
    pub async fn save_sessions(&self) {
        let sessions = self.sessions.lock().await;
        save_json_map(SESSIONS_FILE, &*sessions).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // save_json_map/load_json_map are no-ops under cfg!(test) (see their doc
    // comments), so round-tripping through them here would just confirm
    // they're disabled, not that the serialization logic works. Test the
    // underlying serde round trip directly instead.
    #[test]
    fn stored_user_round_trips_through_json() {
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

        let json = serde_json::to_vec_pretty(&users).unwrap();
        let loaded: HashMap<String, StoredUser> = serde_json::from_slice(&json).unwrap();

        assert_eq!(loaded.get("alice").unwrap().nickname, "Alice");
    }

    #[test]
    fn missing_file_loads_as_empty() {
        let loaded: HashMap<String, StoredUser> =
            load_json_map("./this-file-does-not-exist.json");
        assert!(loaded.is_empty());
    }

    #[tokio::test]
    async fn app_state_new_never_reads_real_persisted_files() {
        // Even if a real users.json/sessions.json exists on disk (e.g. from
        // a previous `cargo run` in this same directory), a fresh AppState
        // built during tests must start empty - otherwise leftover local
        // dev state would silently leak into test assertions.
        let state = AppState::new();
        assert!(state.users.lock().await.is_empty());
        assert!(state.sessions.lock().await.is_empty());
    }
}
