//! Tests for collaboration protocol and room state.

use epicode_kb::collab::protocol::CollabMessage;

#[test]
fn test_collab_message_roundtrip() {
    let messages = vec![
        CollabMessage::SyncStep1(vec![1, 2, 3]),
        CollabMessage::SyncStep2(vec![4, 5, 6]),
        CollabMessage::Update(vec![7, 8, 9]),
        CollabMessage::Awareness(vec![10, 11, 12]),
    ];

    for msg in messages {
        let encoded = msg.encode();
        let parsed = CollabMessage::parse(&encoded).expect("parse should succeed");
        assert_eq!(format!("{:?}", msg), format!("{:?}", parsed));
    }
}

#[test]
fn test_collab_message_parse_empty_returns_none() {
    assert!(CollabMessage::parse(&[]).is_none());
}

#[test]
fn test_collab_message_parse_unknown_kind_returns_none() {
    assert!(CollabMessage::parse(&[99, 1, 2, 3]).is_none());
}
