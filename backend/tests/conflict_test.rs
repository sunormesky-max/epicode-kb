//! Tests for the Conflict detection module.

#[cfg(test)]
mod tests {
    use epicode_kb::conflict::model::{ConflictCandidate, ConflictConfig};

    #[test]
    fn test_conflict_config_default() {
        let config = ConflictConfig::default();
        assert_eq!(config.semantic_threshold, 0.3);
        assert_eq!(config.llm_confidence_threshold, 0.6);
        assert_eq!(config.max_neighbors, 10);
    }

    #[test]
    fn test_conflict_candidate_serialization() {
        let candidate = ConflictCandidate {
            memory_a_id: "mem_a".to_string(),
            memory_b_id: "mem_b".to_string(),
            content_a: "The sky is blue".to_string(),
            content_b: "The sky is green".to_string(),
            semantic_distance: 0.15,
            confidence: 0.85,
            summary: "Contradiction about sky color".to_string(),
        };
        let json = serde_json::to_string(&candidate).unwrap();
        assert!(json.contains("mem_a"));
        assert!(json.contains("mem_b"));
        assert!(json.contains("0.85"));

        let deserialized: ConflictCandidate = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.memory_a_id, "mem_a");
        assert_eq!(deserialized.confidence, 0.85);
    }

    #[test]
    fn test_resolution_enum() {
        use epicode_kb::conflict::model::Resolution;
        assert_eq!(serde_json::to_string(&Resolution::AcceptA).unwrap(), "\"accepta\"");
        assert_eq!(serde_json::to_string(&Resolution::AcceptB).unwrap(), "\"acceptb\"");
        assert_eq!(serde_json::to_string(&Resolution::BothTrue).unwrap(), "\"bothtrue\"");
    }
}

/// Integration tests for the ConflictDetector engine + conflict proposal wiring.
#[cfg(test)]
mod detect_tests {
    use std::sync::Arc;

    use epicode_kb::conflict::detect::ConflictDetector;
    use epicode_kb::conflict::model::{ConflictCandidate, ConflictConfig};
    use epicode_kb::db::repository::{embedding_to_blob, MemoryRepo};
    use epicode_kb::memory::model::{Memory, Provenance};
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

    /// Insert an accepted memory with a given (forced) embedding vector.
    fn seed_memory(conn: &Connection, id: &str, content: &str, embedding: &[f32]) {
        let now = 0;
        let mut mem = Memory::new("sp_test".to_string(), content.to_string(), Provenance::Human);
        mem.id = id.to_string();
        mem.review_status = epicode_kb::memory::model::ReviewStatus::Accepted;
        mem.embedding = Some(embedding.to_vec());
        mem.created_at = now;
        mem.updated_at = now;
        // MemoryRepo::insert re-derives the blob via embedding_to_blob.
        let _ = embedding_to_blob(embedding); // ensure helper is referenced
        MemoryRepo::insert(conn, &mem).unwrap();
    }

    #[test]
    fn test_detect_all_finds_contradiction() {
        let db = mem_db();
        // Two memories with IDENTICAL embeddings (semantic distance = 0) but
        // lexically divergent content (low Jaccard) → high contradiction score.
        let emb = vec![1.0_f32; 8];
        {
            let conn = db.lock().unwrap();
            seed_memory(
                &conn,
                "mem_alpha",
                "Rust compiles blazingly fast and is memory safe",
                &emb,
            );
            seed_memory(
                &conn,
                "mem_beta",
                "Python executes interpreted bytecode dynamically",
                &emb,
            );
        }

        let detector = ConflictDetector::new(
            db.clone(),
            Arc::new(epicode_kb::embed::onnx::RandomEmbedder::new(8)),
            ConflictConfig::default(),
        );
        let conflicts = detector.detect_all("sp_test").unwrap();
        assert!(
            conflicts.iter().any(|c| {
                (c.memory_a_id == "mem_alpha" && c.memory_b_id == "mem_beta")
                    || (c.memory_a_id == "mem_beta" && c.memory_b_id == "mem_alpha")
            }),
            "expected a contradiction between mem_alpha and mem_beta, got {:?}",
            conflicts
        );
    }

    #[test]
    fn test_create_conflict_memory_produces_conflict_provenance() {
        let db = mem_db();
        {
            let conn = db.lock().unwrap();
            seed_memory(&conn, "mem_x", "Statement X content here", &[1.0; 8]);
            seed_memory(&conn, "mem_y", "Statement Y content here", &[1.0; 8]);
        }

        let detector = ConflictDetector::new(
            db.clone(),
            Arc::new(epicode_kb::embed::onnx::RandomEmbedder::new(8)),
            ConflictConfig::default(),
        );

        let candidate = ConflictCandidate {
            memory_a_id: "mem_x".to_string(),
            memory_b_id: "mem_y".to_string(),
            content_a: "Statement X content here".to_string(),
            content_b: "Statement Y content here".to_string(),
            semantic_distance: 0.1,
            confidence: 0.7,
            summary: "test contradiction".to_string(),
        };
        let created = detector.create_conflict_memory(&candidate).unwrap();
        assert_eq!(created.provenance, Provenance::Conflict);
        assert!(created.provenance_meta.is_some());

        // The conflict memory should be listable via MemoryRepo with conflict filter.
        let conn = db.lock().unwrap();
        let (conflicts, total) = MemoryRepo::list(
            &conn,
            "sp_test",
            Some(&[Provenance::Conflict]),
            None,
            None,
            None,
            10,
            0,
        )
        .unwrap();
        assert_eq!(total, 1);
        assert_eq!(conflicts[0].provenance, Provenance::Conflict);
    }
}
