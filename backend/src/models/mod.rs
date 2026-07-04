use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

#[derive(Debug, Deserialize)]
pub struct MergeRequest {
    pub upload_id: String,
    pub filename: String,
    pub relative_path: Option<String>,
    pub total_chunks: usize,
    pub sender: Option<String>,
    pub receiver: Option<String>,
    pub remark: Option<String>,
    pub encrypted: Option<bool>,
    pub salt: Option<String>,
    pub iv: Option<String>,
    pub expires_in_minutes: Option<u64>,
    pub hash_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct FileMeta {
    pub sender: Option<String>,
    pub receiver: Option<String>,
    pub remark: Option<String>,
    pub relative_path: Option<String>,
    pub encrypted: Option<bool>,
    pub salt: Option<String>,
    pub iv: Option<String>,
    pub expires_at: Option<u64>,
    pub hash_type: Option<String>,
    pub hash_value: Option<String>,
    pub received: bool,
    pub received_at: Option<u64>,
    pub received_by: Option<String>,
}
