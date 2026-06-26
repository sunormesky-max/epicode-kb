//! Collaboration WebSocket endpoint + editor context (real-time conflict detection).

use std::sync::Arc;

use axum::{
    extract::{Extension, Path, Query, State, WebSocketUpgrade},
    response::Response,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::auth::model::Actor;
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

/// WebSocket handler for /api/v1/collab/:memory_id (standard yjs protocol).
pub async fn collab_ws(
    State(state): State<Arc<AppState>>,
    Extension(_actor): Extension<Actor>,
    Path(memory_id): Path<String>,
    ws: WebSocketUpgrade,
) -> Result<Response, AppError> {
    let room_manager = state.room_manager.clone();
    let room = room_manager.get_or_create(&memory_id)?;

    Ok(ws.on_upgrade(move |socket| async move {
        if let Err(e) = handle_collab_socket(socket, room).await {
            tracing::warn!("collab socket error for {}: {}", memory_id, e);
        }
    }))
}

/// Drive a single WebSocket connection through the standard yjs sync protocol.
async fn handle_collab_socket(
    socket: axum::extract::ws::WebSocket,
    room: Arc<std::sync::Mutex<crate::collab::room::CollaborationRoom>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use axum::extract::ws::Message;
    use futures::{SinkExt, StreamExt};
    use tokio::sync::mpsc;
    use yrs::sync::protocol::{Message as YMessage, SyncMessage};

    let (mut ws_sink, mut ws_stream) = socket.split();

    // Outbound channel: room broadcasts push Message here; a writer task drains it.
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
    let sub_id = {
        let mut room_guard = room.lock().unwrap();
        room_guard.add_subscriber(tx)
    };

    // Server-initiated sync step 1: send our state vector to the new client.
    let init_frame = {
        let room_guard = room.lock().unwrap();
        let sv = room_guard.state_vector();
        crate::collab::protocol::sync_step1(&sv)
    };
    let _ = ws_sink.send(Message::Binary(init_frame)).await;

    // Writer task: drain the broadcast channel into the socket.
    let writer = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_sink.send(msg).await.is_err() {
                break;
            }
        }
    });

    // Reader loop: socket → process standard yjs messages.
    while let Some(msg) = ws_stream.next().await {
        let msg = match msg {
            Ok(m) => m,
            Err(_) => break,
        };
        let payload: Vec<u8> = match msg {
            Message::Binary(b) => b,
            Message::Text(t) => t.into_bytes(),
            Message::Ping(_) | Message::Pong(_) | Message::Close(_) => continue,
        };

        let ymsg = match crate::collab::protocol::read_message(&payload) {
            Some(m) => m,
            None => {
                tracing::debug!("collab: dropping undecodable message");
                continue;
            }
        };

        match ymsg {
            YMessage::Sync(sync_msg) => match sync_msg {
                SyncMessage::SyncStep1(remote_sv) => {
                    // Client asks for our diff: reply SyncStep2 to it directly.
                    let sv_bytes = yrs::updates::encoder::Encode::encode_v1(&remote_sv);
                    let diff = {
                        let room_guard = room.lock().unwrap();
                        room_guard.diff_update(&sv_bytes).unwrap_or_default()
                    };
                    let frame = crate::collab::protocol::sync_step2(diff);
                    let _ = room.lock().unwrap().broadcast(frame, Some(sub_id));
                    // Also request the client's state so we converge.
                    let sv = {
                        let room_guard = room.lock().unwrap();
                        room_guard.state_vector()
                    };
                    let req = crate::collab::protocol::sync_step1(&sv);
                    let _ = room.lock().unwrap().broadcast(req, Some(sub_id));
                }
                SyncMessage::SyncStep2(_) => {
                    // Client's missing diff for us; nothing to broadcast.
                }
                SyncMessage::Update(update) => {
                    // Apply locally then re-broadcast the original Update frame to others.
                    let frame = payload.clone();
                    {
                        let mut room_guard = room.lock().unwrap();
                        let _ = room_guard.apply_update(&update);
                        room_guard.broadcast(frame, Some(sub_id));
                    }
                }
            },
            YMessage::AwarenessQuery => {
                // Client wants full awareness: nothing to synthesize server-side
                // (we forward awareness opaquely). No-op; clients exchange
                // awareness updates directly via broadcast below.
            }
            YMessage::Awareness(_au) => {
                // Client awareness update: forward raw bytes to other subscribers.
                // We don't maintain a server-side Awareness (it isn't Send/Sync);
                // clients each track awareness and reconcile.
                let _ = room
                    .lock()
                    .unwrap()
                    .broadcast(payload, Some(sub_id));
            }
            _ => {}
        }
    }

    // Cleanup: remove subscriber, drop writer.
    room.lock().unwrap().remove_subscriber(sub_id);
    writer.abort();
    Ok(())
}
