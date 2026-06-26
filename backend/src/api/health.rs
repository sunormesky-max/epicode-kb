//! Health and system API endpoints.

use std::sync::Arc;

use axum::{extract::{Path, Query, State}, response::Response, Json};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::conflict::detect::jaccard_similarity;
use crate::error::AppError;
use crate::memory::model::Provenance;
use crate::state::AppState;

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

/// GET /api/v1/health/live — liveness probe.
pub async fn live() -> Json<serde_json::Value> {
    Json(json!({
        "code": 0,
        "data": { "status": "ok" },
        "message": "ok"
    }))
}

/// GET /api/v1/health/ready — readiness probe.
pub async fn ready(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let _guard = state.db.lock().unwrap();
    Json(json!({
        "code": 0,
        "data": { "status": "ok" },
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

#[derive(Debug, Deserialize)]
pub struct GraphQuery {
    pub space_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct GraphNode {
    id: String,
    label: String,
    provenance: String,
    trust_level: f32,
}

#[derive(Debug, Serialize)]
struct GraphEdge {
    source: String,
    target: String,
    /// "conflict" (red dashed) or "similar" (gray solid).
    #[serde(rename = "type")]
    edge_type: String,
    confidence: Option<f32>,
}

#[derive(Debug, Serialize)]
struct GraphData {
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
}

/// GET /api/v1/graph — knowledge graph with conflict & similarity edges.
///
/// Nodes are accepted memories; edges are either contradictions (from
/// conflict-provenance memories) or content-similarity links (Jaccard > 0.6).
pub async fn graph(
    State(state): State<Arc<AppState>>,
    Query(q): Query<GraphQuery>,
) -> Json<serde_json::Value> {
    let space_id = q.space_id.unwrap_or_else(|| "sp_default".to_string());

    let conn = match state.db.lock() {
        Ok(c) => c,
        Err(e) => {
            return Json(json!({
                "code": 50000, "data": null,
                "message": format!("db lock: {}", e)
            }))
        }
    };

    // Nodes: accepted memories.
    let nodes: Vec<GraphNode> = {
        let mut stmt = match conn.prepare(
            "SELECT id, content, provenance, trust_level FROM memories
             WHERE space_id = ?1 AND review_status = 'accepted' LIMIT 300",
        ) {
            Ok(s) => s,
            Err(e) => {
                return Json(json!({
                    "code": 50000, "data": null, "message": format!("{}", e)
                }))
            }
        };
        let rows = stmt
            .query_map(rusqlite::params![space_id], |row| {
                let content: String = row.get(1)?;
                let label: String = content.chars().take(40).collect();
                Ok(GraphNode {
                    id: row.get(0)?,
                    label,
                    provenance: row.get(2)?,
                    trust_level: row.get(3)?,
                })
            });
        match rows {
            Ok(r) => r.filter_map(|x| x.ok()).collect(),
            Err(e) => {
                return Json(json!({
                    "code": 50000, "data": null, "message": format!("{}", e)
                }))
            }
        }
    };

    // Conflict edges: derived from conflict-provenance memories' provenance_meta.
    let mut edges: Vec<GraphEdge> = Vec::new();
    {
        let conflict_mems = match crate::db::repository::MemoryRepo::list(
            &conn,
            &space_id,
            Some(&[Provenance::Conflict]),
            None,
            None,
            None,
            200,
            0,
        ) {
            Ok((ms, _)) => ms,
            Err(e) => {
                return Json(json!({
                    "code": 50000, "data": null, "message": format!("{}", e)
                }))
            }
        };
        for m in &conflict_mems {
            if let Some(meta) = &m.provenance_meta {
                if let Some(ids) = meta.get("conflicting_ids").and_then(|v| v.as_array()) {
                    if let (Some(a), Some(b)) = (ids.first(), ids.get(1)) {
                        if let (Some(a), Some(b)) = (a.as_str(), b.as_str()) {
                            let confidence = meta
                                .get("confidence")
                                .and_then(|v| v.as_f64())
                                .map(|f| f as f32);
                            edges.push(GraphEdge {
                                source: a.to_string(),
                                target: b.to_string(),
                                edge_type: "conflict".to_string(),
                                confidence,
                            });
                        }
                    }
                }
            }
        }
    }

    // Similarity edges: Jaccard > 0.6 between accepted memory contents.
    let pairs: Vec<(String, String)> = {
        let mut stmt = match conn.prepare(
            "SELECT id, content FROM memories
             WHERE space_id = ?1 AND review_status = 'accepted' LIMIT 100",
        ) {
            Ok(s) => s,
            Err(e) => {
                return Json(json!({
                    "code": 50000, "data": null, "message": format!("{}", e)
                }))
            }
        };
        let rows = stmt.query_map(rusqlite::params![space_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        });
        match rows {
            Ok(r) => r.filter_map(|x| x.ok()).collect(),
            Err(e) => {
                return Json(json!({
                    "code": 50000, "data": null, "message": format!("{}", e)
                }))
            }
        }
    };
    for i in 0..pairs.len() {
        for j in (i + 1)..pairs.len() {
            let jac = jaccard_similarity(&pairs[i].1, &pairs[j].1);
            if jac > 0.6 {
                edges.push(GraphEdge {
                    source: pairs[i].0.clone(),
                    target: pairs[j].0.clone(),
                    edge_type: "similar".to_string(),
                    confidence: Some(jac),
                });
            }
        }
    }

    let data = GraphData { nodes, edges };
    Json(json!({ "code": 0, "data": data, "message": "ok" }))
}

/// GET /api/v1/metrics — Prometheus metrics.
pub async fn metrics(State(state): State<Arc<AppState>>) -> Result<Response, AppError> {
    let output = state.metrics.gather()?;
    Ok(Response::builder()
        .header("content-type", "text/plain; charset=utf-8")
        .body(axum::body::Body::from(output))
        .unwrap())
}

/// System health response.
#[derive(Debug, Serialize)]
pub struct SystemHealth {
    pub status: String,
    pub version: String,
}
