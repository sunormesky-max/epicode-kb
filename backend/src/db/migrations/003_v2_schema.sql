-- ============================================================
-- Migration: 003_v2_schema.sql
-- Description: v0.2.0 schema additions for auth, collaboration,
--              agent integration, and visibility.
-- ============================================================

-- -------------------------------------------------------
-- Users table v2 additions
-- -------------------------------------------------------
ALTER TABLE users ADD COLUMN password_hash TEXT;
ALTER TABLE users ADD COLUMN sso_subject TEXT;
ALTER TABLE users ADD COLUMN is_active INTEGER NOT NULL DEFAULT 1;

-- -------------------------------------------------------
-- Spaces table v2 additions
-- -------------------------------------------------------
ALTER TABLE spaces ADD COLUMN slug TEXT;
ALTER TABLE spaces ADD COLUMN visibility TEXT NOT NULL DEFAULT 'team';
ALTER TABLE spaces ADD COLUMN owner_id TEXT;

-- Ensure the default space has a slug and owner placeholder.
-- Applications should assign a real owner after creating the first admin.
UPDATE spaces SET slug = COALESCE(slug, LOWER(REPLACE(name, ' ', '-')), 'default'), visibility = COALESCE(visibility, 'team') WHERE slug IS NULL;

-- -------------------------------------------------------
-- Memories table v2 additions
-- -------------------------------------------------------
ALTER TABLE memories ADD COLUMN visibility TEXT NOT NULL DEFAULT 'inherit';
ALTER TABLE memories ADD COLUMN version_of TEXT REFERENCES memories(id);
ALTER TABLE memories ADD COLUMN version_seq INTEGER NOT NULL DEFAULT 0;
ALTER TABLE memories ADD COLUMN author_id TEXT;

-- -------------------------------------------------------
-- Memory versions table
-- -------------------------------------------------------
CREATE TABLE IF NOT EXISTS memory_versions (
    id          TEXT PRIMARY KEY,
    memory_id   TEXT NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    space_id    TEXT NOT NULL REFERENCES spaces(id) ON DELETE CASCADE,
    version_seq INTEGER NOT NULL,
    content     TEXT NOT NULL,
    editor_id   TEXT REFERENCES users(id),
    edit_summary TEXT,
    diff        TEXT,
    created_at  INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_memory_versions_memory ON memory_versions(memory_id, version_seq DESC);
CREATE INDEX IF NOT EXISTS idx_memory_versions_space ON memory_versions(space_id, created_at DESC);

-- -------------------------------------------------------
-- Memory permissions table (selected visibility ACL)
-- -------------------------------------------------------
CREATE TABLE IF NOT EXISTS memory_permissions (
    id          TEXT PRIMARY KEY,
    memory_id   TEXT NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    permission  TEXT NOT NULL,
    created_at  INTEGER NOT NULL,
    UNIQUE(memory_id, user_id, permission)
);

CREATE INDEX IF NOT EXISTS idx_memory_permissions_memory ON memory_permissions(memory_id);
CREATE INDEX IF NOT EXISTS idx_memory_permissions_user ON memory_permissions(user_id);

-- -------------------------------------------------------
-- API keys table
-- -------------------------------------------------------
CREATE TABLE IF NOT EXISTS api_keys (
    id          TEXT PRIMARY KEY,
    space_id    TEXT NOT NULL REFERENCES spaces(id) ON DELETE CASCADE,
    user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    key_hash    TEXT NOT NULL,
    scope       TEXT NOT NULL DEFAULT 'write',
    name        TEXT NOT NULL,
    expires_at  INTEGER,
    last_used_at INTEGER,
    created_at  INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_api_keys_space ON api_keys(space_id);
CREATE INDEX IF NOT EXISTS idx_api_keys_hash ON api_keys(key_hash);

-- -------------------------------------------------------
-- Additional v2 indexes
-- -------------------------------------------------------
CREATE INDEX IF NOT EXISTS idx_memories_visibility ON memories(space_id, visibility);
CREATE INDEX IF NOT EXISTS idx_memories_version_of ON memories(version_of);
CREATE INDEX IF NOT EXISTS idx_users_sso ON users(sso_subject);
CREATE INDEX IF NOT EXISTS idx_spaces_slug ON spaces(slug);
