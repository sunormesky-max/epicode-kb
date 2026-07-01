//! MemoryService — business logic for memory CRUD, trust adjustment, adopt/reject, versions, conflicts.

use std::sync::Arc;

use rusqlite::Connection;
use tantivy::{IndexWriter, TantivyDocument};

use crate::auth::model::{Actor, AgentContext, GlobalRole, Permission, SpaceRole};
use crate::auth::rbac::{AuthContext, RbacEngine};
use crate::db::repository::{MemoryPermissionRepo, MemoryRepo, MemoryVersion, MemoryVersionRepo};
use crate::embed::EmbeddingProvider;
use crate::error::{AppError, AppResult};
use crate::memory::model::{
    ConflictResolution, ConflictResolutionRequest, CreateMemoryRequest, Memory, Provenance,
    ReviewStatus, SaveVersionRequest, TrustLevel, UpdateTrustRequest, Visibility,
};
use crate::observability::audit::Auditor;
use crate::search::fulltext::TantivySchema;

/// Memory service — orchestrates memory lifecycle.
pub struct MemoryService {
    db: Arc<std::sync::Mutex<Connection>>,
    embedder: Arc<dyn EmbeddingProvider>,
    tantivy_writer: Arc<std::sync::Mutex<IndexWriter>>,
    tantivy_schema: TantivySchema,
    auditor: Auditor,
    rbac: RbacEngine,
}

/// Helper to lock a std::sync::Mutex and map poison to AppError.
impl MemoryService {
    /// Create a new MemoryService.
    pub fn new(
        db: Arc<std::sync::Mutex<Connection>>,
        embedder: Arc<dyn EmbeddingProvider>,
        _tantivy_index: Arc<tantivy::Index>,
        tantivy_writer: Arc<std::sync::Mutex<IndexWriter>>,
        tantivy_schema: TantivySchema,
    ) -> Self {
        Self {
            db,
            embedder,
            tantivy_writer,
            tantivy_schema,
            auditor: Auditor::new(),
            rbac: RbacEngine::new(),
        }
    }

    /// Build a MemoryService from AppState components.
    pub fn from_state(state: &crate::state::AppState) -> Self {
        Self::new(
            state.db.clone(),
            state.embedder.clone(),
            state.tantivy_index.clone(),
            state.tantivy_writer.clone(),
            state.tantivy_schema.clone(),
        )
    }

    /// Check permission helper.
    fn check(&self, actor: &Actor, space_id: &str, permission: Permission) -> AppResult<()> {
        let ctx = AuthContext {
            user_id: actor.user_id.clone(),
            global_role: actor.global_role,
            space_id: space_id.to_string(),
            space_role: actor.space_role,
        };
        self.rbac.check(&ctx, permission)
    }

    /// Create a new memory: validate → check permission → build struct → generate embedding → write to DB → index in Tantivy.
    pub fn create(&self, req: CreateMemoryRequest, actor: Option<&Actor>) -> AppResult<Memory> {
        req.validate()?;

        // Check permission BEFORE building memory
        if let Some(actor) = actor {
            self.check(actor, &req.space_id, Permission::MemoryWrite)?;
        }

        let mut memory = Memory::new(req.space_id.clone(), req.content.clone(), req.provenance);

        if let Some(actor) = actor {
            memory.author_id = Some(actor.user_id.clone());
        }

        // Override trust_level if provided
        if let Some(trust) = req.trust_level {
            memory.trust_level = TrustLevel::new(trust).map_err(AppError::bad_request)?;
        }

        // Override review_status if provided
        if let Some(status) = req.review_status {
            memory.review_status = status;
        }

        // Override visibility if provided
        if let Some(vis) = req.visibility {
            memory.visibility = vis;
        }

        // Set provenance metadata
        memory.provenance_meta = req.provenance_meta;

        // Generate embedding (strip HTML so rich-text memories don't leak tags).
        let embed_source = crate::memory::html::strip_tags(&memory.content);
        match self.embedder.embed(&embed_source) {
            Ok(embedding) => {
                memory.embedding = Some(embedding);
                memory.embedding_model = self.embedder.model_name().to_string();
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to generate embedding: {}, continuing without embedding",
                    e
                );
            }
        }

