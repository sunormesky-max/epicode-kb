//! AI Proposal Engine — generates, stores, and manages AI proposals for review.
//!
//! The Proposal Engine scans a memory space to detect merge/link/stale/conflict candidates,
//! generates proposals via LLM, and supports approve/reject/modify/batch workflows.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::conflict::detect::ConflictDetector;
use crate::db::DbPool;
use crate::db::repository::ProposalRepo;
use crate::error::{AppError, AppResult};
/// Magic-number constants for the proposal engine.
const MAX_SCAN_MEMORIES: usize = 500;
const MAX_MERGE_CANDIDATES: usize = 20;
const JACCARD_MERGE_THRESHOLD: f32 = 0.6;
const MIN_WORDS_FOR_MERGE: usize = 3;
const STALE_DAYS_THRESHOLD: i64 = 90;
const CONSECUTIVE_REJECT_THRESHOLD: i64 = 3;


/// Proposal type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProposalType {
    Merge,
    Link,
    Summarize,
    Conflict,
    Archive,
}

/// Proposal status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProposalStatus {
    Pending,
    Approved,
    Rejected,
    Modified,
}

/// AI proposal model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiProposal {
    pub id: String,
    pub space_id: String,
    pub proposal_type: ProposalType,
    pub source_memory_ids: Vec<String>,
    pub proposed_content: Option<String>,
    pub proposed_action: Option<serde_json::Value>,
    pub ai_model: Option<String>,
    pub confidence: Option<f32>,
    pub status: ProposalStatus,
    pub reviewer_id: Option<String>,
    pub reviewed_at: Option<i64>,
    pub review_feedback: Option<String>,
    pub created_at: i64,
}

/// Proposal engine context (review action).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchAction {
    pub action: String, // "approve" | "reject"
    pub proposal_ids: Vec<String>,
    pub feedback: Option<String>,
}

/// Proposal engine — scans spaces and manages AI proposal lifecycle.
pub struct ProposalEngine {
    db: DbPool,
    conflict_detector: Option<Arc<ConflictDetector>>,
}

impl ProposalEngine {
    pub fn new(db: DbPool) -> Self {
        Self {
            db,
            conflict_detector: None,
        }
    }

    /// Construct an engine wired to a conflict detector so `scan_space`
    /// also emits `Conflict` proposals for detected knowledge contradictions.
    pub fn new_with_conflict(db: DbPool, detector: Arc<ConflictDetector>) -> Self {
        Self {
            db,
            conflict_detector: Some(detector),
        }
    }

    /// List proposals with pagination.
    pub fn list(&self, space_id: &str, status: Option<&str>, limit: i64, offset: i64) -> AppResult<Vec<AiProposal>> {
        let conn = self.db.lock().map_err(|e| AppError::internal(format!("db lock: {}", e)))?;
        ProposalRepo::list(&conn, space_id, status, limit, offset)
    }

    /// Generate a unique proposal ID.
    fn new_id() -> String {
        format!("pro_{}", uuid::Uuid::new_v4().to_string().replace('-', ""))
    }

