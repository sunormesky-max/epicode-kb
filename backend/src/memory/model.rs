//! Memory domain model: Provenance, TrustLevel, ReviewStatus, Memory, Visibility.

use serde::{Deserialize, Serialize};

use crate::error::AppError;

// ============================================================
// Provenance
// ============================================================

/// Memory provenance marker — tracks where a memory came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Provenance {
    /// Human-authored memory.
    #[default]
    Human,
    /// AI-generated memory.
    Ai,
    /// Human-AI collaboration (human edited AI, or AI assisted human).
    Co,
    /// Conflict memory (contradiction detection result).
    Conflict,
}

impl Provenance {
    /// Default trust level for this provenance.
    pub fn default_trust(&self) -> f32 {
        match self {
            Provenance::Human => 1.0,
            Provenance::Co => 0.8,
            Provenance::Ai => 0.5,
            Provenance::Conflict => 0.3,
        }
    }

    /// Default review status for this provenance.
    pub fn default_review_status(&self) -> ReviewStatus {
        match self {
            Provenance::Human => ReviewStatus::Accepted,
            Provenance::Co => ReviewStatus::Accepted,
            Provenance::Ai => ReviewStatus::Pending,
            Provenance::Conflict => ReviewStatus::Pending,
        }
    }

    /// String representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Provenance::Human => "human",
            Provenance::Ai => "ai",
            Provenance::Co => "co",
            Provenance::Conflict => "conflict",
        }
    }

    /// Parse from string.
    pub fn parse_str(s: &str) -> Result<Self, String> {
        match s {
            "human" => Ok(Provenance::Human),
            "ai" => Ok(Provenance::Ai),
            "co" => Ok(Provenance::Co),
            "conflict" => Ok(Provenance::Conflict),
            _ => Err(format!("invalid provenance: {}", s)),
        }
    }
}

impl std::fmt::Display for Provenance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ============================================================
// TrustLevel
// ============================================================

/// Trust level (0.0 ~ 1.0) — confidence in a memory's accuracy.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TrustLevel(f32);

impl TrustLevel {
    /// Create a new TrustLevel, validating the range.
    pub fn new(value: f32) -> Result<Self, String> {
        if !(0.0..=1.0).contains(&value) {
            return Err(format!("trust_level must be 0.0~1.0, got {}", value));
        }
        Ok(TrustLevel(value))
    }

    /// Get the raw value.
    pub fn value(&self) -> f32 {
        self.0
    }

    /// Adjust trust by a delta, clamping to [0.0, 1.0].
    pub fn adjust(&self, delta: f32) -> Self {
        TrustLevel((self.0 + delta).clamp(0.0, 1.0))
    }
}

impl Default for TrustLevel {
    fn default() -> Self {
        TrustLevel(1.0)
    }
}

impl std::fmt::Display for TrustLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.2}", self.0)
    }
}

// ============================================================
// ReviewStatus
// ============================================================

/// Memory review status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ReviewStatus {
    /// Pending review (AI-generated or conflict memories).
    Pending,
    /// Accepted (reviewed and approved, or human-authored).
    #[default]
    Accepted,
    /// Rejected (reviewed and rejected).
    Rejected,
    /// Expired (not reviewed within the retention period).
    Expired,
}

impl ReviewStatus {
    /// Whether this memory is active (visible in search).
    pub fn is_active(&self) -> bool {
        matches!(self, ReviewStatus::Accepted | ReviewStatus::Pending)
    }

    /// String representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            ReviewStatus::Pending => "pending",
            ReviewStatus::Accepted => "accepted",
            ReviewStatus::Rejected => "rejected",
            ReviewStatus::Expired => "expired",
        }
    }

    /// Parse from string.
    pub fn parse_str(s: &str) -> Result<Self, String> {
        match s {
            "pending" => Ok(ReviewStatus::Pending),
            "accepted" => Ok(ReviewStatus::Accepted),
            "rejected" => Ok(ReviewStatus::Rejected),
            "expired" => Ok(ReviewStatus::Expired),
            _ => Err(format!("invalid review_status: {}", s)),
        }
    }
}

// ============================================================
// Visibility
// ============================================================

/// Memory visibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    /// Inherit space visibility.
    #[default]
    Inherit,
    /// Only space members can see.
    SpaceOnly,
    /// Only author/space owner.
    Private,
    /// Only selected users (memory_permissions table).
    Selected,
}

impl Visibility {
    /// String representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Visibility::Inherit => "inherit",
            Visibility::SpaceOnly => "space_only",
            Visibility::Private => "private",
            Visibility::Selected => "selected",
        }
    }

    /// Parse from string.
    pub fn parse_str(s: &str) -> Result<Self, String> {
        match s {
            "inherit" => Ok(Visibility::Inherit),
            "space_only" => Ok(Visibility::SpaceOnly),
            "private" => Ok(Visibility::Private),
            "selected" => Ok(Visibility::Selected),
            _ => Err(format!("invalid visibility: {}", s)),
        }
    }
}

