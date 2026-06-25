//! Memory API endpoints.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::api::ApiResponse;
use crate::error::AppError;
use crate::memory::model::{
    CreateMemoryRequest, Memory, Provenance, ReviewStatus, UpdateTrustRequest,
};
use crate::memory::service::MemoryService;
use crate::state::AppState;

/// POST /api/v1/remember — write a memory.
pub async fn remember(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateMemoryRequest>,
) -> Result<Json<ApiResponse<RememberResponse>>, AppError> {
    let service = MemoryService::new(
        state.db.clone(),
        state.embedder.clone(),
        state.tantivy_index.clone(),
        state.tantivy_writer.clone(),
        state.tantivy_schema.clone(),
    );

    let memory = service.create(req)?;

    Ok(Json(ApiResponse::ok(RememberResponse {
        id: memory.id.clone(),
        space_id: memory.space_id.clone(),
        content: memory.content.clone(),
        provenance: memory.provenance,
        trust_level: memory.trust_level.value(),
        review_status: memory.review_status,
        embedding_generated: memory.embedding.is_some(),
        created_at: memory.created_at,
    })))
}

/// Response for POST /remember.
#[derive(Debug, Serialize)]
pub struct RememberResponse {
    pub id: String,
    pub space_id: String,
    pub content: String,
    pub provenance: Provenance,
    pub trust_level: f32,
    pub review_status: ReviewStatus,
    pub embedding_generated: bool,
    pub created_at: i64,
}

/// GET /api/v1/memories/:id — get a single memory.
pub async fn get_memory(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Memory>>, AppError> {
    let service = MemoryService::new(
        state.db.clone(),
        state.embedder.clone(),
        state.tantivy_index.clone(),
        state.tantivy_writer.clone(),
        state.tantivy_schema.clone(),
    );

    let memory = service.get_by_id(&id)?;
    Ok(Json(ApiResponse::ok(memory)))
}

/// Query parameters for GET /memories.
#[derive(Debug, Deserialize)]
pub struct ListMemoriesQuery {
    pub space_id: String,
    pub provenance: Option<String>,
    pub min_trust: Option<f32>,
    pub review_status: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Response for GET /memories.
#[derive(Debug, Serialize)]
pub struct ListMemoriesResponse {
    pub memories: Vec<Memory>,
    pub total: usize,
    pub limit: usize,
    pub offset: usize,
}

/// GET /api/v1/memories — list memories with pagination and filters.
pub async fn list_memories(
    State(state): State<Arc<AppState>>,
    Query(q): Query<ListMemoriesQuery>,
) -> Result<Json<ApiResponse<ListMemoriesResponse>>, AppError> {
    let service = MemoryService::new(
        state.db.clone(),
        state.embedder.clone(),
        state.tantivy_index.clone(),
        state.tantivy_writer.clone(),
        state.tantivy_schema.clone(),
    );

    let provenance_filter: Option<Vec<Provenance>> = q.provenance.as_ref().map(|s| {
        s.split(',')
            .filter_map(|p| Provenance::parse_str(p.trim()).ok())
            .collect()
    });

    let review_status = q
        .review_status
        .as_ref()
        .and_then(|s| ReviewStatus::parse_str(s).ok());

    let limit = q.limit.unwrap_or(20).min(100);
    let offset = q.offset.unwrap_or(0);

    let (memories, total) = service.list(
        &q.space_id,
        provenance_filter.as_deref(),
        q.min_trust,
        review_status,
        limit,
        offset,
    )?;

    Ok(Json(ApiResponse::ok(ListMemoriesResponse {
        memories,
        total,
        limit,
        offset,
    })))
}

/// POST /api/v1/memories/:id/trust — adjust trust level.
pub async fn update_trust(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateTrustRequest>,
) -> Result<Json<ApiResponse<TrustUpdateResponse>>, AppError> {
    let service = MemoryService::new(
        state.db.clone(),
        state.embedder.clone(),
        state.tantivy_index.clone(),
        state.tantivy_writer.clone(),
        state.tantivy_schema.clone(),
    );

    let memory = service.update_trust(&id, &req)?;

    Ok(Json(ApiResponse::ok(TrustUpdateResponse {
        id: memory.id,
        trust_level: memory.trust_level.value(),
        updated_at: memory.updated_at,
    })))
}

/// Response for trust update.
#[derive(Debug, Serialize)]
pub struct TrustUpdateResponse {
    pub id: String,
    pub trust_level: f32,
    pub updated_at: i64,
}

/// POST /api/v1/memories/:id/adopt — adopt an AI memory.
pub async fn adopt_memory(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Memory>>, AppError> {
    let service = MemoryService::new(
        state.db.clone(),
        state.embedder.clone(),
        state.tantivy_index.clone(),
        state.tantivy_writer.clone(),
        state.tantivy_schema.clone(),
    );

    let memory = service.adopt(&id)?;
    Ok(Json(ApiResponse::ok(memory)))
}

/// POST /api/v1/memories/:id/reject — reject an AI memory.
pub async fn reject_memory(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Memory>>, AppError> {
    let service = MemoryService::new(
        state.db.clone(),
        state.embedder.clone(),
        state.tantivy_index.clone(),
        state.tantivy_writer.clone(),
        state.tantivy_schema.clone(),
    );

    let memory = service.reject(&id)?;
    Ok(Json(ApiResponse::ok(memory)))
}
