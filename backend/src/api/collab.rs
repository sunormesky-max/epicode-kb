//! Collaboration WebSocket endpoint.

use std::sync::Arc;

use axum::{
    extract::{Extension, Path, State, WebSocketUpgrade},
    response::Response,
};

use crate::auth::model::Actor;
use crate::collab::protocol::CollabMessage;
use crate::error::AppError;
use crate::state::AppState;

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
