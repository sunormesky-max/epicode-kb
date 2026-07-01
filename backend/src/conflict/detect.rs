//! Conflict detection engine — semantic distance + optional LLM fact comparison.

use std::collections::HashMap;
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

        // Fetch the target memory (with embedding — required for comparison).
        let target = crate::db::repository::MemoryRepo::get_by_id_with_embedding(&conn, memory_id)?;

        let embedding = target
            .embedding
            .as_ref()
            .ok_or_else(|| crate::error::AppError::bad_request("memory has no embedding"))?;

        // Fetch all accepted memories in the same space with embeddings
        let rows: Vec<(String, String, Vec<u8>)> = {
            let mut stmt = conn.prepare(
                "SELECT id, content, embedding FROM memories WHERE space_id = ?1 AND review_status = 'accepted' AND embedding IS NOT NULL AND id != ?2 LIMIT 200"
            ).map_err(crate::error::AppError::db)?;

            let mapped = stmt
                .query_map(rusqlite::params![target.space_id, target.id], |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?))
                })
                .map_err(crate::error::AppError::db)?;
            mapped.filter_map(|r| r.ok()).collect()
        };

        drop(conn); // Release DB lock early

        let mut conflicts = Vec::new();
        let target_vec: Vec<f32> = embedding.to_vec();

        // Cache embeddings to avoid repeated blob_to_embedding calls
        let mut embedding_cache: HashMap<String, Vec<f32>> = HashMap::new();

        for (cand_id, cand_content, cand_blob) in &rows {
            let cand_emb = embedding_cache
                .entry(cand_id.clone())
                .or_insert_with(|| crate::db::repository::blob_to_embedding(cand_blob))
                .clone();

            if cand_emb.len() != target_vec.len() {
                continue;
            }
            let similarity = cosine_similarity(&target_vec, &cand_emb);
            let semantic_distance = 1.0 - similarity;
            if semantic_distance > self.config.semantic_threshold {
                continue;
            }

            let jaccard = jaccard_similarity(&target.content, cand_content);
            let score = contradiction_score(semantic_distance, jaccard);
            if score > CONTRADICTION_THRESHOLD {
                conflicts.push(ConflictCandidate {
                    memory_a_id: target.id.clone(),
                    memory_b_id: cand_id.clone(),
                    content_a: target.content.clone(),
                    content_b: cand_content.clone(),
                    semantic_distance,
                    confidence: score,
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
    /// Optimized: fetch all embeddings in one pass, then compare in-memory to avoid O(n²) DB queries.
    pub fn detect_all(&self, space_id: &str) -> AppResult<Vec<ConflictCandidate>> {
        // Collect all memory data first, then release the lock before processing
        let rows: Vec<(String, String, Vec<f32>)> = {
            let conn = self
                .db
                .lock()
                .map_err(|e| crate::error::AppError::internal(format!("db lock: {}", e)))?;

            let mut stmt = conn
                .prepare(
                    "SELECT id, content, embedding FROM memories WHERE space_id = ?1 AND review_status = 'accepted' AND embedding IS NOT NULL LIMIT 200",
                )
                .map_err(crate::error::AppError::db)?;

            let rows = stmt
                .query_map(rusqlite::params![space_id], |row| {
                    let blob: Vec<u8> = row.get(2)?;
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        crate::db::repository::blob_to_embedding(&blob),
                    ))
                })
                .map_err(crate::error::AppError::db)?;

            rows.filter_map(|r| r.ok()).collect()
        };

        let mut all_conflicts = Vec::new();

        // In-memory pairwise comparison — O(n²/2) but no repeated DB queries or blob conversions
        for i in 0..rows.len() {
            let (id_a, content_a, emb_a) = &rows[i];
            for j in (i + 1)..rows.len() {
                let (id_b, content_b, emb_b) = &rows[j];

                if emb_a.len() != emb_b.len() {
                    continue;
                }
                let similarity = cosine_similarity(emb_a, emb_b);
                let semantic_distance = 1.0 - similarity;
                if semantic_distance > self.config.semantic_threshold {
                    continue;
                }

                let jaccard = jaccard_similarity(content_a, content_b);
                let score = contradiction_score(semantic_distance, jaccard);
                if score > CONTRADICTION_THRESHOLD {
                    all_conflicts.push(ConflictCandidate {
                        memory_a_id: id_a.clone(),
                        memory_b_id: id_b.clone(),
                        content_a: content_a.clone(),
                        content_b: content_b.clone(),
                        semantic_distance,
                        confidence: score,
                        summary: format!(
                            "Potential contradiction detected (semantic dist={:.3}, Jaccard={:.3})",
                            semantic_distance, jaccard
                        ),
                    });
                }
            }
        }

        tracing::info!(
            "Full scan of space {}: {} memories, {} conflicts found",
            space_id,
            rows.len(),
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
                "⚠️ Knowledge Conflict

**Statement A** ({}):
{}

**Statement B** ({}):
{}

**Detection**: {}",
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
pub(crate) fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
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

/// Word-overlap (Jaccard) similarity between two text snippets.
pub(crate) fn jaccard_similarity(a: &str, b: &str) -> f32 {
    let words_a: std::collections::HashSet<&str> = a.split_whitespace().collect();
    let words_b: std::collections::HashSet<&str> = b.split_whitespace().collect();
    let union = words_a.union(&words_b).count();
    if union == 0 {
        return 0.0;
    }
    let intersection = words_a.intersection(&words_b).count();
    intersection as f32 / union as f32
}

/// Heuristic contradiction score in [0,1]: high when two snippets are
/// semantically close (small distance) but lexically divergent (low Jaccard).
pub(crate) fn contradiction_score(semantic_distance: f32, jaccard: f32) -> f32 {
    (1.0 - jaccard) * (1.0 - semantic_distance)
}

/// Default contradiction threshold above which a pair is flagged as a conflict.
pub(crate) const CONTRADICTION_THRESHOLD: f32 = 0.3;
