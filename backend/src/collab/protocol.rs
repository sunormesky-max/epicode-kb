//! yjs protocol message parsing for WebSocket collaboration.

/// Message types exchanged over the collab WebSocket.
#[derive(Debug, Clone)]
pub enum CollabMessage {
    /// yjs sync step 1 message (client state vector).
    SyncStep1(Vec<u8>),
    /// yjs sync step 2 message (server update in response to step 1).
    SyncStep2(Vec<u8>),
    /// yjs update message.
    Update(Vec<u8>),
    /// yjs awareness update.
    Awareness(Vec<u8>),
}

impl CollabMessage {
    /// Parse a raw WebSocket binary message into a collab message.
    /// Uses a simple custom framing: first byte is message kind.
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.is_empty() {
            return None;
        }
        match data[0] {
            0 => Some(CollabMessage::SyncStep1(data[1..].to_vec())),
            1 => Some(CollabMessage::SyncStep2(data[1..].to_vec())),
            2 => Some(CollabMessage::Update(data[1..].to_vec())),
            3 => Some(CollabMessage::Awareness(data[1..].to_vec())),
            _ => None,
        }
    }

    /// Encode a collab message into a binary frame.
    pub fn encode(&self) -> Vec<u8> {
        let mut result = Vec::new();
        match self {
            CollabMessage::SyncStep1(payload) => {
                result.push(0);
                result.extend_from_slice(payload);
            }
            CollabMessage::SyncStep2(payload) => {
                result.push(1);
                result.extend_from_slice(payload);
            }
            CollabMessage::Update(payload) => {
                result.push(2);
                result.extend_from_slice(payload);
            }
            CollabMessage::Awareness(payload) => {
                result.push(3);
                result.extend_from_slice(payload);
            }
        }
        result
    }
}
