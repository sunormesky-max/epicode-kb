# 实时协同编辑实现计划 (v0.4.0)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把 epicode-kb 的协同编辑从半成品接通为生产可用的实时协同富文本编辑(多人光标 + 内容实时合并),采用标准 yjs sync protocol。

**Architecture:** 后端重写自定义协议为标准 yjs 协议(yrs `sync` 模块),room 增加 channel 广播与 awareness;前端 MemoryEditor 接入 Tiptap + y-websocket provider。Save 时 HTML 写入 memory.content,经去标签预处理后再 embed/index。

**Tech Stack:** Rust (axum WS, yrs 0.21 sync/awareness/protocol, tokio mpsc), React (Tiptap, yjs, y-websocket)

**关键 yrs 0.21 API(已核实源码):**
- `yrs::sync::protocol::{Message, SyncMessage}` — 顶层消息 enum + 子消息 enum,均 impl `Encode`/`Decode`
- `Message` variants: `Sync(SyncMessage)` / `AwarenessQuery` / `Awareness(AwarenessUpdate)` / `Auth` / `Custom`
- `SyncMessage` variants: `SyncStep1(StateVector)` / `SyncStep2(Vec<u8>)` / `Update(Vec<u8>)`
- `yrs::sync::awareness::Awareness` — `new(doc)`, `set_local_state`, `apply_update`, `update() -> AwarenessUpdate`, `iter()`
- `Encode::encode::<E: Encoder>(&self, &mut E)`, `Decode::decode::<D: Decoder>(&mut D)`
- 编解码用 `EncoderV1` / `DecoderV1`,`Message::encode_v1()` / `Message::decode_v1(&[u8])`

---

### Task 1: HTML 去标签模块

**Files:**
- Create: `backend/src/memory/html.rs`
- Modify: `backend/src/memory/mod.rs` (加 `pub mod html;`)
- Test: `backend/tests/html_test.rs`

- [ ] **Step 1: 写失败测试**

```rust
// backend/tests/html_test.rs
use epicode_kb::memory::html::strip_tags;

#[test]
fn test_strip_basic_tags() {
    assert_eq!(strip_tags("<h1>Title</h1><p>Body text</p>"), "TitleBody text");
}

#[test]
fn test_strip_preserves_text_between_tags() {
    assert_eq!(strip_tags("<ul><li>One</li><li>Two</li></ul>"), "OneTwo");
}

#[test]
fn test_strip_unescapes_common_entities() {
    assert_eq!(strip_tags("a &amp; b &lt;tag&gt; &quot;q&quot;"), "a & b <tag> \"q\"");
}

#[test]
fn test_strip_plain_text_unchanged() {
    assert_eq!(strip_tags("no html here"), "no html here");
}

#[test]
fn test_strip_empty_and_whitespace() {
    assert_eq!(strip_tags(""), "");
    assert_eq!(strip_tags("   <p>  </p>  ").trim(), "");
}

#[test]
fn test_strip_handles_unclosed_tag() {
    assert_eq!(strip_tags("text <b bold"), "text ");
}
```

- [ ] **Step 2: 跑测试验证失败**

Run: `cd backend && cargo test --test html_test`
Expected: FAIL(模块/函数不存在)

- [ ] **Step 3: 实现**

