//! Database repositories: CRUD operations for all entities.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::auth::model::{
    ApiKey, GlobalRole, MemoryPermission, Space, SpaceMember, SpaceRole, SpaceVisibility, User,
};
use crate::error::{AppError, AppResult};
use crate::memory::model::{Memory, Provenance, ReviewStatus, TrustLevel, Visibility};

// ============================================================
// Embedding serialization helpers
// ============================================================

/// Serialize a `Vec<f32>` to a BLOB (little-endian bytes).
pub fn embedding_to_blob(vec: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(vec.len() * 4);
    for &f in vec {
        bytes.extend_from_slice(&f.to_le_bytes());
    }
    bytes
}

/// Deserialize a BLOB to `Vec<f32>` (little-endian bytes).
pub fn blob_to_embedding(blob: &[u8]) -> Vec<f32> {
    blob.chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

// ============================================================
// Memory Repository
// ============================================================

/// Repository for memory CRUD operations.
pub struct MemoryRepo;

impl MemoryRepo {
    /// Insert a new memory into the database.
    pub fn insert(conn: &Connection, memory: &Memory) -> AppResult<()> {
        let embedding_blob = memory.embedding.as_ref().map(|v| embedding_to_blob(v));
        let provenance_meta_str = memory.provenance_meta.as_ref().map(|v| v.to_string());

        conn.execute(
            "INSERT INTO memories (id, space_id, content, embedding, embedding_model, provenance, provenance_meta, trust_level, review_status, visibility, version_of, version_seq, author_id, parent_conflict_id, last_accessed_at, access_count, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
            params![
                memory.id,
                memory.space_id,
                memory.content,
                embedding_blob,
                memory.embedding_model,
                memory.provenance.as_str(),
                provenance_meta_str,
                memory.trust_level.value(),
                memory.review_status.as_str(),
                memory.visibility.as_str(),
                memory.version_of,
                memory.version_seq,
                memory.author_id,
                memory.parent_conflict_id,
                memory.last_accessed_at,
                memory.access_count,
                memory.created_at,
                memory.updated_at,
            ],
        )?;
        Ok(())
    }

    /// Get a memory by ID (without embedding).
    pub fn get_by_id(conn: &Connection, id: &str) -> AppResult<Memory> {
        let row = conn.query_row(
            "SELECT id, space_id, content, embedding_model, provenance, provenance_meta, trust_level, review_status, visibility, version_of, version_seq, author_id, parent_conflict_id, last_accessed_at, access_count, created_at, updated_at
             FROM memories WHERE id = ?1",
            params![id],
            Self::row_to_memory,
        )?;

        Ok(row)
    }

    /// Get a memory by ID (with embedding BLOB deserialized).
    pub fn get_by_id_with_embedding(conn: &Connection, id: &str) -> AppResult<Memory> {
        let mut memory = Self::get_by_id(conn, id)?;
        let blob: Option<Vec<u8>> = conn
            .query_row(
                "SELECT embedding FROM memories WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .ok()
            .flatten();
        memory.embedding = blob.map(|b| blob_to_embedding(&b));
        Ok(memory)
    }

    /// List memories with optional filters and pagination.
    #[allow(clippy::too_many_arguments)]
    pub fn list(
        conn: &Connection,
        space_id: &str,
        provenance: Option<&[Provenance]>,
        min_trust: Option<f32>,
        review_status: Option<ReviewStatus>,
        visibility: Option<Visibility>,
        limit: usize,
        offset: usize,
    ) -> AppResult<(Vec<Memory>, usize)> {
        let mut where_clauses = vec!["space_id = ?1".to_string()];
        let mut param_index = 2;
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(space_id.to_string())];

        if let Some(provs) = provenance {
            if !provs.is_empty() {
                let placeholders: Vec<String> = provs
                    .iter()
                    .map(|p| {
                        let idx = param_index;
                        param_index += 1;
                        params_vec.push(Box::new(p.as_str().to_string()));
                        format!("?{}", idx)
                    })
                    .collect();
                where_clauses.push(format!("provenance IN ({})", placeholders.join(", ")));
            }
        }

        if let Some(trust) = min_trust {
            where_clauses.push(format!("trust_level >= ?{}", param_index));
            params_vec.push(Box::new(trust));
            param_index += 1;
        }

        if let Some(status) = review_status {
            where_clauses.push(format!("review_status = ?{}", param_index));
            params_vec.push(Box::new(status.as_str().to_string()));
            param_index += 1;
        }

        if let Some(vis) = visibility {
            where_clauses.push(format!("visibility = ?{}", param_index));
            params_vec.push(Box::new(vis.as_str().to_string()));
            param_index += 1;
        }

        let where_sql = where_clauses.join(" AND ");

        // Count total
        let count_sql = format!("SELECT COUNT(*) FROM memories WHERE {}", where_sql);
        let params_ref: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        let total: usize = conn.query_row(&count_sql, params_ref.as_slice(), |row| row.get(0))?;

        // Query page
        let query_sql = format!(
            "SELECT id, space_id, content, embedding_model, provenance, provenance_meta, trust_level, review_status, visibility, version_of, version_seq, author_id, parent_conflict_id, last_accessed_at, access_count, created_at, updated_at
             FROM memories WHERE {} ORDER BY created_at DESC LIMIT ?{} OFFSET ?{}",
            where_sql, param_index, param_index + 1
        );
        params_vec.push(Box::new(limit as i64));
        params_vec.push(Box::new(offset as i64));
        let params_ref2: Vec<&dyn rusqlite::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();

        let mut stmt = conn.prepare(&query_sql)?;
        let memories = stmt
            .query_map(params_ref2.as_slice(), Self::row_to_memory)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok((memories, total))
    }

    /// Load all memories with embeddings for a space (for semantic search).
    pub fn load_all_with_embeddings(
        conn: &Connection,
        space_id: &str,
    ) -> AppResult<Vec<(String, Vec<f32>, f32)>> {
        let mut stmt = conn.prepare(
            "SELECT id, embedding, trust_level FROM memories
             WHERE space_id = ?1 AND embedding IS NOT NULL AND review_status = 'accepted'",
        )?;
        let rows = stmt
            .query_map(params![space_id], |row| {
                let id: String = row.get(0)?;
                let blob: Vec<u8> = row.get(1)?;
                let trust: f32 = row.get(2)?;
                Ok((id, blob_to_embedding(&blob), trust))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Update trust level for a memory.
    pub fn update_trust(
        conn: &Connection,
        id: &str,
        trust_level: f32,
        updated_at: i64,
    ) -> AppResult<()> {
        let affected = conn.execute(
            "UPDATE memories SET trust_level = ?1, updated_at = ?2 WHERE id = ?3",
            params![trust_level, updated_at, id],
        )?;
        if affected == 0 {
            return Err(AppError::not_found(format!("memory not found: {}", id)));
        }
        Ok(())
    }

    /// Update review status for a memory.
    pub fn update_review_status(
        conn: &Connection,
        id: &str,
        status: ReviewStatus,
        trust_level: Option<f32>,
        updated_at: i64,
    ) -> AppResult<()> {
        let affected = if let Some(trust) = trust_level {
            conn.execute(
                "UPDATE memories SET review_status = ?1, trust_level = ?2, updated_at = ?3 WHERE id = ?4",
                params![status.as_str(), trust, updated_at, id],
            )?
        } else {
            conn.execute(
                "UPDATE memories SET review_status = ?1, updated_at = ?2 WHERE id = ?3",
                params![status.as_str(), updated_at, id],
            )?
        };
        if affected == 0 {
            return Err(AppError::not_found(format!("memory not found: {}", id)));
        }
        Ok(())
    }

    /// Update memory content and bump version sequence.
    pub fn update_content(
        conn: &Connection,
        id: &str,
        content: &str,
        version_seq: i64,
        updated_at: i64,
    ) -> AppResult<()> {
        let affected = conn.execute(
            "UPDATE memories SET content = ?1, version_seq = ?2, updated_at = ?3 WHERE id = ?4",
            params![content, version_seq, updated_at, id],
        )?;
        if affected == 0 {
            return Err(AppError::not_found(format!("memory not found: {}", id)));
        }
        Ok(())
    }

    /// Update memory visibility.
    pub fn update_visibility(
        conn: &Connection,
        id: &str,
        visibility: Visibility,
        updated_at: i64,
    ) -> AppResult<()> {
        let affected = conn.execute(
            "UPDATE memories SET visibility = ?1, updated_at = ?2 WHERE id = ?3",
            params![visibility.as_str(), updated_at, id],
        )?;
        if affected == 0 {
            return Err(AppError::not_found(format!("memory not found: {}", id)));
        }
        Ok(())
    }

    /// Update access count and last accessed timestamp.
    pub fn update_access(conn: &Connection, ids: &[String], now: i64) -> AppResult<()> {
        for id in ids {
            conn.execute(
                "UPDATE memories SET access_count = access_count + 1, last_accessed_at = ?1 WHERE id = ?2",
                params![now, id],
            )?;
        }
        Ok(())
    }

    /// Get indexable fields for Tantivy.
    pub fn get_indexable_fields(
        conn: &Connection,
        id: &str,
    ) -> AppResult<(String, String, f32, String, i64, String)> {
        let row = conn.query_row(
            "SELECT id, content, trust_level, provenance, created_at, review_status FROM memories WHERE id = ?1",
            params![id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, f32>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, String>(5)?,
                ))
            },
        )?;
        Ok(row)
    }

    fn row_to_memory(row: &rusqlite::Row) -> Result<Memory, rusqlite::Error> {
        let provenance_str: String = row.get(4)?;
        let provenance = Provenance::parse_str(&provenance_str).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, e.into())
        })?;
        let review_status_str: String = row.get(7)?;
        let review_status = ReviewStatus::parse_str(&review_status_str).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(7, rusqlite::types::Type::Text, e.into())
        })?;
        let visibility_str: String = row.get(8)?;
        let visibility = Visibility::parse_str(&visibility_str).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(8, rusqlite::types::Type::Text, e.into())
        })?;
        let trust_value: f32 = row.get(6)?;
        let provenance_meta_str: Option<String> = row.get(5)?;
        let provenance_meta =
            provenance_meta_str.and_then(|s| serde_json::from_str::<Value>(&s).ok());
        Ok(Memory {
            id: row.get(0)?,
            space_id: row.get(1)?,
            content: row.get(2)?,
            embedding: None,
            embedding_model: row.get(3)?,
            provenance,
            provenance_meta,
            trust_level: TrustLevel::new(trust_value).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(6, rusqlite::types::Type::Real, e.into())
            })?,
            review_status,
            visibility,
            version_of: row.get(9)?,
            version_seq: row.get(10)?,
            author_id: row.get(11)?,
            parent_conflict_id: row.get(12)?,
            last_accessed_at: row.get(13)?,
            access_count: row.get(14)?,
            created_at: row.get(15)?,
            updated_at: row.get(16)?,
        })
    }
}

