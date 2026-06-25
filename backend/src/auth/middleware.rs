//! Auth middleware — JWT validation, API key validation, RBAC context injection.

use axum::{
    extract::{Request, State},
    http::HeaderMap,
    middleware::Next,
    response::Response,
};

use crate::auth::model::Actor;
use crate::auth::rbac::AuthContext;
use crate::error::{AppError, AppResult};

/// Extract authorization header bearer token.
fn extract_bearer(headers: &HeaderMap) -> Option<String> {
    headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.trim().to_string())
}

/// Extract agent API key from headers.
fn extract_agent_api_key(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-agent-api-key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string())
}

/// Extract space id from headers (fallback for clients not using JWT space claims).
fn extract_space_id(headers: &HeaderMap) -> String {
    headers
        .get("x-space-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("sp_default")
        .to_string()
}

/// Auth middleware — validates JWT or API key and injects Actor / AuthContext.
pub async fn auth_middleware(
    State(state): State<std::sync::Arc<crate::state::AppState>>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let headers = req.headers();
    let space_id = extract_space_id(headers);

    // Try agent API key first.
    if let Some(api_key) = extract_agent_api_key(headers) {
        match state.auth_service.verify_api_key(&api_key, &space_id) {
            Ok(agent_ctx) => {
                let actor = Actor {
                    user_id: agent_ctx.user_id.clone(),
                    global_role: agent_ctx.user_role,
                    space_role: agent_ctx.space_role,
                };
                req.extensions_mut().insert(actor);
                req.extensions_mut().insert(AuthContext {
                    user_id: agent_ctx.user_id,
                    global_role: agent_ctx.user_role,
                    space_id,
                    space_role: agent_ctx.space_role,
                });
                return Ok(next.run(req).await);
            }
            Err(e) => {
                // If both JWT and API key are present, fall through to JWT.
                if extract_bearer(headers).is_none() {
                    return Err(e);
                }
            }
        }
    }

    // Try JWT bearer token.
    if let Some(token) = extract_bearer(headers) {
        match state.auth_service.verify_access_token(&token) {
            Ok(user) => {
                let space_role = {
                    let conn = state.db.lock().unwrap();
                    crate::db::repository::SpaceMemberRepo::find_role(&conn, &space_id, &user.id)?
                };
                let actor = Actor::new(&user, space_role);
                req.extensions_mut().insert(actor.clone());
                req.extensions_mut().insert(AuthContext {
                    user_id: actor.user_id.clone(),
                    global_role: actor.global_role,
                    space_id: space_id.clone(),
                    space_role,
                });
                req.extensions_mut().insert(user);
                return Ok(next.run(req).await);
            }
            Err(e) => return Err(e),
        }
    }

    // For public endpoints (auth, system health, version), allow anonymous.
    let path = req.uri().path();
    if path.ends_with("/system/health")
        || path.ends_with("/system/version")
        || path.ends_with("/auth/register")
        || path.ends_with("/auth/login")
    {
        return Ok(next.run(req).await);
    }

    Err(AppError::unauthorized("missing or invalid authorization"))
}

/// Extract the actor from a request extension.
pub fn require_actor(req: &Request) -> AppResult<Actor> {
    req.extensions()
        .get::<Actor>()
        .cloned()
        .ok_or_else(|| AppError::unauthorized("actor not found"))
}
