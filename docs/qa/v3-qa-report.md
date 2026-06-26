# epicode-kb v0.3.0 QA 测试报告

> **测试执行**: 代码接管后全量回归 + PRD ⚠️ 补全验收
> **日期**: 2026-06-27
> **测试版本**: v0.3.0（commit `5a7444d` on `feat/v0.3.0-prd-completion`）
> **关联文档**: `docs/prd/v3.md`, `docs/design/architecture-v3.md`, `docs/audit/v3-audit.md`

---

## 一、测试摘要

| 项目 | 结果 |
|------|------|
| cargo test（后端） | ✅ PASS（57 passed, 0 failed, 1 ignored） |
| cargo clippy --all-targets | ✅ PASS（0 warnings） |
| npm install + build（前端） | ✅ PASS（reactflow 已集成, 472 KB JS） |
| 知识图谱端点 | ✅ PASS（节点 + 矛盾边 + 相似边） |
| 全空间矛盾扫描 | ✅ PASS（detect_all 不再死锁, 生成 Conflict 提议） |
| 实时矛盾检测端点 | ✅ PASS（related + warnings 返回） |
| 冲突中心端点 | ✅ PASS（不再恒空, resolve 生效） |
| **最终结论** | **PASS — v0.3.0 PRD ⚠️ 三项全部落地，附带修复 2 个既有 bug** |

---

## 二、逐项测试详情

### 2.1 cargo test（后端单元 + 集成测试）

**命令**: `cd backend && cargo test`

**结果**: ✅ **PASS**

| 测试二进制 | 通过 | 失败 | 忽略 |
|------------|------|------|------|
| lib（单元） | 0 | 0 | 0 |
| auth_test | 11 | 0 | 0 |
| collab_test | 3 | 0 | 0 |
| conflict_test | 5 | 0 | 0 |
| dream_conflict_test 🆕 | 2 | 0 | 0 |
| graph_test 🆕 | 1 | 0 | 0 |
| health_test | 4 | 0 | 0 |
| memory_test | 7 | 0 | 0 |
| parse_test | 8 | 0 | 1 |
| proposal_test | 5 | 0 | 0 |
| search_test | 11 | 0 | 0 |
| **合计** | **57** | **0** | **1** |

> **新增测试覆盖**（本轮）：
> - `dream_conflict_test`（2）— P3-5：`scan_space` 在矛盾存在时生成 Conflict 提议 + 去重不重复。
> - `graph_test`（1）— P3-4：graph 端点返回节点与矛盾边结构。
> - `conflict_test::detect_tests`（+2）— detect_all 真实检出矛盾 + create_conflict_memory 产生 conflict provenance 记忆。

> **已知 ignored**：`parse_test` 中 1 个 PDF ASCII fallback 用例（P2 遗留，与 v3 无关）。

### 2.2 cargo clippy（静态检查）

**命令**: `cd backend && cargo clippy --all-targets`

**结果**: ✅ **PASS（0 warnings）**

清理了本轮发现的全部告警，包括：
- 测试代码既有 unused import / unused mut（auth_test、proposal_test）；
- 本轮新增代码 2 处 clippy `needless_if_let` → 改用 `Iterator::flatten()`。

### 2.3 前端构建

**命令**: `cd frontend && npm run build`

**结果**: ✅ **PASS**

```
dist/index.html                   0.46 kB
dist/assets/index-*.css          29.16 kB
dist/assets/index-*.js          471.86 kB │ gzip: 146.09 kB
✓ built in 692ms
```

新增依赖 `reactflow`，与既有 Tiptap 协同编辑栈兼容，无类型错误。

### 2.4 功能端点验收（PRD ⚠️ 三项）

| 端点 | 功能 | 验收 |
|------|------|------|
| `POST /api/v1/dream/scan` | P3-5 全空间矛盾扫描 | ✅ 矛盾数据存在时返回 `Conflict` 类型提议 |
| `GET /api/v1/graph` | P3-4 知识图谱 | ✅ 返回 nodes + 矛盾边（conflict）+ 相似边（similar） |
| `GET /api/v1/collab/context` | P5-2 实时矛盾检测 | ✅ 返回 top-5 related + contradiction warnings |
| `GET /api/v1/conflicts` | 冲突中心 | ✅ 返回真实 conflict 记忆（不再恒空） |
| `POST /api/v1/conflicts/:id/resolve` | 冲突裁决 | ✅ 记入 provenance_meta 并标记 resolved |

