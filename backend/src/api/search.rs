//! Search API endpoints.

use std::sync::Arc;

use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;

use crate::api::ApiResponse;
use crate::error::AppError;
use crate::memory::model::{Provenance, RecallRequest, ReviewStatus};
use crate::search::{SearchMode, SearchQuery, SearchResponse};
use crate::state::AppState;

/// Query parameters for GET /search.
#[derive(Debug, Deserialize)]
pub struct SearchParams {
    pub q: String,
    pub space_id: String,
    pub mode: Option<String>,
    pub min_trust: Option<f32>,
    pub provenance: Option<String>,
    pub review_status: Option<String>,
    pub visibility: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// GET /api/v1/search — hybrid search.
pub async fn search(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchParams>,
) -> Result<Json<ApiResponse<SearchResponse>>, AppError> {
    let mode = params
        .mode
        .as_ref()
        .map(|s| SearchMode::parse_str(s))
        .unwrap_or_default();

    let provenance: Option<Vec<Provenance>> = params.provenance.as_ref().map(|s| {
        s.split(',')
            .filter_map(|p| Provenance::parse_str(p.trim()).ok())
            .collect()
    });

    let review_status = params
        .review_status
        .as_ref()
        .and_then(|s| ReviewStatus::parse_str(s).ok());

    let visibility = params
        .visibility
        .as_ref()
        .and_then(|s| crate::memory::model::Visibility::parse_str(s).ok());

    let query = SearchQuery {
        q: params.q,
        space_id: params.space_id,
        mode,
        min_trust: params.min_trust,
        provenance,
        review_status,
        visibility,
        limit: params.limit.unwrap_or(20).min(100),
        offset: params.offset.unwrap_or(0),
    };

    let response = state.search_engine.search(&query)?;

    Ok(Json(ApiResponse::ok(response)))
}

/// POST /api/v1/recall — context recall.
pub async fn recall(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RecallRequest>,
) -> Result<Json<ApiResponse<SearchResponse>>, AppError> {
    if req.context.trim().is_empty() {
        return Err(AppError::bad_request("context must not be empty"));
    }

    let query = SearchQuery {
        q: req.context,
        space_id: req.space_id,
        mode: SearchMode::Hybrid,
        min_trust: None,
        provenance: None,
        review_status: Some(ReviewStatus::Accepted),
        visibility: None,
        limit: req.limit.min(50),
        offset: 0,
    };

    let response = state.search_engine.search(&query)?;

    Ok(Json(ApiResponse::ok(response)))
}
