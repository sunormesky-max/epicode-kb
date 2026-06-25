//! epicode-kb: Enterprise knowledge base with memory provenance,
//! hybrid search, real-time collaboration, and AI agent integration.

pub mod api;
pub mod auth;
pub mod collab;
pub mod config;
pub mod conflict;
pub mod db;
pub mod dream;
pub mod embed;
pub mod error;
pub mod health;
pub mod llm;
pub mod mcp;
pub mod memory;
pub mod notify;
pub mod observability;
pub mod parse;
pub mod search;
pub mod state;

/// Generate a prefixed UUID v4 identifier (without hyphens).
/// Example: `generate_id("mem")` → `"mem_a1b2c3d4e5f6..."`
pub fn generate_id(prefix: &str) -> String {
    format!("{}_{}", prefix, uuid::Uuid::new_v4().simple())
}

/// Get current Unix timestamp in seconds.
pub fn now_ts() -> i64 {
    chrono::Utc::now().timestamp()
}
