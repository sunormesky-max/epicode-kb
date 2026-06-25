# epicode-kb v0.3.0 增量系统架构设计

> **文档版本**：v3.0
> **基线版本**：v0.2.0（architecture-v2.md）
> **日期**：2026-06-26
> **作者**：高见远（架构师）+ 齐活林（交付总监）
> **状态**：待评审
> **关联 PRD**：`docs/prd/v3.md`
> **基线架构**：`docs/design/architecture-v2.md`

---

## 1. 实现方案概述

### 1.1 总体目标

v0.3.0 将 epicode-kb 从"基础设施完善"升级为"人机协同认知闭环"，实现 AI 提议审核、语义矛盾检测、知识健康度自感知、协同编辑增强。

### 1.2 与 v0.2.0 的核心差异

| 维度 | v0.2.0 现状 | v0.3.0 目标 | 基于现有骨架 |
|------|------------|------------|-------------|
| **AI 提议** | `dream/proposal.rs` stub（全部返回 501） | 完整 Proposal 状态机 + dream cycle 改造 + 审核 API/UI | ✅ 类型已定义、API stub 已注册 |
| **矛盾检测** | 协同冲突解决（save/revert），无语义矛盾检测 | 语义距离+LLM 事实对比 → Conflict 记忆 → 冲突中心 UI | ❌ 全新模块 |
| **知识健康度** | `observability/metrics.rs` 基础 Prometheus | 查询日志 + 衰减模型 + 缺口/孤岛检测 + 仪表盘 | ❌ 全新模块 |
| **通知系统** | `notify/webhook.rs` stub（NoopNotifier） | Webhook 签名验证 + 事件订阅 | ✅ Notifier trait 已定义 |
| **协同增强** | Tiptap+yjs 实时编辑 | 编辑器上下文召回 + 侧栏建议面板 | ❌ 全新前端组件 |
| **LLM 集成** | `llm/deepseek.rs` + `llm/ollama.rs` 存在 | 接通矛盾事实对比 + 提案生成 | ✅ LLM trait 已定义 |

### 1.3 架构演进总览

```
v0.2.0 已有层           v0.3.0 新增层
─────────────────      ─────────────────
auth/                   dream/          ← 改造：输出 Proposal
collab/                 conflict/       ← 新增：矛盾检测
embed/                  health/         ← 新增：健康度
mcp/                    notify/         ← 改造：接通 Webhook
observability/          llm/            ← 接通：事实对比
parse/                  
memory/                 api/proposal.rs ← 改造：501→真实实现
search/                 api/conflict.rs ← 新增
api/agent.rs            api/health.rs   ← 新增
api/auth.rs             
```

### 1.4 关键技术决策

| 决策 | 方案 | 理由 |
|------|------|------|
| LLM 事实对比 | 接 `llm/deepseek.rs` + `llm/ollama.rs` 已有 trait | 避免重复造轮子，已有 `LlmProvider` trait |
| 矛盾检测阈值 | 默认 0.3（语义距离），可配置 | 平衡误报/漏报 |
| 健康快照 | 异步定时任务（tokio::spawn），每天一次 | 不影响请求延迟 |
| Proposal 存储 | SQLite（复用现有 repo 模式） | 保持架构一致性 |
| 通知 Webhook | HMAC-SHA256 签名 + reqwest POST | 安全 + 轻量 |

---

## 2. 框架选型与依赖

### 2.1 Rust crates（新增/升级）

```toml
# 无需新增 crate。v0.2.0 已具备：
# - reqwest (HTTP webhook)
# - sha2 + hmac (webhook 签名)
# - serde_json (JSON 处理)
# - tokio (异步定时任务)
# - tracing (日志)
#
# LLM 层已有 llm/mod.rs + deepseek.rs + ollama.rs
```

### 2.2 npm packages（新增）

```json
{
  "dependencies": {
    "recharts": "^2.12"   // P4 健康仪表盘图表
  }
}
```

---

## 3. 文件列表

### 3.1 后端文件

