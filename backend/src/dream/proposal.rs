//! AI Proposal Engine — generates, stores, and manages AI proposals for review.
//!
//! The Proposal Engine scans a memory space to detect merge/link/stale/conflict candidates,
//! generates proposals via LLM, and supports approve/reject/modify/batch workflows.

use serde::{Deserialize, Serialize};

use crate::db::DbPool;
use crate::db::repository::ProposalRepo;
use crate::error::{AppError, AppResult};

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
}

impl ProposalEngine {
    pub fn new(db: DbPool) -> Self {
        Self { db }
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
                .prepare("SELECT id, content FROM memories WHERE space_id = ?1 AND review_status = 'accepted' LIMIT 500")
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
                    for j in (i + 1)..rows.len().min(i + 20) {
                        let words_a: std::collections::HashSet<&str> =
                            rows[i].1.split_whitespace().collect();
                        let words_b: std::collections::HashSet<&str> =
                            rows[j].1.split_whitespace().collect();
                        if words_a.is_empty() || words_b.is_empty() {
                            continue;
                        }
                        let intersection = words_a.intersection(&words_b).count();
                        let union = words_a.union(&words_b).count();
                        let jaccard = intersection as f32 / union as f32;
                        if jaccard > 0.6 {
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
            let cutoff = now - 90 * 86400;
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

    /// Approve a proposal and execute its action.
    pub fn approve(&self, proposal_id: &str, reviewer_id: &str) -> AppResult<AiProposal> {
        let conn = self.db.lock().map_err(|e| AppError::internal(format!("db lock: {}", e)))?;
        let proposal = ProposalRepo::get_by_id(&conn, proposal_id)?;
        if proposal.status != ProposalStatus::Pending {
            return Err(AppError::conflict("proposal already reviewed"));
        }
        ProposalRepo::update_status(&conn, proposal_id, "approved", reviewer_id, None)?;

        let approved = AiProposal {
            status: ProposalStatus::Approved,
            reviewer_id: Some(reviewer_id.to_string()),
            reviewed_at: Some(crate::now_ts()),
            ..proposal
        };
        Ok(approved)
    }

    /// Reject a proposal with optional feedback.
    pub fn reject(
        &self,
        proposal_id: &str,
        reviewer_id: &str,
        feedback: Option<&str>,
    ) -> AppResult<AiProposal> {
        let conn = self.db.lock().map_err(|e| AppError::internal(format!("db lock: {}", e)))?;
        let proposal = ProposalRepo::get_by_id(&conn, proposal_id)?;
        if proposal.status != ProposalStatus::Pending {
            return Err(AppError::conflict("proposal already reviewed"));
        }

        // Record feedback for strategy learning
        let fb = feedback.unwrap_or("no feedback");
        ProposalRepo::update_status(&conn, proposal_id, "rejected", reviewer_id, Some(fb))?;

        // Simple learning: if same type rejected 3+ consecutive times, log warning
        let type_str = serde_json::to_string(&proposal.proposal_type).unwrap_or_default();
        let rejected_count = ProposalRepo::count_by_type(
            &conn,
            &proposal.space_id,
            type_str.trim_matches('"'),
            "rejected",
        )?;
        if rejected_count % 3 == 0 && rejected_count > 0 {
            tracing::warn!(
                "Proposal type {:?} has been rejected {} times in space {}. Consider lowering generation frequency.",
                proposal.proposal_type,
                rejected_count,
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

    /// Modify and adopt a proposal.
    pub fn modify(
        &self,
        proposal_id: &str,
        reviewer_id: &str,
        modified_content: &str,
    ) -> AppResult<AiProposal> {
        let conn = self.db.lock().map_err(|e| AppError::internal(format!("db lock: {}", e)))?;
        let proposal = ProposalRepo::get_by_id(&conn, proposal_id)?;
        if proposal.status != ProposalStatus::Pending {
            return Err(AppError::conflict("proposal already reviewed"));
        }
        ProposalRepo::update_status(
            &conn,
            proposal_id,
            "modified",
            reviewer_id,
            Some(modified_content),
        )?;

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
