//! Collaboration room manager and per-room state.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::extract::ws::Message;
use rusqlite::Connection;
use tokio::sync::mpsc::UnboundedSender;
use yrs::updates::decoder::Decode;
use yrs::updates::encoder::Encode;
use yrs::{Doc, GetString, ReadTxn, StateVector, Text, Transact, Update, WriteTxn};

use crate::db::repository::MemoryRepo;
use crate::error::{AppError, AppResult};

/// A subscriber is identified by a unique id and an outbound channel.
pub type Subscriber = (u64, UnboundedSender<Message>);

/// Manager holding all active collaboration rooms.
pub struct RoomManager {
    db: Arc<Mutex<Connection>>,
    rooms: Mutex<HashMap<String, Arc<Mutex<CollaborationRoom>>>>,
}

impl RoomManager {
    /// Create a new room manager.
    pub fn new(db: Arc<Mutex<Connection>>) -> Self {
        Self {
            db,
            rooms: Mutex::new(HashMap::new()),
        }
    }

    /// Get or create a room for a memory.
    pub fn get_or_create(&self, memory_id: &str) -> AppResult<Arc<Mutex<CollaborationRoom>>> {
        let mut rooms = self.rooms.lock().unwrap();
        if let Some(room) = rooms.get(memory_id) {
            return Ok(room.clone());
        }
        let room = Arc::new(Mutex::new(CollaborationRoom::new(
            memory_id.to_string(),
            self.db.clone(),
        )?));
        rooms.insert(memory_id.to_string(), room.clone());
        Ok(room)
    }
}

/// A single collaboration room backed by a yrs document.
///
/// Awareness is forwarded opaquely (raw bytes) rather than parsed server-side,
/// because `yrs::sync::awareness::Awareness` is not `Send`/`Sync` and cannot
/// live behind a shared `Arc<Mutex<..>>` across async tasks.
pub struct CollaborationRoom {
    memory_id: String,
    doc: Doc,
    subscribers: Vec<Subscriber>,
    next_sub_id: u64,
    /// Last-seen awareness frame (raw bytes), replayed to new joiners so they
    /// learn about existing peers' presence without each peer re-broadcasting.
    last_awareness: Vec<u8>,
}

impl CollaborationRoom {
    /// Create a new collaboration room, loading existing memory content as initial state.
    pub fn new(memory_id: String, db: Arc<Mutex<Connection>>) -> AppResult<Self> {
        let doc = Doc::new();
        let conn = db.lock().unwrap();
        if let Ok(memory) = MemoryRepo::get_by_id(&conn, &memory_id) {
            let mut txn = doc.transact_mut();
            let text = txn.get_or_insert_text("content");
            text.push(&mut txn, &memory.content);
            drop(txn);
        }
        drop(conn);
        Ok(Self {
            memory_id,
            doc,
            subscribers: Vec::new(),
            next_sub_id: 1,
            last_awareness: Vec::new(),
        })
    }

    /// Borrow the underlying yrs document.
    pub fn doc(&self) -> &Doc {
        &self.doc
    }

    /// Apply a yrs update to the document.
    pub fn apply_update(&mut self, update: &[u8]) -> AppResult<()> {
        let update = Update::decode_v1(update)
            .map_err(|e| AppError::bad_request(format!("invalid yrs update: {}", e)))?;
        let mut txn = self.doc.transact_mut();
        txn.apply_update(update)
            .map_err(|e| AppError::internal(format!("failed to apply yrs update: {}", e)))?;
        Ok(())
    }

    /// Get the current document content as plain text.
    pub fn current_content(&self) -> String {
        let txn = self.doc.transact();
        if let Some(text) = txn.get_text("content") {
            text.get_string(&txn)
        } else {
            String::new()
        }
    }

    /// Get the current state vector for sync step 1.
    pub fn state_vector(&self) -> Vec<u8> {
        let txn = self.doc.transact();
        txn.state_vector().encode_v1()
    }

    /// Compute a diff update against a remote state vector.
    pub fn diff_update(&self, remote_sv: &[u8]) -> AppResult<Vec<u8>> {
        let sv = StateVector::decode_v1(remote_sv)
            .map_err(|e| AppError::bad_request(format!("invalid state vector: {}", e)))?;
        let txn = self.doc.transact();
        Ok(txn.encode_diff_v1(&sv))
    }

    /// Register a subscriber; returns its unique id.
    pub fn add_subscriber(&mut self, sender: UnboundedSender<Message>) -> u64 {
        let id = self.next_sub_id;
        self.next_sub_id += 1;
        self.subscribers.push((id, sender));
        id
    }

    /// Remove a subscriber by id.
    pub fn remove_subscriber(&mut self, id: u64) {
        self.subscribers.retain(|(sid, _)| *sid != id);
    }

    /// Broadcast a raw message to all subscribers except `exclude`.
    pub fn broadcast(&self, payload: Vec<u8>, exclude: Option<u64>) {
        for (id, sender) in &self.subscribers {
            if Some(*id) == exclude {
                continue;
            }
            let _ = sender.send(Message::Binary(payload.clone()));
        }
    }

    /// Current number of subscribers.
    pub fn subscriber_count(&self) -> usize {
        self.subscribers.len()
    }

    /// Record the latest raw awareness frame so new joiners can replay it.
    pub fn record_awareness(&mut self, payload: Vec<u8>) {
        self.last_awareness = payload;
    }

    /// Get the last-seen awareness frame (raw bytes), if any.
    pub fn last_awareness(&self) -> Option<&[u8]> {
        if self.last_awareness.is_empty() {
            None
        } else {
            Some(&self.last_awareness)
        }
    }

    /// Get the memory_id.
    pub fn memory_id(&self) -> &str {
        &self.memory_id
    }
}