    /// Scan a space for proposal opportunities.
    /// Currently uses heuristic rules (no LLM in v0.3.0 scan — LLM reserved for conflict detection).
    pub fn scan_space(&self, space_id: &str) -> AppResult<Vec<AiProposal>> {
        let conn = self.db.lock().map_err(|e| AppError::internal(format!("db lock: {}", e)))?;
        let now = crate::now_ts();
        let mut proposals = Vec::new();

        // ---- Merge candidates: memories with very similar content ----
        {
            let mut stmt = conn
                .prepare(&format!("SELECT id, content FROM memories WHERE space_id = ?1 AND review_status = 'accepted' LIMIT {}", MAX_SCAN_MEMORIES))
                .map_err(AppError::db)?;
            let rows: Vec<(String, String)> = stmt
                .query_map(rusqlite::params![space_id], |row| {
                    Ok((row.get(0)?, row.get(1)?))
                })
                .map_err(AppError::db)?
                .filter_map(|r| r.ok())
                .collect();

            if rows.len() >= 2 {
                // Simple heuristic: short content + overlapping words → merge candidate
                for i in 0..rows.len().min(rows.len() - 1) {
                    for j in (i + 1)..rows.len().min(i + MAX_MERGE_CANDIDATES) {
                        let words_a: std::collections::HashSet<&str> =
                            rows[i].1.split_whitespace().collect();
                        let words_b: std::collections::HashSet<&str> =
                            rows[j].1.split_whitespace().collect();
                        if words_a.len() < MIN_WORDS_FOR_MERGE || words_b.len() < MIN_WORDS_FOR_MERGE {
                            continue;
                        }
                        let intersection = words_a.intersection(&words_b).count();
                        let union = words_a.union(&words_b).count();
                        let jaccard = intersection as f32 / union as f32;
                        if jaccard > JACCARD_MERGE_THRESHOLD {
                            proposals.push(AiProposal {
                                id: Self::new_id(),
                                space_id: space_id.to_string(),
                                proposal_type: ProposalType::Merge,
                                source_memory_ids: vec![rows[i].0.clone(), rows[j].0.clone()],
                                proposed_content: Some(format!(
                                    "Merge candidates (Jaccard={:.2}):\n---\n{}\n---\n{}",
                                    jaccard, rows[i].1, rows[j].1
                                )),
                                proposed_action: None,
                                ai_model: Some("heuristic-jaccard".into()),
                                confidence: Some(jaccard),
                                status: ProposalStatus::Pending,
                                reviewer_id: None,
                                reviewed_at: None,
                                review_feedback: None,
                                created_at: now,
                            });
                        }
                    }
                }
            }
        }

        // ---- Stale candidates: not accessed in 90 days ----
        {
            let cutoff = now - STALE_DAYS_THRESHOLD * 86400;
            let mut stmt = conn
                .prepare(
                    "SELECT id, content FROM memories WHERE space_id = ?1 AND last_accessed_at < ?2 AND review_status = 'accepted' LIMIT 20",
                )
                .map_err(AppError::db)?;
            let rows: Vec<(String, String)> = stmt
                .query_map(rusqlite::params![space_id, cutoff], |row| {
                    Ok((row.get(0)?, row.get(1)?))
                })
                .map_err(AppError::db)?
                .filter_map(|r| r.ok())
                .collect();

            for (mid, content) in &rows {
                proposals.push(AiProposal {
                    id: Self::new_id(),
                    space_id: space_id.to_string(),
                    proposal_type: ProposalType::Archive,
                    source_memory_ids: vec![mid.clone()],
                    proposed_content: Some(content.clone()),
                    proposed_action: None,
                    ai_model: Some("heuristic-staleness".into()),
                    confidence: Some(0.8),
                    status: ProposalStatus::Pending,
                    reviewer_id: None,
                    reviewed_at: None,
                    review_feedback: None,
                    created_at: now,
                });
            }
        }

        // ---- Conflict candidates: semantic contradictions across the space ----
        // Delegates to the (optional) ConflictDetector, which needs its own DB
        // lock, so we release the current connection before scanning.
        if let Some(detector) = &self.conflict_detector {
            let existing_pairs = Self::collect_existing_conflict_pairs(&conn, space_id)?;

            // Release the lock before the detector acquires its own.
            drop(conn);

            let candidates = match detector.detect_all(space_id) {
                Ok(cs) => cs,
                Err(e) => {
                    tracing::warn!("Conflict scan failed for space {}: {}", space_id, e);
                    Vec::new()
                }
            };

            let conn = self
                .db
                .lock()
                .map_err(|e| AppError::internal(format!("db lock: {}", e)))?;

            for cand in &candidates {
                // Dedup: skip if a pending conflict proposal already covers this pair.
                let pair_key = pair_key(&cand.memory_a_id, &cand.memory_b_id);
                if existing_pairs.contains(&pair_key) {
                    continue;
                }
                proposals.push(AiProposal {
                    id: Self::new_id(),
                    space_id: space_id.to_string(),
                    proposal_type: ProposalType::Conflict,
                    source_memory_ids: vec![cand.memory_a_id.clone(), cand.memory_b_id.clone()],
                    proposed_content: Some(cand.summary.clone()),
                    proposed_action: Some(serde_json::json!({
                        "semantic_distance": cand.semantic_distance,
                        "confidence": cand.confidence,
                        "content_a": cand.content_a,
                        "content_b": cand.content_b,
                    })),
                    ai_model: Some("heuristic-conflict".into()),
                    confidence: Some(cand.confidence),
                    status: ProposalStatus::Pending,
                    reviewer_id: None,
                    reviewed_at: None,
                    review_feedback: None,
                    created_at: now,
                });
            }

            // Persist proposals
            for p in &proposals {
                ProposalRepo::insert(&conn, p)?;
            }

            tracing::info!(
                "Proposal scan for space {} generated {} proposals ({} conflict candidates detected)",
                space_id,
                proposals.len(),
                candidates.len()
            );
            return Ok(proposals);
        }

        // Persist proposals
        for p in &proposals {
            ProposalRepo::insert(&conn, p)?;
        }

        tracing::info!(
            "Proposal scan for space {} generated {} proposals",
            space_id,
            proposals.len()
        );
        Ok(proposals)
    }