```rust
// backend/src/memory/html.rs
//! Minimal HTML → plain-text stripper for embedding/search indexing.
//! No external dependency: char-scan based, handles common entities.

/// Strip HTML tags from a string and unescape common entities.
pub fn strip_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let bytes = html.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'<' {
            // Skip until matching '>'.
            while i < bytes.len() && bytes[i] != b'>' {
                i += 1;
            }
            if i < bytes.len() {
                i += 1; // consume '>'
            }
            continue;
        }
        if bytes[i] == b'&' {
            // Try to unescape a known entity.
            if let Some((ch, consumed)) = unescape_entity(&html[i..]) {
                out.push(ch);
                i += consumed;
                continue;
            }
        }
        // Safe UTF-8 boundary push.
        let ch_len = utf8_len(bytes[i]);
        let end = (i + ch_len).min(bytes.len());
        out.push_str(&html[i..end]);
        i = end;
    }
    out
}

fn utf8_len(first: u8) -> usize {
    if first < 0x80 { 1 }
    else if first >> 5 == 0b110 { 2 }
    else if first >> 4 == 0b1110 { 3 }
    else if first >> 3 == 0b11110 { 4 }
    else { 1 } // invalid continuation byte: advance 1
}

fn unescape_entity(s: &str) -> Option<(char, usize)> {
    let rest = s.strip_prefix('&')?;
    if let Some(name) = rest.strip_prefix("amp;") { return Some(('&', 5)); }
    if let Some(name) = rest.strip_prefix("lt;") { return Some(('<', 4)); }
    if let Some(name) = rest.strip_prefix("gt;") { return Some(('>', 4)); }
    if let Some(name) = rest.strip_prefix("quot;") { return Some(('"', 6)); }
    if let Some(name) = rest.strip_prefix("apos;") { return Some(('\'', 6)); }
    if let Some(name) = rest.strip_prefix("nbsp;") { return Some((' ', 6)); }
    None
}
```

- [ ] **Step 4: 在 `backend/src/memory/mod.rs` 加模块声明**

```rust
pub mod html;
```

- [ ] **Step 5: 跑测试验证通过**

Run: `cd backend && cargo test --test html_test`
Expected: PASS(6 tests)

- [ ] **Step 6: 提交**

```bash
git add backend/src/memory/html.rs backend/src/memory/mod.rs backend/tests/html_test.rs
git commit -m "feat(memory): HTML 去标签模块 strip_tags + 测试"
```

---

### Task 2: protocol.rs 重写为标准 yjs 协议

**Files:**
- Modify: `backend/src/collab/protocol.rs`(完全重写)
- Modify: `backend/tests/collab_test.rs`(更新往返测试为标准协议)

- [ ] **Step 1: 重写 protocol.rs**

```rust
//! Standard yjs sync protocol message handling.
//!
//! Wraps yrs::sync::protocol::Message so the ws loop can encode/decode
//! wire frames in the y-websocket compatible format.

use yrs::sync::protocol::Message;
use yrs::updates::decoder::{Decode, DecoderV1};
use yrs::updates::encoder::{Encode, EncoderV1};

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

/// Convenience: encode a SyncStep1 (server state vector) message.
pub fn sync_step1(state_vector: Vec<u8>) -> Vec<u8> {
    use yrs::sync::protocol::SyncMessage;
    use yrs::StateVector;
    let sv = StateVector::decode_v1(&state_vector).unwrap_or_default();
    write_message(&Message::Sync(SyncMessage::SyncStep1(sv)))
}

/// Convenience: encode a SyncStep2 (diff update) message.
pub fn sync_step2(update: Vec<u8>) -> Vec<u8> {
    use yrs::sync::protocol::SyncMessage;
    write_message(&Message::Sync(SyncMessage::SyncStep2(update)))
}

/// Convenience: encode a Sync Update message.
pub fn sync_update(update: Vec<u8>) -> Vec<u8> {
    use yrs::sync::protocol::SyncMessage;
    write_message(&Message::Sync(SyncMessage::Update(update)))
}
```

- [ ] **Step 2: 删除旧 CollabMessage 枚举(不再使用)**

protocol.rs 中不再有 `CollabMessage` enum。检查所有引用点并迁移:`backend/src/api/collab.rs` 中的 `use crate::collab::protocol::CollabMessage;` 将在 Task 4 重写时移除。

- [ ] **Step 3: 更新 collab_test.rs 往返测试**

