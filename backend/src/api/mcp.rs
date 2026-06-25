//! MCP HTTP/SSE API endpoints.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    response::sse::Sse,
    Extension, Json,
};

use crate::api::ApiResponse;
use crate::auth::model::Actor;
use crate::error::AppError;
use crate::mcp::server::{McpRequest, McpResponse, McpServer};
use crate::mcp::sse::{sse_handler, sse_message_handler, SseSessionStore};
use crate::state::AppState;

/// GET /api/v1/mcp/sse — MCP SSE endpoint.
pub async fn mcp_sse(
    State(state): State<Arc<AppState>>,
    Extension(_actor): Extension<Actor>,
) -> Result<
    Sse<
        tokio_stream::wrappers::ReceiverStream<
            Result<axum::response::sse::Event, std::convert::Infallible>,
        >,
    >,
    AppError,
> {
    let server = Arc::new(McpServer::new(state));
    let store = SseSessionStore::new();
    Ok(sse_handler(State((server, store))).await)
}

/// POST /api/v1/mcp/sse/session/:session_id — MCP SSE message endpoint.
pub async fn mcp_sse_message(
    State(state): State<Arc<AppState>>,
    Extension(_actor): Extension<Actor>,
    session_id: Path<String>,
    Json(body): Json<McpRequest>,
) -> Result<Json<ApiResponse<McpResponse>>, AppError> {
    let server = Arc::new(McpServer::new(state));
    let store = SseSessionStore::new();
    let resp = sse_message_handler(State((server, store)), session_id, Json(body)).await?;
    Ok(Json(ApiResponse::ok(resp.0)))
}
