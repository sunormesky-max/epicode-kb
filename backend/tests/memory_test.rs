//! Integration tests for memory creation and retrieval.

use epicode_kb::memory::model::{CreateMemoryRequest, Provenance};

#[test]
fn test_provenance_defaults() {
    assert_eq!(Provenance::Human.default_trust(), 1.0);
    assert_eq!(Provenance::Co.default_trust(), 0.8);
    assert_eq!(Provenance::Ai.default_trust(), 0.5);
    assert_eq!(Provenance::Conflict.default_trust(), 0.3);

    assert_eq!(
        Provenance::Human.default_review_status().as_str(),
        "accepted"
    );
    assert_eq!(Provenance::Ai.default_review_status().as_str(), "pending");
}

#[test]
fn test_provenance_from_str() {
    assert_eq!(Provenance::parse_str("human").unwrap(), Provenance::Human);
    assert_eq!(Provenance::parse_str("ai").unwrap(), Provenance::Ai);
    assert_eq!(Provenance::parse_str("co").unwrap(), Provenance::Co);
    assert_eq!(
        Provenance::parse_str("conflict").unwrap(),
        Provenance::Conflict
    );
    assert!(Provenance::parse_str("invalid").is_err());
}

#[test]
fn test_trust_level_validation() {
    use epicode_kb::memory::model::TrustLevel;

    assert!(TrustLevel::new(0.0).is_ok());
    assert!(TrustLevel::new(1.0).is_ok());
    assert!(TrustLevel::new(0.5).is_ok());
    assert!(TrustLevel::new(-0.1).is_err());
    assert!(TrustLevel::new(1.1).is_err());
}

#[test]
fn test_trust_level_adjust() {
    use epicode_kb::memory::model::TrustLevel;

    let t = TrustLevel::new(0.5).unwrap();
    assert!((t.adjust(0.3).value() - 0.8).abs() < 1e-5);
    assert!((t.adjust(-0.3).value() - 0.2).abs() < 1e-5);
    assert!((t.adjust(1.0).value() - 1.0).abs() < 1e-5); // clamped
    assert!((t.adjust(-1.0).value() - 0.0).abs() < 1e-5); // clamped
}

#[test]
fn test_memory_new() {
    let memory = epicode_kb::memory::model::Memory::new(
        "sp_test".to_string(),
        "Test content".to_string(),
        Provenance::Human,
    );

    assert!(memory.id.starts_with("mem_"));
    assert_eq!(memory.space_id, "sp_test");
    assert_eq!(memory.content, "Test content");
    assert_eq!(memory.provenance, Provenance::Human);
    assert_eq!(memory.trust_level.value(), 1.0);
    assert_eq!(memory.review_status.as_str(), "accepted");
    assert_eq!(memory.access_count, 0);
    assert!(memory.embedding.is_none());
}

#[test]
fn test_create_memory_request_validation() {
    let req = CreateMemoryRequest {
        space_id: "sp_test".to_string(),
        content: "Valid content".to_string(),
        provenance: Provenance::Human,
        trust_level: None,
        provenance_meta: None,
        review_status: None,
        visibility: None,
    };
    assert!(req.validate().is_ok());

    let invalid_req = CreateMemoryRequest {
        space_id: "".to_string(),
        content: "Content".to_string(),
        provenance: Provenance::Human,
        trust_level: None,
        provenance_meta: None,
        review_status: None,
        visibility: None,
    };
    assert!(invalid_req.validate().is_err());

    let bad_trust_req = CreateMemoryRequest {
        space_id: "sp_test".to_string(),
        content: "Content".to_string(),
        provenance: Provenance::Human,
        trust_level: Some(1.5),
        provenance_meta: None,
        review_status: None,
        visibility: None,
    };
    assert!(bad_trust_req.validate().is_err());
}

#[test]
fn test_embedding_blob_roundtrip() {
    use epicode_kb::db::repository::{blob_to_embedding, embedding_to_blob};

    let original = vec![0.1, 0.2, 0.3, 0.4, 0.5];
    let blob = embedding_to_blob(&original);
    assert_eq!(blob.len(), original.len() * 4);

    let recovered = blob_to_embedding(&blob);
    assert_eq!(recovered.len(), original.len());
    for (a, b) in original.iter().zip(recovered.iter()) {
        assert!((a - b).abs() < 1e-6);
    }
}