```
backend/
├── src/
│   ├── db/migrations/004_v3_schema.sql     ➕ query_logs / knowledge_health 表
│   ├── db/repository.rs                    🔶 新增 ProposalRepo / QueryLogRepo / HealthRepo / NotifySubRepo
│   ├── db/schema.rs                        🔶 导出新表结构
│   │
│   ├── dream/
│   │   ├── mod.rs                          🔶 dream cycle 入口：增加 propose_mode
│   │   └── proposal.rs                     🔶 从 stub 升级为完整实现
│   │
│   ├── conflict/
│   │   ├── mod.rs                          ➕ 矛盾检测模块入口
│   │   ├── detect.rs                       ➕ 语义距离 + LLM 事实对比
│   │   └── model.rs                        ➕ ConflictResult / Resolution 类型
│   │
│   ├── health/
│   │   ├── mod.rs                          ➕ 健康度模块入口
│   │   ├── scanner.rs                      ➕ 全量/增量健康扫描
│   │   ├── staleness.rs                    ➕ 衰减模型
│   │   ├── gaps.rs                         ➕ 知识缺口检测
│   │   └── orphans.rs                      ➕ 孤岛检测
│   │
│   ├── notify/
│   │   ├── mod.rs                          🔶 通知调度
│   │   ├── webhook.rs                      🔶 stub→真实 HTTP POST + HMAC
│   │   └── subscriptions.rs                ➕ 订阅管理
│   │
│   ├── api/
│   │   ├── proposal.rs                     🔶 501 stub→真实 CRUD
│   │   ├── conflict.rs                      ➕ GET /conflicts, POST /conflicts/:id/resolve, POST /conflicts/detect
│   │   ├── health_api.rs                   ➕ GET /health/space/:id, GET /health/gaps, POST /health/scan
│   │   ├── collab.rs                        🔶 新增 GET /collab/context 端点
│   │   └── routes.rs                        🔶 注册新路由组
│   │
│   ├── llm/
│   │   ├── mod.rs                          🔶 增加 fact_compare 方法到 LlmProvider trait
│   │   ├── deepseek.rs                     🔶 实现 fact_compare
│   │   └── ollama.rs                       🔶 实现 fact_compare
│   │
│   ├── config.rs                           🔶 新增冲突阈值 / 健康扫描频率 / webhook 配置
│   ├── state.rs                            🔶 注入 ProposalEngine / ConflictDetector / HealthScanner
│   ├── error.rs                            🔶 新增 409 冲突错误码
│   └── lib.rs                               🔶 注册新模块
│
└── tests/
    ├── proposal_test.rs                    ➕ Proposal 状态机 + API 测试
    ├── conflict_test.rs                    ➕ 矛盾检测 + 解决测试
    ├── health_test.rs                      ➕ 衰减/缺口/孤岛 + 快照测试
    └── notify_test.rs                      ➕ Webhook 签名 + 发送测试
```

### 3.2 前端文件

```
frontend/src/
├── App.tsx                                  🔶 新增 /review, /conflicts, /health 路由
│
├── pages/
│   ├── Review.tsx                           🔶 从占位升级为完整审核队列页面
│   ├── ConflictCenter.tsx                   ➕ 冲突中心（并排对比+解决操作）
│   └── HealthDashboard.tsx                  ➕ 健康仪表盘（评分+4分项卡片+图表）
│
├── components/
│   ├── ProposalCard.tsx                     ➕ 提议卡片（类型图标+置信度+操作按钮）
│   ├── BatchActionBar.tsx                   ➕ 批量操作栏（全选/批量批准/拒绝）
│   ├── ConflictComparison.tsx               ➕ 并排对比视图
│   ├── HealthScoreGauge.tsx                 ➕ 健康度仪表盘组件
│   └── SidePanel.tsx                        ➕ AI 侧边栏建议面板（P1）
│
├── lib/
│   ├── types.ts                             🔶 新增 Proposal/Conflict/HealthSnapshot 类型
│   └── api.ts                               🔶 新增 proposals/conflicts/health API 封装
│
└── pages/
    └── MemoryEditor.tsx                     🔶 右侧增加 SidePanel（P1 上下文召回）
```

### 3.3 部署文件

```
deploy/
├── docker-compose.yml                       🔶 新增 EPICODE_KB_LLM_PROVIDER 等环境变量
└── .env.example                             🔶 新增冲突阈值/扫描频率/webhook 配置
```

---

## 4. 数据结构与接口设计

### 4.1 新增表（004_v3_schema.sql）