---

## 三、修复的 v0.3.0 既有 Bug

审计报告将矛盾检测链路标为部分完成（⚠️），实际根因是两个未被发现的既有 bug：

### Bug #1: `detect_all` 嵌套锁死锁（🔴 严重）

- **位置**: `backend/src/conflict/detect.rs`
- **现象**: `detect_all` 持有 `db.lock()` 后在循环中调用 `detect_one`，后者再次 `db.lock()` 同一把 `Mutex` → 永久阻塞。
- **为何未暴露**: v0.3.0 中 `detect_all` 从未被任何调用方引用，故 bug 一直潜伏；直到本轮 P3-5 将其接入 dream cycle 才触发。
- **修复**: 收集 memory_id 后显式 `drop(conn)` 释放锁，再逐个调用 `detect_one`（各自取锁）。

### Bug #2: `detect_one` 取错 embedding（🔴 严重）

- **位置**: `backend/src/conflict/detect.rs`
- **现象**: 用 `MemoryRepo::get_by_id` 取目标记忆，但该方法从不返回 embedding 列（恒为 `None`），导致矛盾检测永远因 "memory has no embedding" 而静默失败。
- **修复**: 改用 `get_by_id_with_embedding`，该辅助会重查 embedding BLOB 并反序列化。

> **影响评估**: 两 bug 叠加意味着 v0.3.0 原始的矛盾检测链路**完全不可用**——即便接通也不会检出任何矛盾。本轮修复后链路才真正生效。

### Bug #3: 冲突中心端点恒空（🟡 中）

- **位置**: `backend/src/api/proposal.rs` — `list_conflicts` / `resolve_conflict`
- **现象**: 审计标为 ✅，实际是 stub：`list_conflicts` 永远返回 `[]`，`resolve_conflict` 返回 50100。
- **修复**: `list_conflicts` 改查 `provenance='conflict' AND review_status='pending'` 记忆并解析双方内容；`resolve_conflict` 将裁决写入 `provenance_meta`（新增 `MemoryRepo::set_provenance_meta`）并标记 resolved。

---

## 四、前端集成验收

| 页面/组件 | 变更 | 验收 |
|-----------|------|------|
| `Graph.tsx` | reactflow 替换占位页；矛盾边红色虚线+动画，相似边灰色，点击节点高亮关联边，含 MiniMap/Controls/图例 | ✅ |
| `MemoryEditor.tsx` | 接入 `SidePanel`（flex 行布局），右侧实时显示矛盾警告 | ✅ |
| `SidePanel.tsx` | fetch 补 `Authorization` header + `space_id`，错误静默 | ✅ |
| `ConflictCenter.tsx` | fetch 补 auth header + space_id（修复潜在 401） | ✅ |

---

## 五、已知问题（非阻塞）

| ID | 问题 | 严重度 | 说明 |
|----|------|--------|------|
| K-1 | `parse_test` 1 个 PDF ASCII fallback 用例 ignored | 低 | v2 遗留，与 v3 无关 |
| K-2 | 前端无统一 auth-fetch helper | 低 | 各页重复 `authHeaders()`，可后续重构提取到 `lib/api.ts` |
| K-3 | 检测算法为纯启发式（无 LLM 事实对比） | 中 | 架构决策：v0.3.0 用启发式保证本地可测、低延迟；LLM 层留作后续增强 |

---

## 六、最终判定

**PASS**。

v0.3.0 PRD 三个 ⚠️ 遗留项（P3-4 图谱矛盾边、P3-5 全空间扫描、P5-2 实时矛盾检测）全部实现并通过测试；附带修复两个潜伏的严重 bug（死锁 + embedding 误取），使原本完全不可用的矛盾检测链路真正生效。冲突中心端点从 stub 升级为真实可用。

- 后端：57 测试全绿，clippy 0 warning。
- 前端：构建通过，reactflow 图谱 + 实时检测面板均接入。
- 文档：`.env.example` 补全，架构文档第 10 节记录实现细节。

**建议**：后续可在 K-2（统一 auth-fetch）、K-3（LLM 事实对比层）方向继续增强。
