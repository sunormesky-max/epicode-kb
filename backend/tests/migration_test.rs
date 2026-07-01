//! Tests for migration transaction safety.
//!
//! Verifies that a migration which fails partway is fully rolled back and not
//! recorded as applied — so a retry from a clean state succeeds rather than
//! crashing on leftover partial schema (the bug observed during collab smoke).

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    /// Recreate the migration runner's transaction helper inline and assert
    /// that a failing SQL batch leaves `_migrations` untouched.
    #[test]
    fn test_failed_migration_is_rolled_back() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS _migrations (
                name TEXT PRIMARY KEY,
                applied_at INTEGER NOT NULL
            );",
        )
        .unwrap();

        // A migration whose first statement succeeds but second fails.
        let bad_sql = "CREATE TABLE tmp_marker (id INTEGER); THIS IS NOT VALID SQL;";

        let result = apply_in_txn(&conn, "bad", bad_sql);
        assert!(result.is_err(), "bad migration must error");

        // Must NOT be recorded as applied.
        let applied: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM _migrations WHERE name = 'bad'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(applied, 0, "failed migration must not be marked applied");

        // The partial DDL (tmp_marker) must have been rolled back, so recreating
        // it via a fresh successful migration must not "duplicate".
        let ok_sql = "CREATE TABLE tmp_marker (id INTEGER);";
        let result = apply_in_txn(&conn, "ok", ok_sql);
        assert!(result.is_ok(), "retry after rollback should succeed: {:?}", result);

        let applied_ok: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM _migrations WHERE name = 'ok'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(applied_ok, 1, "successful migration must be marked applied");
    }

    /// Mirror of the production transaction wrapper for isolated testing.
    fn apply_in_txn(conn: &Connection, name: &str, sql: &str) -> Result<(), rusqlite::Error> {
        conn.execute_batch("BEGIN")?;
        match conn.execute_batch(sql) {
            Ok(()) => {
                conn.execute(
                    "INSERT INTO _migrations (name, applied_at) VALUES (?1, ?2)",
                    rusqlite::params![name, 0i64],
                )?;
                conn.execute_batch("COMMIT")?;
                Ok(())
            }
            Err(e) => {
                let _ = conn.execute_batch("ROLLBACK");
                Err(e)
            }
        }
    }
}