```sql
-- 查询日志
CREATE TABLE IF NOT EXISTS query_logs (
    id TEXT PRIMARY KEY,
    space_id TEXT NOT NULL,
    user_id TEXT,
    query TEXT NOT NULL,
    result_count INTEGER NOT NULL DEFAULT 0,
    query_type TEXT NOT NULL DEFAULT 'search',
    created_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_query_logs_space ON query_logs(space_id, created_at);

-- 知识健康快照
CREATE TABLE IF NOT EXISTS knowledge_health (
    id TEXT PRIMARY KEY,
    space_id TEXT NOT NULL,
    snapshot_date TEXT NOT NULL,
    total_memories INTEGER NOT NULL DEFAULT 0,
    human_ratio REAL DEFAULT 0,
    ai_ratio REAL DEFAULT 0,
    co_ratio REAL DEFAULT 0,
    conflict_count INTEGER DEFAULT 0,
    avg_trust REAL DEFAULT 0,
    stale_count INTEGER DEFAULT 0,
    orphan_count INTEGER DEFAULT 0,
    gap_count INTEGER DEFAULT 0,
    health_score REAL DEFAULT 0,
    created_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_health_space ON knowledge_health(space_id, snapshot_date);

-- 通知订阅
CREATE TABLE IF NOT EXISTS notify_subscriptions (
    id TEXT PRIMARY KEY,
    space_id TEXT NOT NULL,
    event_type TEXT NOT NULL,  -- health_report | conflict_detected | proposal_ready
    webhook_url TEXT NOT NULL,
    webhook_secret TEXT NOT NULL,  -- HMAC-SHA256 key
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL
);
```

### 4.2 核心枚举扩展

```rust
// ========== dream/proposal.rs ==========
// ProposalType, ProposalStatus, AiProposal — 已有，直接使用

// ========== conflict/model.rs ==========
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictCandidate {
    pub memory_a_id: String,
    pub memory_b_id: String,
    pub semantic_distance: f32,
    pub confidence: f32,     // LLM 判断矛盾的置信度
    pub summary: String,     // 矛盾摘要
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Resolution {
    AcceptA,
    AcceptB,
    BothTrue,
}

// ========== health/staleness.rs ==========
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StalenessScore {
    pub memory_id: String,
    pub score: f32,           // 0.0=fresh, 1.0=completely stale
    pub days_since_access: i64,
    pub access_count: i64,
}

// ========== health/scanner.rs ==========
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthSnapshot {
    pub space_id: String,
    pub total: usize,
    pub human_ratio: f32,
    pub ai_ratio: f32,
    pub co_ratio: f32,
    pub conflict_count: usize,
    pub avg_trust: f32,
    pub stale_count: usize,
    pub orphan_count: usize,
    pub gap_count: usize,
    pub health_score: f32,
}
```

### 4.3 新增 API 端点

| Method | Path | Phase | 说明 |
|--------|------|-------|------|
| `GET` | `/api/v1/proposals?space_id=&status=&page=` | P2 | 审核队列列表 |
| `POST` | `/api/v1/proposals/:id/approve` | P2 | 批准提议 |
| `POST` | `/api/v1/proposals/:id/reject` | P2 | 拒绝提议（含 feedback） |
| `POST` | `/api/v1/proposals/:id/modify` | P2 | 修改后采纳 |
| `POST` | `/api/v1/proposals/batch` | P2 | 批量操作 |
| `POST` | `/api/v1/dream/scan` | P2 | 手动触发 dream cycle 扫描 |
| `GET` | `/api/v1/conflicts?space_id=` | P3 | 未解决冲突列表 |
| `POST` | `/api/v1/conflicts/:id/resolve` | P3 | 解决冲突 |
| `POST` | `/api/v1/conflicts/detect` | P3 | 手动触发全空间矛盾检测 |
| `GET` | `/api/v1/health/space/:id` | P4 | 空间健康快照 |
| `GET` | `/api/v1/health/gaps?space_id=` | P4 | 知识缺口列表 |
| `GET` | `/api/v1/health/stale?space_id=` | P4 | 过时知识列表 |
| `POST` | `/api/v1/health/scan` | P4 | 手动触发健康扫描 |
| `GET` | `/api/v1/collab/context?memory_id=&cursor=` | P5 | 编辑器上下文召回（P1） |

