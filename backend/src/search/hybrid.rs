//! Hybrid search — combines semantic and full-text search with RRF fusion + trust weighting.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use rusqlite::Connection;

use crate::db::repository::{MemoryRepo, QueryLogRepo};
use crate::error::AppResult;
use crate::memory::model::Memory;
use crate::search::fulltext::FulltextSearcher;
use crate::search::semantic::SemanticSearcher;
use crate::search::{passes_filters, SearchQuery, SearchResponse, SearchResult};

/// RRF (Reciprocal Rank Fusion) constant.
const RRF_K: usize = 60;

/// Hybrid searcher — combines semantic + full-text with RRF and trust weighting.
pub struct HybridSearcher {
    db: Arc<std::sync::Mutex<Connection>>,
    semantic: SemanticSearcher,
    fulltext: FulltextSearcher,
}

impl HybridSearcher {
    /// Create a new HybridSearcher.
    pub fn new(
        db: Arc<std::sync::Mutex<Connection>>,
        semantic: SemanticSearcher,
        fulltext: FulltextSearcher,
    ) -> Self {
        Self {
            db,
            semantic,
            fulltext,
        }
    }

    /// Execute a hybrid search query.
    pub fn search(&self, query: &SearchQuery) -> AppResult<SearchResponse> {
        let start = Instant::now();

        // Fetch limit is increased to account for filtering
        let fetch_limit = (query.limit + query.offset) * 3;

        // Run searches based on mode — release locks as soon as possible
        let (semantic_results, fulltext_results) = match query.mode {
            crate::search::SearchMode::Semantic => {
                let sem = self
                    .semantic
                    .search_and_load(&query.q, &query.space_id, fetch_limit)?;
                (sem, Vec::new())
            }
            crate::search::SearchMode::Fulltext => {
                let ft = {
                    let conn = self.db.lock().unwrap();
                    self.fulltext.search_and_filter(
                        &query.q,
                        &query.space_id,
                        fetch_limit,
                        &conn,
                    )?
                };
                (Vec::new(), ft)
            }
            crate::search::SearchMode::Hybrid => {
                let sem = self
                    .semantic
                    .search_and_load(&query.q, &query.space_id, fetch_limit)?;
                let ft = {
                    let conn = self.db.lock().unwrap();
                    self.fulltext.search_and_filter(
                        &query.q,
                        &query.space_id,
                        fetch_limit,
                        &conn,
                    )?
                };
                (sem, ft)
            }
        };

        // Merge results using RRF + trust weighting (no DB lock needed)
        let merged = self.merge_results(&semantic_results, &fulltext_results, query);

        let total = merged.len();

        // Apply offset and limit
        let results: Vec<SearchResult> = merged
            .into_iter()
            .skip(query.offset)
            .take(query.limit)
            .collect();

        // Update access counts for returned memories — minimal lock time
        {
            let conn = self.db.lock().unwrap();
            let ids: Vec<String> = results.iter().map(|r| r.memory.id.clone()).collect();
            if !ids.is_empty() {
                let _ = MemoryRepo::update_access(&conn, &ids, crate::now_ts());
            }

            // Log the query
            let log_id = crate::generate_id("qlog");
            let _ = QueryLogRepo::log(
                &conn,
                &log_id,
                &query.space_id,
                &query.q,
                total,
                "search",
                None,
            );
        }

        let query_time_ms = start.elapsed().as_millis() as u64;

        Ok(SearchResponse {
            results,
            total,
            query_time_ms,
        })
    }

    /// Merge semantic and full-text results using RRF + trust weighting.
    fn merge_results(
        &self,
        semantic: &[(Memory, f32)],
        fulltext: &[(Memory, f32)],
        query: &SearchQuery,
    ) -> Vec<SearchResult> {
        // Build rank maps for RRF
        let semantic_ranks: HashMap<String, usize> = semantic
            .iter()
            .enumerate()
            .map(|(rank, (mem, _))| (mem.id.clone(), rank))
            .collect();

        let fulltext_ranks: HashMap<String, usize> = fulltext
            .iter()
            .enumerate()
            .map(|(rank, (mem, _))| (mem.id.clone(), rank))
            .collect();

        // Collect all unique memory IDs — avoid cloning entire Memory objects
        let mut memory_map: HashMap<String, (&Memory, Option<f32>, Option<f32>)> = HashMap::new();

        for (mem, score) in semantic {
            memory_map.insert(mem.id.clone(), (mem, Some(*score), None));
        }

        for (mem, score) in fulltext {
            memory_map
                .entry(mem.id.clone())
                .and_modify(|(_, _, ft)| {
                    *ft = Some(*score);
                })
                .or_insert((mem, None, Some(*score)));
        }

        // Compute RRF scores
        let mut scored: Vec<SearchResult> = memory_map
            .into_values()
            .filter_map(|(memory, sem_score, ft_score)| {
                // Apply filters
                if !passes_filters(
                    memory,
                    query.min_trust,
                    query.provenance.as_deref(),
                    query.review_status,
                    query.visibility,
                ) {
                    return None;
                }

                // RRF fusion
                let mut rrf_score = 0.0f32;
                if let Some(&rank) = semantic_ranks.get(&memory.id) {
                    rrf_score += 1.0 / (RRF_K as f32 + rank as f32 + 1.0);
                }
                if let Some(&rank) = fulltext_ranks.get(&memory.id) {
                    rrf_score += 1.0 / (RRF_K as f32 + rank as f32 + 1.0);
                }

                // If only one mode was used, use the raw score
                if sem_score.is_some() && ft_score.is_none() {
                    rrf_score = sem_score.unwrap_or(0.0);
                } else if ft_score.is_some() && sem_score.is_none() {
                    rrf_score = ft_score.unwrap_or(0.0);
                }

                // Trust weight: trust_level^0.5 (higher trust = higher weight)
                let trust_weight = memory.trust_level.value().sqrt();

                // Final score: RRF * trust_weight
                let final_score = rrf_score * trust_weight;

                // Generate highlight
                let highlight = self.fulltext.highlight(&query.q, &memory.content, 200);

                Some(SearchResult {
                    memory: memory.clone(),
                    score: final_score,
                    semantic_score: sem_score,
                    fulltext_score: ft_score,
                    trust_weight,
                    highlight,
                })
            })
            .collect();

        // Sort by final score descending
        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        scored
    }
}
