-- ============================================================
-- Migration: 004_v3_schema.sql
-- Description: v0.3.0 schema additions for AI proposals,
--              conflict detection, knowledge health, and notifications.
-- ============================================================

-- -------------------------------------------------------
-- AI proposals table
-- -------------------------------------------------------
CREATE TABLE IF NOT EXISTS ai_proposals (
    id               TEXT PRIMARY KEY,
    space_id         TEXT NOT NULL REFERENCES spaces(id) ON DELETE CASCADE,
    proposal_type    TEXT NOT NULL,
    source_memory_ids TEXT NOT NULL,
    proposed_content TEXT,
    proposed_action  TEXT,
    ai_model         TEXT,
    confidence       REAL,
    status           TEXT NOT NULL DEFAULT 'pending',
    reviewer_id      TEXT REFERENCES users(id),
    reviewed_at      INTEGER,
    review_feedback  TEXT,
    created_at       INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_proposals_space_status ON ai_proposals(space_id, status);
CREATE INDEX IF NOT EXISTS idx_proposals_type ON ai_proposals(space_id, proposal_type);
CREATE INDEX IF NOT EXISTS idx_proposals_created ON ai_proposals(space_id, created_at DESC);

-- -------------------------------------------------------
-- Query logs table (for knowledge gap detection)
-- -------------------------------------------------------
CREATE TABLE IF NOT EXISTS query_logs (
    id           TEXT PRIMARY KEY,
    space_id     TEXT NOT NULL REFERENCES spaces(id) ON DELETE CASCADE,
    user_id      TEXT,
    query        TEXT NOT NULL,
    result_count INTEGER NOT NULL DEFAULT 0,
    query_type   TEXT NOT NULL DEFAULT 'search',
    created_at   INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_query_logs_space ON query_logs(space_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_query_logs_zero ON query_logs(space_id, created_at DESC) WHERE result_count = 0;

-- -------------------------------------------------------
-- Knowledge health snapshots table
-- -------------------------------------------------------
CREATE TABLE IF NOT EXISTS knowledge_health (
    id             TEXT PRIMARY KEY,
    space_id       TEXT NOT NULL REFERENCES spaces(id) ON DELETE CASCADE,
    snapshot_date  TEXT NOT NULL,
    total_memories INTEGER NOT NULL DEFAULT 0,
    human_ratio    REAL DEFAULT 0,
    ai_ratio       REAL DEFAULT 0,
    co_ratio       REAL DEFAULT 0,
    conflict_count INTEGER DEFAULT 0,
    avg_trust      REAL DEFAULT 0,
    stale_count    INTEGER DEFAULT 0,
    orphan_count   INTEGER DEFAULT 0,
    gap_count      INTEGER DEFAULT 0,
    health_score   REAL DEFAULT 0,
    created_at     INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_health_space ON knowledge_health(space_id, snapshot_date DESC);

-- -------------------------------------------------------
-- Notify subscriptions table
-- -------------------------------------------------------
CREATE TABLE IF NOT EXISTS notify_subscriptions (
    id             TEXT PRIMARY KEY,
    space_id       TEXT NOT NULL REFERENCES spaces(id) ON DELETE CASCADE,
    event_type     TEXT NOT NULL,
    webhook_url    TEXT NOT NULL,
    webhook_secret TEXT NOT NULL,
    is_active      INTEGER NOT NULL DEFAULT 1,
    created_at     INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_notify_space ON notify_subscriptions(space_id, event_type);
