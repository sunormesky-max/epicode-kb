//! Database repositories: CRUD operations for all entities.

use rusqlite::{params, Connection};
use serde_json::Value;

use crate::error::{AppError, AppResult};
use crate::memory::model::{Memory, Provenance, ReviewStatus, TrustLevel};

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
            "INSERT INTO memories (id, space_id, content, embedding, embedding_model, provenance, provenance_meta, trust_level, review_status, parent_conflict_id, last_accessed_at, access_count, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
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
            "SELECT id, space_id, content, embedding_model, provenance, provenance_meta, trust_level, review_status, parent_conflict_id, last_accessed_at, access_count, created_at, updated_at
             FROM memories WHERE id = ?1",
            params![id],
            |row| {
                let provenance_str: String = row.get(4)?;
                let provenance = Provenance::parse_str(&provenance_str)
                    .map_err(|e| rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, e.into()))?;
                let review_status_str: String = row.get(7)?;
                let review_status = ReviewStatus::parse_str(&review_status_str)
                    .map_err(|e| rusqlite::Error::FromSqlConversionFailure(7, rusqlite::types::Type::Text, e.into()))?;
                let trust_value: f32 = row.get(6)?;
                let provenance_meta_str: Option<String> = row.get(5)?;
                let provenance_meta = provenance_meta_str
                    .and_then(|s| serde_json::from_str::<Value>(&s).ok());
                Ok(Memory {
                    id: row.get(0)?,
                    space_id: row.get(1)?,
                    content: row.get(2)?,
                    embedding: None,
                    embedding_model: row.get(3)?,
                    provenance,
                    provenance_meta,
                    trust_level: TrustLevel::new(trust_value)
                        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(6, rusqlite::types::Type::Real, e.into()))?,
                    review_status,
                    parent_conflict_id: row.get(8)?,
                    last_accessed_at: row.get(9)?,
                    access_count: row.get(10)?,
                    created_at: row.get(11)?,
                    updated_at: row.get(12)?,
                })
            },
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
    pub fn list(
        conn: &Connection,
        space_id: &str,
        provenance: Option<&[Provenance]>,
        min_trust: Option<f32>,
        review_status: Option<ReviewStatus>,
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

        let where_sql = where_clauses.join(" AND ");

        // Count total
        let count_sql = format!("SELECT COUNT(*) FROM memories WHERE {}", where_sql);
        let params_ref: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        let total: usize = conn.query_row(&count_sql, params_ref.as_slice(), |row| row.get(0))?;

        // Query page
        let query_sql = format!(
            "SELECT id, space_id, content, embedding_model, provenance, provenance_meta, trust_level, review_status, parent_conflict_id, last_accessed_at, access_count, created_at, updated_at
             FROM memories WHERE {} ORDER BY created_at DESC LIMIT ?{} OFFSET ?{}",
            where_sql, param_index, param_index + 1
        );
        params_vec.push(Box::new(limit as i64));
        params_vec.push(Box::new(offset as i64));
        let params_ref2: Vec<&dyn rusqlite::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();

        let mut stmt = conn.prepare(&query_sql)?;
        let memories = stmt
            .query_map(params_ref2.as_slice(), |row| {
                let provenance_str: String = row.get(4)?;
                let provenance = Provenance::parse_str(&provenance_str).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        4,
                        rusqlite::types::Type::Text,
                        e.into(),
                    )
                })?;
                let review_status_str: String = row.get(7)?;
                let review_status = ReviewStatus::parse_str(&review_status_str).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        7,
                        rusqlite::types::Type::Text,
                        e.into(),
                    )
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
                        rusqlite::Error::FromSqlConversionFailure(
                            6,
                            rusqlite::types::Type::Real,
                            e.into(),
                        )
                    })?,
                    review_status,
                    parent_conflict_id: row.get(8)?,
                    last_accessed_at: row.get(9)?,
                    access_count: row.get(10)?,
                    created_at: row.get(11)?,
                    updated_at: row.get(12)?,
                })
            })?
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

    /// Update Tantivy-indexed fields (provenance, trust_level, review_status).
    /// This is a no-op at DB level — Tantivy re-indexing is handled by the service layer.
    pub fn get_indexable_fields(
        conn: &Connection,
        id: &str,
    ) -> AppResult<(String, String, f32, String, i64)> {
        let row = conn.query_row(
            "SELECT id, content, trust_level, provenance, created_at FROM memories WHERE id = ?1",
            params![id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, f32>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            },
        )?;
        Ok(row)
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
}

// ============================================================
// Proposal Repository (stub — Sprint 3)
// ============================================================

/// Repository for AI proposals.
/// TODO: Implement full CRUD in Sprint 3.
pub struct ProposalRepo;

impl ProposalRepo {
    /// List proposals by space and status.
    /// TODO: Implement in Sprint 3.
    pub fn list(
        _conn: &Connection,
        _space_id: &str,
        _status: Option<&str>,
    ) -> AppResult<Vec<serde_json::Value>> {
        Err(AppError::not_implemented(
            "proposal listing (planned for Sprint 3)",
        ))
    }

    /// Insert a new proposal.
    /// TODO: Implement in Sprint 3.
    pub fn insert(_conn: &Connection, _proposal: &serde_json::Value) -> AppResult<()> {
        Err(AppError::not_implemented(
            "proposal creation (planned for Sprint 3)",
        ))
    }

    /// Update proposal status.
    /// TODO: Implement in Sprint 3.
    pub fn update_status(_conn: &Connection, _id: &str, _status: &str) -> AppResult<()> {
        Err(AppError::not_implemented(
            "proposal status update (planned for Sprint 3)",
        ))
    }
}

// ============================================================
// Notification Repository (stub — Sprint 3)
// ============================================================

/// Repository for notifications.
/// TODO: Implement full CRUD in Sprint 3.
pub struct NotificationRepo;

impl NotificationRepo {
    /// List notifications for a user.
    /// TODO: Implement in Sprint 3.
    pub fn list_by_user(_conn: &Connection, _user_id: &str) -> AppResult<Vec<serde_json::Value>> {
        Err(AppError::not_implemented(
            "notification listing (planned for Sprint 3)",
        ))
    }

    /// Create a notification.
    /// TODO: Implement in Sprint 3.
    pub fn create(_conn: &Connection, _notif: &serde_json::Value) -> AppResult<()> {
        Err(AppError::not_implemented(
            "notification creation (planned for Sprint 3)",
        ))
    }
}
