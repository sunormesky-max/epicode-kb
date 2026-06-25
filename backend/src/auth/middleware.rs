//! Auth middleware (stub — Sprint 2).
//!
//! Sprint 1: Reads `X-Space-Id` and `X-User-Id` headers and injects into request extension.
//! Sprint 2: JWT validation + RBAC permission checks.

use axum::{extract::Request, http::HeaderMap, middleware::Next, response::Response};

/// Extract space_id from request headers.
/// In Sprint 1, we use the `X-Space-Id` header for single-user development mode.
/// TODO: In Sprint 2, replace with JWT token extraction.
pub fn extract_space_id(headers: &HeaderMap) -> String {
    headers
        .get("x-space-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("sp_default")
        .to_string()
}

/// Extract user_id from request headers.
/// In Sprint 1, we use the `X-User-Id` header for single-user development mode.
/// TODO: In Sprint 2, replace with JWT token extraction.
pub fn extract_user_id(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

/// Auth middleware — Sprint 1: pass-through with header extraction.
/// TODO: Sprint 2 — JWT validation + RBAC permission checks.
pub async fn auth_middleware(mut req: Request, next: Next) -> Response {
    let space_id = extract_space_id(req.headers());
    let user_id = extract_user_id(req.headers());

    // Inject into request extensions for handlers to use
    req.extensions_mut()
        .insert(SpaceContext { space_id, user_id });

    next.run(req).await
}

/// Space context injected by auth middleware.
#[derive(Debug, Clone)]
pub struct SpaceContext {
    pub space_id: String,
    pub user_id: Option<String>,
}
