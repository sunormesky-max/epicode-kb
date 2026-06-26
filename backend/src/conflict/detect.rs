//! Conflict detection engine — semantic distance + optional LLM fact comparison.

use std::sync::Arc;

use crate::db::DbPool;
use crate::embed::EmbeddingProvider;
use crate::error::AppResult;
use crate::memory::model::{Memory, Provenance, TrustLevel};

use super::model::{ConflictCandidate, ConflictConfig};

/// Detects knowledge contradictions between memories.
pub struct ConflictDetector {
    db: DbPool,
    #[allow(dead_code)]
    embedder: Arc<dyn EmbeddingProvider>,
    config: ConflictConfig,
}

impl ConflictDetector {
    pub fn new(
        db: DbPool,
        embedder: Arc<dyn EmbeddingProvider>,
        config: ConflictConfig,
    ) -> Self {
        Self {
            db,
            embedder,
            config,
        }
    }

    /// Detect contradictions between a target memory and its nearest neighbors.
    pub fn detect_one(&self, memory_id: &str) -> AppResult<Vec<ConflictCandidate>> {
        let conn = self
            .db
            .lock()
            .map_err(|e| crate::error::AppError::internal(format!("db lock: {}", e)))?;

        // Fetch the target memory
        let target = crate::db::repository::MemoryRepo::get_by_id(&conn, memory_id)?;

        let embedding = target
            .embedding
            .as_ref()
            .ok_or_else(|| crate::error::AppError::bad_request("memory has no embedding"))?;

        // Fetch all accepted memories in the same space with embeddings
        let mut stmt = conn.prepare(
            "SELECT id, content, embedding FROM memories WHERE space_id = ?1 AND review_status = 'accepted' AND embedding IS NOT NULL AND id != ?2 LIMIT 200"
        ).map_err(crate::error::AppError::db)?;

        let rows: Vec<(String, String, Vec<u8>)> = stmt
            .query_map(rusqlite::params![target.space_id, target.id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .map_err(crate::error::AppError::db)?
            .filter_map(|r| r.ok())
            .collect();

        let mut conflicts = Vec::new();
        let target_vec: Vec<f32> = embedding.to_vec();

        for (cand_id, cand_content, cand_blob) in &rows {
            let cand_emb: Vec<f32> = crate::db::repository::blob_to_embedding(cand_blob);
            if cand_emb.len() != target_vec.len() {
                continue;
            }
            let similarity = cosine_similarity(&target_vec, &cand_emb);
            let semantic_distance = 1.0 - similarity;
            if semantic_distance > self.config.semantic_threshold {
                continue;
            }

            let words_a: std::collections::HashSet<&str> =
                target.content.split_whitespace().collect();
            let words_b: std::collections::HashSet<&str> =
                cand_content.split_whitespace().collect();
            let intersection = words_a.intersection(&words_b).count();
            let union = words_a.union(&words_b).count();
            let jaccard = if union > 0 {
                intersection as f32 / union as f32
            } else {
                0.0
            };

            let contradiction_score = (1.0 - jaccard) * (1.0 - semantic_distance);
            if contradiction_score > 0.3 {
                conflicts.push(ConflictCandidate {
                    memory_a_id: target.id.clone(),
                    memory_b_id: cand_id.clone(),
                    content_a: target.content.clone(),
                    content_b: cand_content.clone(),
                    semantic_distance,
                    confidence: contradiction_score,
                    summary: format!(
                        "Potential contradiction detected (semantic dist={:.3}, Jaccard={:.3})",
                        semantic_distance, jaccard
                    ),
                });
            }
        }

        tracing::info!(
            "Conflict detection for memory {}: found {} potential contradictions",
            memory_id,
            conflicts.len()
        );
        Ok(conflicts)
    }

    /// Scan all memories in a space for contradictions.
    pub fn detect_all(&self, space_id: &str) -> AppResult<Vec<ConflictCandidate>> {
        let conn = self
            .db
            .lock()
            .map_err(|e| crate::error::AppError::internal(format!("db lock: {}", e)))?;

        // Get all reviewed memories in the space
        let mut stmt = conn
            .prepare(
                "SELECT id FROM memories WHERE space_id = ?1 AND review_status = 'accepted' AND embedding IS NOT NULL LIMIT 200",
            )
            .map_err(crate::error::AppError::db)?;

        let memory_ids: Vec<String> = stmt
            .query_map(rusqlite::params![space_id], |row| row.get(0))
            .map_err(crate::error::AppError::db)?
            .filter_map(|r| r.ok())
            .collect();

        let mut all_conflicts = Vec::new();
        for mid in &memory_ids {
            match self.detect_one(mid) {
                Ok(conflicts) => all_conflicts.extend(conflicts),
                Err(e) => tracing::warn!("Conflict detection failed for {}: {}", mid, e),
            }
        }

        tracing::info!(
            "Full scan of space {}: {} memories, {} conflicts found",
            space_id,
            memory_ids.len(),
            all_conflicts.len()
        );
        Ok(all_conflicts)
    }

    /// Create a Conflict memory for the given candidate.
    pub fn create_conflict_memory(
        &self,
        candidate: &ConflictCandidate,
    ) -> AppResult<Memory> {
        let conn = self
            .db
            .lock()
            .map_err(|e| crate::error::AppError::internal(format!("db lock: {}", e)))?;

        let memory_a = crate::db::repository::MemoryRepo::get_by_id(&conn, &candidate.memory_a_id)?;
        let now = crate::now_ts();

        let conflict_memory = Memory {
            id: format!("cf_{}", uuid::Uuid::new_v4().to_string().replace('-', "")),
            space_id: memory_a.space_id.clone(),
            content: format!(
                "⚠️ Knowledge Conflict\n\n**Statement A** ({}):\n{}\n\n**Statement B** ({}):\n{}\n\n**Detection**: {}",
                candidate.memory_a_id,
                candidate.content_a,
                candidate.memory_b_id,
                candidate.content_b,
                candidate.summary,
            ),
            embedding: None,
            embedding_model: "none".into(),
            provenance: Provenance::Conflict,
            provenance_meta: Some(serde_json::json!({
                "conflicting_ids": [&candidate.memory_a_id, &candidate.memory_b_id],
                "detected_by": "conflict-engine-v0.3.0",
                "detected_at": now,
                "semantic_distance": candidate.semantic_distance,
                "confidence": candidate.confidence,
            })),
            trust_level: TrustLevel::new(0.3).unwrap_or_default(),
            review_status: crate::memory::model::ReviewStatus::Pending,
            visibility: crate::memory::model::Visibility::Inherit,
            version_of: None,
            version_seq: 0,
            author_id: Some("system".into()),
            parent_conflict_id: None,
            last_accessed_at: Some(now),
            access_count: 0,
            created_at: now,
            updated_at: now,
        };

        crate::db::repository::MemoryRepo::insert(&conn, &conflict_memory)?;
        Ok(conflict_memory)
    }
}

/// Compute cosine similarity between two float vectors.
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let mut dot = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;
    for i in 0..a.len().min(b.len()) {
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }
    let denom = (norm_a * norm_b).sqrt();
    if denom == 0.0 { 0.0 } else { dot / denom }
}