### 4.4 类图

```
classDiagram
    class LlmProvider {
        <<trait>>
        +complete(prompt) Result~String~
        +fact_compare(statement_a, statement_b) Result~FactCompareResult~  ← 新增
    }

    class FactCompareResult {
        +is_contradiction: bool
        +confidence: f32
        +summary: String
    }

    class ConflictDetector {
        -embedder: Arc~dyn EmbeddingProvider~
        -llm: Arc~dyn LlmProvider~
        -searcher: HybridSearcher
        +detect_one(memory_id) Result~Vec~ConflictCandidate~~
        +detect_all(space_id) Result~Vec~ConflictCandidate~~
        +resolve(conflict_id, resolution, actor) Result~Memory~
    }

    class ProposalEngine {
        -repo: ProposalRepo
        -memory_svc: MemoryService
        -llm: Arc~dyn LlmProvider~
        +scan_space(space_id) Result~Vec~AiProposal~~
        +approve(id, reviewer) Result~AiProposal~
        +reject(id, reviewer, feedback) Result~AiProposal~
        +modify(id, reviewer, content) Result~AiProposal~
        +batch(action, ids, reviewer) Result~Vec~AiProposal~
        -record_feedback(proposal_type, rejected) ()  ← 策略学习
    }

    class HealthScanner {
        -repo: Repository
        -query_log_repo: QueryLogRepo
        -health_repo: HealthRepo
        +full_scan(space_id) Result~HealthSnapshot~
        +scan_staleness(space_id) Result~Vec~StalenessScore~
        +scan_gaps(space_id) Result~Vec~GapEntry~
        +scan_orphans(space_id) Result~Vec~OrphanGroup~
        +compute_score(snapshot) f32
    }

    class NotifyManager {
        -subscriptions: NotifySubRepo
        -webhook: WebhookNotifier
        +subscribe(space_id, event_type, url, secret) Result
        +notify(space_id, event) Result
        -sign_payload(payload, secret) String
    }

    DeepSeekLlm --|> LlmProvider
    OllamaLlm --|> LlmProvider

    ConflictDetector --> LlmProvider
    ConflictDetector --> EmbeddingProvider
    ProposalEngine --> LlmProvider
    ProposalEngine --> MemoryService
```

---

## 5. 关键调用流程

### 5.1 Dream Cycle 扫描 → 生成 Proposal

```
定时任务 / POST /api/v1/dream/scan
  → ProposalEngine::scan_space(space_id)
    → MemoryRepo::get_all_by_space(space_id)
    → 分组：按嵌入聚类 + 标题相似度
    → 每组候选：
      - merge 候选：语义距离<0.15 + 标题相似>0.7
      - link 候选：知识图谱中近邻但无边的记忆对
      - stale 候选：last_accessed_at > 90天
    → 对每个候选调用 LlmProvider::complete() 生成 proposed_content
    → 插入 ai_proposals 表 (status=pending)
    → 返回 proposal 列表
```

### 5.2 矛盾检测（单条）

```
POST /api/v1/conflicts/detect { memory_id }
OR 编辑器保存时自动触发
  → ConflictDetector::detect_one(memory_id)
    → MemoryRepo::get(memory_id) → 获取 content
    → EmbeddingProvider::embed(content) → 向量
    → HybridSearcher::search_by_vector(vector, k=10, min_similarity=0.7)
    → 对每个候选（排除同源记忆）：
      - 语义距离 = 1 - cosine_similarity(a, b)
      - 语义距离 < 0.3 → 送入 LLM 事实对比
      - LlmProvider::fact_compare(content_a, content_b)
        → prompt: "这两段内容是否矛盾？只回答 YES/NO + 一句话理由"
    → 筛选 llm confidence > 0.6 的候选
    → 创建 Conflict 记忆（provenance=conflict, conflicting_ids=[a, b]）
    → Audit
    → NotifyManager::notify(conflict_detected)
```

### 5.3 健康扫描

