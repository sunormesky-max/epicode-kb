# epicode-kb

> **人机协同企业知识库** — AI 提议审核、协同编辑、知识健康监控的系统。

## 项目简介

epicode-kb 是一个基于 Rust + React 的人机协同企业知识库，核心特性：

- **记忆溯源链路**：每条记忆携带 provenance（来源）、trust_level（信任等级）、review_status（审核状态），贯穿写入→存储→检索→展示全链路
- **混合检索**：语义向量检索 + Tantivy 全文检索，RRF 融合排序 + trust 加权
- **本地嵌入推理**：ONNX Runtime 本地 embedding，隐私友好
- **AI 提议引擎**：AI 自动检测重复/矛盾/聚类，生成合并/链接/摘要提议，人工审核后采纳
- **协同编辑**：yrs CRDT 多人实时编辑 + 版本历史 + 冲突解决
- **权限控制**：JWT + RBAC 三级权限（全局/空间/记忆级 ACL）
- **知识健康度**：自动检测知识缺口、陈旧记忆、孤儿记忆，综合健康评分
- **Agent 集成**：MCP Server + Agent API Key，Agent 写入自动标记 provenance=ai
- **矛盾检测**：语义距离 + Jaccard 启发式检测知识冲突
- **通知系统**：Webhook HMAC-SHA256 签名推送

## 技术栈

| 层 | 技术 | 版本 |
|----|------|------|
| 后端 | Rust + Axum | 1.78+ / 0.7 |
| 数据库 | SQLite (rusqlite) | 0.31 |
| 全文检索 | Tantivy | 0.22 |
| 嵌入推理 | ONNX Runtime (ort) | 2.x |
| LLM | DeepSeek | — |
| 前端 | React + Vite + Tailwind | 19 / 5 / 3.4 |
| API 层 | tRPC (vanilla) + zod | 11 / 3.23 |

## 快速启动

### 前置要求

- Rust 1.78+
- Node.js 22+
- Python 3.13+ (可选，用于脚本)

### 开发模式

```bash
# 1. 克隆项目
git clone <repo-url>
cd epicode-kb

# 2. 启动后端
cd backend
cp .env.example .env
cargo run

# 3. 启动前端（新终端）
cd frontend
npm install
npm run dev
```

或使用开发脚本：

```bash
chmod +x scripts/dev.sh
./scripts/dev.sh
```

### Docker 部署

```bash
cd deploy
docker compose up --build
```

- 后端：http://localhost:3000
- 前端：http://localhost:5173

## 架构概览

```
┌──────────────────────────────────────────────────────┐
│                   API 路由层 (Axum)                    │
│  routes.rs · memory.rs · search.rs · upload.rs · ...  │
├──────────────────────────────────────────────────────┤
│                   领域服务层                            │
│  memory/service · search/hybrid · dream/proposal      │
├──────────────────────────────────────────────────────┤
│                   基础设施层                            │
│  db/repository · embed/onnx · parse/* · llm/*         │
├──────────────────────────────────────────────────────┤
│                   数据存储层                            │
│  SQLite (rusqlite) · Tantivy Index · ONNX Model       │
└──────────────────────────────────────────────────────┘
```

## 项目结构

```
epicode-kb/
├── backend/          # Rust 后端（Axum + rusqlite + Tantivy）
├── frontend/         # React 前端（Vite + Tailwind + tRPC）
├── deploy/           # Docker Compose 部署
├── docs/             # 文档（PRD + 架构设计）
├── scripts/          # 开发脚本
└── .github/          # CI/CD
```

## Sprint 规划

| Sprint | 功能 | 状态 |
|--------|------|------|
| S1 | 记忆写入、文档上传、混合检索 | ✅ 已实现 |
| S2 | RBAC 鉴权、知识图谱 | 🔶 规划中 |
| S3 | AI 提议引擎、审核队列 | 🔶 规划中 |
| S4 | 冲突检测与解决 | 📦 待规划 |
| S5 | 知识健康度 | 📦 待规划 |

## API 概览

| 方法 | 路径 | 描述 |
|------|------|------|
| POST | /api/v1/remember | 写入记忆 |
| GET | /api/v1/search | 混合检索 |
| POST | /api/v1/recall | 上下文召回 |
| POST | /api/v1/upload | 文档上传 |
| GET | /api/v1/memories/:id | 获取记忆详情 |
| GET | /api/v1/memories | 列出记忆 |
| POST | /api/v1/memories/:id/trust | 调整信任 |
| POST | /api/v1/memories/:id/adopt | 采纳 AI 记忆 |
| POST | /api/v1/memories/:id/reject | 拒绝 AI 记忆 |
| GET | /api/v1/system/health | 健康检查 |
| GET | /api/v1/system/version | 版本信息 |

## License

MIT
