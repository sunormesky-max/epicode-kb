//! MemoryService — business logic for memory CRUD, trust adjustment, and adopt/reject.

use std::sync::Arc;

use rusqlite::Connection;
use tantivy::{IndexWriter, TantivyDocument};

use crate::db::repository::MemoryRepo;
use crate::embed::EmbeddingProvider;
use crate::error::{AppError, AppResult};
use crate::memory::model::{
    CreateMemoryRequest, Memory, Provenance, ReviewStatus, TrustLevel, UpdateTrustRequest,
};
use crate::search::fulltext::TantivySchema;

/// Memory service — orchestrates memory lifecycle.
pub struct MemoryService {
    db: Arc<std::sync::Mutex<Connection>>,
    embedder: Arc<dyn EmbeddingProvider>,
    tantivy_writer: Arc<std::sync::Mutex<IndexWriter>>,
    tantivy_schema: TantivySchema,
}

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
        }
    }

    /// Create a new memory: build struct → generate embedding → write to DB → index in Tantivy.
    pub fn create(&self, req: CreateMemoryRequest) -> AppResult<Memory> {
        req.validate()?;

        let mut memory = Memory::new(req.space_id, req.content, req.provenance);

        // Override trust_level if provided
        if let Some(trust) = req.trust_level {
            memory.trust_level = TrustLevel::new(trust).map_err(AppError::bad_request)?;
        }

        // Override review_status if provided
        if let Some(status) = req.review_status {
            memory.review_status = status;
        }

        // Set provenance metadata
        memory.provenance_meta = req.provenance_meta;

        // Generate embedding
        match self.embedder.embed(&memory.content) {
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
            let conn = self.db.lock().unwrap();
            MemoryRepo::insert(&conn, &memory)?;
        }

        // Index in Tantivy
        self.index_fulltext(&memory)?;

        tracing::info!(
            "Created memory: id={}, space={}",
            memory.id,
            memory.space_id
        );
        Ok(memory)
    }

    /// Get a memory by ID (updates access count).
    pub fn get_by_id(&self, id: &str) -> AppResult<Memory> {
        let conn = self.db.lock().unwrap();
        let memory = MemoryRepo::get_by_id(&conn, id)?;
        MemoryRepo::update_access(&conn, &[id.to_string()], crate::now_ts())?;
        Ok(memory)
    }

    /// List memories with filters and pagination.
    pub fn list(
        &self,
        space_id: &str,
        provenance: Option<&[Provenance]>,
        min_trust: Option<f32>,
        review_status: Option<ReviewStatus>,
        limit: usize,
        offset: usize,
    ) -> AppResult<(Vec<Memory>, usize)> {
        let conn = self.db.lock().unwrap();
        MemoryRepo::list(
            &conn,
            space_id,
            provenance,
            min_trust,
            review_status,
            limit,
            offset,
        )
    }

    /// Update trust level for a memory.
    pub fn update_trust(&self, id: &str, req: &UpdateTrustRequest) -> AppResult<Memory> {
        req.validate()?;

        let now = crate::now_ts();
        {
            let conn = self.db.lock().unwrap();
            MemoryRepo::update_trust(&conn, id, req.trust_level, now)?;
        }

        // Re-index in Tantivy (delete old, add new with updated trust)
        let memory = self.get_by_id(id)?;
        self.reindex_fulltext(&memory)?;

        tracing::info!("Updated trust: id={}, trust={:.2}", id, req.trust_level);
        Ok(memory)
    }

    /// Adopt an AI memory (accept pending review, boost trust).
    pub fn adopt(&self, id: &str) -> AppResult<Memory> {
        let now = crate::now_ts();
        {
            let conn = self.db.lock().unwrap();
            // Check current memory exists and is pending
            let memory = MemoryRepo::get_by_id(&conn, id)?;
            if memory.review_status != ReviewStatus::Pending {
                return Err(AppError::bad_request(format!(
                    "memory {} is not pending review (current: {})",
                    id,
                    memory.review_status.as_str()
                )));
            }
            // Set to accepted and boost trust
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

        tracing::info!("Adopted AI memory: id={}", id);
        Ok(memory)
    }

    /// Reject an AI memory (set to rejected).
    pub fn reject(&self, id: &str) -> AppResult<Memory> {
        let now = crate::now_ts();
        {
            let conn = self.db.lock().unwrap();
            let memory = MemoryRepo::get_by_id(&conn, id)?;
            if memory.review_status != ReviewStatus::Pending {
                return Err(AppError::bad_request(format!(
                    "memory {} is not pending review (current: {})",
                    id,
                    memory.review_status.as_str()
                )));
            }
            MemoryRepo::update_review_status(&conn, id, ReviewStatus::Rejected, None, now)?;
        }

        // Remove from Tantivy index (rejected memories should not be searchable)
        self.delete_from_fulltext(id)?;

        let memory = self.get_by_id(id)?;
        tracing::info!("Rejected AI memory: id={}", id);
        Ok(memory)
    }

    // ============================================================
    // Private helpers
    // ============================================================

    /// Index a memory in the Tantivy full-text index.
    fn index_fulltext(&self, memory: &Memory) -> AppResult<()> {
        let mut writer = self.tantivy_writer.lock().unwrap();
        let schema = &self.tantivy_schema;
        let mut doc = TantivyDocument::new();
        doc.add_text(schema.memory_id, &memory.id);
        doc.add_text(schema.space_id, &memory.space_id);
        doc.add_text(schema.content, &memory.content);
        doc.add_text(schema.provenance, memory.provenance.as_str());
        doc.add_f64(schema.trust_level, memory.trust_level.value() as f64);
        doc.add_i64(schema.created_at, memory.created_at);
        writer.add_document(doc)?;
        writer.commit()?;
        Ok(())
    }

    /// Re-index a memory (delete old, add new).
    fn reindex_fulltext(&self, memory: &Memory) -> AppResult<()> {
        let mut writer = self.tantivy_writer.lock().unwrap();
        let schema = &self.tantivy_schema;
        let term = tantivy::Term::from_field_text(schema.memory_id, &memory.id);
        writer.delete_term(term);
        let mut doc = TantivyDocument::new();
        doc.add_text(schema.memory_id, &memory.id);
        doc.add_text(schema.space_id, &memory.space_id);
        doc.add_text(schema.content, &memory.content);
        doc.add_text(schema.provenance, memory.provenance.as_str());
        doc.add_f64(schema.trust_level, memory.trust_level.value() as f64);
        doc.add_i64(schema.created_at, memory.created_at);
        writer.add_document(doc)?;
        writer.commit()?;
        Ok(())
    }

    /// Delete a memory from the Tantivy index.
    fn delete_from_fulltext(&self, id: &str) -> AppResult<()> {
        let mut writer = self.tantivy_writer.lock().unwrap();
        let term = tantivy::Term::from_field_text(self.tantivy_schema.memory_id, id);
        writer.delete_term(term);
        writer.commit()?;
        Ok(())
    }
}
