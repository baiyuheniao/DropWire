use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    Json,
};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::io::AsyncWriteExt;

use crate::models::{ApiResponse, MergeRequest};
use crate::state::{AppState, UploadEntry, UploadProgress, UploadStatus};

const TEMP_DIR: &str = "./temp_chunks";
const OUTPUT_DIR: &str = "./uploads";

pub async fn upload_chunk(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<()>>, StatusCode> {
    let mut upload_id = String::new();
    let mut filename = String::new();
    let mut chunk_index: usize = 0;
    let mut total_chunks: usize = 0;
    let mut chunk_data: Vec<u8> = Vec::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?
    {
        match field.name().unwrap_or("") {
            "upload_id" => {
                upload_id = field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?
            }
            "filename" => {
                filename = field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?
            }
            "chunk_index" => {
                chunk_index = field
                    .text()
                    .await
                    .map_err(|_| StatusCode::BAD_REQUEST)?
                    .parse()
                    .map_err(|_| StatusCode::BAD_REQUEST)?
            }
            "total_chunks" => {
                total_chunks = field
                    .text()
                    .await
                    .map_err(|_| StatusCode::BAD_REQUEST)?
                    .parse()
                    .map_err(|_| StatusCode::BAD_REQUEST)?
            }
            "chunk" => {
                chunk_data = field
                    .bytes()
                    .await
                    .map_err(|_| StatusCode::BAD_REQUEST)?
                    .to_vec()
            }
            _ => {}
        }
    }

    if upload_id.is_empty() || filename.is_empty() || chunk_data.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Sanitize upload_id to prevent path traversal
    let safe_upload_id = Path::new(&upload_id)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let chunk_dir = PathBuf::from(TEMP_DIR).join(&safe_upload_id);
    fs::create_dir_all(&chunk_dir)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let chunk_path = chunk_dir.join(format!("{}.chunk", chunk_index));
    let mut file = fs::File::create(&chunk_path)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    file.write_all(&chunk_data)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let progress = {
        let mut uploads = state.uploads.lock().await;
        let entry = uploads.entry(upload_id.clone()).or_insert_with(|| UploadEntry {
            progress: UploadProgress {
                upload_id: upload_id.clone(),
                filename: filename.clone(),
                total_chunks,
                received_chunks: 0,
                status: UploadStatus::Uploading,
            },
            chunk_received: vec![false; total_chunks],
        });

        entry.chunk_received[chunk_index] = true;
        entry.progress.received_chunks = entry.chunk_received.iter().filter(|&&x| x).count();
        entry.progress.clone()
    };

    let _ = state
        .progress_tx
        .send(serde_json::to_string(&progress).unwrap());

    Ok(Json(ApiResponse {
        success: true,
        message: format!("chunk {} received", chunk_index),
        data: None,
    }))
}

pub async fn merge_chunks(
    State(state): State<Arc<AppState>>,
    Json(req): Json<MergeRequest>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    {
        let mut uploads = state.uploads.lock().await;
        if let Some(entry) = uploads.get_mut(&req.upload_id) {
            entry.progress.status = UploadStatus::Merging;
            let _ = state
                .progress_tx
                .send(serde_json::to_string(&entry.progress).unwrap());
        }
    }

    let chunk_dir = PathBuf::from(TEMP_DIR).join(&req.upload_id);
    let output_dir = PathBuf::from(OUTPUT_DIR);
    fs::create_dir_all(&output_dir)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Sanitize filename to prevent path traversal
    let safe_name = Path::new(&req.filename)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let output_path = output_dir.join(&safe_name);
    let mut out = fs::File::create(&output_path)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    for i in 0..req.total_chunks {
        let chunk_path = chunk_dir.join(format!("{}.chunk", i));
        let data = fs::read(&chunk_path)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        out.write_all(&data)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    let _ = fs::remove_dir_all(&chunk_dir).await;

    {
        let mut uploads = state.uploads.lock().await;
        if let Some(entry) = uploads.get_mut(&req.upload_id) {
            entry.progress.status = UploadStatus::Completed;
            let _ = state
                .progress_tx
                .send(serde_json::to_string(&entry.progress).unwrap());
        }
    }

    Ok(Json(ApiResponse {
        success: true,
        message: "file merged successfully".to_string(),
        data: Some(req.filename),
    }))
}
