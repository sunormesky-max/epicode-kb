-- ============================================================
-- Migration: 001_init.sql
-- Description: epicode-kb initial schema
-- ============================================================

-- -------------------------------------------------------
-- Users table (RBAC foundation)
-- -------------------------------------------------------
CREATE TABLE IF NOT EXISTS users (
    id          TEXT PRIMARY KEY,
    email       TEXT UNIQUE NOT NULL,
    name        TEXT NOT NULL,
    global_role TEXT NOT NULL DEFAULT 'viewer',
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL
);

-- -------------------------------------------------------
-- Spaces table (multi-tenant logical isolation)
-- -------------------------------------------------------
CREATE TABLE IF NOT EXISTS spaces (
    id                      TEXT PRIMARY KEY,
    name                    TEXT NOT NULL,
    description             TEXT,
    ai_write_enabled        INTEGER NOT NULL DEFAULT 1,
    default_ai_trust_level  REAL NOT NULL DEFAULT 0.5,
    retention_days          INTEGER,
    created_at              INTEGER NOT NULL,
    updated_at              INTEGER NOT NULL
);

-- -------------------------------------------------------
-- Space members table (space-level RBAC)
-- -------------------------------------------------------
CREATE TABLE IF NOT EXISTS space_members (
    id         TEXT PRIMARY KEY,
    space_id   TEXT NOT NULL REFERENCES spaces(id) ON DELETE CASCADE,
    user_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role       TEXT NOT NULL DEFAULT 'viewer',
    created_at INTEGER NOT NULL,
    UNIQUE(space_id, user_id)
);

-- -------------------------------------------------------
-- Memories table (core)
-- -------------------------------------------------------
CREATE TABLE IF NOT EXISTS memories (
    id                 TEXT PRIMARY KEY,
    space_id           TEXT NOT NULL REFERENCES spaces(id) ON DELETE CASCADE,
    content            TEXT NOT NULL,
    embedding          BLOB,
    embedding_model    TEXT DEFAULT 'all-MiniLM-L6-v2',
    provenance         TEXT NOT NULL DEFAULT 'human',
    provenance_meta    TEXT,
    trust_level        REAL NOT NULL DEFAULT 1.0,
    review_status      TEXT NOT NULL DEFAULT 'accepted',
    parent_conflict_id TEXT REFERENCES memories(id),
    last_accessed_at   INTEGER,
    access_count       INTEGER NOT NULL DEFAULT 0,
    created_at         INTEGER NOT NULL,
    updated_at         INTEGER NOT NULL
);

-- -------------------------------------------------------
-- AI proposals table
-- -------------------------------------------------------
CREATE TABLE IF NOT EXISTS ai_proposals (
    id                 TEXT PRIMARY KEY,
    space_id           TEXT NOT NULL REFERENCES spaces(id) ON DELETE CASCADE,
    proposal_type      TEXT NOT NULL,
    source_memory_ids  TEXT NOT NULL,
    proposed_content   TEXT,
    proposed_action    TEXT,
    ai_model           TEXT,
    confidence         REAL,
    status             TEXT NOT NULL DEFAULT 'pending',
    reviewer_id        TEXT REFERENCES users(id),
    reviewed_at        INTEGER,
    review_feedback    TEXT,
    created_at         INTEGER NOT NULL
);

-- -------------------------------------------------------
-- Memory relations table (knowledge graph)
-- -------------------------------------------------------
CREATE TABLE IF NOT EXISTS memory_relations (
    id                 TEXT PRIMARY KEY,
    space_id           TEXT NOT NULL REFERENCES spaces(id) ON DELETE CASCADE,
    source_memory_id   TEXT NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    target_memory_id   TEXT NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    relation_type      TEXT NOT NULL,
    weight             REAL DEFAULT 1.0,
    created_by         TEXT,
    created_at         INTEGER NOT NULL,
    UNIQUE(source_memory_id, target_memory_id, relation_type)
);

-- -------------------------------------------------------
-- Query logs (knowledge gap detection)
-- -------------------------------------------------------
CREATE TABLE IF NOT EXISTS query_logs (
    id           TEXT PRIMARY KEY,
    space_id     TEXT NOT NULL REFERENCES spaces(id) ON DELETE CASCADE,
    user_id      TEXT REFERENCES users(id),
    query        TEXT NOT NULL,
    result_count INTEGER NOT NULL,
    query_type   TEXT NOT NULL,
    filters      TEXT,
    created_at   INTEGER NOT NULL
);

-- -------------------------------------------------------
-- Knowledge health snapshots
-- -------------------------------------------------------
CREATE TABLE IF NOT EXISTS knowledge_health (
    id              TEXT PRIMARY KEY,
    space_id        TEXT NOT NULL REFERENCES spaces(id) ON DELETE CASCADE,
    snapshot_date   TEXT NOT NULL,
    total_memories  INTEGER,
    human_ratio     REAL,
    ai_ratio        REAL,
    co_ratio        REAL,
    conflict_count  INTEGER,
    avg_trust       REAL,
    stale_count     INTEGER,
    orphan_count    INTEGER,
    gap_count       INTEGER,
    health_score    REAL,
    created_at      INTEGER NOT NULL,
    UNIQUE(space_id, snapshot_date)
);

-- -------------------------------------------------------
-- Notifications table
-- -------------------------------------------------------
CREATE TABLE IF NOT EXISTS notifications (
    id         TEXT PRIMARY KEY,
    space_id   TEXT NOT NULL REFERENCES spaces(id) ON DELETE CASCADE,
    user_id    TEXT REFERENCES users(id),
    type       TEXT NOT NULL,
    title      TEXT NOT NULL,
    body       TEXT,
    ref_id     TEXT,
    ref_type   TEXT,
    read       INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL
);

-- -------------------------------------------------------
-- Audit logs table
-- -------------------------------------------------------
CREATE TABLE IF NOT EXISTS audit_logs (
    id          TEXT PRIMARY KEY,
    space_id    TEXT NOT NULL,
    user_id     TEXT,
    action      TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id   TEXT,
    details     TEXT,
    created_at  INTEGER NOT NULL
);
