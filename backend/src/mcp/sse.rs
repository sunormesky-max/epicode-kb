//! MCP SSE transport adapter.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::{
    extract::{Path, State},
    response::sse::{Event, Sse},
    Json,
};
use serde::Deserialize;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use crate::error::{AppError, AppResult};
use crate::mcp::server::{McpRequest, McpResponse, McpServer};

/// Session store for SSE connections.
#[derive(Clone, Default)]
pub struct SseSessionStore {
    sessions: Arc<Mutex<HashMap<String, mpsc::Sender<String>>>>,
}

impl SseSessionStore {
    /// Create a new session store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new session and return its ID.
    pub fn create_session(&self) -> (String, mpsc::Receiver<String>) {
        let id = crate::generate_id("mcp_session");
        let (tx, rx) = mpsc::channel::<String>(64);
        self.sessions.lock().unwrap().insert(id.clone(), tx);
        (id, rx)
    }

    /// Send a message to a session.
    pub fn send(&self, session_id: &str, message: String) -> AppResult<()> {
        let sessions = self.sessions.lock().unwrap();
        let tx = sessions
            .get(session_id)
            .ok_or_else(|| AppError::not_found("session not found"))?;
        tx.try_send(message)
            .map_err(|e| AppError::internal(format!("failed to send to session: {}", e)))?;
        Ok(())
    }
}

/// Query parameters for SSE connection.
#[derive(Debug, Deserialize)]
pub struct SseQuery {
    pub session_id: Option<String>,
}

/// SSE endpoint for MCP clients.
pub async fn sse_handler(
    State((_server, store)): State<(Arc<McpServer>, SseSessionStore)>,
) -> Sse<ReceiverStream<Result<Event, std::convert::Infallible>>> {
    let (session_id, mut rx) = store.create_session();
    let (tx, stream) = mpsc::channel::<Result<Event, std::convert::Infallible>>(4);

    let endpoint = format!("/api/v1/mcp/sse/session/{}", session_id);
    let _ = tx
        .send(Ok(Event::default().event("endpoint").data(endpoint)))
        .await;

    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if tx
                .send(Ok(Event::default().event("message").data(msg)))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    Sse::new(ReceiverStream::new(stream))
}

/// POST endpoint for MCP messages over SSE session.
pub async fn sse_message_handler(
    State((server, store)): State<(Arc<McpServer>, SseSessionStore)>,
    Path(session_id): Path<String>,
    Json(req): Json<McpRequest>,
) -> Result<Json<McpResponse>, AppError> {
    let resp = server.handle_request(req).await;
    let json = serde_json::to_string(&resp).unwrap();
    let _ = store.send(&session_id, json);
    Ok(Json(resp))
}