// ============================================================
// User Repository
// ============================================================

/// Repository for users.
pub struct UserRepo;

impl UserRepo {
    /// Find a user by email.
    pub fn find_by_email(conn: &Connection, email: &str) -> AppResult<Option<User>> {
        let mut stmt = conn.prepare(
            "SELECT id, email, name, password_hash, sso_subject, global_role, is_active, created_at, updated_at FROM users WHERE email = ?1"
        )?;
        let mut rows = stmt.query_map(params![email], Self::row_to_user)?;
        Ok(rows.next().transpose()?)
    }

    /// Get a user by ID.
    pub fn get_by_id(conn: &Connection, id: &str) -> AppResult<User> {
        let row = conn.query_row(
            "SELECT id, email, name, password_hash, sso_subject, global_role, is_active, created_at, updated_at FROM users WHERE id = ?1",
            params![id],
            Self::row_to_user,
        )?;
        Ok(row)
    }

    /// Insert a new user.
    pub fn insert(conn: &Connection, user: &User) -> AppResult<()> {
        conn.execute(
            "INSERT INTO users (id, email, name, password_hash, sso_subject, global_role, is_active, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                user.id,
                user.email,
                user.name,
                user.password_hash,
                user.sso_subject,
                user.global_role.as_str(),
                if user.is_active { 1 } else { 0 },
                user.created_at,
                user.updated_at,
            ],
        )?;
        Ok(())
    }

    /// List users with pagination.
    pub fn list(conn: &Connection, limit: usize, offset: usize) -> AppResult<(Vec<User>, usize)> {
        let total: usize = conn.query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))?;
        let mut stmt = conn.prepare(
            "SELECT id, email, name, password_hash, sso_subject, global_role, is_active, created_at, updated_at FROM users ORDER BY created_at DESC LIMIT ?1 OFFSET ?2"
        )?;
        let users = stmt
            .query_map(params![limit as i64, offset as i64], |row| {
                Self::row_to_user(row)
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok((users, total))
    }

    /// Update user global role.
    pub fn update_global_role(
        conn: &Connection,
        id: &str,
        role: GlobalRole,
        updated_at: i64,
    ) -> AppResult<()> {
        let affected = conn.execute(
            "UPDATE users SET global_role = ?1, updated_at = ?2 WHERE id = ?3",
            params![role.as_str(), updated_at, id],
        )?;
        if affected == 0 {
            return Err(AppError::not_found(format!("user not found: {}", id)));
        }
        Ok(())
    }

    fn row_to_user(row: &rusqlite::Row) -> Result<User, rusqlite::Error> {
        let role_str: String = row.get(5)?;
        let role = GlobalRole::parse_str(&role_str).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(5, rusqlite::types::Type::Text, e.into())
        })?;
        Ok(User {
            id: row.get(0)?,
            email: row.get(1)?,
            name: row.get(2)?,
            password_hash: row.get(3)?,
            sso_subject: row.get(4)?,
            global_role: role,
            is_active: row.get::<_, i64>(6)? != 0,
            created_at: row.get(7)?,
            updated_at: row.get(8)?,
        })
    }
}

