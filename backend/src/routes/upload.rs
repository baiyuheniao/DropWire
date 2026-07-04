use axum::{
    body::Body,
    extract::{Multipart, Path, State},
    http::{header, StatusCode},
    response::Response,
    Json,
};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::fs;
use tokio::io::AsyncWriteExt;

use crate::models::{ApiResponse, FileMeta, MergeRequest};
use crate::state::{AppState, UploadEntry, UploadProgress, UploadStatus};

const TEMP_DIR: &str = "./temp_chunks";
const OUTPUT_DIR: &str = "./uploads";
const META_DIR: &str = "./uploads_meta";

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[derive(Serialize, Clone)]
pub struct FileInfo {
    filename: String,
    relative_path: Option<String>,
    size: u64,
    modified_at: Option<u64>,
    sender: Option<String>,
    receiver: Option<String>,
    remark: Option<String>,
    encrypted: Option<bool>,
    salt: Option<String>,
    iv: Option<String>,
    expires_at: Option<u64>,
    received: bool,
    received_at: Option<u64>,
    received_by: Option<String>,
}

#[derive(Serialize)]
pub struct UploadStatusResponse {
    upload_id: String,
    filename: Option<String>,
    received_chunks: Vec<usize>,
}

fn build_relative_file_path(relative_path: Option<&str>, filename: &str) -> String {
    match relative_path {
        Some(rp) if !rp.is_empty() => format!("{}/{}", rp, filename),
        _ => filename.to_string(),
    }
}

fn meta_path_for(relative_file_path: &str) -> PathBuf {
    PathBuf::from(META_DIR).join(format!("{}.json", relative_file_path))
}

async fn read_file_meta(relative_file_path: &str) -> FileMeta {
    let path = meta_path_for(relative_file_path);
    match fs::read(&path).await {
        Ok(data) => serde_json::from_slice(&data).unwrap_or_default(),
        Err(_) => FileMeta::default(),
    }
}

async fn visit_dir(
    dir: &std::path::Path,
    base: &std::path::Path,
    files: &mut Vec<FileInfo>,
    now: u64,
) -> Result<(), StatusCode> {
    let mut stack: Vec<PathBuf> = vec![dir.to_path_buf()];

    while let Some(current) = stack.pop() {
        let mut entries = fs::read_dir(&current)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        {
            let metadata = entry
                .metadata()
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            let path = entry.path();
            if metadata.is_dir() {
                stack.push(path);
            } else if metadata.is_file() {
                let relative_file_path = path
                    .strip_prefix(base)
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                    .to_string_lossy()
                    .to_string();
                let filename = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&relative_file_path)
                    .to_string();
                let modified_at = metadata
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs());
                let meta = read_file_meta(&relative_file_path).await;
                if let Some(exp) = meta.expires_at {
                    if exp > 0 && exp <= now {
                        continue;
                    }
                }
                files.push(FileInfo {
                    filename,
                    relative_path: meta.relative_path.clone(),
                    size: metadata.len(),
                    modified_at,
                    sender: meta.sender,
                    receiver: meta.receiver,
                    remark: meta.remark,
                    encrypted: meta.encrypted,
                    salt: meta.salt,
                    iv: meta.iv,
                    expires_at: meta.expires_at,
                    received: meta.received,
                    received_at: meta.received_at,
                    received_by: meta.received_by,
                });
            }
        }
    }
    Ok(())
}

pub async fn list_files() -> Result<Json<ApiResponse<Vec<FileInfo>>>, StatusCode> {
    let output_dir = PathBuf::from(OUTPUT_DIR);
    let mut files = Vec::new();
    let now = now_secs();

    if output_dir.exists() {
        visit_dir(&output_dir, &output_dir, &mut files, now).await?;
    }

    files.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));

    Ok(Json(ApiResponse {
        success: true,
        message: "ok".to_string(),
        data: Some(files),
    }))
}

