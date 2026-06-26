//! Collaboration WebSocket endpoint + editor context (real-time conflict detection).

use std::sync::Arc;

use axum::{
    extract::{Extension, Path, Query, State, WebSocketUpgrade},
    response::Response,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::auth::model::Actor;
use crate::collab::protocol::CollabMessage;
use crate::conflict::detect::{contradiction_score, cosine_similarity, jaccard_similarity, CONTRADICTION_THRESHOLD};
use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ContextParams {
    pub memory_id: String,
    /// Recent text near the cursor (e.g. the last paragraph).
    pub cursor: String,
    pub space_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct ContextItem {
    id: String,
    content: String,
    provenance: String,
    trust_level: f32,
    semantic_distance: f32,
}

#[derive(Debug, Serialize)]
pub struct ContextResponse {
    related: Vec<ContextItem>,
    warnings: Vec<String>,
}

/// GET /api/v1/collab/context — editor context recall + real-time conflict detection.
///
/// Given the text around the cursor, returns the top semantically-related
/// memories and flags potential contradictions against existing knowledge.
pub async fn get_context(
    State(state): State<Arc<AppState>>,
    Extension(actor): Extension<Actor>,
    Query(params): Query<ContextParams>,
) -> Result<Json<crate::api::ApiResponse<ContextResponse>>, AppError> {
    if params.cursor.trim().is_empty() {
        return Ok(Json(crate::api::ApiResponse::ok(ContextResponse {
            related: vec![],
            warnings: vec![],
        })));
    }

    let space_id = params
        .space_id
        .or_else(|| actor.space_role.as_ref().map(|_| "sp_default".to_string()))
        .unwrap_or_else(|| "sp_default".to_string());

    // Embed the cursor text.
    let cursor_vec = match state.embedder.embed(&params.cursor) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("context embed failed: {}", e);
            return Ok(Json(crate::api::ApiResponse::ok(ContextResponse {
                related: vec![],
                warnings: vec![],
            })));
        }
    };

    // Load accepted memories (with embeddings) in the space.
    let conn = state.db.lock().map_err(|e| {
        AppError::internal(format!("db lock: {}", e))
    })?;
    let candidates: Vec<(String, String, String, String, f32, Vec<f32>)> = {
        let mut stmt = conn
            .prepare(
                "SELECT id, content, provenance, embedding_model, trust_level, embedding
                 FROM memories
                 WHERE space_id = ?1
                   AND review_status = 'accepted'
                   AND embedding IS NOT NULL
                   AND id != ?2
                 LIMIT 200",
            )
            .map_err(AppError::db)?;
        let rows = stmt
            .query_map(rusqlite::params![space_id, params.memory_id], |row| {
                let blob: Vec<u8> = row.get(5)?;
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, f32>(4)?,
                    crate::db::repository::blob_to_embedding(&blob),
                ))
            })
            .map_err(AppError::db)?;
        let mut out = Vec::new();
        for r in rows.flatten() {
            out.push(r);
        }
        out
    };
    drop(conn);

    // Score each candidate: similarity for recall, contradiction heuristic for warnings.
    let mut scored: Vec<(f32, String, String, String, f32)> = candidates
        .iter()
        .filter_map(|(id, content, provenance, _model, trust, emb)| {
            if emb.len() != cursor_vec.len() {
                return None;
            }
            let sim = cosine_similarity(&cursor_vec, emb);
            let sem_dist = 1.0 - sim;
            Some((sem_dist, id.clone(), content.clone(), provenance.clone(), *trust))
        })
        .collect();
    scored.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    let related: Vec<ContextItem> = scored
        .iter()
        .take(5)
        .map(|(sem_dist, id, content, provenance, trust)| ContextItem {
            id: id.clone(),
            content: content.clone(),
            provenance: provenance.clone(),
            trust_level: *trust,
            semantic_distance: *sem_dist,
        })
        .collect();

    // Warnings: flag memories that are semantically close but lexically divergent.
    let mut warnings: Vec<String> = Vec::new();
    for (sem_dist, _id, content, _provenance, _trust) in &scored {
        if *sem_dist > 0.3 {
            break;
        }
        let jac = jaccard_similarity(&params.cursor, content);
        let score = contradiction_score(*sem_dist, jac);
        if score > CONTRADICTION_THRESHOLD {
            warnings.push(format!(
                "This may contradict existing knowledge (confidence {:.0}%): \"{}\"",
                score * 100.0,
                content.chars().take(120).collect::<String>()
            ));
            if warnings.len() >= 3 {
                break;
            }
        }
    }

    Ok(Json(crate::api::ApiResponse::ok(ContextResponse {
        related,
        warnings,
    })))
}

/// WebSocket handler for /api/v1/collab/:memory_id.
pub async fn collab_ws(
    State(state): State<Arc<AppState>>,
    Extension(_actor): Extension<Actor>,
    Path(memory_id): Path<String>,
    ws: WebSocketUpgrade,
) -> Result<Response, AppError> {
    let room_manager = state.room_manager.clone();
    let room = room_manager.get_or_create(&memory_id)?;

    Ok(ws.on_upgrade(move |mut socket| async move {
        // Send initial sync step 1 with server state vector.
        let state_vector = {
            let room_guard = room.lock().unwrap();
            room_guard.state_vector()
        };
        let msg = CollabMessage::SyncStep1(state_vector);
        let _ = socket
            .send(axum::extract::ws::Message::Binary(msg.encode()))
            .await;

        // Add socket as subscriber and handle incoming messages.
        {
            let mut room_guard = room.lock().unwrap();
            room_guard.add_subscriber(socket);
        }
    }))
}
