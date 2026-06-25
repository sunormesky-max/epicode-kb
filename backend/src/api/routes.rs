//! Route registration — assembles the Axum Router with all route groups.

use std::sync::Arc;

use axum::{
    middleware,
    routing::{get, post},
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
        // Memory routes
        .route("/remember", post(crate::api::memory::remember))
        .route("/memories", get(crate::api::memory::list_memories))
        .route("/memories/:id", get(crate::api::memory::get_memory))
        .route(
            "/memories/:id/trust",
            post(crate::api::memory::update_trust),
        )
        .route(
            "/memories/:id/adopt",
            post(crate::api::memory::adopt_memory),
        )
        .route(
            "/memories/:id/reject",
            post(crate::api::memory::reject_memory),
        )
        // Search routes
        .route("/search", get(crate::api::search::search))
        .route("/recall", post(crate::api::search::recall))
        // Upload route
        .route("/upload", post(crate::api::upload::upload))
        // Proposal routes (stub)
        .route("/proposals", get(crate::api::proposal::list_proposals))
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
        // Health routes (stub)
        .route("/health/space/:id", get(crate::api::health::space_health))
        .route("/health/gaps", get(crate::api::health::knowledge_gaps))
        .route("/health/scan", post(crate::api::health::scan))
        // Conflict routes (stub)
        .route("/conflicts", get(crate::api::proposal::list_conflicts))
        .route(
            "/conflicts/:id/resolve",
            post(crate::api::proposal::resolve_conflict),
        )
        // Graph route (stub)
        .route("/graph", get(crate::api::health::graph))
        // Auth routes (stub)
        .route("/auth/login", post(crate::api::auth::login))
        // Space routes (stub)
        .route(
            "/spaces",
            get(crate::api::auth::list_spaces).post(crate::api::auth::create_space),
        )
        // Notification routes (stub)
        .route(
            "/notifications",
            get(crate::api::proposal::list_notifications),
        )
        // System routes
        .route("/system/health", get(crate::api::health::system_health))
        .route("/system/version", get(crate::api::health::system_version));

    Router::new()
        .nest("/api/v1", api_v1)
        .layer(middleware::from_fn(auth_middleware))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}
