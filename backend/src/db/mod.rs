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

/// Run all pending migrations, tracking applied migrations in `_migrations` table.
fn run_migrations(conn: &Connection) -> AppResult<()> {
    tracing::info!("Running database migrations...");

    // Ensure the migration tracking table exists.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS _migrations (
            name TEXT PRIMARY KEY,
            applied_at INTEGER NOT NULL
        );",
    )?;

    /// Check whether a migration has already been applied.
    fn is_applied(conn: &Connection, name: &str) -> AppResult<bool> {
        let count: i64 =
            conn.query_row("SELECT COUNT(*) FROM _migrations WHERE name = ?1", [name], |row| {
                row.get(0)
            })?;
        Ok(count > 0)
    }

    /// Record a migration as applied.
    fn mark_applied(conn: &Connection, name: &str) -> AppResult<()> {
        let now = crate::now_ts();
        conn.execute(
            "INSERT INTO _migrations (name, applied_at) VALUES (?1, ?2)",
            rusqlite::params![name, now],
        )?;
        Ok(())
    }

    let migrations: &[(&str, &str)] = &[
        ("001_init", schema::MIGRATION_001_INIT),
        ("002_indexes", schema::MIGRATION_002_INDEXES),
        ("003_v2_schema", schema::MIGRATION_003_V2_SCHEMA),
    ];

    for (name, sql) in migrations {
        if is_applied(conn, name)? {
            tracing::info!("  Skipping already-applied migration {}", name);
            continue;
        }
        conn.execute_batch(sql)?;
        mark_applied(conn, name)?;
        tracing::info!("  Applied migration {}", name);
    }

    Ok(())
}

/// Create a default space if none exists.
pub fn ensure_default_space(conn: &Connection) -> AppResult<()> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM spaces", [], |row| row.get(0))?;

    if count == 0 {
        let now = crate::now_ts();
        conn.execute(
            "INSERT INTO spaces (id, name, slug, description, visibility, owner_id, ai_write_enabled, default_ai_trust_level, retention_days, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, ?9, ?10)",
            rusqlite::params![
                "sp_default",
                "Default Space",
                "default",
                "Default workspace for epicode-kb",
                "team",
                "usr_default",
                1,
                0.5,
                now,
                now,
            ],
        )?;
        // Ensure a placeholder default user exists to satisfy owner FK.
        conn.execute(
            "INSERT OR IGNORE INTO users (id, email, name, global_role, password_hash, sso_subject, is_active, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, NULL, NULL, 1, ?5, ?6)",
            rusqlite::params![
                "usr_default",
                "admin@example.com",
                "Default Admin",
                "admin",
                now,
                now,
            ],
        )?;
        // Ensure the default user is a member of the default space.
        conn.execute(
            "INSERT OR IGNORE INTO space_members (id, space_id, user_id, role, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                "sm_default",
                "sp_default",
                "usr_default",
                "owner",
                now,
            ],
        )?;
        tracing::info!("Created default space: sp_default");
    }

    Ok(())
}