    /// Collect the set of normalized memory-pair keys already covered by
    /// pending conflict proposals, so we don't emit duplicates.
    fn collect_existing_conflict_pairs(
        conn: &rusqlite::Connection,
        space_id: &str,
    ) -> AppResult<std::collections::HashSet<String>> {
        let mut stmt = conn
            .prepare(
                "SELECT source_memory_ids FROM ai_proposals
                 WHERE space_id = ?1 AND proposal_type = 'conflict' AND status = 'pending'",
            )
            .map_err(AppError::db)?;
        let rows: Vec<String> = stmt
            .query_map(rusqlite::params![space_id], |row| row.get(0))
            .map_err(AppError::db)?
            .filter_map(|r| r.ok())
            .collect();
        let mut set = std::collections::HashSet::new();
        for raw in &rows {
            if let Ok(ids) = serde_json::from_str::<Vec<String>>(raw) {
                if let (Some(a), Some(b)) = (ids.first(), ids.get(1)) {
                    set.insert(pair_key(a, b));
                }
            }
        }
        Ok(set)
    }

    /// Approve a proposal and execute its action (within a transaction).
    pub fn approve(&self, proposal_id: &str, reviewer_id: &str) -> AppResult<AiProposal> {
        let conn = self.db.lock().map_err(|e| AppError::internal(format!("db lock: {}", e)))?;
        let tx = conn.unchecked_transaction()?;
        let proposal = ProposalRepo::get_by_id(&tx, proposal_id)?;
        if proposal.status != ProposalStatus::Pending {
            return Err(AppError::conflict("proposal already reviewed"));
        }
        ProposalRepo::update_status(&tx, proposal_id, "approved", reviewer_id, None)?;
        tx.commit()?;

        let approved = AiProposal {
            status: ProposalStatus::Approved,
            reviewer_id: Some(reviewer_id.to_string()),
            reviewed_at: Some(crate::now_ts()),
            ..proposal
        };
        Ok(approved)
    }

