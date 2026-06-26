//! Tests for the collaboration protocol (standard yjs sync format).

use epicode_kb::collab::protocol::{read_message, write_message};
use yrs::sync::protocol::{Message, SyncMessage};
use yrs::StateVector;

#[test]
fn test_sync_step1_roundtrip() {
    let msg = Message::Sync(SyncMessage::SyncStep1(StateVector::default()));
    let encoded = write_message(&msg);
    let decoded = read_message(&encoded).expect("decode should succeed");
    assert_eq!(msg, decoded);
}

#[test]
fn test_sync_update_roundtrip() {
    let msg = Message::Sync(SyncMessage::Update(vec![1, 2, 3, 4]));
    let encoded = write_message(&msg);
    let decoded = read_message(&encoded).expect("decode should succeed");
    assert_eq!(msg, decoded);
}

#[test]
fn test_read_message_garbage_returns_none() {
    assert!(read_message(&[0xFF, 0xFF]).is_none());
}
