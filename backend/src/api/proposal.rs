//! Proposal API endpoints (stub — Sprint 3).
//!
//! All endpoints return 501 Not Implemented.

use axum::{extract::Path, Json};
use serde_json::json;

/// GET /api/v1/proposals — list pending proposals (stub).
pub async fn list_proposals() -> Json<serde_json::Value> {
    Json(json!({
        "code": 50100,
        "data": null,
        "message": "not implemented: list proposals (planned for Sprint 3)"
    }))
}

/// POST /api/v1/proposals/:id/approve — approve a proposal (stub).
pub async fn approve_proposal(Path(_id): Path<String>) -> Json<serde_json::Value> {
    Json(json!({
        "code": 50100,
        "data": null,
        "message": "not implemented: approve proposal (planned for Sprint 3)"
    }))
}

/// POST /api/v1/proposals/:id/reject — reject a proposal (stub).
pub async fn reject_proposal(Path(_id): Path<String>) -> Json<serde_json::Value> {
    Json(json!({
        "code": 50100,
        "data": null,
        "message": "not implemented: reject proposal (planned for Sprint 3)"
    }))
}

/// POST /api/v1/proposals/:id/modify — modify and adopt a proposal (stub).
pub async fn modify_proposal(Path(_id): Path<String>) -> Json<serde_json::Value> {
    Json(json!({
        "code": 50100,
        "data": null,
        "message": "not implemented: modify proposal (planned for Sprint 3)"
    }))
}

/// GET /api/v1/conflicts — list unresolved conflicts (stub).
pub async fn list_conflicts() -> Json<serde_json::Value> {
    Json(json!({
        "code": 50100,
        "data": null,
        "message": "not implemented: list conflicts (planned for Sprint 4)"
    }))
}

/// POST /api/v1/conflicts/:id/resolve — resolve a conflict (stub).
pub async fn resolve_conflict(Path(_id): Path<String>) -> Json<serde_json::Value> {
    Json(json!({
        "code": 50100,
        "data": null,
        "message": "not implemented: resolve conflict (planned for Sprint 4)"
    }))
}

/// GET /api/v1/notifications — list notifications (stub).
pub async fn list_notifications() -> Json<serde_json::Value> {
    Json(json!({
        "code": 50100,
        "data": null,
        "message": "not implemented: list notifications (planned for Sprint 3)"
    }))
}
