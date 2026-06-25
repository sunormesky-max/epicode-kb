//! Conflict data models.

use serde::{Deserialize, Serialize};

/// A candidate contradiction between two memories.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictCandidate {
    pub memory_a_id: String,
    pub memory_b_id: String,
    pub content_a: String,
    pub content_b: String,
    pub semantic_distance: f32,
    pub confidence: f32,
    pub summary: String,
}

/// Resolution action for a conflict.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Resolution {
    AcceptA,
    AcceptB,
    BothTrue,
}

/// Conflict detector configuration.
#[derive(Debug, Clone)]
pub struct ConflictConfig {
    /// Semantic distance threshold below which two memories are candidates for contradiction check.
    pub semantic_threshold: f32,
    /// Minimum LLM confidence to flag a contradiction.
    pub llm_confidence_threshold: f32,
    /// Max number of near neighbors to check per memory.
    pub max_neighbors: usize,
}

impl Default for ConflictConfig {
    fn default() -> Self {
        Self {
            semantic_threshold: 0.3,
            llm_confidence_threshold: 0.6,
            max_neighbors: 10,
        }
    }
}
