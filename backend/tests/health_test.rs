//! Tests for the Health scanner module.

#[cfg(test)]
mod tests {
    use epicode_kb::health::scanner::HealthScanner;
    use std::sync::Arc;

    fn create_test_db() -> Arc<std::sync::Mutex<rusqlite::Connection>> {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(include_str!("../src/db/migrations/001_init.sql")).unwrap();
        conn.execute_batch(include_str!("../src/db/migrations/002_indexes.sql")).unwrap();
        conn.execute_batch(include_str!("../src/db/migrations/003_v2_schema.sql")).unwrap();
        conn.execute_batch(include_str!("../src/db/migrations/004_v3_schema.sql")).unwrap();
        let now = epicode_kb::now_ts();
        conn.execute(
            "INSERT INTO spaces (id, name, created_at, updated_at) VALUES ('sp_test', 'Test', ?1, ?2)",
            rusqlite::params![now, now],
        ).unwrap();
        conn.execute(
            "INSERT INTO users (id, email, name, global_role, is_active, created_at, updated_at) VALUES ('usr_test', 'test@test.com', 'Test', 'admin', 1, ?1, ?2)",
            rusqlite::params![now, now],
        ).unwrap();
        Arc::new(std::sync::Mutex::new(conn))
    }

    #[test]
    fn test_full_scan_empty_space() {
        let db = create_test_db();
        let scanner = HealthScanner::new(db);
        let snap = scanner.full_scan("sp_test").unwrap();
        assert_eq!(snap.total, 0);
        // Empty space: activity=1.0, completeness=1.0, freshness=1.0, trust=0.0 (no data)
        // score = (1.0*0.3 + 1.0*0.3 + 1.0*0.2 + 0.0*0.2) * 100 = 80.0
        assert_eq!(snap.health_score, 80.0, "Empty space health = 80 (trust=0 due to no data)");
        assert_eq!(snap.stale_count, 0);
        assert_eq!(snap.gap_count, 0);
    }

    #[test]
    fn test_full_scan_with_memories() {
        let db = create_test_db();
        let now = epicode_kb::now_ts();

        {
            let conn = db.lock().unwrap();
            // Insert 3 human memories and 1 AI memory
            for i in 0..3 {
                conn.execute(
                    "INSERT INTO memories (id, space_id, content, embedding_model, provenance, trust_level, review_status, visibility, version_seq, created_at, updated_at) VALUES (?1, 'sp_test', ?2, 'test', 'human', 1.0, 'accepted', 'inherit', 0, ?3, ?3)",
                    rusqlite::params![format!("mem_h{}", i), format!("content {}", i), now],
                ).unwrap();
            }
            conn.execute(
                "INSERT INTO memories (id, space_id, content, embedding_model, provenance, trust_level, review_status, visibility, version_seq, created_at, updated_at) VALUES ('mem_ai1', 'sp_test', 'ai content', 'test', 'ai', 0.5, 'accepted', 'inherit', 0, ?1, ?1)",
                rusqlite::params![now],
            ).unwrap();
        }

        let scanner = HealthScanner::new(db);
        let snap = scanner.full_scan("sp_test").unwrap();
        assert_eq!(snap.total, 4);
        assert!((snap.human_ratio - 0.75).abs() < 0.01, "3/4 should be 75% human");
        assert!((snap.ai_ratio - 0.25).abs() < 0.01, "1/4 should be 25% AI");
        assert!(snap.health_score > 0.0);
    }

    #[test]
    fn test_staleness_detection() {
        let db = create_test_db();
        let now = epicode_kb::now_ts();
        let old = now - 100 * 86400; // 100 days ago

        {
            let conn = db.lock().unwrap();
            conn.execute(
                "INSERT INTO memories (id, space_id, content, embedding_model, provenance, trust_level, review_status, visibility, version_seq, created_at, updated_at, last_accessed_at, access_count) VALUES ('mem_old', 'sp_test', 'old content', 'test', 'human', 1.0, 'accepted', 'inherit', 0, ?1, ?1, ?2, 1)",
                rusqlite::params![old, old],
            ).unwrap();
        }

        let scanner = HealthScanner::new(db);
        let stale = scanner.scan_staleness("sp_test").unwrap();
        assert!(!stale.is_empty(), "Should find stale memory");
        assert!(stale[0].days_since_access >= 100);
        assert!(stale[0].score > 0.5, "100-day-old memory should have high staleness");
    }

    #[test]
    fn test_gap_detection() {
        let db = create_test_db();
        let now = epicode_kb::now_ts();

        {
            let conn = db.lock().unwrap();
            // Insert zero-result queries
            for q in &["how to deploy", "how to deploy", "api reference"] {
                conn.execute(
                    "INSERT INTO query_logs (id, space_id, query, result_count, query_type, created_at) VALUES (?1, 'sp_test', ?2, 0, 'search', ?3)",
                    rusqlite::params![format!("ql_{}", uuid::Uuid::new_v4().simple()), q, now],
                ).unwrap();
            }
        }

        let scanner = HealthScanner::new(db);
        let gaps = scanner.scan_gaps("sp_test").unwrap();
        assert_eq!(gaps.len(), 2, "Should find 2 unique gap queries");
        assert!(gaps.iter().any(|g| g.query == "how to deploy" && g.count == 2));
    }
}