```rust
// backend/tests/collab_test.rs —— 完全重写
use epicode_kb::collab::protocol::{read_message, write_message};
use yrs::sync::protocol::{Message, SyncMessage};
use yrs::StateVector;

#[test]
fn test_sync_step1_roundtrip() {
    let msg = Message::Sync(SyncMessage::SyncStep1(StateVector::default()));
    let encoded = write_message(&msg);
    let decoded = read_message(&encoded).expect("decode ok");
    assert_eq!(msg, decoded);
}

#[test]
fn test_sync_update_roundtrip() {
    let msg = Message::Sync(SyncMessage::Update(vec![1, 2, 3, 4]));
    let encoded = write_message(&msg);
    let decoded = read_message(&encoded).expect("decode ok");
    assert_eq!(msg, decoded);
}

#[test]
fn test_read_message_garbage_returns_none() {
    assert!(read_message(&[0xFF, 0xFF]).is_none());
}
```

- [ ] **Step 4: 跑测试**

Run: `cd backend && cargo test --test collab_test`
Expected: PASS(3 tests)。若 collab.rs 仍引用旧 CollabMessage 导致编译失败,先在 Task 4 修复后再统一验证。

- [ ] **Step 5: 提交**

```bash
git add backend/src/collab/protocol.rs backend/tests/collab_test.rs
git commit -m "refactor(collab): protocol.rs 改为标准 yjs sync 协议"
```

---

### Task 3: room.rs 增强 broadcast + awareness

**Files:**
- Modify: `backend/src/collab/room.rs`

- [ ] **Step 1: 改造 subscribers 为 channel sender,加 broadcast 方法**

替换整个 `room.rs`(保留现有 yrs Doc 加载/apply/diff 逻辑,增加广播与 awareness):

```rust
//! Collaboration room manager and per-room state.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::extract::ws::Message;
use rusqlite::Connection;
use tokio::sync::mpsc::UnboundedSender;
use yrs::sync::awareness::Awareness;
use yrs::updates::decoder::Decode;
use yrs::updates::encoder::Encode;
use yrs::{Doc, GetString, ReadTxn, StateVector, Text, Transact, Update};

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
    pub fn new(db: Arc<Mutex<Connection>>) -> Self {
        Self { db, rooms: Mutex::new(HashMap::new()) }
    }

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

/// A single collaboration room backed by a yrs document + awareness.
pub struct CollaborationRoom {
    memory_id: String,
    awareness: Awareness,
    subscribers: Vec<Subscriber>,
    next_sub_id: u64,
}

impl CollaborationRoom {
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
            awareness: Awareness::new(doc),
            subscribers: Vec::new(),
            next_sub_id: 1,
        })
    }

    /// Borrow the awareness (for applying updates / reading local state).
    pub fn awareness(&self) -> &Awareness {
        &self.awareness
    }

    /// Borrow the awareness mutably.
    pub fn awareness_mut(&mut self) -> &mut Awareness {
        &mut self.awareness
    }

    /// Borrow the underlying doc.
    pub fn doc(&self) -> &Doc {
        self.awareness.doc()
    }

    /// Apply a yrs update to the document.
    pub fn apply_update(&mut self, update: &[u8]) -> AppResult<()> {
        let update = Update::decode_v1(update)
            .map_err(|e| AppError::bad_request(format!("invalid yrs update: {}", e)))?;
        let mut txn = self.doc().transact_mut();
        txn.apply_update(update)
            .map_err(|e| AppError::internal(format!("failed to apply yrs update: {}", e)))?;
        Ok(())
    }

    /// Current document content as plain text.
    pub fn current_content(&self) -> String {
        let txn = self.doc().transact();
        if let Some(text) = txn.get_text("content") {
            text.get_string(&txn)
        } else {
            String::new()
        }
    }

    /// State vector for sync step 1.
    pub fn state_vector(&self) -> Vec<u8> {
        let txn = self.doc().transact();
        txn.state_vector().encode_v1()
    }

    /// Diff update against a remote state vector.
    pub fn diff_update(&self, remote_sv: &[u8]) -> AppResult<Vec<u8>> {
        let sv = StateVector::decode_v1(remote_sv)
            .map_err(|e| AppError::bad_request(format!("invalid state vector: {}", e)))?;
        let txn = self.doc().transact();
        Ok(txn.encode_diff_v1(&sv))
    }

    /// Register a subscriber; returns its id.
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

    pub fn subscriber_count(&self) -> usize {
        self.subscribers.len()
    }
}
```

