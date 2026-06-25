-- ============================================================
-- Migration: 002_indexes.sql
-- ============================================================

CREATE INDEX IF NOT EXISTS idx_memories_space        ON memories(space_id);
CREATE INDEX IF NOT EXISTS idx_memories_provenance    ON memories(space_id, provenance);
CREATE INDEX IF NOT EXISTS idx_memories_trust         ON memories(space_id, trust_level);
CREATE INDEX IF NOT EXISTS idx_memories_review        ON memories(space_id, review_status);
CREATE INDEX IF NOT EXISTS idx_memories_created       ON memories(space_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_proposals_space_status ON ai_proposals(space_id, status);
CREATE INDEX IF NOT EXISTS idx_proposals_created      ON ai_proposals(created_at DESC);

CREATE INDEX IF NOT EXISTS idx_relations_source       ON memory_relations(source_memory_id);
CREATE INDEX IF NOT EXISTS idx_relations_target       ON memory_relations(target_memory_id);
CREATE INDEX IF NOT EXISTS idx_relations_space        ON memory_relations(space_id, relation_type);

CREATE INDEX IF NOT EXISTS idx_query_logs_space       ON query_logs(space_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_query_logs_zero        ON query_logs(space_id, result_count) WHERE result_count = 0;

CREATE INDEX IF NOT EXISTS idx_notifications_user     ON notifications(user_id, read, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_notifications_space    ON notifications(space_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_audit_space            ON audit_logs(space_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_audit_entity           ON audit_logs(entity_type, entity_id);

CREATE INDEX IF NOT EXISTS idx_space_members_user     ON space_members(user_id);
CREATE INDEX IF NOT EXISTS idx_space_members_space    ON space_members(space_id);