```
POST /api/v1/health/scan { space_id }
  → HealthScanner::full_scan(space_id)
    → 统计：MemoryRepo::count_by_provenance(space_id)
    → 衰减：每条记忆计算 staleness = 1/(1+e^(-(days_since_access-45)/15))
              stale_count = staleness>0.7 的数量
    → 缺口：QueryLogRepo::get_zero_result_queries(space_id, 30d)
            按频率聚合 → top-20
    → 孤岛：KnowledgeGraph::find_orphans(space_id)
            零入链 + access_count<3 的记忆群
    → 综合评分：
      activity = 1 - (stale_count / total)
      completeness = 1 - (gap_count / (gap_count + 10))
      freshness = 1 - (avg_staleness)
      trust = avg_trust
      health_score = (activity*0.3 + completeness*0.3 + freshness*0.2 + trust*0.2) * 100
    → 写入 knowledge_health 表
    → NotifyManager::notify(health_report)
```

---

## 6. 任务列表

### 6.1 任务依赖图

```
T01(004 schema) ──→ T02(Proposal CRUD) ──→ T03(Dream cycle 改造) ──→ T06(审核队列 UI)
                 ──→ T04(冲突检测)        ──→ T07(冲突中心 UI)
                 ──→ T05(健康度)          ──→ T08(健康仪表盘) ──→ T09(通知)
T01 ──→ T10(协同增强 P1)
```

### 6.2 任务详情

#### T01：数据库 Schema 扩展

| 属性 | 内容 |
|------|------|
| **编号** | T01 |
| **优先级** | P0 |
| **前置** | 无 |
| **文件** | `backend/src/db/migrations/004_v3_schema.sql`, `backend/src/db/schema.rs`, `backend/src/db/mod.rs` |
| **验收** | `cargo test` 通过；migration 幂等（_migrations 表追踪） |
| **内容** | 执行 004 migration：query_logs、knowledge_health、notify_subscriptions 三张表及索引 |

#### T02：Proposal 引擎完整实现

| 属性 | 内容 |
|------|------|
| **编号** | T02 |
| **优先级** | P0 |
| **前置** | T01 |
| **文件** | `backend/src/dream/proposal.rs`, `backend/src/db/repository.rs` (+ProposalRepo), `backend/src/api/proposal.rs`, `backend/src/api/routes.rs` |
| **验收** | `cargo test proposal_test` 全部通过；API 返回 200 非 501 |
| **内容** | 1. ProposalRepo：CRUD + 按 space_id/status 分页查询 + batch update；2. ProposalEngine::scan_space 真实实现（按嵌入聚类找 merge/link/stale 候选，LLM 生成内容）；3. ProposalEngine::approve/reject/modify 真实执行；4. reject 时记录 feedback，连续 3 次拒绝同类型→降低该类型生成频率；5. API 端点从 501 stub 升级为真实 handler |

#### T03：Dream Cycle 改造 + Space 配置

| 属性 | 内容 |
|------|------|
| **编号** | T03 |
| **优先级** | P0 |
| **前置** | T02 |
| **文件** | `backend/src/dream/mod.rs`, `backend/src/config.rs`, `backend/src/state.rs`, `frontend/src/pages/SpaceSettings.tsx` |
| **验收** | dream cycle 触发后 ai_proposals 表有新记录；SpaceSettings 页面可切换行为模式 |
| **内容** | 1. dream cycle 增加 propose_mode：输出改为 Proposal 而非直接修改；2. SpaceSettings API 增加 ai_mode 字段（auto/propose/off）；3. 前端 SpaceSettings 页面增加 AI 行为模式切换 + 信任阈值滑块 |

#### T04：语义矛盾检测

| 属性 | 内容 |
|------|------|
| **编号** | T04 |
| **优先级** | P0 |
| **前置** | T01 |
| **文件** | `backend/src/conflict/mod.rs`, `backend/src/conflict/detect.rs`, `backend/src/conflict/model.rs`, `backend/src/llm/mod.rs`, `backend/src/llm/deepseek.rs`, `backend/src/llm/ollama.rs`, `backend/src/api/conflict.rs` |
| **验收** | `cargo test conflict_test` 全部通过；手动传入矛盾文本对，detect 返回 conflict |
| **内容** | 1. LlmProvider trait 增加 `fact_compare(a, b) → FactCompareResult`；2. DeepSeekLlm + OllamaLlm 分别实现（prompt: "这两段内容是否矛盾？回答 YES/NO + 一句话理由"）；3. ConflictDetector：embed 近邻搜索 + LLM 事实对比；4. detect_one / detect_all / resolve API；5. 创建 Conflict 记忆（provenance=conflict, conflicting_ids） |