// ============================================================
// Memory
// ============================================================

/// Memory — the core domain model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    /// Unique identifier (prefixed UUID v4).
    pub id: String,
    /// Space ID for multi-tenant isolation.
    pub space_id: String,
    /// Memory content text.
    pub content: String,
    /// Embedding vector (not serialized in API responses).
    #[serde(skip)]
    pub embedding: Option<Vec<f32>>,
    /// Embedding model name.
    pub embedding_model: String,
    /// Provenance marker.
    pub provenance: Provenance,
    /// Provenance metadata (JSON: source, author, model, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provenance_meta: Option<serde_json::Value>,
    /// Trust level (0.0 ~ 1.0).
    pub trust_level: TrustLevel,
    /// Review status.
    pub review_status: ReviewStatus,
    /// Memory visibility.
    pub visibility: Visibility,
    /// For version snapshots, points to the root memory.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_of: Option<String>,
    /// Version sequence number (0 = original).
    pub version_seq: i64,
    /// Author user ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_id: Option<String>,
    /// Parent conflict memory ID (if this is a conflict resolution).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_conflict_id: Option<String>,
    /// Last accessed timestamp (Unix seconds).
    pub last_accessed_at: Option<i64>,
    /// Access count.
    pub access_count: i64,
    /// Creation timestamp (Unix seconds).
    pub created_at: i64,
    /// Last update timestamp (Unix seconds).
    pub updated_at: i64,
}

impl Memory {
    /// Create a new Memory with defaults based on provenance.
    pub fn new(space_id: String, content: String, provenance: Provenance) -> Self {
        let now = crate::now_ts();
        let trust_level = TrustLevel::new(provenance.default_trust()).unwrap_or_default();
        let review_status = provenance.default_review_status();
        Self {
            id: crate::generate_id("mem"),
            space_id,
            content,
            embedding: None,
            embedding_model: "all-MiniLM-L6-v2".to_string(),
            provenance,
            provenance_meta: None,
            trust_level,
            review_status,
            visibility: Visibility::Inherit,
            version_of: None,
            version_seq: 0,
            author_id: None,
            parent_conflict_id: None,
            last_accessed_at: None,
            access_count: 0,
            created_at: now,
            updated_at: now,
        }
    }
}

// ============================================================
// Request DTOs
// ============================================================

/// Request to create a new memory.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateMemoryRequest {
    /// Space ID.
    pub space_id: String,
    /// Memory content.
    pub content: String,
    /// Provenance marker (default: human).
    #[serde(default = "Provenance::default")]
    pub provenance: Provenance,
    /// Optional trust level override.
    pub trust_level: Option<f32>,
    /// Optional provenance metadata.
    pub provenance_meta: Option<serde_json::Value>,
    /// Optional review status override.
    pub review_status: Option<ReviewStatus>,
    /// Optional visibility override.
    pub visibility: Option<Visibility>,
}

impl CreateMemoryRequest {
    /// Validate the request.
    pub fn validate(&self) -> Result<(), AppError> {
        if self.content.trim().is_empty() {
            return Err(AppError::bad_request("content must not be empty"));
        }
        if self.space_id.trim().is_empty() {
            return Err(AppError::bad_request("space_id must not be empty"));
        }
        if let Some(trust) = self.trust_level {
            if !(0.0..=1.0).contains(&trust) {
                return Err(AppError::bad_request(
                    "trust_level must be between 0.0 and 1.0",
                ));
            }
        }
        Ok(())
    }
}

/// Request to update trust level.
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateTrustRequest {
    /// New trust level (0.0 ~ 1.0).
    pub trust_level: f32,
    /// Optional reason for the change.
    pub reason: Option<String>,
}

impl UpdateTrustRequest {
    /// Validate the request.
    pub fn validate(&self) -> Result<(), AppError> {
        if !(0.0..=1.0).contains(&self.trust_level) {
            return Err(AppError::bad_request(
                "trust_level must be between 0.0 and 1.0",
            ));
        }
        Ok(())
    }
}

/// Request to update memory visibility.
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateVisibilityRequest {
    pub visibility: Visibility,
}

/// Request to save a new version.
#[derive(Debug, Clone, Deserialize)]
pub struct SaveVersionRequest {
    pub content: String,
    pub edit_summary: Option<String>,
}

/// Request to resolve a conflict.
#[derive(Debug, Clone, Deserialize)]
pub struct ConflictResolutionRequest {
    pub resolution: ConflictResolution,
    pub content: Option<String>,
}

/// Conflict resolution strategy.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictResolution {
    Mine,
    Theirs,
    Merge,
}

/// Request to recall context.
#[derive(Debug, Clone, Deserialize)]
pub struct RecallRequest {
    /// Context text to find related memories for.
    pub context: String,
    /// Space ID.
    pub space_id: String,
    /// Maximum number of memories to return.
    #[serde(default = "default_recall_limit")]
    pub limit: usize,
}

fn default_recall_limit() -> usize {
    10
}
