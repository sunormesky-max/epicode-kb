//! Search engine: trait + query/response types + SearchMode enum.

pub mod fulltext;
pub mod hybrid;
pub mod semantic;

use serde::{Deserialize, Serialize};

use crate::error::AppResult;
use crate::memory::model::{Memory, Provenance, ReviewStatus, Visibility};

/// Search engine trait.
pub trait SearchEngine: Send + Sync {
    /// Execute a search query.
    fn search(&self, query: &SearchQuery) -> AppResult<SearchResponse>;
}

/// Search query parameters.
#[derive(Debug, Clone, Deserialize)]
pub struct SearchQuery {
    /// Search query text.
    pub q: String,
    /// Space ID for isolation.
    pub space_id: String,
    /// Search mode (semantic, fulltext, or hybrid).
    #[serde(default)]
    pub mode: SearchMode,
    /// Minimum trust level filter.
    pub min_trust: Option<f32>,
    /// Provenance filter.
    pub provenance: Option<Vec<Provenance>>,
    /// Review status filter.
    pub review_status: Option<ReviewStatus>,
    /// Visibility filter.
    pub visibility: Option<Visibility>,
    /// Maximum number of results.
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Pagination offset.
    #[serde(default)]
    pub offset: usize,
}

fn default_limit() -> usize {
    20
}

/// Search mode.
#[derive(Debug, Clone, Copy, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SearchMode {
    /// Semantic vector search only.
    Semantic,
    /// Full-text search only.
    Fulltext,
    /// Hybrid: semantic + full-text with RRF fusion (default).
    #[default]
    Hybrid,
}

impl SearchMode {
    /// Parse from string.
    pub fn parse_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "semantic" => SearchMode::Semantic,
            "fulltext" => SearchMode::Fulltext,
            _ => SearchMode::Hybrid,
        }
    }
}

/// A single search result.
#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    /// The matched memory.
    pub memory: Memory,
    /// Combined score.
    pub score: f32,
    /// Semantic similarity score (if applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_score: Option<f32>,
    /// Full-text match score (if applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fulltext_score: Option<f32>,
    /// Trust weight applied.
    pub trust_weight: f32,
    /// Highlighted content snippet.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub highlight: Option<String>,
}

/// Search response.
#[derive(Debug, Clone, Serialize)]
pub struct SearchResponse {
    /// Search results.
    pub results: Vec<SearchResult>,
    /// Total matching count.
    pub total: usize,
    /// Query execution time in milliseconds.
    pub query_time_ms: u64,
}

/// Apply filters to a search result.
pub fn passes_filters(
    memory: &Memory,
    min_trust: Option<f32>,
    provenance: Option<&[Provenance]>,
    review_status: Option<ReviewStatus>,
    visibility: Option<Visibility>,
) -> bool {
    if let Some(trust) = min_trust {
        if memory.trust_level.value() < trust {
            return false;
        }
    }
    if let Some(provs) = provenance {
        if !provs.is_empty() && !provs.contains(&memory.provenance) {
            return false;
        }
    }
    if let Some(status) = review_status {
        if memory.review_status != status {
            return false;
        }
    }
    if let Some(vis) = visibility {
        if memory.visibility != vis {
            return false;
        }
    }
    true
}
