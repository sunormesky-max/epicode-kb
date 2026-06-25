//! Health and wellness API endpoints.

use axum::{extract::{Path, Query, State}, Json};
use serde::Deserialize;
use std::sync::Arc;

use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct HealthScanQuery {
    pub space_id: String,
}

/// GET /api/v1/health/space/:id — latest health snapshot.
pub async fn get_space_health(
    State(state): State<Arc<AppState>>,
    Path(space_id): Path<String>,
) -> Json<serde_json::Value> {
    match &state.health_scanner {
        Some(scanner) => match scanner.full_scan(&space_id) {
            Ok(snap) => Json(serde_json::json!({
                "code": 0,
                "data": snap,
                "message": "ok"
            })),
            Err(e) => Json(serde_json::json!({
                "code": 50000,
                "data": null,
                "message": format!("{}", e)
            })),
        },
        None => Json(serde_json::json!({
            "code": 50100,
            "data": null,
            "message": "health scanner not configured"
        })),
    }
}

/// GET /api/v1/health/gaps — knowledge gaps.
pub async fn get_gaps(
    State(state): State<Arc<AppState>>,
    Query(q): Query<HealthScanQuery>,
) -> Json<serde_json::Value> {
    match &state.health_scanner {
        Some(scanner) => match scanner.scan_gaps(&q.space_id) {
            Ok(gaps) => Json(serde_json::json!({
                "code": 0,
                "data": gaps,
                "message": "ok"
            })),
            Err(e) => Json(serde_json::json!({
                "code": 50000,
                "data": null,
                "message": format!("{}", e)
            })),
        },
        None => Json(serde_json::json!({
            "code": 50100,
            "data": null,
            "message": "health scanner not configured"
        })),
    }
}

/// GET /api/v1/health/stale — stale memories.
pub async fn get_stale(
    State(state): State<Arc<AppState>>,
    Query(q): Query<HealthScanQuery>,
) -> Json<serde_json::Value> {
    match &state.health_scanner {
        Some(scanner) => match scanner.scan_staleness(&q.space_id) {
            Ok(stale) => Json(serde_json::json!({
                "code": 0,
                "data": stale,
                "message": "ok"
            })),
            Err(e) => Json(serde_json::json!({
                "code": 50000,
                "data": null,
                "message": format!("{}", e)
            })),
        },
        None => Json(serde_json::json!({
            "code": 50100,
            "data": null,
            "message": "health scanner not configured"
        })),
    }
}

/// POST /api/v1/health/scan — trigger health scan.
pub async fn trigger_scan(
    State(state): State<Arc<AppState>>,
    Json(body): Json<HealthScanQuery>,
) -> Json<serde_json::Value> {
    match &state.health_scanner {
        Some(scanner) => match scanner.full_scan(&body.space_id) {
            Ok(snap) => Json(serde_json::json!({
                "code": 0,
                "data": snap,
                "message": "scan complete"
            })),
            Err(e) => Json(serde_json::json!({
                "code": 50000,
                "data": null,
                "message": format!("{}", e)
            })),
        },
        None => Json(serde_json::json!({
            "code": 50100,
            "data": null,
            "message": "health scanner not configured"
        })),
    }
}
