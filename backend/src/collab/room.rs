//! Collaboration room manager and per-room state.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::extract::ws::WebSocket;
use rusqlite::Connection;
use yrs::updates::decoder::Decode;
use yrs::{
    updates::encoder::Encode, Doc, GetString, ReadTxn, StateVector, Text, Transact, Update,
    WriteTxn,
};

use crate::db::repository::MemoryRepo;
use crate::error::{AppError, AppResult};

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
pub struct CollaborationRoom {
    memory_id: String,
    doc: Doc,
    #[allow(dead_code)]
    db: Arc<Mutex<Connection>>,
    subscribers: Vec<WebSocket>,
}

impl CollaborationRoom {
    /// Create a new collaboration room, loading existing memory content as initial state.
    pub fn new(memory_id: String, db: Arc<Mutex<Connection>>) -> AppResult<Self> {
        let doc = Doc::new();

        // Load existing content into the document.
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
            db,
            subscribers: Vec::new(),
        })
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

    /// Add a websocket subscriber.
    pub fn add_subscriber(&mut self, socket: WebSocket) {
        self.subscribers.push(socket);
    }

    /// Get the memory_id.
    pub fn memory_id(&self) -> &str {
        &self.memory_id
    }
}