#### T05：知识健康度引擎

| 属性 | 内容 |
|------|------|
| **编号** | T05 |
| **优先级** | P0 |
| **前置** | T01 |
| **文件** | `backend/src/health/mod.rs`, `backend/src/health/scanner.rs`, `backend/src/health/staleness.rs`, `backend/src/health/gaps.rs`, `backend/src/health/orphans.rs`, `backend/src/api/health_api.rs`, `backend/src/db/repository.rs` (+QueryLogRepo, HealthRepo) |
| **验收** | `cargo test health_test` 全部通过；health scan 返回有意义的数据 |
| **内容** | 1. QueryLogRepo：search API 自动写入 query_logs（含 result_count）；2. StalenessScanner：sigmoid 衰减模型；3. GapDetector：分析 0 结果查询；4. OrphanFinder：零入链+低访问记忆群；5. HealthScanner：聚合 4 个子扫描器→生成快照；6. health API：GET space/id、GET gaps、GET stale、POST scan |

#### T06：审核队列前端

| 属性 | 内容 |
|------|------|
| **编号** | T06 |
| **优先级** | P0 |
| **前置** | T02 |
| **文件** | `frontend/src/pages/Review.tsx`, `frontend/src/components/ProposalCard.tsx`, `frontend/src/components/BatchActionBar.tsx`, `frontend/src/lib/types.ts`, `frontend/src/lib/api.ts`, `frontend/src/App.tsx` |
| **验收** | `npm run build` 通过；页面可加载 pending proposals；可单条/批量操作 |
| **内容** | 1. ProposalCard：类型标签（merge/link/conflict/archive）+ 置信度 + source memories 预览 + 操作按钮；2. BatchActionBar：全选 + 批量批准/拒绝；3. Review 页面：分页 + 筛选（类型/状态）+ 列表 + 批量操作栏 |

#### T07：冲突中心前端

| 属性 | 内容 |
|------|------|
| **编号** | T07 |
| **优先级** | P0 |
| **前置** | T04 |
| **文件** | `frontend/src/pages/ConflictCenter.tsx`, `frontend/src/components/ConflictComparison.tsx`, `frontend/src/lib/types.ts`, `frontend/src/lib/api.ts`, `frontend/src/App.tsx` |
| **验收** | `npm run build` 通过；并排对比可渲染；解决操作可触发 API |
| **内容** | 1. ConflictCenter 页面：未解决冲突列表；2. ConflictComparison：左侧记忆 A、右侧记忆 B、中间 diff/高亮；3. 解决操作：accept-A / accept-B / both-true 按钮 |

#### T08：健康仪表盘前端

| 属性 | 内容 |
|------|------|
| **编号** | T08 |
| **优先级** | P0 |
| **前置** | T05 |
| **文件** | `frontend/src/pages/HealthDashboard.tsx`, `frontend/src/components/HealthScoreGauge.tsx`, `frontend/package.json` |
| **验收** | `npm run build` 通过；仪表盘展示健康评分+4分项 |
| **内容** | 1. HealthScoreGauge：0-100 环形仪表盘；2. 4 个分项卡片（活跃度/完整度/时效性/信任度）；3. 缺口/过时/孤岛可展开列表 |

#### T09：通知系统

| 属性 | 内容 |
|------|------|
| **编号** | T09 |
| **优先级** | P0 |
| **前置** | T05 |
| **文件** | `backend/src/notify/webhook.rs`, `backend/src/notify/subscriptions.rs`, `backend/src/notify/mod.rs`, `backend/src/state.rs` |
| **验收** | `cargo test notify_test` 通过；Webhook 可发送签名 POST |
| **内容** | 1. WebhookNotifier 从 stub 升级：reqwest POST + HMAC-SHA256 签名；2. 订阅管理：subscribe/unsubscribe API；3. NotifyManager 调度：health_report / conflict_detected / proposal_ready 事件→匹配订阅→发送 |

#### T10：协同编辑增强（P1）

