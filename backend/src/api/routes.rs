//! Route registration — assembles the Axum Router with all route groups.

use std::sync::Arc;

use axum::{
    middleware,
    routing::{delete, get, post, put},
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::auth::middleware::auth_middleware;
use crate::state::AppState;

/// Create the application router with all routes.
pub fn create_router(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let api_v1 = Router::new()
        // Auth routes
        .route("/auth/register", post(crate::api::auth::register))
        .route("/auth/login", post(crate::api::auth::login))
        .route("/auth/refresh", post(crate::api::auth::refresh))
        .route("/auth/me", get(crate::api::auth::me))
        // Memory routes
        .route("/remember", post(crate::api::memory::remember))
        .route("/memories", get(crate::api::memory::list_memories))
        .route("/memories/:id", get(crate::api::memory::get_memory))
        .route(
            "/memories/:id/trust",
            post(crate::api::memory::update_trust),
        )
        .route(
            "/memories/:id/visibility",
            put(crate::api::memory::update_visibility),
        )
        .route(
            "/memories/:id/adopt",
            post(crate::api::memory::adopt_memory),
        )
        .route(
            "/memories/:id/reject",
            post(crate::api::memory::reject_memory),
        )
        .route("/memories/:id/save", post(crate::api::memory::save_version))
        .route(
            "/memories/:id/versions",
            get(crate::api::memory::list_versions),
        )
        .route(
            "/memories/:id/versions/:vid/revert",
            post(crate::api::memory::revert_version),
        )
        .route(
            "/memories/:id/resolve",
            post(crate::api::memory::resolve_conflict),
        )
        // Search routes
        .route("/search", get(crate::api::search::search))
        .route("/recall", post(crate::api::search::recall))
        // Upload route
        .route("/upload", post(crate::api::upload::upload))
        // Agent routes
        .route("/agents/search", post(crate::api::agent::search))
        .route("/agents/remember", post(crate::api::agent::remember))
        .route("/agents/memories/:id", get(crate::api::agent::get_memory))
        .route(
            "/spaces/:id/api-keys",
            post(crate::api::agent::create_api_key).get(crate::api::agent::list_api_keys),
        )
        .route(
            "/spaces/:id/api-keys/:key_id",
            delete(crate::api::agent::revoke_api_key),
        )
        // Space routes
        .route(
            "/spaces",
            get(crate::api::auth::list_spaces).post(crate::api::auth::create_space),
        )
        .route("/spaces/:id", get(crate::api::auth::get_space))
        .route(
            "/spaces/:id/visibility",
            put(crate::api::auth::update_space_visibility),
        )
        .route(
            "/spaces/:id/members",
            get(crate::api::auth::list_space_members).post(crate::api::auth::invite_space_member),
        )
        // Collaboration routes
        .route("/collab/:memory_id", get(crate::api::collab::collab_ws))
        // MCP routes
        .route("/mcp/sse", get(crate::api::mcp::mcp_sse))
        .route(
            "/mcp/sse/session/:session_id",
            post(crate::api::mcp::mcp_sse_message),
        )
        // Proposal routes
        .route("/proposals", get(crate::api::proposal::list_proposals))
        .route(
            "/proposals/batch",
            post(crate::api::proposal::batch_proposals),
        )
        .route(
            "/proposals/:id/approve",
            post(crate::api::proposal::approve_proposal),
        )
        .route(
            "/proposals/:id/reject",
            post(crate::api::proposal::reject_proposal),
        )
        .route(
            "/proposals/:id/modify",
            post(crate::api::proposal::modify_proposal),
        )
        // Dream cycle trigger
        .route("/dream/scan", post(crate::api::proposal::scan_proposals))
        // Health routes
        .route("/health/live", get(crate::api::health::live))
        .route("/health/ready", get(crate::api::health::ready))
        .route("/health/space/:id", get(crate::api::health::space_health))
        .route("/health/gaps", get(crate::api::health::knowledge_gaps))
        .route("/health/scan", post(crate::api::health::scan))
        // Health v3 routes
        .route("/v3/health/space/:id", get(crate::api::health_api::get_space_health))
        .route("/v3/health/gaps", get(crate::api::health_api::get_gaps))
        .route("/v3/health/stale", get(crate::api::health_api::get_stale))
        .route("/v3/health/scan", post(crate::api::health_api::trigger_scan))
        // Conflict routes (stub)
        .route("/conflicts", get(crate::api::proposal::list_conflicts))
        .route(
            "/conflicts/:id/resolve",
            post(crate::api::proposal::resolve_conflict),
        )
        // Graph route (stub)
        .route("/graph", get(crate::api::health::graph))
        // Metrics route
        .route("/metrics", get(crate::api::health::metrics))
        // System routes
        .route("/system/health", get(crate::api::health::system_health))
        .route("/system/version", get(crate::api::health::system_version));

    Router::new()
        .nest("/api/v1", api_v1)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}
