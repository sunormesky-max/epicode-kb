//! Integration tests: ProposalEngine wired to ConflictDetector (P3-5).
//!
//! Verifies that `scan_space` surfaces detected knowledge contradictions
//! as `Conflict`-type proposals when the engine is built with
//! `new_with_conflict`.

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use epicode_kb::conflict::detect::ConflictDetector;
    use epicode_kb::conflict::model::ConflictConfig;
    use epicode_kb::db::repository::MemoryRepo;
    use epicode_kb::dream::proposal::{ProposalEngine, ProposalType};
    use epicode_kb::embed::onnx::RandomEmbedder;
    use epicode_kb::memory::model::{Memory, Provenance, ReviewStatus};
    use rusqlite::Connection;

    /// Build an in-memory DB with all migrations + a default space.
    fn mem_db() -> Arc<std::sync::Mutex<Connection>> {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(include_str!("../src/db/migrations/001_init.sql"))
            .unwrap();
        conn.execute_batch(include_str!("../src/db/migrations/002_indexes.sql"))
            .unwrap();
        conn.execute_batch(include_str!("../src/db/migrations/003_v2_schema.sql"))
            .unwrap();
        conn.execute_batch(include_str!("../src/db/migrations/004_v3_schema.sql"))
            .unwrap();
        conn.execute(
            "INSERT INTO spaces (id, name, visibility, created_at, updated_at) VALUES ('sp_test', 'Test', 'public', 0, 0)",
            [],
        )
        .unwrap();
        Arc::new(std::sync::Mutex::new(conn))
    }

    /// Insert an accepted memory with a forced embedding vector.
    fn seed_memory(conn: &Connection, id: &str, content: &str, embedding: &[f32]) {
        let mut mem = Memory::new("sp_test".to_string(), content.to_string(), Provenance::Human);
        mem.id = id.to_string();
        mem.review_status = ReviewStatus::Accepted;
        mem.embedding = Some(embedding.to_vec());
        mem.created_at = 0;
        mem.updated_at = 0;
        MemoryRepo::insert(conn, &mem).unwrap();
    }

    #[test]
    fn test_scan_space_emits_conflict_proposal() {
        let db = mem_db();
        // Identical embeddings (semantic distance 0) + divergent wording
        // (low Jaccard) → contradiction detected → Conflict proposal.
        let emb = vec![1.0_f32; 8];
        {
            let conn = db.lock().unwrap();
            seed_memory(&conn, "mem_one", "Rust compiles blazingly fast and is memory safe", &emb);
            seed_memory(&conn, "mem_two", "Python executes interpreted bytecode dynamically", &emb);
        }

        let detector = Arc::new(ConflictDetector::new(
            db.clone(),
            Arc::new(RandomEmbedder::new(8)),
            ConflictConfig::default(),
        ));
        let engine = ProposalEngine::new_with_conflict(db.clone(), detector);

        let proposals = engine.scan_space("sp_test").unwrap();
        let conflict_proposals: Vec<_> = proposals
            .iter()
            .filter(|p| p.proposal_type == ProposalType::Conflict)
            .collect();

        assert!(
            !conflict_proposals.is_empty(),
            "expected at least one Conflict proposal, got {:?}",
            proposals.iter().map(|p| p.proposal_type).collect::<Vec<_>>()
        );

        // The conflict proposal should reference both source memories.
        let cp = &conflict_proposals[0];
        assert!(cp.source_memory_ids.contains(&"mem_one".to_string()));
        assert!(cp.source_memory_ids.contains(&"mem_two".to_string()));
        assert!(cp.confidence.unwrap_or(0.0) > 0.3);
    }

    #[test]
    fn test_scan_space_dedups_conflict_proposals() {
        let db = mem_db();
        let emb = vec![1.0_f32; 8];
        {
            let conn = db.lock().unwrap();
            seed_memory(&conn, "mem_p", "alpha beta gamma delta epsilon zeta", &emb);
            seed_memory(&conn, "mem_q", "one two three four five six seven", &emb);
        }

        let detector = Arc::new(ConflictDetector::new(
            db.clone(),
            Arc::new(RandomEmbedder::new(8)),
            ConflictConfig::default(),
        ));
        let engine = ProposalEngine::new_with_conflict(db.clone(), detector);

        // First scan produces the conflict proposal.
        let first = engine.scan_space("sp_test").unwrap();
        let first_conflicts = first
            .iter()
            .filter(|p| p.proposal_type == ProposalType::Conflict)
            .count();

        // Second scan must not duplicate pending conflict proposals.
        let second = engine.scan_space("sp_test").unwrap();
        let second_conflicts = second
            .iter()
            .filter(|p| p.proposal_type == ProposalType::Conflict)
            .count();

        assert!(
            first_conflicts >= 1,
            "first scan should detect a conflict (got {})",
            first_conflicts
        );
        assert_eq!(
            second_conflicts, 0,
            "second scan should not duplicate pending conflict proposals"
        );
    }
}