- [ ] **Step 2: 跑 `cargo check`**

Run: `cd backend && cargo check`
Expected: 可能因 `api/collab.rs` 仍引用旧 `add_subscriber(socket)` / `CollabMessage` 报错 —— 这些在 Task 4 修复。确认错误仅来自 collab.rs。

- [ ] **Step 3: 提交**

```bash
git add backend/src/collab/room.rs
git commit -m "refactor(collab): room 改为 channel 广播 + awareness"
```

---

### Task 4: collab.rs 重写消息循环 + WS handler

**Files:**
- Modify: `backend/src/api/collab.rs`(重写 `collab_ws`,保留 `get_context`)

- [ ] **Step 1: 重写 collab_ws 为标准 yjs 消息循环**

替换 `collab_ws` 函数(保留文件顶部的 `get_context` 及其 imports)。新 `collab_ws`:

```rust
/// WebSocket handler for /api/v1/collab/:memory_id (standard yjs protocol).
pub async fn collab_ws(
    State(state): State<Arc<AppState>>,
    Extension(_actor): Extension<Actor>,
    Path(memory_id): Path<String>,
    ws: WebSocketUpgrade,
) -> Result<Response, AppError> {
    let room_manager = state.room_manager.clone();
    let room = room_manager.get_or_create(&memory_id)?;

    Ok(ws.on_upgrade(move |socket| async move {
        if let Err(e) = handle_collab_socket(socket, room).await {
            tracing::warn!("collab socket error for {}: {}", memory_id, e);
        }
    }))
}

async fn handle_collab_socket(
    socket: axum::extract::ws::WebSocket,
    room: Arc<std::sync::Mutex<crate::collab::room::CollaborationRoom>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use axum::extract::ws::{Message, WebSocket};
    use tokio::sync::mpsc;
    use yrs::sync::protocol::{Message as YMessage, SyncMessage};
    use yrs::updates::encoder::Encode;

    let (mut ws_sink, mut ws_stream) = socket.split();

    // Outbound channel: room broadcasts push Message here; a writer task drains it.
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
    let sub_id = {
        let mut room_guard = room.lock().unwrap();
        room_guard.add_subscriber(tx)
    };

    // Server-initiated sync step 1: send our state vector.
    {
        let room_guard = room.lock().unwrap();
        let sv = room_guard.state_vector();
        let frame = crate::collab::protocol::sync_step1(sv);
        let _ = ws_sink.send(Message::Binary(frame)).await;
    }

    // Writer task: drain broadcast channel → ws.
    let writer = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_sink.send(msg).await.is_err() {
                break;
            }
        }
        ws_sink
    });

    // Reader loop: ws → process.
    while let Some(msg) = ws_stream.next().await {
        let msg = match msg {
            Ok(m) => m,
            Err(_) => break,
        };
        let payload = match msg {
            Message::Binary(b) => b,
            Message::Text(t) => t.into_bytes(),
            Message::Ping(_) | Message::Pong(_) | Message::Close(_) => continue,
        };

        let ymsg = match crate::collab::protocol::read_message(&payload) {
            Some(m) => m,
            None => {
                tracing::debug!("collab: dropping undecodable message");
                continue;
            }
        };

        match ymsg {
            YMessage::Sync(sync_msg) => match sync_msg {
                SyncMessage::SyncStep1(remote_sv) => {
                    // Client wants our diff: reply SyncStep2.
                    let diff = {
                        let room_guard = room.lock().unwrap();
                        room_guard.diff_update(&remote_sv).unwrap_or_default()
                    };
                    let frame = crate::collab::protocol::sync_step2(diff);
                    let _ = room.lock().unwrap().broadcast(frame, None);
                    // Also send our own SyncStep1 to request client state.
                    let sv = {
                        let room_guard = room.lock().unwrap();
                        room_guard.state_vector()
                    };
                    let _ = room
                        .lock()
                        .unwrap()
                        .broadcast(crate::collab::protocol::sync_step1(sv), Some(sub_id));
                }
                SyncMessage::SyncStep2(_) => {
                    // Client sent its missing diff to us; nothing to broadcast.
                }
                SyncMessage::Update(update) => {
                    // Apply + broadcast to others.
                    let frame = payload.clone(); // re-broadcast the original Update frame
                    {
                        let mut room_guard = room.lock().unwrap();
                        let _ = room_guard.apply_update(&update);
                        room_guard.broadcast(frame, Some(sub_id));
                    }
                }
            },
            YMessage::AwarenessQuery => {
                // Client asks for full awareness; broadcast our awareness.
                let frame = {
                    let room_guard = room.lock().unwrap();
                    match room_guard.awareness().update() {
                        Ok(au) => crate::collab::protocol::write_message(
                            &YMessage::Awareness(au),
                        ),
                        Err(_) => continue,
                    }
                };
                let _ = room.lock().unwrap().broadcast(frame, Some(sub_id));
            }
            YMessage::Awareness(au) => {
                // Client awareness update: apply + broadcast to others.
                {
                    let mut room_guard = room.lock().unwrap();
                    let _ = room_guard.awareness_mut().apply_update(au);
                }
                let _ = room.lock().unwrap().broadcast(payload, Some(sub_id));
            }
            _ => {}
        }
    }

    // Cleanup: remove subscriber and close writer.
    room.lock().unwrap().remove_subscriber(sub_id);
    let _ = writer.await;
    Ok(())
}
```

