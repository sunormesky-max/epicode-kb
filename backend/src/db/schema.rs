//! Database schema constants — embedded SQL migration files.

/// Migration 001: Initial schema (all 10 tables).
pub const MIGRATION_001_INIT: &str = include_str!("migrations/001_init.sql");

/// Migration 002: Indexes.
pub const MIGRATION_002_INDEXES: &str = include_str!("migrations/002_indexes.sql");
