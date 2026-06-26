# 实时协同编辑设计 (v0.4.0)

> **日期**: 2026-06-27
> **状态**: 已批准,待实现
> **关联**: `docs/design/architecture-v3.md` §10,前端 `MemoryEditor.tsx`,后端 `src/collab/`

## 1. 背景与现状

epicode-kb 前端已安装 yjs / y-websocket / tiptap 全套协同编辑依赖,但 `MemoryEditor` 完全未使用它们(仍是纯 textarea)。后端 `src/collab/` 有半成品实现:

- `room.rs`:服务端 yrs Doc 状态管理,**核心能力已写好**(state_vector、diff_update、apply_update、current_content、加载初始内容)。
- `collab.rs` 的 `collab_ws`:**只发出握手第一步(SyncStep1)就返回**,没有消息循环——不处理客户端 update、不广播给其他订阅者。
- `protocol.rs`:自定义的 1 字节消息类型协议,**与 y-websocket 客户端的标准 yjs sync protocol 不兼容**(y-websocket 默认走标准协议:varint 编码的消息类型 + payload)。

结论:协同编辑链路完全不可用。本设计将其接通为生产可用的实时协同编辑。

## 2. 关键决策(已与用户确认)

| 决策点 | 选择 | 理由 |
|--------|------|------|
| WebSocket 协议 | **标准 yjs sync protocol** | 前端 y-websocket provider 开箱即用,复用 yrs 官方 encode/decode,生态兼容 |
| 持久化策略 | **内存文档 + 手动 Save** | 与现有版本机制一致;重启丢失未保存内容可接受 |
| 感知范围 | **内容同步 + Awareness 光标感知** | 完整协同体验,看到他人实时光标/用户名/颜色 |
| 编辑器形态 | **Tiptap 富文本,存 HTML** | 内容更丰富;需配套去标签预处理以兼容 search/embedding |

## 3. 架构

```
前端 MemoryEditor                       后端 collab_ws 循环
┌─────────────────────┐                ┌──────────────────────────┐
│ Tiptap Editor       │   WebSocket    │ 标准 yjs 协议消息循环     │
│ + Collaboration ext │ ◄────────────► │  - SyncStep1/2 握手       │
│ + CollabCursor ext  │  (y-websocket) │  - Update 接收→apply→广播  │
│ y-websocket provider│                │  - Awareness 接收→广播    │
└──────────┬──────────┘                └────────────┬─────────────┘
           │ getHTML()                               │ room (yrs Doc)
           ▼                                         ▼
   Save 按钮 → POST /memories/:id/save    CollaborationRoom (内存 yrs Doc)
   (写 HTML 到 memory.content,            初始加载 memory.content
    触发去标签 embedding)                 Save 时 room 同步最新内容
```

## 4. 组件设计

### 4.1 后端 `protocol.rs`(重写)

实现标准 yjs sync protocol 编解码。利用 yrs 官方 `yrs::sync` 模块的 `SyncMessage` 枚举:

- 消息类型字节:0 = Sync(内含 step1/step2/update),1 = Awareness,2 = QueryAwareness,3 = Update(historical awareness)。
- 编码:1 字节类型 + varint 长度前缀 + payload。
- 提供 `read_message(buf)` / `write_message(msg)` 顶层函数,供 ws 循环使用。
- 废弃当前自定义 1 字节协议。`CollabMessage` 枚举改为 wrapping `yrs::sync::SyncMessage`。

### 4.2 后端 `room.rs`(增强)

- subscribers 从 `Vec<WebSocket>` 改为 `Vec<mpsc::UnboundedSender<Message>>`:每个连接持有一个出站 channel,room 广播时遍历 sender 发送。这解决 axum WebSocket 不可克隆的问题。
- `broadcast_update(update, exclude)`:把 update 编码后发给除来源连接外的所有订阅者。
- `broadcast_awareness(payload, exclude)`:同理广播 awareness。
- `current_content()` 已存在;Save 时后端可读取。

### 4.3 后端 `collab.rs`(重写消息循环)

`collab_ws` 改为:
1. 握手:服务端主动发 SyncStep1(state vector)。
2. 进入持续读循环:每收到一条消息,按标准协议解码。
   - SyncStep2(客户端的状态差):服务端 `diff_update` 回补。
   - Update(客户端编辑):`apply_update` + `broadcast_update`。
   - Awareness(光标):`broadcast_awareness`。
3. 每个连接 spawn 一个写 task 消费其 channel,主循环负责读。

### 4.4 后端 Save 链路:HTML 去标签

`MemoryService::create` / `save_version` 在 embed/index 前,对 content 做去标签预处理(`strip_html → plain_text`)。新增 `crate::memory::html::strip_tags(html) -> String`(基于轻量正则或字符扫描,不引入新依赖)。memory.content 仍存原始 HTML。

### 4.5 前端 `MemoryEditor.tsx`(重写)

- 用 `@tiptap/react` 的 `useEditor` + `EditorContent`,加载 StarterKit + Collaboration + CollaborationCursor 扩展。
- Yjs doc 通过 `WebsocketProvider('ws://...', memory_id, ydoc)` 连接。
- Save 按钮调 `editor.getHTML()` → `POST /memories/:id/save`。
- 保留右侧 SidePanel(上一轮已接)。

### 4.6 前端用户标识

Awareness 需要用户名/颜色。从 `localStorage`/auth 取用户名,颜色按 user_id 哈希生成。无登录态时降级为匿名。

## 5. 数据流

### 编辑流程
1. 用户 A 在 Tiptap 输入 → Collaboration 扩展产生 Yjs Update。
2. y-websocket provider 把 Update 经 WS 发到后端。
3. 后端 apply_update 到 room 的 yrs Doc,broadcast 给用户 B。
4. 用户 B 的 provider 收到 Update → Yjs doc 合并 → Collaboration 扩展更新 Tiptap → 界面实时刷新。

### 光标流程
1. 用户 A 移动光标 → CollaborationCursor 扩展写 awareness。
2. 经 WS 发到后端 → broadcast 给用户 B。
3. 用户 B 的 provider 更新 awareness → 渲染 A 的光标标签。

### 保存流程
1. 任一用户点 Save → getHTML() → POST /save。
2. 后端存 memory.content(HTML),strip 后 embed + index。
3. 版本记录写入 memory_versions(现有机制)。

## 6. 错误处理

- WS 断连:room 移除该 subscriber;重连后 SyncStep1 重新同步(幂等)。
- 无效 update:yrs apply 返回 Err → 记 warn 日志,丢弃该消息(不崩连接)。
- room 内存丢失(重启):重新 get_or_create 时从 DB 重新加载 memory.content 为初始状态。

## 7. 测试策略

- 后端单元:protocol 编解码往返;room 的 apply/broadcast;html::strip_tags。
- 后端集成:模拟两路 WS 连接同一 memory_id,验证 update 互通。
- 现有 collab_test 的 3 个 protocol 往返测试需更新为标准协议。
- 前端:npm build 通过(Tiptap 集成编译)。

## 8. 范围外(YAGNI)

- 不做 Yjs 二进制 state 持久化(决策 2:手动 Save 即可)。
- 不做权限粒度的协同(谁能编辑)——沿用现有 RBAC,有写权限即可加入 room。
- 不做离线编辑/重连合并测试自动化(Yjs CRDT 本身保证收敛)。