// ============================================================
// Space Repository
// ============================================================

/// Repository for spaces.
pub struct SpaceRepo;

impl SpaceRepo {
    /// Get a space by ID.
    pub fn get_by_id(conn: &Connection, id: &str) -> AppResult<Space> {
        let row = conn.query_row(
            "SELECT id, name, slug, description, visibility, owner_id, ai_write_enabled, default_ai_trust_level, retention_days, created_at, updated_at FROM spaces WHERE id = ?1",
            params![id],
            Self::row_to_space,
        )?;
        Ok(row)
    }

    /// Get a space by slug.
    pub fn get_by_slug(conn: &Connection, slug: &str) -> AppResult<Space> {
        let row = conn.query_row(
            "SELECT id, name, slug, description, visibility, owner_id, ai_write_enabled, default_ai_trust_level, retention_days, created_at, updated_at FROM spaces WHERE slug = ?1",
            params![slug],
            Self::row_to_space,
        )?;
        Ok(row)
    }

    /// Insert a new space.
    pub fn insert(conn: &Connection, space: &Space) -> AppResult<()> {
        conn.execute(
            "INSERT INTO spaces (id, name, slug, description, visibility, owner_id, ai_write_enabled, default_ai_trust_level, retention_days, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                space.id,
                space.name,
                space.slug,
                space.description,
                space.visibility.as_str(),
                space.owner_id,
                if space.ai_write_enabled { 1 } else { 0 },
                space.default_ai_trust_level,
                space.retention_days,
                space.created_at,
                space.updated_at,
            ],
        )?;
        Ok(())
    }

    /// List spaces with pagination.
    pub fn list(conn: &Connection, limit: usize, offset: usize) -> AppResult<(Vec<Space>, usize)> {
        let total: usize = conn.query_row("SELECT COUNT(*) FROM spaces", [], |row| row.get(0))?;
        let mut stmt = conn.prepare(
            "SELECT id, name, slug, description, visibility, owner_id, ai_write_enabled, default_ai_trust_level, retention_days, created_at, updated_at FROM spaces ORDER BY created_at DESC LIMIT ?1 OFFSET ?2"
        )?;
        let spaces = stmt
            .query_map(params![limit as i64, offset as i64], |row| {
                Self::row_to_space(row)
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok((spaces, total))
    }

    /// Update space visibility.
    pub fn update_visibility(
        conn: &Connection,
        id: &str,
        visibility: SpaceVisibility,
        updated_at: i64,
    ) -> AppResult<()> {
        let affected = conn.execute(
            "UPDATE spaces SET visibility = ?1, updated_at = ?2 WHERE id = ?3",
            params![visibility.as_str(), updated_at, id],
        )?;
        if affected == 0 {
            return Err(AppError::not_found(format!("space not found: {}", id)));
        }
        Ok(())
    }

    /// Update space AI write enabled flag.
    pub fn update_ai_write_enabled(
        conn: &Connection,
        id: &str,
        enabled: bool,
        updated_at: i64,
    ) -> AppResult<()> {
        let affected = conn.execute(
            "UPDATE spaces SET ai_write_enabled = ?1, updated_at = ?2 WHERE id = ?3",
            params![if enabled { 1 } else { 0 }, updated_at, id],
        )?;
        if affected == 0 {
            return Err(AppError::not_found(format!("space not found: {}", id)));
        }
        Ok(())
    }

    fn row_to_space(row: &rusqlite::Row) -> Result<Space, rusqlite::Error> {
        let vis_str: String = row.get(4)?;
        let visibility = SpaceVisibility::parse_str(&vis_str).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, e.into())
        })?;
        Ok(Space {
            id: row.get(0)?,
            name: row.get(1)?,
            slug: row.get(2)?,
            description: row.get(3)?,
            visibility,
            owner_id: row.get(5)?,
            ai_write_enabled: row.get::<_, i64>(6)? != 0,
            default_ai_trust_level: row.get(7)?,
            retention_days: row.get(8)?,
            created_at: row.get(9)?,
            updated_at: row.get(10)?,
        })
    }
}

