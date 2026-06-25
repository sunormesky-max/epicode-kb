//! Upload API endpoint — multipart file upload → parse → create memories.

use std::sync::Arc;
use std::time::Instant;

use axum::{
    extract::{Extension, Multipart, State},
    Json,
};
use serde::Serialize;

use crate::api::ApiResponse;
use crate::auth::model::Actor;
use crate::error::AppError;
use crate::memory::model::{CreateMemoryRequest, Provenance};
use crate::memory::service::MemoryService;
use crate::parse::{get_parser_for_file, FileType};
use crate::state::AppState;

/// POST /api/v1/upload — upload a document (multipart/form-data).
pub async fn upload(
    State(state): State<Arc<AppState>>,
    Extension(actor): Extension<Actor>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<UploadResponse>>, AppError> {
    let start = Instant::now();

    let mut file_data: Option<Vec<u8>> = None;
    let mut file_name: Option<String> = None;
    let mut space_id: Option<String> = None;
    let mut provenance: Option<Provenance> = None;
    let mut chunk_size: Option<usize> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::bad_request(format!("multipart error: {}", e)))?
    {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "file" => {
                file_name = field.file_name().map(|s| s.to_string());
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|e| AppError::bad_request(format!("read file error: {}", e)))?;
                if bytes.len() > state.config.max_upload_size {
                    return Err(AppError::bad_request(format!(
                        "file too large: {} bytes (max: {})",
                        bytes.len(),
                        state.config.max_upload_size
                    )));
                }
                file_data = Some(bytes.to_vec());
            }
            "space_id" => {
                space_id =
                    Some(field.text().await.map_err(|e| {
                        AppError::bad_request(format!("read space_id error: {}", e))
                    })?);
            }
            "provenance" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| AppError::bad_request(format!("read provenance error: {}", e)))?;
                provenance = Provenance::parse_str(&text).ok();
            }
            "chunk_size" => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| AppError::bad_request(format!("read chunk_size error: {}", e)))?;
                chunk_size = text.parse().ok();
            }
            _ => {
                let _ = field.bytes().await;
            }
        }
    }

    let file_bytes = file_data.ok_or_else(|| AppError::bad_request("missing 'file' field"))?;
    let filename = file_name.ok_or_else(|| AppError::bad_request("missing file name"))?;
    let space = space_id.unwrap_or_else(|| "sp_default".to_string());
    let prov = provenance.unwrap_or(Provenance::Human);

    let parser = get_parser_for_file(&filename)?;
    let file_type = FileType::from_filename(&filename).unwrap_or(FileType::Text);

    // Parse file into chunks using configured chunk size.
    let chunk_size = chunk_size.unwrap_or(state.config.chunk_size);
    let chunks =
        parser.parse_with_chunk_size(&file_bytes, chunk_size, state.config.chunk_overlap)?;
    let total_chunks = chunks.len();

    let service = MemoryService::from_state(&state);
    let mut memories_created: Vec<MemoryCreated> = Vec::new();

    for chunk in &chunks {
        let req = CreateMemoryRequest {
            space_id: space.clone(),
            content: chunk.content.clone(),
            provenance: prov,
            trust_level: None,
            provenance_meta: chunk.metadata.clone(),
            review_status: None,
            visibility: None,
        };

        match service.create(req, Some(&actor)) {
            Ok(memory) => {
                let preview = if memory.content.len() > 100 {
                    format!("{}...", &memory.content[..100])
                } else {
                    memory.content.clone()
                };
                memories_created.push(MemoryCreated {
                    id: memory.id,
                    chunk_index: chunk.chunk_index,
                    content_preview: preview,
                });
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to create memory for chunk {}: {}",
                    chunk.chunk_index,
                    e
                );
            }
        }
    }

    let processing_time_ms = start.elapsed().as_millis() as u64;

    Ok(Json(ApiResponse::ok(UploadResponse {
        file_name: filename,
        file_type: file_type.as_str().to_string(),
        total_chunks,
        memories_created,
        processing_time_ms,
    })))
}

/// Response for POST /upload.
#[derive(Debug, Serialize)]
pub struct UploadResponse {
    pub file_name: String,
    pub file_type: String,
    pub total_chunks: usize,
    pub memories_created: Vec<MemoryCreated>,
    pub processing_time_ms: u64,
}

/// A created memory from upload.
#[derive(Debug, Serialize)]
pub struct MemoryCreated {
    pub id: String,
    pub chunk_index: usize,
    pub content_preview: String,
}
