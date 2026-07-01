//! Collaboration WebSocket endpoint + editor context (real-time conflict detection).

use std::sync::Arc;

use axum::{
    extract::{Extension, Path, Query, State, WebSocketUpgrade},
    response::Response,
    Json,
};
use axum::extract::ws::{Message, WebSocket};
use futures::{stream::StreamExt, SinkExt};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedReceiver;

use crate::auth::model::Actor;
use crate::conflict::detect::{contradiction_score, cosine_similarity, jaccard_similarity, CONTRADICTION_THRESHOLD};
use crate::error::AppError;
use crate::state::AppState;

/// Maximum WebSocket message size in bytes (16 MB).
const MAX_WS_MESSAGE_SIZE: usize = 16 * 1024 * 1024;

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
                "This may contradict existing knowledge (semantic distance={:.3}, Jaccard={:.3})",
                sem_dist, jac
            ));
        }
    }

    Ok(Json(crate::api::ApiResponse::ok(ContextResponse {
        related,
        warnings,
    })))
}

/// WebSocket upgrade handler for real-time collaboration.
///
/// Authenticates the client via the `?token=` query parameter before upgrading.
pub async fn collab_ws(
    State(state): State<Arc<AppState>>,
    Path(memory_id): Path<String>,
    ws: WebSocketUpgrade,
) -> Response {
    // The auth middleware already validated the token from ?token= query param
    // and injected the Actor into extensions. We just need to accept the upgrade.
    ws.on_upgrade(move |socket| handle_collab_socket(state, memory_id, socket))
}

/// Handle an individual WebSocket connection for a collaboration room.
async fn handle_collab_socket(
    state: Arc<AppState>,
    memory_id: String,
    socket: WebSocket,
) {
    use yrs::sync::protocol::Message as YMessage;
    use yrs::sync::protocol::SyncMessage;

    let room = match state.room_manager.get_or_create(&memory_id) {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("Failed to get/create room {}: {}", memory_id, e);
            return;
        }
    };

    let (mut ws_sink, mut ws_stream) = socket.split();

    // Create a broadcast channel for this subscriber.
    let (tx, mut rx): (tokio::sync::mpsc::UnboundedSender<Message>, UnboundedReceiver<Message>) =
        tokio::sync::mpsc::unbounded_channel();

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

    // Replay last-seen awareness to the new client so it learns existing peers.
    let awareness_frame = {
        let room_guard = room.lock().unwrap();
        room_guard.last_awareness().map(|b| b.to_vec())
    };
    if let Some(frame) = awareness_frame {
        let _ = ws_sink.send(Message::Binary(frame)).await;
    }

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

        // Enforce message size limit
        let payload: Vec<u8> = match msg {
            Message::Binary(b) => {
                if b.len() > MAX_WS_MESSAGE_SIZE {
                    tracing::warn!("WebSocket message too large ({} bytes), dropping", b.len());
                    continue;
                }
                b
            }
            Message::Text(t) => {
                if t.len() > MAX_WS_MESSAGE_SIZE {
                    tracing::warn!("WebSocket message too large ({} bytes), dropping", t.len());
                    continue;
                }
                t.into_bytes()
            }
            Message::Ping(_) | Message::Pong(_) | Message::Close(_) => continue,
        };

        let ymsg = match crate::collab::protocol::read_message(&payload) {
            Some(m) => {
                tracing::debug!("collab recv msg variant: {}", msg_variant_name(&m));
                m
            }
            None => {
                let hex: String = payload.iter().map(|b| format!("{:02x}", b)).collect();
                tracing::warn!(
                    "collab: dropping undecodable message ({} bytes): {}",
                    payload.len(),
                    hex
                );
                continue;
            }
        };

        match ymsg {
            YMessage::Sync(sync_msg) => match sync_msg {
                SyncMessage::SyncStep1(remote_sv) => {
                    // Client asks for our diff: reply SyncStep2. We broadcast to
                    // ALL subscribers (exclude None) because the requester must
                    // receive it, and others simply ignore content they already have.
                    let sv_bytes = yrs::updates::encoder::Encode::encode_v1(&remote_sv);
                    let diff = {
                        let room_guard = room.lock().unwrap();
                        room_guard.diff_update(&sv_bytes).unwrap_or_default()
                    };
                    let frame = crate::collab::protocol::sync_step2(diff);
                    room.lock().unwrap().broadcast(frame, None);
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
                // New client asks for current awareness: replay last frame.
                let frame = {
                    let room_guard = room.lock().unwrap();
                    room_guard
                        .last_awareness()
                        .map(|b| b.to_vec())
                        .filter(|b| !b.is_empty())
                };
                if let Some(bytes) = frame {
                    room.lock().unwrap().broadcast(bytes, None);
                }
            }
            YMessage::Awareness(_au) => {
                // Client awareness update: record it, then forward to others.
                {
                    let mut room_guard = room.lock().unwrap();
                    room_guard.record_awareness(payload.clone());
                    room_guard.broadcast(payload, Some(sub_id));
                }
            }
            _ => {}
        }
    }

    // Cleanup: remove subscriber, then gracefully abort and await the writer task.
    room.lock().unwrap().remove_subscriber(sub_id);
    writer.abort();
    // Give the writer task a moment to clean up, but don't block indefinitely.
    let _ = tokio::time::timeout(std::time::Duration::from_secs(2), writer).await;
}

/// Human-readable name for a yjs Message variant (debug logging).
fn msg_variant_name(msg: &yrs::sync::protocol::Message) -> &'static str {
    use yrs::sync::protocol::{Message, SyncMessage};
    match msg {
        Message::Sync(SyncMessage::SyncStep1(_)) => "SyncStep1",
        Message::Sync(SyncMessage::SyncStep2(_)) => "SyncStep2",
        Message::Sync(SyncMessage::Update(_)) => "Update",
        Message::Awareness(_) => "Awareness",
        Message::AwarenessQuery => "AwarenessQuery",
        _ => "Other",
    }
}
