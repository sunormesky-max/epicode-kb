//! Standard yjs sync protocol message handling.
//!
//! Wraps `yrs::sync::protocol::Message` so the WebSocket loop can encode/decode
//! wire frames in the y-websocket compatible format (no custom framing).

use yrs::sync::protocol::{Message, SyncMessage};
use yrs::updates::decoder::{Decode, DecoderV1};
use yrs::updates::encoder::{Encode, Encoder, EncoderV1};
use yrs::StateVector;

/// Decode a standard yjs wire message from a raw byte frame.
pub fn read_message(data: &[u8]) -> Option<Message> {
    let mut decoder = DecoderV1::from(data);
    Message::decode(&mut decoder).ok()
}

/// Encode a standard yjs wire message into a raw byte frame.
pub fn write_message(msg: &Message) -> Vec<u8> {
    let mut encoder = EncoderV1::new();
    msg.encode(&mut encoder);
    encoder.to_vec()
}

/// Encode a SyncStep1 (state vector) message from an already-serialized SV.
pub fn sync_step1(state_vector: &[u8]) -> Vec<u8> {
    let sv = StateVector::decode_v1(state_vector).unwrap_or_default();
    write_message(&Message::Sync(SyncMessage::SyncStep1(sv)))
}

/// Encode a SyncStep2 (diff update) message.
pub fn sync_step2(update: Vec<u8>) -> Vec<u8> {
    write_message(&Message::Sync(SyncMessage::SyncStep2(update)))
}

/// Encode a Sync Update message.
pub fn sync_update(update: Vec<u8>) -> Vec<u8> {
    write_message(&Message::Sync(SyncMessage::Update(update)))
}
