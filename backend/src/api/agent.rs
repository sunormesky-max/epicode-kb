//! Agent API endpoints — API key authentication for external agents.

use std::sync::Arc;

use axum::{
    extract::{Extension, Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::api::ApiResponse;
use crate::auth::model::{Actor, AgentContext, ApiKey};
use crate::auth::service::hash_api_key;
use crate::db::repository::ApiKeyRepo;
use crate::error::AppError;
use crate::memory::model::{CreateMemoryRequest, Provenance, Visibility};
use crate::memory::service::MemoryService;
use crate::search::{SearchMode, SearchQuery, SearchResponse};
use crate::state::AppState;

/// POST /api/v1/agents/search — agent search.
pub async fn search(
    State(state): State<Arc<AppState>>,
    Extension(actor): Extension<Actor>,
    Json(body): Json<AgentSearchRequest>,
) -> Result<Json<ApiResponse<SearchResponse>>, AppError> {
    // Build a synthetic agent context from the authenticated actor.
    let _agent = AgentContext {
        api_key_id: "api".to_string(),
        space_id: body.space_id.clone(),
        user_id: actor.user_id.clone(),
        user_role: actor.global_role,
        space_role: actor.space_role,
        scope: "read".to_string(),
    };

    let query = SearchQuery {
        q: body.q,
        space_id: body.space_id,
        mode: SearchMode::Hybrid,
        min_trust: body.min_trust,
        provenance: None,
        review_status: body
            .review_status
            .as_ref()
            .and_then(|s| crate::memory::model::ReviewStatus::parse_str(s).ok()),
        visibility: None,
        limit: body.limit.unwrap_or(20).min(100),
        offset: body.offset.unwrap_or(0),
    };

    let response = state.search_engine.search(&query)?;
    Ok(Json(ApiResponse::ok(response)))
}

/// POST /api/v1/agents/remember — agent write.
pub async fn remember(
    State(state): State<Arc<AppState>>,
    Extension(actor): Extension<Actor>,
    Json(body): Json<AgentRememberRequest>,
) -> Result<Json<ApiResponse<AgentRememberResponse>>, AppError> {
    let service = MemoryService::from_state(&state);
    let agent = AgentContext {
        api_key_id: "api".to_string(),
        space_id: body.space_id.clone(),
        user_id: actor.user_id.clone(),
        user_role: actor.global_role,
        space_role: actor.space_role,
        scope: "write".to_string(),
    };

    let create_req = CreateMemoryRequest {
        space_id: body.space_id,
        content: body.content,
        provenance: Provenance::Ai,
        trust_level: None,
        provenance_meta: body.provenance_meta,
        review_status: Some(crate::memory::model::ReviewStatus::Pending),
        visibility: Some(Visibility::Inherit),
    };

    let memory = service.create_from_agent(create_req, &agent)?;
    Ok(Json(ApiResponse::ok(AgentRememberResponse {
        id: memory.id,
        review_status: memory.review_status.as_str().to_string(),
    })))
}

/// GET /api/v1/agents/memories/:id — get a memory via agent API.
pub async fn get_memory(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<crate::memory::model::Memory>>, AppError> {
    let service = MemoryService::from_state(&state);
    let memory = service.get_by_id(&id)?;
    Ok(Json(ApiResponse::ok(memory)))
}

/// POST /api/v1/spaces/:id/api-keys — create an API key.
pub async fn create_api_key(
    State(state): State<Arc<AppState>>,
    Extension(actor): Extension<Actor>,
    Path(space_id): Path<String>,
    Json(body): Json<CreateApiKeyRequest>,
) -> Result<Json<ApiResponse<CreateApiKeyResponse>>, AppError> {
    state.auth_service.rbac().check(
        &crate::auth::rbac::AuthContext {
            user_id: actor.user_id.clone(),
            global_role: actor.global_role,
            space_id: space_id.clone(),
            space_role: actor.space_role,
        },
        crate::auth::model::Permission::ApiKeyManage,
    )?;

    let raw_key = format!("ak_{}", uuid::Uuid::new_v4().simple());
    let key_hash = hash_api_key(&raw_key, state.config.api_key_salt.as_deref());

    let api_key = ApiKey {
        id: crate::generate_id("key"),
        space_id: space_id.clone(),
        user_id: actor.user_id.clone(),
        key_hash,
        scope: body.scope,
        name: body.name,
        expires_at: body.expires_at,
        last_used_at: None,
        created_at: crate::now_ts(),
    };

    let conn = state.db.lock().unwrap();
    ApiKeyRepo::insert(&conn, &api_key)?;

    Ok(Json(ApiResponse::ok(CreateApiKeyResponse {
        id: api_key.id,
        key: raw_key,
        name: api_key.name,
        scope: api_key.scope,
        expires_at: api_key.expires_at,
        created_at: api_key.created_at,
    })))
}

/// GET /api/v1/spaces/:id/api-keys — list API keys.
pub async fn list_api_keys(
    State(state): State<Arc<AppState>>,
    Extension(actor): Extension<Actor>,
    Path(space_id): Path<String>,
    Query(q): Query<ListApiKeysQuery>,
) -> Result<Json<ApiResponse<ListApiKeysResponse>>, AppError> {
    state.auth_service.rbac().check(
        &crate::auth::rbac::AuthContext {
            user_id: actor.user_id.clone(),
            global_role: actor.global_role,
            space_id: space_id.clone(),
            space_role: actor.space_role,
        },
        crate::auth::model::Permission::ApiKeyManage,
    )?;

    let limit = q.limit.unwrap_or(20).min(100);
    let offset = q.offset.unwrap_or(0);
    let conn = state.db.lock().unwrap();
    let (keys, total) = ApiKeyRepo::list_by_space(&conn, &space_id, limit, offset)?;
    Ok(Json(ApiResponse::ok(ListApiKeysResponse {
        keys: keys
            .into_iter()
            .map(|k| ApiKeyResponse {
                id: k.id,
                space_id: k.space_id,
                user_id: k.user_id,
                scope: k.scope,
                name: k.name,
                expires_at: k.expires_at,
                last_used_at: k.last_used_at,
                created_at: k.created_at,
            })
            .collect(),
        total,
    })))
}

/// DELETE /api/v1/spaces/:id/api-keys/:key_id — revoke API key.
pub async fn revoke_api_key(
    State(state): State<Arc<AppState>>,
    Extension(actor): Extension<Actor>,
    Path((space_id, key_id)): Path<(String, String)>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    state.auth_service.rbac().check(
        &crate::auth::rbac::AuthContext {
            user_id: actor.user_id.clone(),
            global_role: actor.global_role,
            space_id,
            space_role: actor.space_role,
        },
        crate::auth::model::Permission::ApiKeyManage,
    )?;

    let conn = state.db.lock().unwrap();
    ApiKeyRepo::delete(&conn, &key_id)?;
    Ok(Json(ApiResponse::ok(
        serde_json::json!({"deleted": key_id}),
    )))
}

// ============================================================
// Request / response DTOs
// ============================================================

#[derive(Debug, Deserialize)]
pub struct AgentSearchRequest {
    pub q: String,
    pub space_id: String,
    pub min_trust: Option<f32>,
    pub review_status: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct AgentRememberRequest {
    pub space_id: String,
    pub content: String,
    pub provenance_meta: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct AgentRememberResponse {
    pub id: String,
    pub review_status: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub scope: String,
    pub expires_at: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct CreateApiKeyResponse {
    pub id: String,
    pub key: String,
    pub name: String,
    pub scope: String,
    pub expires_at: Option<i64>,
    pub created_at: i64,
}

#[derive(Debug, Serialize)]
pub struct ApiKeyResponse {
    pub id: String,
    pub space_id: String,
    pub user_id: String,
    pub scope: String,
    pub name: String,
    pub expires_at: Option<i64>,
    pub last_used_at: Option<i64>,
    pub created_at: i64,
}

#[derive(Debug, Deserialize)]
pub struct ListApiKeysQuery {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct ListApiKeysResponse {
    pub keys: Vec<ApiKeyResponse>,
    pub total: usize,
}