// ============================================================
// Space Member Repository
// ============================================================

/// Repository for space membership.
pub struct SpaceMemberRepo;

impl SpaceMemberRepo {
    /// Find a member's role in a space.
    pub fn find_role(
        conn: &Connection,
        space_id: &str,
        user_id: &str,
    ) -> AppResult<Option<SpaceRole>> {
        let mut stmt =
            conn.prepare("SELECT role FROM space_members WHERE space_id = ?1 AND user_id = ?2")?;
        let mut rows = stmt.query_map(params![space_id, user_id], |row| {
            let s: String = row.get(0)?;
            Ok(s)
        })?;
        if let Some(role_str) = rows.next().transpose()? {
            let role = SpaceRole::parse_str(&role_str).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, e.into())
            })?;
            Ok(Some(role))
        } else {
            Ok(None)
        }
    }

    /// List members of a space.
    pub fn list_by_space(
        conn: &Connection,
        space_id: &str,
        limit: usize,
        offset: usize,
    ) -> AppResult<(Vec<SpaceMember>, usize)> {
        let total: usize = conn.query_row(
            "SELECT COUNT(*) FROM space_members WHERE space_id = ?1",
            params![space_id],
            |row| row.get(0),
        )?;
        let mut stmt = conn.prepare(
            "SELECT id, space_id, user_id, role, created_at FROM space_members WHERE space_id = ?1 ORDER BY created_at DESC LIMIT ?2 OFFSET ?3"
        )?;
        let members = stmt
            .query_map(params![space_id, limit as i64, offset as i64], |row| {
                Self::row_to_member(row)
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok((members, total))
    }

    /// Insert or replace a space member.
    pub fn upsert(conn: &Connection, member: &SpaceMember) -> AppResult<()> {
        conn.execute(
            "INSERT OR REPLACE INTO space_members (id, space_id, user_id, role, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![member.id, member.space_id, member.user_id, member.role.as_str(), member.created_at],
        )?;
        Ok(())
    }

    /// Remove a member from a space.
    pub fn remove(conn: &Connection, space_id: &str, user_id: &str) -> AppResult<()> {
        conn.execute(
            "DELETE FROM space_members WHERE space_id = ?1 AND user_id = ?2",
            params![space_id, user_id],
        )?;
        Ok(())
    }

    fn row_to_member(row: &rusqlite::Row) -> Result<SpaceMember, rusqlite::Error> {
        let role_str: String = row.get(3)?;
        let role = SpaceRole::parse_str(&role_str).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, e.into())
        })?;
        Ok(SpaceMember {
            id: row.get(0)?,
            space_id: row.get(1)?,
            user_id: row.get(2)?,
            role,
            created_at: row.get(4)?,
        })
    }
}

