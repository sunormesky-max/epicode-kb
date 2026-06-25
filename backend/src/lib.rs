//! epicode-kb: Enterprise knowledge base with memory provenance,
//! hybrid search, and AI proposal engine.

pub mod api;
pub mod auth;
pub mod config;
pub mod db;
pub mod dream;
pub mod embed;
pub mod error;
pub mod llm;
pub mod memory;
pub mod notify;
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
