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
}