// ============================================================
// API Key Repository
// ============================================================

/// Repository for API keys.
pub struct ApiKeyRepo;

impl ApiKeyRepo {
    /// Insert a new API key.
    pub fn insert(conn: &Connection, key: &ApiKey) -> AppResult<()> {
        conn.execute(
            "INSERT INTO api_keys (id, space_id, user_id, key_hash, scope, name, expires_at, last_used_at, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                key.id,
                key.space_id,
                key.user_id,
                key.key_hash,
                key.scope,
                key.name,
                key.expires_at,
                key.last_used_at,
                key.created_at,
            ],
        )?;
        Ok(())
    }

    /// Find an API key by hash and space.
    pub fn find_by_hash_and_space(
        conn: &Connection,
        key_hash: &str,
        space_id: &str,
    ) -> AppResult<Option<ApiKey>> {
        let mut stmt = conn.prepare(
            "SELECT id, space_id, user_id, key_hash, scope, name, expires_at, last_used_at, created_at FROM api_keys WHERE key_hash = ?1 AND space_id = ?2"
        )?;
        let mut rows = stmt.query_map(params![key_hash, space_id], Self::row_to_api_key)?;
        Ok(rows.next().transpose()?)
    }

    /// List API keys for a space.
    pub fn list_by_space(
        conn: &Connection,
        space_id: &str,
        limit: usize,
        offset: usize,
    ) -> AppResult<(Vec<ApiKey>, usize)> {
        let total: usize = conn.query_row(
            "SELECT COUNT(*) FROM api_keys WHERE space_id = ?1",
            params![space_id],
            |row| row.get(0),
        )?;
        let mut stmt = conn.prepare(
            "SELECT id, space_id, user_id, key_hash, scope, name, expires_at, last_used_at, created_at FROM api_keys WHERE space_id = ?1 ORDER BY created_at DESC LIMIT ?2 OFFSET ?3"
        )?;
        let keys = stmt
            .query_map(params![space_id, limit as i64, offset as i64], |row| {
                Self::row_to_api_key(row)
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok((keys, total))
    }

    /// Update last used timestamp.
    pub fn update_last_used(conn: &Connection, id: &str, now: i64) -> AppResult<()> {
        conn.execute(
            "UPDATE api_keys SET last_used_at = ?1 WHERE id = ?2",
            params![now, id],
        )?;
        Ok(())
    }

    /// Delete an API key.
    pub fn delete(conn: &Connection, id: &str) -> AppResult<()> {
        let affected = conn.execute("DELETE FROM api_keys WHERE id = ?1", params![id])?;
        if affected == 0 {
            return Err(AppError::not_found(format!("api key not found: {}", id)));
        }
        Ok(())
    }

    fn row_to_api_key(row: &rusqlite::Row) -> Result<ApiKey, rusqlite::Error> {
        Ok(ApiKey {
            id: row.get(0)?,
            space_id: row.get(1)?,
            user_id: row.get(2)?,
            key_hash: row.get(3)?,
            scope: row.get(4)?,
            name: row.get(5)?,
            expires_at: row.get(6)?,
            last_used_at: row.get(7)?,
            created_at: row.get(8)?,
        })
    }
}

// ============================================================
// Memory Version Repository
// ============================================================

/// Repository for memory version snapshots.
pub struct MemoryVersionRepo;

/// Memory version record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryVersion {
    pub id: String,
    pub memory_id: String,
    pub space_id: String,
    pub version_seq: i64,
    pub content: String,
    pub editor_id: Option<String>,
    pub edit_summary: Option<String>,
    pub diff: Option<String>,
    pub created_at: i64,
}

