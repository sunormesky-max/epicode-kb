//! Tests for the AI Proposal engine.

#[cfg(test)]
mod tests {
    use epicode_kb::dream::proposal::{ProposalEngine, ProposalStatus, ProposalType};
    use std::sync::Arc;

    fn create_test_db() -> Arc<std::sync::Mutex<rusqlite::Connection>> {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(include_str!("../src/db/migrations/001_init.sql")).unwrap();
        conn.execute_batch(include_str!("../src/db/migrations/002_indexes.sql")).unwrap();
        conn.execute_batch(include_str!("../src/db/migrations/003_v2_schema.sql")).unwrap();
        conn.execute_batch(include_str!("../src/db/migrations/004_v3_schema.sql")).unwrap();
        // Insert default space
        let now = epicode_kb::now_ts();
        conn.execute(
            "INSERT INTO spaces (id, name, created_at, updated_at) VALUES ('sp_test', 'Test', ?1, ?2)",
            rusqlite::params![now, now],
        ).unwrap();
        // Insert a test user
        conn.execute(
            "INSERT INTO users (id, email, name, global_role, is_active, created_at, updated_at) VALUES ('usr_test', 'test@test.com', 'Test', 'admin', 1, ?1, ?2)",
            rusqlite::params![now, now],
        ).unwrap();
        Arc::new(std::sync::Mutex::new(conn))
    }

    #[test]
    fn test_proposal_status_serde() {
        let s = serde_json::to_string(&ProposalStatus::Pending).unwrap();
        assert_eq!(s, "\"pending\"");
        let s = serde_json::to_string(&ProposalStatus::Approved).unwrap();
        assert_eq!(s, "\"approved\"");
    }

    #[test]
    fn test_proposal_type_serde() {
        let s = serde_json::to_string(&ProposalType::Merge).unwrap();
        assert_eq!(s, "\"merge\"");
        let s = serde_json::to_string(&ProposalType::Archive).unwrap();
        assert_eq!(s, "\"archive\"");
    }

    #[test]
    fn test_scan_space_returns_empty_for_empty_space() {
        let db = create_test_db();
        let engine = ProposalEngine::new(db);
        let result = engine.scan_space("sp_test").unwrap();
        assert!(result.is_empty(), "Empty space should yield no proposals");
    }

    #[test]
    fn test_scan_space_finds_merge_candidates() {
        let db = create_test_db();
        let now = epicode_kb::now_ts();

        // Insert two very similar memories
        {
            let conn = db.lock().unwrap();
            for i in 0..2 {
                conn.execute(
                    "INSERT INTO memories (id, space_id, content, embedding_model, provenance, trust_level, review_status, visibility, version_seq, created_at, updated_at) VALUES (?1, 'sp_test', ?2, 'test', 'human', 1.0, 'accepted', 'inherit', 0, ?3, ?3)",
                    rusqlite::params![format!("mem_{}", i), "Rust is a systems programming language focused on safety and performance", now],
                ).unwrap();
            }
        }

        let engine = ProposalEngine::new(db);
        let result = engine.scan_space("sp_test").unwrap();
        assert!(!result.is_empty(), "Should find merge candidates for identical content");
        assert_eq!(result[0].proposal_type, ProposalType::Merge);
    }

    #[test]
    fn test_approve_reject_modify() {
        let db = create_test_db();
        let now = epicode_kb::now_ts();

        // Manually insert a proposal
        {
            let conn = db.lock().unwrap();
            conn.execute(
                "INSERT INTO ai_proposals (id, space_id, proposal_type, source_memory_ids, proposed_content, ai_model, confidence, status, created_at) VALUES ('pro_test1', 'sp_test', 'merge', '[\"mem_a\",\"mem_b\"]', 'merge these', 'test', 0.9, 'pending', ?1)",
                rusqlite::params![now],
            ).unwrap();
        }

        let engine = ProposalEngine::new(db.clone());

        // Approve
        let approved = engine.approve("pro_test1", "usr_test").unwrap();
        assert_eq!(approved.status, ProposalStatus::Approved);

        // Verify it can't be approved again
        let engine2 = ProposalEngine::new(db);
        let result = engine2.approve("pro_test1", "usr_test");
        assert!(result.is_err(), "Should not be able to approve twice");
    }
}