- [ ] **Step 2: 调整 collab.rs imports**

移除 `use crate::collab::protocol::CollabMessage;`(已不存在)。保留 `get_context` 用到的 imports。若 `Encode` 在 collab.rs 未用则不加。

- [ ] **Step 3: 确认 `futures`/`tokio` 依赖可用**

`socket.split()` 需要 `futures::StreamExt`。检查 Cargo.toml 是否有 `futures`。若无,在 Task 4 Step 用 `tokio::stream` 或加依赖。

Run: `cd backend && cargo check 2>&1 | grep -i "split\|StreamExt"`
若无 `futures`,添加 `futures = "0.3"` 到 Cargo.toml。

- [ ] **Step 4: cargo check 通过**

Run: `cd backend && cargo check`
Expected: PASS

- [ ] **Step 5: 提交**

```bash
git add backend/src/api/collab.rs backend/Cargo.toml backend/Cargo.lock
git commit -m "feat(collab): 标准yjs消息循环+广播+awareness"
```

---

### Task 5: Save 链路接入去标签

**Files:**
- Modify: `backend/src/memory/service.rs`(create / create_from_agent / save_version 中 embed/index 前去标签)

- [ ] **Step 1: 定位 embed 调用点**

Run: `cd backend && grep -n "embedder.embed(&memory.content)" src/memory/service.rs`

三处:`create`(~101)、`create_from_agent`(~174)、可能 `save_version`。

- [ ] **Step 2: 在 embed 前注入去标签**

每处把:
```rust
match self.embedder.embed(&memory.content) {
```
改为:
```rust
let embed_source = crate::memory::html::strip_tags(&memory.content);
match self.embedder.embed(&embed_source) {
```

注意:`memory.content` 本身不改(仍存 HTML),只改喂给 embedder 的字符串。

- [ ] **Step 3: 确认 tantivy 索引侧**