impl MemoryVersionRepo {
    /// Insert a new memory version.
    pub fn insert(conn: &Connection, version: &MemoryVersion) -> AppResult<()> {
        conn.execute(
            "INSERT INTO memory_versions (id, memory_id, space_id, version_seq, content, editor_id, edit_summary, diff, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                version.id,
                version.memory_id,
                version.space_id,
                version.version_seq,
                version.content,
                version.editor_id,
                version.edit_summary,
                version.diff,
                version.created_at,
            ],
        )?;
        Ok(())
    }

    /// List versions of a memory.
    pub fn list_by_memory(
        conn: &Connection,
        memory_id: &str,
        limit: usize,
        offset: usize,
    ) -> AppResult<(Vec<MemoryVersion>, usize)> {
        let total: usize = conn.query_row(
            "SELECT COUNT(*) FROM memory_versions WHERE memory_id = ?1",
            params![memory_id],
            |row| row.get(0),
        )?;
        let mut stmt = conn.prepare(
            "SELECT id, memory_id, space_id, version_seq, content, editor_id, edit_summary, diff, created_at FROM memory_versions WHERE memory_id = ?1 ORDER BY version_seq DESC LIMIT ?2 OFFSET ?3"
        )?;
        let versions = stmt
            .query_map(params![memory_id, limit as i64, offset as i64], |row| {
                Self::row_to_version(row)
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok((versions, total))
    }

    /// Get a specific version by ID.
    pub fn get_by_id(conn: &Connection, id: &str) -> AppResult<MemoryVersion> {
        let row = conn.query_row(
            "SELECT id, memory_id, space_id, version_seq, content, editor_id, edit_summary, diff, created_at FROM memory_versions WHERE id = ?1",
            params![id],
            Self::row_to_version,
        )?;
        Ok(row)
    }

    fn row_to_version(row: &rusqlite::Row) -> Result<MemoryVersion, rusqlite::Error> {
        Ok(MemoryVersion {
            id: row.get(0)?,
            memory_id: row.get(1)?,
            space_id: row.get(2)?,
            version_seq: row.get(3)?,
            content: row.get(4)?,
            editor_id: row.get(5)?,
            edit_summary: row.get(6)?,
            diff: row.get(7)?,
            created_at: row.get(8)?,
        })
    }
}

// ============================================================
// Memory Permission Repository
// ============================================================

/// Repository for memory-level ACLs.
pub struct MemoryPermissionRepo;

impl MemoryPermissionRepo {
    /// List permissions for a memory.
    pub fn list_by_memory(conn: &Connection, memory_id: &str) -> AppResult<Vec<MemoryPermission>> {
        let mut stmt = conn.prepare(
            "SELECT id, memory_id, user_id, permission, created_at FROM memory_permissions WHERE memory_id = ?1"
        )?;
        let perms = stmt
            .query_map(params![memory_id], Self::row_to_perm)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(perms)
    }

    /// Grant a permission.
    pub fn grant(conn: &Connection, perm: &MemoryPermission) -> AppResult<()> {
        conn.execute(
            "INSERT OR REPLACE INTO memory_permissions (id, memory_id, user_id, permission, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![perm.id, perm.memory_id, perm.user_id, perm.permission, perm.created_at],
        )?;
        Ok(())
    }

    /// Revoke a permission.
    pub fn revoke(
        conn: &Connection,
        memory_id: &str,
        user_id: &str,
        permission: &str,
    ) -> AppResult<()> {
        conn.execute(
            "DELETE FROM memory_permissions WHERE memory_id = ?1 AND user_id = ?2 AND permission = ?3",
            params![memory_id, user_id, permission],
        )?;
        Ok(())
    }

    fn row_to_perm(row: &rusqlite::Row) -> Result<MemoryPermission, rusqlite::Error> {
        Ok(MemoryPermission {
            id: row.get(0)?,
            memory_id: row.get(1)?,
            user_id: row.get(2)?,
            permission: row.get(3)?,
            created_at: row.get(4)?,
        })
    }
}

// ============================================================
// Audit Log Repository
// ============================================================

/// Repository for audit logs.
pub struct AuditRepo;

/// Audit log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: String,
    pub space_id: String,
    pub user_id: Option<String>,
    pub action: String,
    pub entity_type: String,
    pub entity_id: Option<String>,
    pub details: Option<Value>,
    pub created_at: i64,
}