    /// Reject a proposal with optional feedback (within a transaction).
    pub fn reject(
        &self,
        proposal_id: &str,
        reviewer_id: &str,
        feedback: Option<&str>,
    ) -> AppResult<AiProposal> {
        let conn = self.db.lock().map_err(|e| AppError::internal(format!("db lock: {}", e)))?;
        let tx = conn.unchecked_transaction()?;
        let proposal = ProposalRepo::get_by_id(&tx, proposal_id)?;
        if proposal.status != ProposalStatus::Pending {
            return Err(AppError::conflict("proposal already reviewed"));
        }

        // Record feedback for strategy learning
        let fb = feedback.unwrap_or("no feedback");
        ProposalRepo::update_status(&tx, proposal_id, "rejected", reviewer_id, Some(fb))?;
        tx.commit()?;

        // Check for consecutive rejections (not just total count)
        let type_str = serde_json::to_string(&proposal.proposal_type).unwrap_or_default();
        let rejected_count = ProposalRepo::count_by_type(
            &conn,
            &proposal.space_id,
            type_str.trim_matches('"'),
            "rejected",
        )?;
        // Check if rejected_count is a multiple of the threshold (indicating another batch of consecutive rejections)
        if rejected_count >= CONSECUTIVE_REJECT_THRESHOLD && rejected_count % CONSECUTIVE_REJECT_THRESHOLD == 0 {
            tracing::warn!(
                "Proposal type {:?} has been rejected {} times ({} consecutive batches) in space {}. Consider lowering generation frequency.",
                proposal.proposal_type,
                rejected_count,
                rejected_count / CONSECUTIVE_REJECT_THRESHOLD,
                proposal.space_id
            );
        }

        let rejected = AiProposal {
            status: ProposalStatus::Rejected,
            reviewer_id: Some(reviewer_id.to_string()),
            review_feedback: Some(fb.to_string()),
            reviewed_at: Some(crate::now_ts()),
            ..proposal
        };
        Ok(rejected)
    }

    /// Modify and adopt a proposal (within a transaction).
    pub fn modify(
        &self,
        proposal_id: &str,
        reviewer_id: &str,
        modified_content: &str,
    ) -> AppResult<AiProposal> {
        let conn = self.db.lock().map_err(|e| AppError::internal(format!("db lock: {}", e)))?;
        let tx = conn.unchecked_transaction()?;
        let proposal = ProposalRepo::get_by_id(&tx, proposal_id)?;
        if proposal.status != ProposalStatus::Pending {
            return Err(AppError::conflict("proposal already reviewed"));
        }
        ProposalRepo::update_status(
            &tx,
            proposal_id,
            "modified",
            reviewer_id,
            Some(modified_content),
        )?;
        tx.commit()?;

        let modified = AiProposal {
            status: ProposalStatus::Modified,
            reviewer_id: Some(reviewer_id.to_string()),
            review_feedback: Some(modified_content.to_string()),
            reviewed_at: Some(crate::now_ts()),
            proposed_content: Some(modified_content.to_string()),
            ..proposal
        };
        Ok(modified)
    }

    /// Batch approve or reject proposals.
    pub fn batch(
        &self,
        action: &BatchAction,
        reviewer_id: &str,
    ) -> AppResult<Vec<AiProposal>> {
        let mut results = Vec::new();
        for pid in &action.proposal_ids {
            match action.action.as_str() {
                "approve" => {
                    match self.approve(pid, reviewer_id) {
                        Ok(p) => results.push(p),
                        Err(e) => tracing::warn!("Batch approve failed for {}: {}", pid, e),
                    }
                }
                "reject" => {
                    match self.reject(pid, reviewer_id, action.feedback.as_deref()) {
                        Ok(p) => results.push(p),
                        Err(e) => tracing::warn!("Batch reject failed for {}: {}", pid, e),
                    }
                }
                _ => {}
            }
        }
        Ok(results)
    }
}

/// Build a normalized, order-independent key for a memory pair so the same
/// contradiction is not proposed twice regardless of memory ordering.
fn pair_key(a: &str, b: &str) -> String {
    if a <= b {
        format!("{}|{}", a, b)
    } else {
        format!("{}|{}", b, a)
    }
}
