use std::collections::HashMap;
use tokio::sync::{broadcast, Mutex};
use serde::Serialize;

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
}

impl AppState {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(256);
        Self {
            uploads: Mutex::new(HashMap::new()),
            progress_tx: tx,
        }
    }
}
