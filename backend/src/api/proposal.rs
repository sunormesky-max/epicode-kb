//! Proposal API endpoints — review queue, approve, reject, modify, batch.

use axum::{extract::{Path, Query, State}, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::dream::proposal::BatchAction;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ListProposalsQuery {
    pub space_id: String,
    pub status: Option<String>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct RejectBody {
    pub feedback: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ModifyBody {
    pub modified_content: String,
}

#[derive(Debug, Deserialize)]
pub struct BatchBody {
    pub action: String,
    pub proposal_ids: Vec<String>,
    pub feedback: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ScanBody {
    pub space_id: String,
}

#[derive(Debug, Serialize)]
struct ApiResponse<T: Serialize> {
    code: i32,
    data: T,
    message: String,
}

fn ok_response<T: Serialize>(data: T) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "code": 0,
        "data": data,
        "message": "ok"
    }))
}

/// GET /api/v1/proposals — list pending proposals.
pub async fn list_proposals(
    State(state): State<Arc<AppState>>,
    Query(q): Query<ListProposalsQuery>,
) -> Json<serde_json::Value> {
    let limit = q.limit.unwrap_or(20).min(100);
    let offset = ((q.page.unwrap_or(1) - 1) * limit).max(0);
    match state.proposal_engine.list(&q.space_id, q.status.as_deref(), limit, offset) {
        Ok(proposals) => ok_response(proposals),
        Err(e) => Json(serde_json::json!({
            "code": 50000,
            "data": null,
            "message": format!("{}", e)
        })),
    }
}

/// POST /api/v1/proposals/:id/approve — approve a proposal.
pub async fn approve_proposal(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let reviewer_id = "system"; // TODO: extract from auth context
    match state.proposal_engine.approve(&id, reviewer_id) {
        Ok(proposal) => ok_response(proposal),
        Err(e) => Json(serde_json::json!({
            "code": 50000,
            "data": null,
            "message": format!("{}", e)
        })),
    }
}

/// POST /api/v1/proposals/:id/reject — reject a proposal.
pub async fn reject_proposal(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<RejectBody>,
) -> Json<serde_json::Value> {
    let reviewer_id = "system";
    match state.proposal_engine.reject(&id, reviewer_id, body.feedback.as_deref()) {
        Ok(proposal) => ok_response(proposal),
        Err(e) => Json(serde_json::json!({
            "code": 50000,
            "data": null,
            "message": format!("{}", e)
        })),
    }
}

/// POST /api/v1/proposals/:id/modify — modify and adopt a proposal.
pub async fn modify_proposal(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<ModifyBody>,
) -> Json<serde_json::Value> {
    let reviewer_id = "system";
    match state.proposal_engine.modify(&id, reviewer_id, &body.modified_content) {
        Ok(proposal) => ok_response(proposal),
        Err(e) => Json(serde_json::json!({
            "code": 50000,
            "data": null,
            "message": format!("{}", e)
        })),
    }
}

/// POST /api/v1/proposals/batch — batch approve/reject.
pub async fn batch_proposals(
    State(state): State<Arc<AppState>>,
    Json(body): Json<BatchBody>,
) -> Json<serde_json::Value> {
    let reviewer_id = "system";
    let action = BatchAction {
        action: body.action,
        proposal_ids: body.proposal_ids,
        feedback: body.feedback,
    };
    match state.proposal_engine.batch(&action, reviewer_id) {
        Ok(proposals) => ok_response(proposals),
        Err(e) => Json(serde_json::json!({
            "code": 50000,
            "data": null,
            "message": format!("{}", e)
        })),
    }
}

/// POST /api/v1/dream/scan — trigger proposal scan.
pub async fn scan_proposals(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ScanBody>,
) -> Json<serde_json::Value> {
    match state.proposal_engine.scan_space(&body.space_id) {
        Ok(proposals) => ok_response(proposals),
        Err(e) => Json(serde_json::json!({
            "code": 50000,
            "data": null,
            "message": format!("{}", e)
        })),
    }
}

/// GET /api/v1/conflicts — list unresolved conflicts.
pub async fn list_conflicts(
    State(_state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    // For now, conflicts are read from memories with provenance='conflict'
    Json(serde_json::json!({
        "code": 0,
        "data": [],
        "message": "no active conflicts"
    }))
}

/// POST /api/v1/conflicts/:id/resolve — resolve a conflict.
pub async fn resolve_conflict(
    Path(_id): Path<String>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "code": 50100,
        "data": null,
        "message": "conflict resolution via standalone API — use memory service resolve_conflict"
    }))
}
