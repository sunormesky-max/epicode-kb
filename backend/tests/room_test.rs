//! Unit tests for CollaborationRoom broadcast + awareness replay logic.
//!
//! These cover the runtime behavior that the smoke test exercised manually:
//! subscriber add/remove, broadcast exclusion, and awareness record/replay.

use std::sync::Arc;

use epicode_kb::collab::room::CollaborationRoom;
use rusqlite::Connection;
use tokio::sync::mpsc;

/// In-memory DB with migrations + a default space + a seed memory.
fn mem_db_with_memory() -> (Arc<std::sync::Mutex<Connection>>, String) {
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
        "INSERT INTO spaces (id, name, visibility, created_at, updated_at) VALUES ('sp_test', 'T', 'public', 0, 0)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO memories (id, space_id, content, embedding_model, provenance, trust_level, review_status, visibility, version_seq, created_at, updated_at)
         VALUES ('mem_room1', 'sp_test', 'room test content', 'test', 'human', 1.0, 'accepted', 'inherit', 0, 0, 0)",
        [],
    )
    .unwrap();
    (Arc::new(std::sync::Mutex::new(conn)), "mem_room1".to_string())
}

fn make_room() -> (Arc<std::sync::Mutex<CollaborationRoom>>, String) {
    let (db, mid) = mem_db_with_memory();
    let room = Arc::new(std::sync::Mutex::new(
        CollaborationRoom::new(mid.clone(), db).unwrap(),
    ));
    (room, mid)
}

#[tokio::test]
async fn test_broadcast_reaches_all_subscribers() {
    let (room, _) = make_room();
    let (tx1, mut rx1) = mpsc::unbounded_channel();
    let (tx2, mut rx2) = mpsc::unbounded_channel();

    {
        let mut r = room.lock().unwrap();
        r.add_subscriber(tx1);
        r.add_subscriber(tx2);
    }
    room.lock().unwrap().broadcast(vec![1, 2, 3], None);

    let m1 = rx1.recv().await.unwrap();
    let m2 = rx2.recv().await.unwrap();
    // Both should receive the binary payload.
    assert!(matches!(m1, axum::extract::ws::Message::Binary(_)));
    assert!(matches!(m2, axum::extract::ws::Message::Binary(_)));
}

#[tokio::test]
async fn test_broadcast_excludes_sender() {
    let (room, _) = make_room();
    let (tx1, mut rx1) = mpsc::unbounded_channel();
    let (tx2, mut rx2) = mpsc::unbounded_channel();

    let id1;
    {
        let mut r = room.lock().unwrap();
        id1 = r.add_subscriber(tx1);
        r.add_subscriber(tx2);
    }
    // Exclude subscriber 1.
    room.lock().unwrap().broadcast(vec![9, 9], Some(id1));

    // rx1 should NOT receive; rx2 should.
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(100), rx1.recv()).await.is_err(),
        "excluded subscriber must not receive"
    );
    assert!(rx2.recv().await.is_some(), "non-excluded subscriber must receive");
}

#[test]
fn test_remove_subscriber() {
    let (room, _) = make_room();
    let (tx, _rx) = mpsc::unbounded_channel();
    let id = room.lock().unwrap().add_subscriber(tx);
    assert_eq!(room.lock().unwrap().subscriber_count(), 1);
    room.lock().unwrap().remove_subscriber(id);
    assert_eq!(room.lock().unwrap().subscriber_count(), 0);
}

#[test]
fn test_awareness_record_and_replay() {
    let (room, _) = make_room();
    // Initially empty.
    assert!(room.lock().unwrap().last_awareness().is_none());

    room.lock().unwrap().record_awareness(vec![0x01, 0x02, 0x03]);
    let replayed = room.lock().unwrap().last_awareness().map(|b| b.to_vec());
    assert_eq!(replayed, Some(vec![0x01, 0x02, 0x03]));

    // Overwrite with a newer frame.
    room.lock().unwrap().record_awareness(vec![0x04]);
    let replayed = room.lock().unwrap().last_awareness().map(|b| b.to_vec());
    assert_eq!(replayed, Some(vec![0x04]));
}

#[tokio::test]
async fn test_new_subscriber_replays_awareness_after_connect() {
    // Simulate the connect handshake: record awareness first, then a new
    // subscriber's handshake should see the replayed frame.
    let (room, _) = make_room();
    room.lock().unwrap().record_awareness(vec![0xAA, 0xBB]);

    // A second subscriber joins later.
    let (tx2, mut rx2) = mpsc::unbounded_channel();
    room.lock().unwrap().add_subscriber(tx2);

    // Server replays last awareness to the new joiner.
    let frame = room
        .lock()
        .unwrap()
        .last_awareness()
        .map(|b| b.to_vec());
    if let Some(bytes) = frame {
        room.lock().unwrap().broadcast(bytes, None);
    }
    let m = rx2.recv().await.unwrap();
    if let axum::extract::ws::Message::Binary(b) = m {
        assert_eq!(b, vec![0xAA, 0xBB]);
    } else {
        panic!("expected binary");
    }
}

#[test]
fn test_room_loads_initial_content_from_db() {
    let (room, _) = make_room();
    let content = room.lock().unwrap().current_content();
    assert_eq!(content, "room test content");
}