检查 `src/search/hybrid.rs` 或 `fulltext.rs` 索引时是否也用 content。若用,在 `get_indexable_fields` 返回的 content 上同样去标签(或在索引层去标签)。优先在写入索引处去标签,保持搜索纯文本匹配。

Run: `cd backend && grep -rn "get_indexable_fields\|add_document\|content" src/search/ | grep -i index`

若索引直接用 memory.content,在 service 层新增一个 `plain_content()` 供索引使用,或在 `MemoryRepo::get_indexable_fields` 处去标签。最小改动:在调用索引写入处去标签。

- [ ] **Step 4: cargo test 验证不回归**

Run: `cd backend && cargo test`
Expected: 全部通过(现有记忆是纯文本,strip_tags 不改变它们)

- [ ] **Step 5: 提交**

```bash
git add backend/src/memory/service.rs backend/src/search/
git commit -m "feat(memory): embed/index 前去HTML标签,兼容富文本"
```

---

### Task 6: 前端 MemoryEditor 接入 Tiptap + y-websocket

**Files:**
- Modify: `frontend/src/pages/MemoryEditor.tsx`(完全重写)

- [ ] **Step 1: 重写 MemoryEditor.tsx**

```tsx
import { useEffect, useState } from 'react'
import { useParams } from 'react-router-dom'
import { useEditor, EditorContent } from '@tiptap/react'
import StarterKit from '@tiptap/starter-kit'
import Collaboration from '@tiptap/extension-collaboration'
import CollaborationCursor from '@tiptap/extension-collaboration-cursor'
import * as Y from 'yjs'
import { WebsocketProvider } from 'y-websocket'
import SidePanel from '../components/SidePanel'

interface Memory { id: string; content: string; space_id: string }

const COLORS = ['#f58231', '#911eb4', '#46f0f0', '#f032e6', '#bcf60c', '#fabed4']

function userColor(userId: string): string {
  let h = 0
  for (let i = 0; i < userId.length; i++) h = (h * 31 + userId.charCodeAt(i)) >>> 0
  return COLORS[h % COLORS.length]
}

export default function MemoryEditor() {
  const { id } = useParams<{ id: string }>()
  const [ydoc, setYdoc] = useState<Y.Doc | null>(null)
  const [provider, setProvider] = useState<WebsocketProvider | null>(null)
  const [loaded, setLoaded] = useState(false)
  const [saving, setSaving] = useState(false)

  // Load existing memory content, then init Yjs doc + provider.
  useEffect(() => {
    if (!id) return
    let doc: Y.Doc | null = null
    let prov: WebsocketProvider | null = null
    fetch(`/api/v1/memories/${id}`, { headers: authHeaders() })
      .then((r) => r.json())
      .then((data) => {
        if (data.code !== 0 || !data.data) return
        const content = data.data.content || ''
        doc = new Y.Doc()
        if (content) {
          const text = doc.getText('content')
          // content may be HTML; y-websocket sync will reconcile with server.
          text.insert(0, content)
        }
        const wsUrl = `${location.protocol === 'https:' ? 'wss' : 'ws'}://${location.host}/api/v1/collab/${id}`
        prov = new WebsocketProvider(wsUrl, id, doc)
        prov.awareness.setLocalStateField('user', {
          name: localStorage.getItem('user_name') || 'Anonymous',
          color: userColor(localStorage.getItem('user_id') || 'anon'),
        })
        setYdoc(doc)
        setProvider(prov)
        setLoaded(true)
      })
    return () => {
      prov?.destroy()
      doc?.destroy()
    }
  }, [id])

  const editor = useEditor({
    extensions: [
      StarterKit.configure({ history: false }), // Yjs handles undo
      ...(ydoc ? [Collaboration.configure({ document: ydoc, field: 'content' })] : []),
      ...(provider
        ? [CollaborationCursor.configure({ provider, user: { name: 'me', color: '#f58231' } })]
        : []),
    ],
    editorProps: {
      attributes: { class: 'prose max-w-none min-h-[400px] p-4 focus:outline-none' },
    },
  }, [ydoc, provider])

  // Update cursor user info once editor/provider ready.
  useEffect(() => {
    if (provider) {
      provider.awareness.setLocalStateField('user', {
        name: localStorage.getItem('user_name') || 'Anonymous',
        color: userColor(localStorage.getItem('user_id') || 'anon'),
      })
    }
  }, [provider])

  const handleSave = async () => {
    if (!editor || !id) return
    setSaving(true)
    try {
      const html = editor.getHTML()
      await fetch(`/api/v1/memories/${id}/save`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', ...authHeaders() },
        body: JSON.stringify({ content: html }),
      })
    } finally {
      setSaving(false)
    }
  }

  if (!loaded || !editor) {
    return <div className="p-8 text-gray-500">Loading editor…</div>
  }

  return (
    <div className="flex gap-4 h-full">
      <div className="flex-1 p-8 max-w-4xl mx-auto">
        <div className="flex items-center justify-between mb-4">
          <h1 className="text-2xl font-bold">Edit Memory</h1>
          <button
            onClick={handleSave}
            disabled={saving}
            className="bg-blue-600 text-white py-2 px-4 rounded hover:bg-blue-700 disabled:opacity-50"
          >
            {saving ? 'Saving…' : 'Save Version'}
          </button>
        </div>
        <div className="bg-white rounded-lg shadow border border-gray-200">
          <EditorContent editor={editor} />
        </div>
      </div>
      {id && <SidePanel memoryId={id} editorContent={editor.getText()} />}
    </div>
  )
}

