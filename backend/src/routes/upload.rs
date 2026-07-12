use axum::{
    body::Body,
    extract::{Multipart, Path, State},
    http::{header, HeaderMap, StatusCode},
    response::Response,
    Json,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::{Component, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::fs;
use tokio::io::AsyncWriteExt;

use crate::models::{ApiResponse, FileMeta, MergeRequest};
use crate::routes::auth_middleware::CurrentUser;
use crate::state::{AppState, UploadEntry, UploadProgress, UploadStatus};

const TEMP_DIR: &str = "./temp_chunks";
const OUTPUT_DIR: &str = "./uploads";
const META_DIR: &str = "./uploads_meta";

fn content_type_for(filename: &str) -> &'static str {
    let ext = std::path::Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    match ext.to_ascii_lowercase().as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "bmp" => "image/bmp",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "ogg" => "video/ogg",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "webm" => "audio/webm",
        "txt" => "text/plain; charset=utf-8",
        "md" => "text/markdown; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "xml" => "application/xml; charset=utf-8",
        "pdf" => "application/pdf",
        "html" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" => "application/javascript",
        "ts" => "application/typescript",
        _ => "application/octet-stream",
    }
}

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
    expires_at: Option<u64>,
    hash_type: Option<String>,
    hash_value: Option<String>,
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

/// Sanitize a relative path by rejecting any `..` components.
fn sanitize_relative_path(input: &str) -> Option<String> {
    if input.is_empty() {
        return Some(String::new());
    }
    let path = PathBuf::from(input);
    for component in path.components() {
        match component {
            Component::Normal(_) => {}
            _ => return None,
        }
    }
    Some(path.to_string_lossy().to_string())
}

fn build_relative_file_path(relative_path: Option<&str>, filename: &str) -> Option<String> {
    let filename = sanitize_relative_path(filename)?;
    if filename.is_empty() {
        return None;
    }
    match relative_path {
        Some(rp) if !rp.is_empty() => {
            let rp = sanitize_relative_path(rp)?;
            Some(format!("{}/{}", rp, filename))
        }
        _ => Some(filename),
    }
}

fn compute_file_hash(data: &[u8], hash_type: &str) -> Result<String, StatusCode> {
    match hash_type.to_ascii_lowercase().as_str() {
        "md5" => {
            use md5::Digest;
            Ok(format!("{:x}", md5::Md5::digest(data)))
        }
        "sha-1" | "sha1" => {
            use sha1::Digest;
            Ok(format!("{:x}", sha1::Sha1::digest(data)))
        }
        "sha-256" | "sha256" => {
            use sha2::{Digest, Sha256};
            Ok(format!("{:x}", Sha256::digest(data)))
        }
        "sha-384" | "sha384" => {
            use sha2::{Digest, Sha384};
            Ok(format!("{:x}", Sha384::digest(data)))
        }
        "sha-512" | "sha512" => {
            use sha2::{Digest, Sha512};
            Ok(format!("{:x}", Sha512::digest(data)))
        }
        "crc32" => {
            let mut hasher = crc32fast::Hasher::new();
            hasher.update(data);
            Ok(format!("{:08x}", hasher.finalize()))
        }
        _ => Err(StatusCode::BAD_REQUEST),
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
    current_user: &str,
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
                // Server-side access control: users can only see files sent to
                // them, sent by them, or public files without a receiver.
                let is_sender = meta.sender.as_deref() == Some(current_user);
                let is_receiver = meta.receiver.as_deref() == Some(current_user);
                let is_public = meta.receiver.is_none();
                if !is_sender && !is_receiver && !is_public {
                    continue;
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
                    expires_at: meta.expires_at,
                    hash_type: meta.hash_type.clone(),
                    hash_value: meta.hash_value.clone(),
                    received: meta.received,
                    received_at: meta.received_at,
                    received_by: meta.received_by,
                });
            }
        }
    }
    Ok(())
}

pub async fn list_files(
    user: CurrentUser,
) -> Result<Json<ApiResponse<Vec<FileInfo>>>, StatusCode> {
    let current_user = user.username;
    let output_dir = PathBuf::from(OUTPUT_DIR);
    let mut files = Vec::new();
    let now = now_secs();

    if output_dir.exists() {
        visit_dir(&output_dir, &output_dir, &mut files, now, &current_user).await?;
    }

    files.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));

    Ok(Json(ApiResponse {
        success: true,
        message: "ok".to_string(),
        data: Some(files),
    }))
}