pub async fn download_file(Path(filename): Path<String>) -> Result<Response, StatusCode> {
    let output_dir = PathBuf::from(OUTPUT_DIR);
    let file_path = output_dir.join(&filename);

    // 防止目录遍历
    let canonical_output = output_dir
        .canonicalize()
        .unwrap_or_else(|_| output_dir.clone());
    let canonical_file = file_path
        .canonicalize()
        .map_err(|_| StatusCode::NOT_FOUND)?;
    if !canonical_file.starts_with(&canonical_output) {
        return Err(StatusCode::FORBIDDEN);
    }

    let meta = read_file_meta(&filename).await;
    if let Some(exp) = meta.expires_at {
        if exp > 0 && exp <= now_secs() {
            return Err(StatusCode::GONE);
        }
    }

    let data = fs::read(&canonical_file)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let disposition_filename = std::path::Path::new(&filename)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&filename);
    let disposition = format!("attachment; filename=\"{}\"", disposition_filename);

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(header::CONTENT_DISPOSITION, disposition)
        .body(Body::from(data))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?)
}

#[derive(Debug, serde::Deserialize)]
pub struct ReceivedRequest {
    pub filename: String,
    pub received_by: Option<String>,
}

pub async fn mark_file_received(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ReceivedRequest>,
) -> Result<Json<ApiResponse<FileInfo>>, StatusCode> {
    let output_dir = PathBuf::from(OUTPUT_DIR);
    let file_path = output_dir.join(&req.filename);

    let canonical_output = output_dir
        .canonicalize()
        .unwrap_or_else(|_| output_dir.clone());
    let canonical_file = file_path
        .canonicalize()
        .map_err(|_| StatusCode::NOT_FOUND)?;
    if !canonical_file.starts_with(&canonical_output) {
        return Err(StatusCode::FORBIDDEN);
    }

    let mut meta = read_file_meta(&req.filename).await;
    meta.received = true;
    meta.received_at = Some(now_secs());
    meta.received_by = req.received_by;

    let meta_path = meta_path_for(&req.filename);
    if let Some(parent) = meta_path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    fs::write(
        &meta_path,
        serde_json::to_vec_pretty(&meta).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Broadcast a received event so the sender can see confirmation in real time.
    let event = serde_json::json!({
        "event": "received",
        "data": {
            "filename": req.filename,
            "received_at": meta.received_at,
            "received_by": meta.received_by,
        }
    });
    let _ = state.progress_tx.send(event.to_string());

    let metadata = fs::metadata(&canonical_file)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ApiResponse {
        success: true,
        message: "ok".to_string(),
        data: Some(FileInfo {
            filename: std::path::Path::new(&req.filename)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&req.filename)
                .to_string(),
            relative_path: meta.relative_path.clone(),
            size: metadata.len(),
            modified_at: metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                .map(|d| d.as_secs()),
            sender: meta.sender.clone(),
            receiver: meta.receiver.clone(),
            remark: meta.remark.clone(),
            encrypted: meta.encrypted,
            salt: meta.salt.clone(),
            iv: meta.iv.clone(),
            expires_at: meta.expires_at,
            received: meta.received,
            received_at: meta.received_at,
            received_by: meta.received_by.clone(),
        }),
    }))
}