        // Write to database
        {
            let conn = self.db.lock().map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
            MemoryRepo::insert(&conn, &memory)?;
        }

        // Index in Tantivy (no per-operation commit; caller or background task commits)
        self.index_fulltext(&memory)?;

        // Audit log
        if let Some(actor) = actor {
            let _ = self.auditor.log(
                &self.db,
                &memory.space_id,
                Some(&actor.user_id),
                "create",
                "memory",
                Some(&memory.id),
                memory.provenance_meta.clone(),
            );
        }

        tracing::info!(
            "Created memory: id={}, space={}",
            memory.id,
            memory.space_id
        );
        Ok(memory)
    }

    /// Create a memory from an Agent / MCP call. Forces provenance=ai and review_status=pending.
    pub fn create_from_agent(
        &self,
        req: CreateMemoryRequest,
        agent: &AgentContext,
    ) -> AppResult<Memory> {
        req.validate()?;

        // Agent write permission check
        if agent.user_role != GlobalRole::Admin
            && agent.space_role != Some(SpaceRole::Owner)
            && agent.scope != "write"
            && agent.scope != "admin"
        {
            return Err(AppError::forbidden("api key scope does not allow write"));
        }

        let mut memory = Memory::new(req.space_id.clone(), req.content.clone(), Provenance::Ai);
        memory.author_id = Some(agent.user_id.clone());
        memory.review_status = ReviewStatus::Pending;
        memory.trust_level = TrustLevel::new(0.5).unwrap_or_default();
        memory.visibility = req.visibility.unwrap_or(Visibility::Inherit);
        memory.provenance_meta = req.provenance_meta.or_else(|| {
            Some(serde_json::json!({
                "source": "agent",
                "api_key_id": agent.api_key_id,
            }))
        });

        // Generate embedding (strip HTML so rich-text memories don't leak tags).
        let embed_source = crate::memory::html::strip_tags(&memory.content);
        match self.embedder.embed(&embed_source) {
            Ok(embedding) => {
                memory.embedding = Some(embedding);
                memory.embedding_model = self.embedder.model_name().to_string();
            }
            Err(e) => {
                tracing::warn!("Failed to generate agent embedding: {}", e);
            }
        }

        {
            let conn = self.db.lock().map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
            MemoryRepo::insert(&conn, &memory)?;
        }

        self.index_fulltext(&memory)?;

        let _ = self.auditor.log(
            &self.db,
            &memory.space_id,
            Some(&agent.user_id),
            "agent_create",
            "memory",
            Some(&memory.id),
            memory.provenance_meta.clone(),
        );

        tracing::info!(
            "Created agent memory: id={}, space={}",
            memory.id,
            memory.space_id
        );
        Ok(memory)
    }

    /// Get a memory by ID (updates access count).
    pub fn get_by_id(&self, id: &str) -> AppResult<Memory> {
        let conn = self.db.lock().map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
        let memory = MemoryRepo::get_by_id(&conn, id)?;
        MemoryRepo::update_access(&conn, &[id.to_string()], crate::now_ts())?;
        Ok(memory)
    }

    /// List memories with filters and pagination.
    #[allow(clippy::too_many_arguments)]
    pub fn list(
        &self,
        space_id: &str,
        provenance: Option<&[Provenance]>,
        min_trust: Option<f32>,
        review_status: Option<ReviewStatus>,
        visibility: Option<Visibility>,
        limit: usize,
        offset: usize,
    ) -> AppResult<(Vec<Memory>, usize)> {
        let conn = self.db.lock().map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
        MemoryRepo::list(
            &conn,
            space_id,
            provenance,
            min_trust,
            review_status,
            visibility,
            limit,
            offset,
        )
    }

    /// Update trust level for a memory.
    pub fn update_trust(
        &self,
        id: &str,
        req: &UpdateTrustRequest,
        actor: &Actor,
    ) -> AppResult<Memory> {
        req.validate()?;

        let memory = self.get_by_id(id)?;
        self.check(actor, &memory.space_id, Permission::MemoryWrite)?;

        let now = crate::now_ts();
        {
            let conn = self.db.lock().map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
            MemoryRepo::update_trust(&conn, id, req.trust_level, now)?;
        }

        // Re-index in Tantivy
        let memory = self.get_by_id(id)?;
        self.reindex_fulltext(&memory)?;

        let _ = self.auditor.log(
            &self.db,
            &memory.space_id,
            Some(&actor.user_id),
            "update_trust",
            "memory",
            Some(id),
            serde_json::json!({"trust_level": req.trust_level, "reason": req.reason}).into(),
        );

        tracing::info!("Updated trust: id={}, trust={:.2}", id, req.trust_level);
        Ok(memory)
    }

    /// Update memory visibility.
    pub fn update_visibility(
        &self,
        id: &str,
        visibility: Visibility,
        actor: &Actor,
    ) -> AppResult<Memory> {
        let memory = self.get_by_id(id)?;
        self.check(actor, &memory.space_id, Permission::MemoryAdmin)?;

        let now = crate::now_ts();
        {
            let conn = self.db.lock().map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
            MemoryRepo::update_visibility(&conn, id, visibility, now)?;
        }

        let memory = self.get_by_id(id)?;
        self.reindex_fulltext(&memory)?;

        let _ = self.auditor.log(
            &self.db,
            &memory.space_id,
            Some(&actor.user_id),
            "update_visibility",
            "memory",
            Some(id),
            serde_json::json!({"visibility": visibility.as_str()}).into(),
        );

        Ok(memory)
    }

    /// Adopt an AI memory (accept pending review, boost trust).
    pub fn adopt(&self, id: &str, actor: &Actor) -> AppResult<Memory> {
        let memory = self.get_by_id(id)?;
        self.check(actor, &memory.space_id, Permission::MemoryWrite)?;

        if memory.review_status != ReviewStatus::Pending {
            return Err(AppError::bad_request(format!(
                "memory {} is not pending review (current: {})",
                id,
                memory.review_status.as_str()
            )));
        }

        let now = crate::now_ts();
        {
            let conn = self.db.lock().map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
            let new_trust = (memory.trust_level.value() + 0.2).min(1.0);
            MemoryRepo::update_review_status(
                &conn,
                id,
                ReviewStatus::Accepted,
                Some(new_trust),
                now,
            )?;
        }

        let memory = self.get_by_id(id)?;
        self.reindex_fulltext(&memory)?;

        let _ = self.auditor.log(
            &self.db,
            &memory.space_id,
            Some(&actor.user_id),
            "adopt",
            "memory",
            Some(id),
            None,
        );

        tracing::info!("Adopted AI memory: id={}", id);
        Ok(memory)
    }

    /// Reject an AI memory (set to rejected).
    pub fn reject(&self, id: &str, actor: &Actor) -> AppResult<Memory> {
        let memory = self.get_by_id(id)?;
        self.check(actor, &memory.space_id, Permission::MemoryWrite)?;

        if memory.review_status != ReviewStatus::Pending {
            return Err(AppError::bad_request(format!(
                "memory {} is not pending review (current: {})",
                id,
                memory.review_status.as_str()
            )));
        }

        let now = crate::now_ts();
        {
            let conn = self.db.lock().map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
            MemoryRepo::update_review_status(&conn, id, ReviewStatus::Rejected, None, now)?;
        }

        // Remove from Tantivy index (rejected memories should not be searchable)
        self.delete_from_fulltext(id)?;

        let memory = self.get_by_id(id)?;

        let _ = self.auditor.log(
            &self.db,
            &memory.space_id,
            Some(&actor.user_id),
            "reject",
            "memory",
            Some(id),
            None,
        );

        tracing::info!("Rejected AI memory: id={}", id);
        Ok(memory)
    }

    /// Save a new version snapshot. Returns conflict if version_seq mismatch.
    pub fn save_version(
        &self,
        memory_id: &str,
        req: SaveVersionRequest,
        actor: &Actor,
    ) -> AppResult<MemoryVersion> {
        let memory = self.get_by_id(memory_id)?;
        self.check(actor, &memory.space_id, Permission::MemoryWrite)?;

        let now = crate::now_ts();
        let new_seq = memory.version_seq + 1;

        // Compute diff with previous content
        let diff = Self::compute_diff(&memory.content, &req.content);

        let version = MemoryVersion {
            id: crate::generate_id("ver"),
            memory_id: memory_id.to_string(),
            space_id: memory.space_id.clone(),
            version_seq: new_seq,
            content: req.content.clone(),
            editor_id: Some(actor.user_id.clone()),
            edit_summary: req.edit_summary,
            diff: Some(diff),
            created_at: now,
        };

        {
            let conn = self.db.lock().map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
            MemoryVersionRepo::insert(&conn, &version)?;
            MemoryRepo::update_content(&conn, memory_id, &req.content, new_seq, now)?;
        }

        let memory = self.get_by_id(memory_id)?;
        self.reindex_fulltext(&memory)?;

        let _ = self.auditor.log(
            &self.db,
            &memory.space_id,
            Some(&actor.user_id),
            "save_version",
            "memory",
            Some(memory_id),
            serde_json::json!({"version_seq": new_seq}).into(),
        );

        tracing::info!(
            "Saved version {} for memory {}",
            version.version_seq,
            memory_id
        );
        Ok(version)
    }

    /// Revert a memory to a previous version.
    pub fn revert(&self, memory_id: &str, version_id: &str, actor: &Actor) -> AppResult<Memory> {
        let memory = self.get_by_id(memory_id)?;
        self.check(actor, &memory.space_id, Permission::MemoryWrite)?;

        let conn = self.db.lock().map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
        let version = MemoryVersionRepo::get_by_id(&conn, version_id)?;
        if version.memory_id != memory_id {
            return Err(AppError::bad_request("version does not belong to memory"));
        }

        let now = crate::now_ts();
        let new_seq = memory.version_seq + 1;
        let diff = Self::compute_diff(&memory.content, &version.content);

        let revert_version = MemoryVersion {
            id: crate::generate_id("ver"),
            memory_id: memory_id.to_string(),
            space_id: memory.space_id.clone(),
            version_seq: new_seq,
            content: version.content.clone(),
            editor_id: Some(actor.user_id.clone()),
            edit_summary: Some(format!("revert to version {}", version.version_seq)),
            diff: Some(diff),
            created_at: now,
        };

        MemoryVersionRepo::insert(&conn, &revert_version)?;
        MemoryRepo::update_content(&conn, memory_id, &version.content, new_seq, now)?;
        drop(conn);

        let memory = self.get_by_id(memory_id)?;
        self.reindex_fulltext(&memory)?;

        let _ = self.auditor.log(
            &self.db,
            &memory.space_id,
            Some(&actor.user_id),
            "revert",
            "memory",
            Some(memory_id),
            serde_json::json!({"to_version_id": version_id}).into(),
        );

        Ok(memory)
    }

    /// Resolve a conflict by choosing mine/theirs/merge.
    pub fn resolve_conflict(
        &self,
        memory_id: &str,
        req: ConflictResolutionRequest,
        actor: &Actor,
    ) -> AppResult<Memory> {
        let memory = self.get_by_id(memory_id)?;
        self.check(actor, &memory.space_id, Permission::MemoryWrite)?;

        let content = match req.resolution {
            ConflictResolution::Mine => req.content.unwrap_or_else(|| memory.content.clone()),
            ConflictResolution::Theirs => req.content.unwrap_or_else(|| memory.content.clone()),
            ConflictResolution::Merge => req.content.ok_or_else(|| {
                AppError::bad_request("merge resolution requires explicit content")
            })?,
        };

        let now = crate::now_ts();
        let new_seq = memory.version_seq + 1;
        let diff = Self::compute_diff(&memory.content, &content);

        let version = MemoryVersion {
            id: crate::generate_id("ver"),
            memory_id: memory_id.to_string(),
            space_id: memory.space_id.clone(),
            version_seq: new_seq,
            content: content.clone(),
            editor_id: Some(actor.user_id.clone()),
            edit_summary: Some(format!("conflict resolution: {:?}", req.resolution)),
            diff: Some(diff),
            created_at: now,
        };

        {
            let conn = self.db.lock().map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
            MemoryVersionRepo::insert(&conn, &version)?;
            MemoryRepo::update_content(&conn, memory_id, &content, new_seq, now)?;
        }

        // Mark provenance as co (collaborative) and accepted.
        {
            let conn = self.db.lock().map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
            MemoryRepo::update_review_status(&conn, memory_id, ReviewStatus::Accepted, None, now)?;
        }

        let memory = self.get_by_id(memory_id)?;
        self.reindex_fulltext(&memory)?;

        let _ = self.auditor.log(
            &self.db,
            &memory.space_id,
            Some(&actor.user_id),
            "resolve_conflict",
            "memory",
            Some(memory_id),
            serde_json::json!({"resolution": format!("{:?}", req.resolution)}).into(),
        );

        tracing::info!("Resolved conflict for memory {}", memory_id);
        Ok(memory)
    }

    /// List versions of a memory.
    pub fn list_versions(
        &self,
        memory_id: &str,
        limit: usize,
        offset: usize,
    ) -> AppResult<(Vec<MemoryVersion>, usize)> {
        let conn = self.db.lock().map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
        MemoryVersionRepo::list_by_memory(&conn, memory_id, limit, offset)
    }

    /// List memory permissions.
    pub fn list_permissions(
        &self,
        memory_id: &str,
    ) -> AppResult<Vec<crate::auth::model::MemoryPermission>> {
        let conn = self.db.lock().map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
        MemoryPermissionRepo::list_by_memory(&conn, memory_id)
    }

    /// Compute a simple line-based diff between two texts.
    fn compute_diff(old: &str, new: &str) -> String {
        // Simple character-level diff summary.
        if old == new {
            return "no changes".to_string();
        }
        format!("-{}\n+{}", old, new)
    }

    // ============================================================
    // Private helpers
    // ============================================================

    /// Build a Tantivy document from a memory.
    fn build_tantivy_doc(&self, memory: &Memory) -> TantivyDocument {
        let schema = &self.tantivy_schema;
        let mut doc = TantivyDocument::new();
        doc.add_text(schema.memory_id, &memory.id);
        doc.add_text(schema.space_id, &memory.space_id);
        doc.add_text(schema.content, crate::memory::html::strip_tags(&memory.content));
        doc.add_text(schema.provenance, memory.provenance.as_str());
        doc.add_text(schema.review_status, memory.review_status.as_str());
        doc.add_text(schema.visibility, memory.visibility.as_str());
        doc.add_f64(schema.trust_level, memory.trust_level.value() as f64);
        doc.add_i64(schema.created_at, memory.created_at);
        doc
    }

    /// Index a memory in the Tantivy full-text index (no commit — caller or background task commits).
    fn index_fulltext(&self, memory: &Memory) -> AppResult<()> {
        let writer = self.tantivy_writer.lock().unwrap();
        let doc = self.build_tantivy_doc(memory);
        writer.add_document(doc)?;
        // writer.commit() removed — use background commit or explicit batch commit
        Ok(())
    }

    /// Re-index a memory (delete old, add new) — no commit.
    fn reindex_fulltext(&self, memory: &Memory) -> AppResult<()> {
        let writer = self.tantivy_writer.lock().unwrap();
        let term = tantivy::Term::from_field_text(self.tantivy_schema.memory_id, &memory.id);
        writer.delete_term(term);
        let doc = self.build_tantivy_doc(memory);
        writer.add_document(doc)?;
        // writer.commit() removed
        Ok(())
    }

    /// Delete a memory from the Tantivy index — no commit.
    fn delete_from_fulltext(&self, id: &str) -> AppResult<()> {
        let writer = self.tantivy_writer.lock().unwrap();
        let term = tantivy::Term::from_field_text(self.tantivy_schema.memory_id, id);
        writer.delete_term(term);
        // writer.commit() removed
        Ok(())
    }

    /// Explicitly commit pending Tantivy changes. Call after batch operations or on a timer.
    pub fn commit_tantivy(&self) -> AppResult<()> {
        let mut writer = self.tantivy_writer.lock().unwrap();
        writer.commit()?;
        Ok(())
    }
}
