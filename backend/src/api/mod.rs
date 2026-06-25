//! API route layer.

pub mod agent;
pub mod auth;
pub mod collab;
pub mod health;
pub mod health_api;
pub mod mcp;
pub mod memory;
pub mod proposal;
pub mod routes;
pub mod search;
pub mod upload;

use serde::Serialize;

/// Unified API response wrapper.
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub code: i32,
    pub data: T,
    pub message: String,
}

impl<T: Serialize> ApiResponse<T> {
    /// Create a successful response.
    pub fn ok(data: T) -> Self {
        Self {
            code: 0,
            data,
            message: "ok".to_string(),
        }
    }
}

/// A null data response for stubs.
pub fn not_implemented_response(feature: &str) -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "code": 50100,
        "data": null,
        "message": format!("not implemented: {} (planned for Sprint 2/3)", feature)
    }))
}