fn resolve_output_path(filename: &str) -> Result<(PathBuf, PathBuf), StatusCode> {
    let output_dir = PathBuf::from(OUTPUT_DIR);
    let file_path = output_dir.join(filename);

    let canonical_output = output_dir
        .canonicalize()
        .unwrap_or_else(|_| output_dir.clone());
    let canonical_file = file_path
        .canonicalize()
        .unwrap_or_else(|_| file_path.clone());
    if !canonical_file.starts_with(&canonical_output) {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok((canonical_output, canonical_file))
}

pub async fn download_file(
    Path(filename): Path<String>,
    headers: HeaderMap,
    user: CurrentUser,
) -> Result<Response, StatusCode> {
    let current_user = user.username;

    let (_, canonical_file) = resolve_output_path(&filename)?;

    let meta = read_file_meta(&filename).await;

    // Server-side access control for downloads.
    let is_sender = meta.sender.as_deref() == Some(&current_user);
    let is_receiver = meta.receiver.as_deref() == Some(&current_user);
    let is_public = meta.receiver.is_none();
    if !is_sender && !is_receiver && !is_public {
        return Err(StatusCode::FORBIDDEN);
    }

    if let Some(exp) = meta.expires_at {
        if exp > 0 && exp <= now_secs() {
            return Err(StatusCode::GONE);
        }
    }

    let metadata = fs::metadata(&canonical_file)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let file_size = metadata.len();

    let disposition_filename = std::path::Path::new(&filename)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&filename);
    let disposition = format!("attachment; filename=\"{}\"", disposition_filename);
    let content_type = content_type_for(disposition_filename);

    // Parse Range header for resumable download.
    let (start, end) = if let Some(range_header) = headers.get(header::RANGE) {
        let range_str = range_header.to_str().map_err(|_| StatusCode::BAD_REQUEST)?;
        if let Some((s, e)) = parse_range(range_str, file_size) {
            (s, e)
        } else {
            return Err(StatusCode::RANGE_NOT_SATISFIABLE);
        }
    } else {
        (0, file_size.saturating_sub(1))
    };

    let length = end - start + 1;
    let data = if length < file_size {
        let mut file = fs::File::open(&canonical_file)
            .await
            .map_err(|_| StatusCode::NOT_FOUND)?;
        use tokio::io::AsyncSeekExt;
        file.seek(std::io::SeekFrom::Start(start))
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let mut buf = vec![0u8; length as usize];
        use tokio::io::AsyncReadExt;
        file.read_exact(&mut buf)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        buf
    } else {
        fs::read(&canonical_file)
            .await
            .map_err(|_| StatusCode::NOT_FOUND)?
    };

    let (status, extra_headers) = if length < file_size {
        (
            StatusCode::PARTIAL_CONTENT,
            format!(
                "content-range: bytes {}-{}/{}",
                start, end, file_size
            ),
        )
    } else {
        (StatusCode::OK, "accept-ranges: bytes".to_string())
    };

    let mut builder = Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CONTENT_DISPOSITION, disposition)
        .header(header::CONTENT_LENGTH, length.to_string());

    for part in extra_headers.split("\r\n") {
        if let Some((name, value)) = part.split_once(": ") {
            builder = builder.header(name, value);
        }
    }

    builder
        .body(Body::from(data))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

/// Parse a `Range: bytes=start-end` header, returning inclusive byte offsets.
fn parse_range(range_str: &str, file_size: u64) -> Option<(u64, u64)> {
    let s = range_str.strip_prefix("bytes=")?;
    let (start_part, end_part) = s.split_once('-')?;

    let start: u64 = start_part.trim().parse().ok()?;
    let end: u64 = if end_part.trim().is_empty() {
        file_size.saturating_sub(1)
    } else {
        end_part.trim().parse().ok()?
    };

    if start > end || start >= file_size {
        return None;
    }
    let end = end.min(file_size.saturating_sub(1));
    Some((start, end))
}

#[derive(Debug, serde::Deserialize)]
pub struct ReceivedRequest {
    pub filename: String,
    pub received_by: Option<String>,
}

pub async fn mark_file_received(
    State(state): State<Arc<AppState>>,
    user: CurrentUser,
    Json(req): Json<ReceivedRequest>,
) -> Result<Json<ApiResponse<FileInfo>>, StatusCode> {
    let current_user = user.username;

    let (_, canonical_file) = resolve_output_path(&req.filename)?;

    let mut meta = read_file_meta(&req.filename).await;

    let is_sender = meta.sender.as_deref() == Some(&current_user);
    let is_receiver = meta.receiver.as_deref() == Some(&current_user);
    let is_public = meta.receiver.is_none();
    if !is_sender && !is_receiver && !is_public {
        return Err(StatusCode::FORBIDDEN);
    }

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
            expires_at: meta.expires_at,
            hash_type: meta.hash_type.clone(),
            hash_value: meta.hash_value.clone(),
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
        let uploads = state.uploads.lock().await;
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

async fn fail_upload(state: &AppState, upload_id: &str, reason: String) {
    let mut uploads = state.uploads.lock().await;
    if let Some(entry) = uploads.get_mut(upload_id) {
        entry.progress.status = UploadStatus::Failed(reason);
        broadcast_progress(&state.progress_tx, &entry.progress);
    }
}

fn sha256_hex(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(data))
}

