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
    pub total_chunks: usize,
    pub sender: Option<String>,
    pub receiver: Option<String>,
    pub remark: Option<String>,
    pub encrypted: Option<bool>,
    pub salt: Option<String>,
    pub iv: Option<String>,
    pub expires_in_minutes: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct FileMeta {
    pub sender: Option<String>,
    pub receiver: Option<String>,
    pub remark: Option<String>,
    pub encrypted: Option<bool>,
    pub salt: Option<String>,
    pub iv: Option<String>,
    pub expires_at: Option<u64>,
}