| 属性 | 内容 |
|------|------|
| **编号** | T10 |
| **优先级** | P1 |
| **前置** | T01 |
| **文件** | `backend/src/api/collab.rs`, `frontend/src/pages/MemoryEditor.tsx`, `frontend/src/components/SidePanel.tsx` |
| **验收** | `npm run build` 通过；编辑器右侧出现上下文召回面板 |
| **内容** | 1. 后端：GET /api/v1/collab/context?memory_id=&cursor= — 按当前段落 embedding 搜索相关知识；2. 前端：MemoryEditor 右侧增加 SidePanel，debounce 3s 调用 context API；3. P1 范围：三类建议卡片（已有结论/可能遗漏/链接建议） |

---

## 7. 共享知识

### 7.1 环境变量（新增）

```bash
# 矛盾检测
EPICODE_KB_CONFLICT_THRESHOLD=0.3       # 语义距离阈值
EPICODE_KB_CONFLICT_LLM_CONFIDENCE=0.6  # LLM 置信度阈值

# 健康扫描
EPICODE_KB_HEALTH_SCAN_INTERVAL=86400   # 扫描间隔（秒），默认每天

# 通知
EPICODE_KB_WEBHOOK_DEFAULT_URL=         # 默认 Webhook URL
EPICODE_KB_NOTIFY_RETRY_MAX=3           # 通知重试次数

# LLM
EPICODE_KB_LLM_PROVIDER=ollama          # ollama | deepseek
```

### 7.2 API 统一约定

- Proposal ID 格式：`pro_` + UUID
- Conflict ID 格式：`cf_` + UUID
- Health snapshot ID 格式：`hs_` + UUID + `_` + YYYYMMDD
- 所有时间戳使用 Unix 秒（i64）
- 分页：limit（默认 20，最大 100）+ offset

### 7.3 LLM Prompt 规范

```
fact_compare prompt template:
"Compare these two statements and determine if they contradict each other.
Statement A: {content_a}
Statement B: {content_b}
Answer ONLY with: YES|NO followed by a brief reason.
Format: YES|NO: reason"
```

### 7.4 错误码扩展

```rust
// 409xx: Conflict errors
ConflictAlreadyExists = 40901,
ProposalAlreadyReviewed = 40902,

// 404xx: Resource not found
ProposalNotFound = 40410,
ConflictNotFound = 40411,
HealthSnapshotNotFound = 40412,
```

---

## 8. 待明确事项

| # | 问题 | 建议 |
|---|------|------|
| Q1 | **Proposal 存储位置**：`ai_proposals` 表在 v0.2.0 003 migration 中已存在还是全新创建？ | 查 003_v2_schema.sql。如不存在则在 004 中创建；如存在则用 ALTER 补齐字段 |
| Q2 | **Dream cycle 触发方式**：前端"扫描"按钮 vs 定时任务 vs 两者都有？ | P0 前端按钮 + 手动触发；定时任务用 tokio::spawn 后台执行，默认关闭 |
| Q3 | **知识图谱孤岛检测依赖**：当前是否有入链/出链查询 API？ | memory 表无图结构字段。P0 用"零被引用 + 低访问"近似检测；P1 引入 memory_links 表 |
| Q4 | **LLM 不可用时矛盾检测降级**：回退到纯语义距离判断吗？ | 是：LLM 不可用时，语义距离<0.2 直接标记为疑似矛盾（confidence=0.5），人工确认 |
| Q5 | **健康仪表盘图表库**：recharts vs chart.js vs 纯 SVG？ | recharts（React 原生，轻量） |

---

## 9. 风险与回退

| 风险 | 回退策略 |
|------|---------|
| LLM fact_compare 幻觉率高 | 降级到纯语义距离判断；结果标注 `ai_detected`；需人工确认 |
| Dream cycle 扫描性能退化 | 限制每空间每次最多生成 20 条 proposal；增加分页 |
| 健康 full_scan 在大空间超时 | 异步执行 + 增量扫描（仅最近 7 天变更记忆）；结果缓存 1h |
| Webhook 发送失败导致通知丢失 | 重试 3 次 + 失败记录 audit_log |
| P5 上下文召回延迟影响编辑体验 | debounce 3s + top-5 + 结果缓存 30s；P1 非 P0 不阻塞 |

---

> **本文档为 epicode-kb v0.3.0 增量架构设计，聚焦 P2/P3/P4/P5 四个 Phase 的实现方案与任务分解。详细接口实现以各任务代码与单元测试为准。**
