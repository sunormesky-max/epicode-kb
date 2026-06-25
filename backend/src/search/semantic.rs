//! Semantic search — cosine similarity over stored embeddings.

use std::sync::Arc;

use rusqlite::Connection;

use crate::db::repository::MemoryRepo;
use crate::embed::EmbeddingProvider;
use crate::error::AppResult;
use crate::memory::model::Memory;

/// Semantic searcher — loads all embeddings for a space and computes cosine similarity.
pub struct SemanticSearcher {
    db: Arc<std::sync::Mutex<Connection>>,
    embedder: Arc<dyn EmbeddingProvider>,
}

impl SemanticSearcher {
    /// Create a new SemanticSearcher.
    pub fn new(
        db: Arc<std::sync::Mutex<Connection>>,
        embedder: Arc<dyn EmbeddingProvider>,
    ) -> Self {
        Self { db, embedder }
    }

    /// Search for memories semantically similar to the query text.
    /// Returns (memory_id, semantic_score, trust_level) pairs, sorted by score descending.
    pub fn search(
        &self,
        query_text: &str,
        space_id: &str,
        limit: usize,
    ) -> AppResult<Vec<(String, f32, f32)>> {
        // Generate query embedding
        let query_vec = self.embedder.embed(query_text)?;

        // Load all embeddings for the space
        let embeddings = {
            let conn = self.db.lock().unwrap();
            MemoryRepo::load_all_with_embeddings(&conn, space_id)?
        };

        // Compute cosine similarity for each
        let mut scored: Vec<(String, f32, f32)> = embeddings
            .into_iter()
            .map(|(id, embedding, trust)| {
                let score = cosine_similarity(&query_vec, &embedding);
                (id, score, trust)
            })
            .collect();

        // Sort by score descending
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Return top-K
        scored.truncate(limit);
        Ok(scored)
    }

    /// Search and load memories from DB.
    pub fn search_and_load(
        &self,
        query_text: &str,
        space_id: &str,
        limit: usize,
    ) -> AppResult<Vec<(Memory, f32)>> {
        let scored = self.search(query_text, space_id, limit)?;

        let conn = self.db.lock().unwrap();
        let mut results = Vec::new();
        for (memory_id, score, _trust) in scored {
            match MemoryRepo::get_by_id(&conn, &memory_id) {
                Ok(memory) => {
                    results.push((memory, score));
                }
                Err(e) => {
                    tracing::warn!("Failed to load memory {}: {}", memory_id, e);
                }
            }
        }

        Ok(results)
    }
}

/// Compute cosine similarity between two vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let mut dot = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;

    for i in 0..a.len() {
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }

    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom == 0.0 {
        0.0
    } else {
        dot / denom
    }
}
