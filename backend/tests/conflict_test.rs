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
