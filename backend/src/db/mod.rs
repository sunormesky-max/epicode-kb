//! Database layer: SQLite connection, migrations, and repositories.

pub mod repository;
pub mod schema;

use std::sync::Arc;

use rusqlite::Connection;
use std::sync::Mutex;

use crate::error::AppResult;

/// Thread-safe SQLite connection wrapper.
pub type DbPool = Arc<Mutex<Connection>>;

/// Initialize the SQLite database: open connection, run migrations.
pub fn init_database(database_url: &str) -> AppResult<Connection> {
    // Ensure parent directory exists
    if let Some(parent) = std::path::Path::new(database_url).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let conn = Connection::open(database_url)?;
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;
    conn.execute_batch("PRAGMA foreign_keys=ON;")?;

    // Run migrations
    run_migrations(&conn)?;

    tracing::info!("Database initialized at {}", database_url);
    Ok(conn)
}

/// Run all pending migrations.
fn run_migrations(conn: &Connection) -> AppResult<()> {
    tracing::info!("Running database migrations...");

    conn.execute_batch(schema::MIGRATION_001_INIT)?;
    tracing::info!("  Applied migration 001_init.sql");

    conn.execute_batch(schema::MIGRATION_002_INDEXES)?;
    tracing::info!("  Applied migration 002_indexes.sql");

    Ok(())
}

/// Create a default space if none exists.
pub fn ensure_default_space(conn: &Connection) -> AppResult<()> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM spaces", [], |row| row.get(0))?;

    if count == 0 {
        let now = crate::now_ts();
        conn.execute(
            "INSERT INTO spaces (id, name, description, ai_write_enabled, default_ai_trust_level, retention_days, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6, ?7)",
            rusqlite::params![
                "sp_default",
                "Default Space",
                "Default workspace for epicode-kb",
                1,
                0.5,
                now,
                now,
            ],
        )?;
        tracing::info!("Created default space: sp_default");
    }

    Ok(())
}
