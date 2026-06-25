//! AI Proposal Engine (stub — Sprint 3).
//!
//! TODO: Implement full proposal engine in Sprint 3:
//! - scan_space(): detect duplicates, clusters, contradictions
//! - generate proposals via LLM
//! - approve/reject/modify proposal actions

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

/// Proposal type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProposalType {
    /// Merge duplicate memories.
    Merge,
    /// Link related memories.
    Link,
    /// Summarize a cluster of memories.
    Summarize,
    /// Flag conflicting memories.
    Conflict,
    /// Archive stale memories.
    Archive,
}

/// Proposal status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProposalStatus {
    /// Awaiting review.
    Pending,
    /// Approved by reviewer.
    Approved,
    /// Rejected by reviewer.
    Rejected,
    /// Modified then adopted.
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

/// Proposal engine — scans spaces and generates AI proposals.
/// TODO: Implement in Sprint 3.
pub struct ProposalEngine;

impl ProposalEngine {
    /// Create a new ProposalEngine.
    pub fn new() -> Self {
        Self
    }

    /// Scan a space for proposal opportunities (duplicates, clusters, contradictions).
    /// TODO: Implement in Sprint 3.
    pub async fn scan_space(&self, _space_id: &str) -> AppResult<Vec<AiProposal>> {
        Err(AppError::not_implemented(
            "proposal engine scan_space (planned for Sprint 3)",
        ))
    }

    /// Approve a proposal and execute its action.
    /// TODO: Implement in Sprint 3.
    pub async fn approve(&self, _proposal_id: &str, _reviewer_id: &str) -> AppResult<AiProposal> {
        Err(AppError::not_implemented(
            "proposal approve (planned for Sprint 3)",
        ))
    }

    /// Reject a proposal.
    /// TODO: Implement in Sprint 3.
    pub async fn reject(
        &self,
        _proposal_id: &str,
        _reviewer_id: &str,
        _feedback: Option<&str>,
    ) -> AppResult<AiProposal> {
        Err(AppError::not_implemented(
            "proposal reject (planned for Sprint 3)",
        ))
    }

    /// Modify and adopt a proposal.
    /// TODO: Implement in Sprint 3.
    pub async fn modify(
        &self,
        _proposal_id: &str,
        _reviewer_id: &str,
        _modified_content: &str,
    ) -> AppResult<AiProposal> {
        Err(AppError::not_implemented(
            "proposal modify (planned for Sprint 3)",
        ))
    }
}

impl Default for ProposalEngine {
    fn default() -> Self {
        Self::new()
    }
}