pub async fn upload_chunk(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<()>>, StatusCode> {
    let mut upload_id = String::new();
    let mut filename = String::new();
    let mut chunk_index: usize = 0;
    let mut total_chunks: usize = 0;
    let mut chunk_hash = String::new();
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
            "chunk_hash" => {
                chunk_hash = field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?
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

    // Validate chunk hash if provided.
    if !chunk_hash.is_empty() && chunk_hash != sha256_hex(&chunk_data) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let chunk_dir = PathBuf::from(TEMP_DIR).join(&upload_id);
    if let Err(e) = fs::create_dir_all(&chunk_dir).await {
        fail_upload(&state, &upload_id, format!("create chunk dir failed: {e}")).await;
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    let chunk_path = chunk_dir.join(format!("{}.chunk", chunk_index));
    let mut file = match fs::File::create(&chunk_path).await {
        Ok(f) => f,
        Err(e) => {
            fail_upload(&state, &upload_id, format!("create chunk file failed: {e}")).await;
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    if let Err(e) = file.write_all(&chunk_data).await {
        fail_upload(&state, &upload_id, format!("write chunk failed: {e}")).await;
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

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
            chunk_hashes: vec![None; total_chunks],
        });

        entry.chunk_received[chunk_index] = true;
        entry.chunk_hashes[chunk_index] = Some(chunk_hash.clone());
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
        let mut uploads = state.uploads.lock().await;
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

    let relative_file_path = build_relative_file_path(req.relative_path.as_deref(), &req.filename)
        .ok_or(StatusCode::BAD_REQUEST)?;
    let output_path = output_dir.join(&relative_file_path);
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    // Verify all chunks are present and hashes match before writing the output.
    let chunk_hashes = {
        let uploads = state.uploads.lock().await;
        uploads
            .get(&req.upload_id)
            .map(|e| e.chunk_hashes.clone())
            .unwrap_or_default()
    };

    let mut out = match fs::File::create(&output_path).await {
        Ok(f) => f,
        Err(e) => {
            fail_upload(&state, &req.upload_id, format!("create output file failed: {e}")).await;
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let mut hasher = Sha256::new();
    for i in 0..req.total_chunks {
        let chunk_path = chunk_dir.join(format!("{}.chunk", i));
        let data = match fs::read(&chunk_path).await {
            Ok(d) => d,
            Err(e) => {
                fail_upload(&state, &req.upload_id, format!("missing chunk {}: {}", i, e)).await;
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        };
        if let Some(Some(expected)) = chunk_hashes.get(i) {
            if !expected.is_empty() && sha256_hex(&data) != *expected {
                fail_upload(
                    &state,
                    &req.upload_id,
                    format!("chunk {} hash mismatch", i),
                )
                .await;
                return Err(StatusCode::BAD_REQUEST);
            }
        }
        if let Err(e) = out.write_all(&data).await {
            fail_upload(&state, &req.upload_id, format!("merge write failed: {e}")).await;
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
        hasher.update(&data);
    }

    let expires_at = req.expires_in_minutes.and_then(|mins| {
        if mins > 0 {
            Some(now_secs() + mins * 60)
        } else {
            None
        }
    });

    let hash_type = req
        .hash_type
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or("sha256")
        .to_string();

    let hash_value = if hash_type.eq_ignore_ascii_case("sha-256") || hash_type.eq_ignore_ascii_case("sha256") {
        Some(format!("{:x}", hasher.finalize()))
    } else {
        match fs::read(&output_path).await {
            Ok(data) => compute_file_hash(&data, &hash_type).ok(),
            Err(_) => None,
        }
    };
    let hash_type = hash_value.as_ref().map(|_| hash_type);

    let meta = FileMeta {
        sender: req.sender.clone(),
        receiver: req.receiver.clone(),
        remark: req.remark.clone(),
        relative_path: req.relative_path.clone(),
        encrypted: req.encrypted,
        salt: req.salt.clone(),
        iv: req.iv.clone(),
        expires_at,
        hash_type,
        hash_value,
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
        let mut uploads = state.uploads.lock().await;
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
