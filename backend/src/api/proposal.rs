//! Proposal API endpoints — review queue, approve, reject, modify, batch.

use axum::{extract::{Path, Query, State}, Json};
use serde::Deserialize;
use std::sync::Arc;

use crate::dream::proposal::BatchAction;
use crate::memory::model::{Provenance, ReviewStatus};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ListProposalsQuery {
    pub space_id: String,
    pub status: Option<String>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ListConflictsQuery {
    pub space_id: Option<String>,
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

#[derive(Debug, Deserialize)]
pub struct ResolveConflictBody {
    /// "accept_a" | "accept_b" | "both_true"
    pub resolution: String,
}

fn ok_response<T: serde::Serialize>(data: T) -> Json<serde_json::Value> {
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

/// GET /api/v1/conflicts — list unresolved conflict-provenance memories.
///
/// Conflict memories carry `provenance = 'conflict'` and a `provenance_meta`
/// JSON blob with the two conflicting source memories (`conflicting_ids`).
pub async fn list_conflicts(
    State(state): State<Arc<AppState>>,
    Query(q): Query<ListConflictsQuery>,
) -> Json<serde_json::Value> {
    let space_id = q.space_id.unwrap_or_else(|| "sp_default".to_string());
    let conn = match state.db.lock() {
        Ok(c) => c,
        Err(e) => {
            return Json(serde_json::json!({
                "code": 50000,
                "data": null,
                "message": format!("db lock: {}", e)
            }))
        }
    };

    let (memories, _total) = match crate::db::repository::MemoryRepo::list(
        &conn,
        &space_id,
        Some(&[Provenance::Conflict]),
        None,
        Some(ReviewStatus::Pending),
        None,
        100,
        0,
    ) {
        Ok(v) => v,
        Err(e) => {
            return Json(serde_json::json!({
                "code": 50000,
                "data": null,
                "message": format!("{}", e)
            }))
        }
    };

    let items: Vec<serde_json::Value> = memories
        .iter()
        .map(|m| {
            // Parse conflicting source ids + their contents from provenance_meta.
            let (id_a, id_b, content_a, content_b, confidence) = m
                .provenance_meta
                .as_ref()
                .and_then(|meta| {
                    let arr = meta.get("conflicting_ids")?.as_array()?;
                    let id_a = arr.first()?.as_str().map(String::from)?;
                    let id_b = arr.get(1)?.as_str().map(String::from)?;
                    let content_a = crate::db::repository::MemoryRepo::get_by_id(&conn, &id_a)
                        .ok()
                        .map(|x| x.content);
                    let content_b = crate::db::repository::MemoryRepo::get_by_id(&conn, &id_b)
                        .ok()
                        .map(|x| x.content);
                    let confidence = meta.get("confidence").and_then(|v| v.as_f64());
                    Some((id_a, id_b, content_a, content_b, confidence))
                })
                .unwrap_or_default();

            serde_json::json!({
                "id": m.id,
                "content": m.content,
                "conflicting_id_a": id_a,
                "conflicting_id_b": id_b,
                "conflicting_content_a": content_a,
                "conflicting_content_b": content_b,
                "confidence": confidence,
                "created_at": m.created_at,
            })
        })
        .collect();

    ok_response(items)
}

/// POST /api/v1/conflicts/:id/resolve — resolve a conflict memory.
///
/// Marks the conflict-provenance memory as resolved (accepted) and records
/// the chosen resolution in its `provenance_meta`.
pub async fn resolve_conflict(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<ResolveConflictBody>,
) -> Json<serde_json::Value> {
    let resolution = body.resolution.as_str();
    if !matches!(resolution, "accept_a" | "accept_b" | "both_true") {
        return Json(serde_json::json!({
            "code": 40000,
            "data": null,
            "message": "resolution must be one of: accept_a, accept_b, both_true"
        }));
    }

    let conn = match state.db.lock() {
        Ok(c) => c,
        Err(e) => {
            return Json(serde_json::json!({
                "code": 50000,
                "data": null,
                "message": format!("db lock: {}", e)
            }))
        }
    };

    // Load the conflict memory and its existing meta.
    let memory = match crate::db::repository::MemoryRepo::get_by_id(&conn, &id) {
        Ok(m) => m,
        Err(e) => {
            return Json(serde_json::json!({
                "code": 40400,
                "data": null,
                "message": format!("conflict not found: {}", e)
            }))
        }
    };
    if memory.provenance != Provenance::Conflict {
        return Json(serde_json::json!({
            "code": 40000,
            "data": null,
            "message": "memory is not a conflict record"
        }));
    }

    // Record the resolution into provenance_meta, then mark resolved.
    let mut meta = memory.provenance_meta.clone().unwrap_or_else(|| serde_json::json!({}));
    if let Some(obj) = meta.as_object_mut() {
        obj.insert("resolution".into(), serde_json::json!(resolution));
        obj.insert("resolved_at".into(), serde_json::json!(crate::now_ts()));
    } else {
        meta = serde_json::json!({ "resolution": resolution, "resolved_at": crate::now_ts() });
    }

    if let Err(e) = crate::db::repository::MemoryRepo::set_provenance_meta(&conn, &id, &meta) {
        return Json(serde_json::json!({
            "code": 50000,
            "data": null,
            "message": format!("{}", e)
        }));
    }
    if let Err(e) = crate::db::repository::MemoryRepo::update_review_status(
        &conn,
        &id,
        ReviewStatus::Accepted,
        None,
        crate::now_ts(),
    ) {
        return Json(serde_json::json!({
            "code": 50000,
            "data": null,
            "message": format!("{}", e)
        }));
    }

    ok_response(serde_json::json!({
        "id": id,
        "resolution": resolution,
        "status": "resolved"
    }))
}