function authHeaders(): Record<string, string> {
  const token = localStorage.getItem('access_token')
  return token ? { Authorization: `Bearer ${token}` } : {}
}
```

- [ ] **Step 2: npm build 验证**

Run: `cd frontend && npm run build`
Expected: PASS(无 TS 错误)。若 tiptap extensions 类型不符,调整 import。

- [ ] **Step 3: 提交**

```bash
git add frontend/src/pages/MemoryEditor.tsx
git commit -m "feat(editor): MemoryEditor 接入 Tiptap+y-websocket 实时协同"
```

---

### Task 7: 全量验证

- [ ] **Step 1: cargo test 全绿**

Run: `cd backend && cargo test`
Expected: 全部通过(含新增 html_test 6 + 更新 collab_test 3)

- [ ] **Step 2: cargo clippy 0 warning**

Run: `cd backend && cargo clippy --all-targets`
Expected: 0 warnings。修复任何新引入的 warning。

- [ ] **Step 3: npm build 通过**

Run: `cd frontend && npm run build`
Expected: PASS

- [ ] **Step 4: 提交(若有修复)+ 最终验证报告**

记录测试数、clippy、build 结果。
```

---

## Self-Review 记录

**Spec 覆盖:** 设计 §4.1→Task2, §4.2→Task3, §4.3→Task4, §4.4→Task1+5, §4.5+4.6→Task6。✅
**Placeholder:** 无 TBD/TODO。✅
**Type 一致性:** `read_message`/`write_message`/`sync_step1/2`/`sync_update` 在 Task2 定义后 Task4 使用,签名一致。`add_subscriber(sender)->u64`/`broadcast(payload,exclude)`/`remove_subscriber(id)` 一致。✅

**已知风险(执行时关注):**
- `socket.split()` 需 `futures::StreamExt` —— Task4 Step3 已纳入检查。
- yrs `Awareness::apply_update` / `update()` 的借用:awareness 内部锁,锁 room 时调用需注意顺序(每步都短期锁)。
- 前端 y-websocket 连接 wsUrl 路径需与后端路由 `/api/v1/collab/:memory_id` 一致,且 auth:ws 连接默认不带 JWT header,需确认中间件是否对 ws 放行或用 query token。**这是执行时第一个要验证的点。**