impl AuditRepo {
    /// Insert an audit log entry.
    pub fn insert(conn: &Connection, log: &AuditLog) -> AppResult<()> {
        conn.execute(
            "INSERT INTO audit_logs (id, space_id, user_id, action, entity_type, entity_id, details, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                log.id,
                log.space_id,
                log.user_id,
                log.action,
                log.entity_type,
                log.entity_id,
                log.details.as_ref().map(|v| v.to_string()),
                log.created_at,
            ],
        )?;
        Ok(())
    }

    /// List audit logs for a space.
    pub fn list_by_space(
        conn: &Connection,
        space_id: &str,
        limit: usize,
        offset: usize,
    ) -> AppResult<(Vec<AuditLog>, usize)> {
        let total: usize = conn.query_row(
            "SELECT COUNT(*) FROM audit_logs WHERE space_id = ?1",
            params![space_id],
            |row| row.get(0),
        )?;
        let mut stmt = conn.prepare(
            "SELECT id, space_id, user_id, action, entity_type, entity_id, details, created_at FROM audit_logs WHERE space_id = ?1 ORDER BY created_at DESC LIMIT ?2 OFFSET ?3"
        )?;
        let logs = stmt
            .query_map(params![space_id, limit as i64, offset as i64], |row| {
                Self::row_to_audit(row)
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok((logs, total))
    }

    fn row_to_audit(row: &rusqlite::Row) -> Result<AuditLog, rusqlite::Error> {
        let details_str: Option<String> = row.get(6)?;
        let details = details_str.and_then(|s| serde_json::from_str(&s).ok());
        Ok(AuditLog {
            id: row.get(0)?,
            space_id: row.get(1)?,
            user_id: row.get(2)?,
            action: row.get(3)?,
            entity_type: row.get(4)?,
            entity_id: row.get(5)?,
            details,
            created_at: row.get(7)?,
        })
    }
}

// ============================================================
// Query Log Repository
// ============================================================

/// Repository for query logs.
pub struct QueryLogRepo;

impl QueryLogRepo {
    /// Log a search query.
    pub fn log(
        conn: &Connection,
        id: &str,
        space_id: &str,
        query: &str,
        result_count: usize,
        query_type: &str,
        filters: Option<&Value>,
    ) -> AppResult<()> {
        conn.execute(
            "INSERT INTO query_logs (id, space_id, user_id, query, result_count, query_type, filters, created_at)
             VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?6, ?7)",
            params![
                id,
                space_id,
                query,
                result_count as i64,
                query_type,
                filters.map(|v| v.to_string()),
                crate::now_ts(),
            ],
        )?;
        Ok(())
    }

    pub fn insert_simple(conn: &Connection, space_id: &str, user_id: Option<&str>, query: &str, result_count: i64, query_type: &str) -> AppResult<()> {
        let id = format!("ql_{}", uuid::Uuid::new_v4().to_string().replace('-', ""));
        let now = crate::now_ts();
        conn.execute(
            "INSERT INTO query_logs (id, space_id, user_id, query, result_count, query_type, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, space_id, user_id, query, result_count, query_type, now],
        )?;
        Ok(())
    }

    pub fn get_zero_result_queries(conn: &Connection, space_id: &str, since_days: i64) -> AppResult<Vec<(String, i64)>> {
        let cutoff = crate::now_ts() - since_days * 86400;
        let mut stmt = conn.prepare(
            "SELECT query, COUNT(*) as cnt FROM query_logs WHERE space_id = ?1 AND result_count = 0 AND created_at > ?2 GROUP BY query ORDER BY cnt DESC LIMIT 20",
        )?;
        let rows = stmt.query_map(params![space_id, cutoff], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        let mut result = Vec::new();
        for r in rows {
            result.push(r?);
        }
        Ok(result)
    }
}

// ============================================================
// Proposal Repository (stub — Sprint 3)
// ============================================================

/// Repository for AI proposals.
pub struct ProposalRepo;

impl ProposalRepo {
    pub fn insert(conn: &Connection, proposal: &crate::dream::proposal::AiProposal) -> AppResult<()> {
        let source_ids = serde_json::to_string(&proposal.source_memory_ids).unwrap_or_default();
        let proposed_action = proposal.proposed_action.as_ref().map(|v| v.to_string());
        conn.execute(
            "INSERT INTO ai_proposals (id, space_id, proposal_type, source_memory_ids, proposed_content, proposed_action, ai_model, confidence, status, reviewer_id, reviewed_at, review_feedback, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                proposal.id,
                proposal.space_id,
                serde_json::to_string(&proposal.proposal_type).unwrap_or_default().trim_matches('"'),
                source_ids,
                proposal.proposed_content,
                proposed_action,
                proposal.ai_model,
                proposal.confidence,
                "pending",
                proposal.reviewer_id,
                proposal.reviewed_at,
                proposal.review_feedback,
                proposal.created_at,
            ],
        )?;
        Ok(())
    }

    pub fn list(
        conn: &Connection,
        space_id: &str,
        status: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> AppResult<Vec<crate::dream::proposal::AiProposal>> {
        let sql = if let Some(s) = status {
            format!(
                "SELECT id, space_id, proposal_type, source_memory_ids, proposed_content, proposed_action, ai_model, confidence, status, reviewer_id, reviewed_at, review_feedback, created_at
                 FROM ai_proposals WHERE space_id = ?1 AND status = ?2 ORDER BY created_at DESC LIMIT ?3 OFFSET ?4"
            )
        } else {
            format!(
                "SELECT id, space_id, proposal_type, source_memory_ids, proposed_content, proposed_action, ai_model, confidence, status, reviewer_id, reviewed_at, review_feedback, created_at
                 FROM ai_proposals WHERE space_id = ?1 ORDER BY created_at DESC LIMIT ?2 OFFSET ?3"
            )
        };
        let mut stmt = conn.prepare(&sql)?;
        let rows: Vec<_> = if let Some(s) = status {
            stmt.query_map(params![space_id, s, limit, offset], row_to_proposal)?
                .collect()
        } else {
            stmt.query_map(params![space_id, limit, offset], row_to_proposal)?
                .collect()
        };
        let mut proposals = Vec::new();
        for r in rows {
            proposals.push(r?);
        }
        Ok(proposals)
    }

    pub fn get_by_id(conn: &Connection, id: &str) -> AppResult<crate::dream::proposal::AiProposal> {
        conn.query_row(
            "SELECT id, space_id, proposal_type, source_memory_ids, proposed_content, proposed_action, ai_model, confidence, status, reviewer_id, reviewed_at, review_feedback, created_at
             FROM ai_proposals WHERE id = ?1",
            params![id],
            row_to_proposal,
        )
        .map_err(|e| {
            if e == rusqlite::Error::QueryReturnedNoRows {
                AppError::not_found("proposal")
            } else {
                AppError::db(e)
            }
        })
    }

    pub fn update_status(
        conn: &Connection,
        id: &str,
        status: &str,
        reviewer_id: &str,
        feedback: Option<&str>,
    ) -> AppResult<()> {
        let now = crate::now_ts();
        conn.execute(
            "UPDATE ai_proposals SET status = ?1, reviewer_id = ?2, reviewed_at = ?3, review_feedback = ?4 WHERE id = ?5",
            params![status, reviewer_id, now, feedback, id],
        )?;
        Ok(())
    }

    pub fn count_by_type(conn: &Connection, space_id: &str, proposal_type: &str, status: &str) -> AppResult<i64> {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM ai_proposals WHERE space_id = ?1 AND proposal_type = ?2 AND status = ?3",
            params![space_id, proposal_type, status],
            |row| row.get(0),
        )?;
        Ok(count)
    }
}