pub async fn get_upload_status(
    Path(upload_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<UploadStatusResponse>>, StatusCode> {
    let chunk_dir = PathBuf::from(TEMP_DIR).join(&upload_id);

    // Prefer in-memory state for the filename, but also scan disk in case the
    // server was restarted while chunks were left in temp_chunks.
    let filename = {
        let uploads = state.uploads.lock().unwrap();
        uploads.get(&upload_id).map(|e| e.progress.filename.clone())
    };

    let mut received = std::collections::HashSet::new();

    if chunk_dir.exists() {
        let mut entries = fs::read_dir(&chunk_dir)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        {
            if let Ok(metadata) = entry.metadata().await {
                if metadata.is_file() {
                    if let Some(stem) = entry.file_name().to_str() {
                        if let Some(index_str) = stem.strip_suffix(".chunk") {
                            if let Ok(index) = index_str.parse::<usize>() {
                                received.insert(index);
                            }
                        }
                    }
                }
            }
        }
    }

    let mut received_chunks: Vec<usize> = received.into_iter().collect();
    received_chunks.sort();

    Ok(Json(ApiResponse {
        success: true,
        message: "ok".to_string(),
        data: Some(UploadStatusResponse {
            upload_id,
            filename,
            received_chunks,
        }),
    }))
}

fn fail_upload(state: &AppState, upload_id: &str, reason: String) {
    let mut uploads = state.uploads.lock().unwrap();
    if let Some(entry) = uploads.get_mut(upload_id) {
        entry.progress.status = UploadStatus::Failed(reason);
        broadcast_progress(&state.progress_tx, &entry.progress);
    }
}

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

    if upload_id.is_empty() || filename.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let chunk_dir = PathBuf::from(TEMP_DIR).join(&upload_id);
    if let Err(e) = fs::create_dir_all(&chunk_dir).await {
        fail_upload(&state, &upload_id, format!("create chunk dir failed: {e}"));
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    let chunk_path = chunk_dir.join(format!("{}.chunk", chunk_index));
    let mut file = match fs::File::create(&chunk_path).await {
        Ok(f) => f,
        Err(e) => {
            fail_upload(&state, &upload_id, format!("create chunk file failed: {e}"));
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    if let Err(e) = file.write_all(&chunk_data).await {
        fail_upload(&state, &upload_id, format!("write chunk failed: {e}"));
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    let progress = {
        let mut uploads = state.uploads.lock().unwrap();
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

    broadcast_progress(&state.progress_tx, &progress);

    Ok(Json(ApiResponse {
        success: true,
        message: format!("chunk {} received", chunk_index),
        data: None,
    }))
}

fn broadcast_progress(tx: &tokio::sync::broadcast::Sender<String>, progress: &UploadProgress) {
    let event = serde_json::json!({ "event": "progress", "data": progress });
    let _ = tx.send(event.to_string());
}

pub async fn merge_chunks(
    State(state): State<Arc<AppState>>,
    Json(req): Json<MergeRequest>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    {
        let mut uploads = state.uploads.lock().unwrap();
        if let Some(entry) = uploads.get_mut(&req.upload_id) {
            entry.progress.status = UploadStatus::Merging;
            broadcast_progress(&state.progress_tx, &entry.progress);
        }
    }

    let chunk_dir = PathBuf::from(TEMP_DIR).join(&req.upload_id);
    let output_dir = PathBuf::from(OUTPUT_DIR);
    fs::create_dir_all(&output_dir)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let relative_file_path = build_relative_file_path(req.relative_path.as_deref(), &req.filename);
    let output_path = output_dir.join(&relative_file_path);
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    let mut out = fs::File::create(&output_path)
        .await
        .map_err(|e| {
            fail_upload(&state, &req.upload_id, format!("create output file failed: {e}"));
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    for i in 0..req.total_chunks {
        let chunk_path = chunk_dir.join(format!("{}.chunk", i));
        let data = match fs::read(&chunk_path).await {
            Ok(d) => d,
            Err(e) => {
                fail_upload(&state, &req.upload_id, format!("missing chunk {}: {}", i, e));
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        };
        if let Err(e) = out.write_all(&data).await {
            fail_upload(&state, &req.upload_id, format!("merge write failed: {e}"));
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }

    let expires_at = req.expires_in_minutes.and_then(|mins| {
        if mins > 0 {
            Some(now_secs() + mins * 60)
        } else {
            None
        }
    });

    let meta = FileMeta {
        sender: req.sender.clone(),
        receiver: req.receiver.clone(),
        remark: req.remark.clone(),
        relative_path: req.relative_path.clone(),
        encrypted: req.encrypted,
        salt: req.salt.clone(),
        iv: req.iv.clone(),
        expires_at,
        ..Default::default()
    };
    let meta_path = meta_path_for(&relative_file_path);
    if let Some(parent) = meta_path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    fs::write(
        &meta_path,
        serde_json::to_vec_pretty(&meta).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let _ = fs::remove_dir_all(&chunk_dir).await;

    {
        let mut uploads = state.uploads.lock().unwrap();
        if let Some(entry) = uploads.get_mut(&req.upload_id) {
            entry.progress.status = UploadStatus::Completed;
            broadcast_progress(&state.progress_tx, &entry.progress);
        }
    }

    Ok(Json(ApiResponse {
        success: true,
        message: "file merged successfully".to_string(),
        data: Some(req.filename),
    }))
}
