//! Health and system API endpoints.

use axum::{extract::Path, Json};
use serde::Serialize;
use serde_json::json;

/// GET /api/v1/system/health — system health check.
pub async fn system_health() -> Json<serde_json::Value> {
    Json(json!({
        "code": 0,
        "data": {
            "status": "ok",
            "version": env!("CARGO_PKG_VERSION"),
            "uptime_seconds": 0
        },
        "message": "ok"
    }))
}

/// GET /api/v1/system/version — version information.
pub async fn system_version() -> Json<serde_json::Value> {
    Json(json!({
        "code": 0,
        "data": {
            "version": env!("CARGO_PKG_VERSION"),
            "name": env!("CARGO_PKG_NAME"),
            "description": env!("CARGO_PKG_DESCRIPTION"),
        },
        "message": "ok"
    }))
}

/// GET /api/v1/health/space/:id — space health (stub).
pub async fn space_health(Path(_id): Path<String>) -> Json<serde_json::Value> {
    Json(json!({
        "code": 50100,
        "data": null,
        "message": "not implemented: space health (planned for Sprint 5)"
    }))
}

/// GET /api/v1/health/gaps — knowledge gaps (stub).
pub async fn knowledge_gaps() -> Json<serde_json::Value> {
    Json(json!({
        "code": 50100,
        "data": null,
        "message": "not implemented: knowledge gaps (planned for Sprint 5)"
    }))
}

/// POST /api/v1/health/scan — trigger health scan (stub).
pub async fn scan() -> Json<serde_json::Value> {
    Json(json!({
        "code": 50100,
        "data": null,
        "message": "not implemented: health scan (planned for Sprint 5)"
    }))
}

/// GET /api/v1/graph — knowledge graph data (stub).
pub async fn graph() -> Json<serde_json::Value> {
    Json(json!({
        "code": 50100,
        "data": null,
        "message": "not implemented: knowledge graph (planned for Sprint 2)"
    }))
}

/// System health response.
#[derive(Debug, Serialize)]
pub struct SystemHealth {
    pub status: String,
    pub version: String,
}