fn row_to_proposal(row: &rusqlite::Row) -> rusqlite::Result<crate::dream::proposal::AiProposal> {
    use crate::dream::proposal::{ProposalStatus, ProposalType};
    let type_str: String = row.get(2)?;
    let proposal_type: ProposalType = serde_json::from_str(&format!("\"{}\"", type_str)).unwrap_or(ProposalType::Merge);
    let status_str: String = row.get(8)?;
    let status: ProposalStatus = serde_json::from_str(&format!("\"{}\"", status_str)).unwrap_or(ProposalStatus::Pending);
    let source_ids_str: String = row.get(3)?;
    let source_memory_ids: Vec<String> = serde_json::from_str(&source_ids_str).unwrap_or_default();
    let action_str: Option<String> = row.get(5)?;
    let proposed_action = action_str.and_then(|s| serde_json::from_str(&s).ok());
    Ok(crate::dream::proposal::AiProposal {
        id: row.get(0)?,
        space_id: row.get(1)?,
        proposal_type,
        source_memory_ids,
        proposed_content: row.get(4)?,
        proposed_action,
        ai_model: row.get(6)?,
        confidence: row.get(7)?,
        status,
        reviewer_id: row.get(9)?,
        reviewed_at: row.get(10)?,
        review_feedback: row.get(11)?,
        created_at: row.get(12)?,
    })
}

// ============================================================
// Health Repository
// ============================================================

/// Repository for knowledge health snapshots.
pub struct HealthRepo;

impl HealthRepo {
    pub fn insert_snapshot(conn: &Connection, snap: &crate::health::scanner::HealthSnapshot) -> AppResult<()> {
        let id = format!("hs_{}", uuid::Uuid::new_v4().to_string().replace('-', ""));
        let now = crate::now_ts();
        conn.execute(
            "INSERT INTO knowledge_health (id, space_id, snapshot_date, total_memories, human_ratio, ai_ratio, co_ratio, conflict_count, avg_trust, stale_count, orphan_count, gap_count, health_score, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                id, snap.space_id, snap.snapshot_date,
                snap.total as i64, snap.human_ratio, snap.ai_ratio, snap.co_ratio,
                snap.conflict_count as i64, snap.avg_trust,
                snap.stale_count as i64, snap.orphan_count as i64, snap.gap_count as i64,
                snap.health_score, now,
            ],
        )?;
        Ok(())
    }

    pub fn get_latest(conn: &Connection, space_id: &str) -> AppResult<Option<crate::health::scanner::HealthSnapshot>> {
        let result = conn.query_row(
            "SELECT space_id, snapshot_date, total_memories, human_ratio, ai_ratio, co_ratio, conflict_count, avg_trust, stale_count, orphan_count, gap_count, health_score
             FROM knowledge_health WHERE space_id = ?1 ORDER BY snapshot_date DESC LIMIT 1",
            params![space_id],
            |row| {
                Ok(crate::health::scanner::HealthSnapshot {
                    space_id: row.get(0)?,
                    snapshot_date: row.get(1)?,
                    total: row.get::<_, i64>(2)? as usize,
                    human_ratio: row.get(3)?,
                    ai_ratio: row.get(4)?,
                    co_ratio: row.get(5)?,
                    conflict_count: row.get::<_, i64>(6)? as usize,
                    avg_trust: row.get(7)?,
                    stale_count: row.get::<_, i64>(8)? as usize,
                    orphan_count: row.get::<_, i64>(9)? as usize,
                    gap_count: row.get::<_, i64>(10)? as usize,
                    health_score: row.get(11)?,
                })
            },
        );
        match result {
            Ok(snap) => Ok(Some(snap)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::db(e)),
        }
    }
}

/// Repository for notification subscriptions.
pub struct NotifySubRepo;

impl NotifySubRepo {
    pub fn list_active(conn: &Connection, space_id: &str, event_type: &str) -> AppResult<Vec<(String, String, String)>> {
        let mut stmt = conn.prepare(
            "SELECT id, webhook_url, webhook_secret FROM notify_subscriptions WHERE space_id = ?1 AND event_type = ?2 AND is_active = 1",
        )?;
        let rows = stmt.query_map(params![space_id, event_type], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
        })?;
        let mut result = Vec::new();
        for r in rows {
            result.push(r?);
        }
        Ok(result)
    }

    pub fn subscribe(conn: &Connection, space_id: &str, event_type: &str, webhook_url: &str, webhook_secret: &str) -> AppResult<String> {
        let id = format!("sub_{}", uuid::Uuid::new_v4().to_string().replace('-', ""));
        let now = crate::now_ts();
        conn.execute(
            "INSERT INTO notify_subscriptions (id, space_id, event_type, webhook_url, webhook_secret, is_active, created_at) VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6)",
            params![id, space_id, event_type, webhook_url, webhook_secret, now],
        )?;
        Ok(id)
    }
}
