//! Auth API endpoints (stub — Sprint 2).
//!
//! All endpoints return 501 Not Implemented.

use axum::Json;
use serde_json::json;

/// POST /api/v1/auth/login — login (stub).
pub async fn login() -> Json<serde_json::Value> {
    Json(json!({
        "code": 50100,
        "data": null,
        "message": "not implemented: auth login (planned for Sprint 2)"
    }))
}

/// GET /api/v1/spaces — list spaces (stub).
pub async fn list_spaces() -> Json<serde_json::Value> {
    // Return the default space for Sprint 1 single-user mode
    Json(json!({
        "code": 0,
        "data": {
            "spaces": [
                {
                    "id": "sp_default",
                    "name": "Default Space",
                    "description": "Default workspace for epicode-kb",
                    "ai_write_enabled": true,
                    "default_ai_trust_level": 0.5
                }
            ],
            "total": 1
        },
        "message": "ok"
    }))
}

/// POST /api/v1/spaces — create space (stub).
pub async fn create_space() -> Json<serde_json::Value> {
    Json(json!({
        "code": 50100,
        "data": null,
        "message": "not implemented: create space (planned for Sprint 2)"
    }))
}
