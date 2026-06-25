//! Memory API endpoints.

use std::sync::Arc;

use axum::{
    extract::{Extension, Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::api::ApiResponse;
use crate::auth::model::Actor;
use crate::error::AppError;
use crate::memory::model::{
    ConflictResolutionRequest, CreateMemoryRequest, Memory, Provenance, ReviewStatus,
    SaveVersionRequest, UpdateTrustRequest, UpdateVisibilityRequest,
};
use crate::memory::service::MemoryService;
use crate::state::AppState;

/// POST /api/v1/remember — write a memory.
pub async fn remember(
    State(state): State<Arc<AppState>>,
    Extension(actor): Extension<Actor>,
    Json(body): Json<CreateMemoryRequest>,
) -> Result<Json<ApiResponse<RememberResponse>>, AppError> {
    let service = MemoryService::from_state(&state);
    let memory = service.create(body, Some(&actor))?;

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

/// GET /api/v1/memories/:id — get a single memory.
pub async fn get_memory(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Memory>>, AppError> {
    let service = MemoryService::from_state(&state);
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
    pub visibility: Option<String>,
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
    let service = MemoryService::from_state(&state);

    let provenance_filter: Option<Vec<Provenance>> = q.provenance.as_ref().map(|s| {
        s.split(',')
            .filter_map(|p| Provenance::parse_str(p.trim()).ok())
            .collect()
    });

    let review_status = q
        .review_status
        .as_ref()
        .and_then(|s| ReviewStatus::parse_str(s).ok());

    let visibility = q
        .visibility
        .as_ref()
        .and_then(|s| crate::memory::model::Visibility::parse_str(s).ok());

    let limit = q.limit.unwrap_or(20).min(100);
    let offset = q.offset.unwrap_or(0);

    let (memories, total) = service.list(
        &q.space_id,
        provenance_filter.as_deref(),
        q.min_trust,
        review_status,
        visibility,
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
    Extension(actor): Extension<Actor>,
    Path(id): Path<String>,
    Json(body): Json<UpdateTrustRequest>,
) -> Result<Json<ApiResponse<TrustUpdateResponse>>, AppError> {
    let service = MemoryService::from_state(&state);
    let memory = service.update_trust(&id, &body, &actor)?;

    Ok(Json(ApiResponse::ok(TrustUpdateResponse {
        id: memory.id,
        trust_level: memory.trust_level.value(),
        updated_at: memory.updated_at,
    })))
}

/// PUT /api/v1/memories/:id/visibility — update memory visibility.
pub async fn update_visibility(
    State(state): State<Arc<AppState>>,
    Extension(actor): Extension<Actor>,
    Path(id): Path<String>,
    Json(body): Json<UpdateVisibilityRequest>,
) -> Result<Json<ApiResponse<Memory>>, AppError> {
    let service = MemoryService::from_state(&state);
    let memory = service.update_visibility(&id, body.visibility, &actor)?;
    Ok(Json(ApiResponse::ok(memory)))
}

/// POST /api/v1/memories/:id/adopt — adopt an AI memory.
pub async fn adopt_memory(
    State(state): State<Arc<AppState>>,
    Extension(actor): Extension<Actor>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Memory>>, AppError> {
    let service = MemoryService::from_state(&state);
    let memory = service.adopt(&id, &actor)?;
    Ok(Json(ApiResponse::ok(memory)))
}

/// POST /api/v1/memories/:id/reject — reject an AI memory.
pub async fn reject_memory(
    State(state): State<Arc<AppState>>,
    Extension(actor): Extension<Actor>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Memory>>, AppError> {
    let service = MemoryService::from_state(&state);
    let memory = service.reject(&id, &actor)?;
    Ok(Json(ApiResponse::ok(memory)))
}

/// POST /api/v1/memories/:id/save — save a new version.
pub async fn save_version(
    State(state): State<Arc<AppState>>,
    Extension(actor): Extension<Actor>,
    Path(id): Path<String>,
    Json(body): Json<SaveVersionRequest>,
) -> Result<Json<ApiResponse<crate::db::repository::MemoryVersion>>, AppError> {
    let service = MemoryService::from_state(&state);
    let version = service.save_version(&id, body, &actor)?;
    Ok(Json(ApiResponse::ok(version)))
}

/// GET /api/v1/memories/:id/versions — list versions.
pub async fn list_versions(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(q): Query<ListVersionsQuery>,
) -> Result<Json<ApiResponse<ListVersionsResponse>>, AppError> {
    let service = MemoryService::from_state(&state);
    let limit = q.limit.unwrap_or(20).min(100);
    let offset = q.offset.unwrap_or(0);
    let (versions, total) = service.list_versions(&id, limit, offset)?;
    Ok(Json(ApiResponse::ok(ListVersionsResponse {
        versions,
        total,
        limit,
        offset,
    })))
}

/// POST /api/v1/memories/:id/versions/:vid/revert — revert to a version.
pub async fn revert_version(
    State(state): State<Arc<AppState>>,
    Extension(actor): Extension<Actor>,
    Path((id, vid)): Path<(String, String)>,
) -> Result<Json<ApiResponse<Memory>>, AppError> {
    let service = MemoryService::from_state(&state);
    let memory = service.revert(&id, &vid, &actor)?;
    Ok(Json(ApiResponse::ok(memory)))
}

/// POST /api/v1/memories/:id/resolve — resolve a conflict.
pub async fn resolve_conflict(
    State(state): State<Arc<AppState>>,
    Extension(actor): Extension<Actor>,
    Path(id): Path<String>,
    Json(body): Json<ConflictResolutionRequest>,
) -> Result<Json<ApiResponse<Memory>>, AppError> {
    let service = MemoryService::from_state(&state);
    let memory = service.resolve_conflict(&id, body, &actor)?;
    Ok(Json(ApiResponse::ok(memory)))
}

// ============================================================
// Response DTOs
// ============================================================

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

/// Response for trust update.
#[derive(Debug, Serialize)]
pub struct TrustUpdateResponse {
    pub id: String,
    pub trust_level: f32,
    pub updated_at: i64,
}

#[derive(Debug, Deserialize)]
pub struct ListVersionsQuery {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct ListVersionsResponse {
    pub versions: Vec<crate::db::repository::MemoryVersion>,
    pub total: usize,
    pub limit: usize,
    pub offset: usize,
}
