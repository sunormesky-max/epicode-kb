//! Database schema constants — embedded SQL migration files.

/// Migration 001: Initial schema (all 10 tables).
pub const MIGRATION_001_INIT: &str = include_str!("migrations/001_init.sql");

/// Migration 002: Indexes.
pub const MIGRATION_002_INDEXES: &str = include_str!("migrations/002_indexes.sql");

/// Migration 003: v0.2.0 schema additions.
pub const MIGRATION_003_V2_SCHEMA: &str = include_str!("migrations/003_v2_schema.sql");

/// Migration 004: v0.3.0 schema additions (ai_proposals, query_logs, knowledge_health, notify_subscriptions).
pub const MIGRATION_004_V3_SCHEMA: &str = include_str!("migrations/004_v3_schema.sql");
