//! Full-text search using Tantivy.

use std::path::Path;
use std::sync::Arc;

use rusqlite::Connection;
use tantivy::{
    collector::TopDocs,
    query::QueryParser,
    schema::{Field, Schema, Value, INDEXED, STORED, STRING, TEXT},
    Index, IndexReader, ReloadPolicy, TantivyDocument, Term,
};

use crate::db::repository::MemoryRepo;
use crate::error::{AppError, AppResult};
use crate::memory::model::Memory;

/// Tantivy schema field handles.
#[derive(Debug, Clone)]
pub struct TantivySchema {
    pub memory_id: Field,
    pub space_id: Field,
    pub content: Field,
    pub provenance: Field,
    pub trust_level: Field,
    pub created_at: Field,
}

impl TantivySchema {
    /// Extract field handles from an existing index schema.
    pub fn from_index(index: &Index) -> Self {
        let schema = index.schema();
        Self {
            memory_id: schema
                .get_field("memory_id")
                .expect("missing field: memory_id"),
            space_id: schema
                .get_field("space_id")
                .expect("missing field: space_id"),
            content: schema.get_field("content").expect("missing field: content"),
            provenance: schema
                .get_field("provenance")
                .expect("missing field: provenance"),
            trust_level: schema
                .get_field("trust_level")
                .expect("missing field: trust_level"),
            created_at: schema
                .get_field("created_at")
                .expect("missing field: created_at"),
        }
    }
}

/// Create the Tantivy schema for memory indexing.
pub fn create_schema() -> Schema {
    let mut builder = Schema::builder();
    builder.add_text_field("memory_id", STRING | STORED);
    builder.add_text_field("space_id", STRING | STORED);
    builder.add_text_field("content", TEXT | STORED);
    builder.add_text_field("provenance", STRING | STORED);
    builder.add_f64_field("trust_level", STORED | INDEXED);
    builder.add_i64_field("created_at", STORED | INDEXED);
    builder.build()
}

/// Create or open a Tantivy index at the given path.
pub fn create_or_open_index(path: &str, schema: Schema) -> AppResult<Index> {
    let dir = Path::new(path);
    std::fs::create_dir_all(dir)?;

    let index = if dir.join("meta.json").exists() {
        Index::open_in_dir(dir)?
    } else {
        Index::create_in_dir(dir, schema)?
    };

    Ok(index)
}

/// Full-text searcher using Tantivy.
pub struct FulltextSearcher {
    index: Arc<Index>,
    reader: Arc<IndexReader>,
    schema: TantivySchema,
}

impl FulltextSearcher {
    /// Create a new FulltextSearcher.
    pub fn new(index: Arc<Index>, schema: TantivySchema) -> AppResult<Self> {
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;
        Ok(Self {
            index,
            reader: Arc::new(reader),
            schema,
        })
    }

    /// Search for memories matching the query text.
    /// Returns (memory_id, score) pairs.
    pub fn search(
        &self,
        query_text: &str,
        space_id: &str,
        limit: usize,
    ) -> AppResult<Vec<(String, f32)>> {
        let searcher = self.reader.searcher();
        let query_parser = QueryParser::for_index(&self.index, vec![self.schema.content]);

        let query = match query_parser.parse_query(query_text) {
            Ok(q) => q,
            Err(e) => {
                tracing::warn!("Tantivy query parse error: {}, trying escaped query", e);
                let escaped = query_text.replace(
                    [
                        '+', '-', '&', '|', '!', '(', ')', '{', '}', '[', ']', '^', '"', '~', '*',
                        '?', ':', '\\',
                    ],
                    " ",
                );
                query_parser
                    .parse_query(&escaped)
                    .map_err(|e2| AppError::TantivyQueryParse(e2.to_string()))?
            }
        };

        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;

        let mut results = Vec::new();
        for (score, doc_address) in top_docs {
            let doc: TantivyDocument = searcher.doc(doc_address)?;
            let memory_id = doc
                .get_first(self.schema.memory_id)
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let doc_space_id = doc
                .get_first(self.schema.space_id)
                .and_then(|v| v.as_str())
                .unwrap_or("");

            // Filter by space_id
            if doc_space_id == space_id {
                results.push((memory_id.to_string(), score));
            }
        }

        Ok(results)
    }

    /// Get memory IDs from Tantivy search results, filtered by space_id.
    pub fn search_and_filter(
        &self,
        query_text: &str,
        space_id: &str,
        limit: usize,
        conn: &Connection,
    ) -> AppResult<Vec<(Memory, f32)>> {
        let raw_results = self.search(query_text, space_id, limit)?;

        let mut memories = Vec::new();
        for (memory_id, score) in raw_results {
            match MemoryRepo::get_by_id(conn, &memory_id) {
                Ok(memory) => {
                    memories.push((memory, score));
                }
                Err(e) => {
                    tracing::warn!("Failed to load memory {} from DB: {}", memory_id, e);
                }
            }
        }

        Ok(memories)
    }

    /// Generate a highlighted snippet for the query.
    pub fn highlight(&self, query_text: &str, content: &str, max_chars: usize) -> Option<String> {
        if query_text.is_empty() {
            return None;
        }

        let query_lower = query_text.to_lowercase();
        let content_lower = content.to_lowercase();

        // Find the first match position
        let pos = content_lower.find(&query_lower)?;

        // Extract a window around the match
        let start = pos.saturating_sub(max_chars / 3);
        let end = (pos + query_lower.len() + max_chars * 2 / 3).min(content.len());

        let prefix = if start > 0 { "..." } else { "" };
        let suffix = if end < content.len() { "..." } else { "" };

        let window = &content[start..end];
        let window_lower = window.to_lowercase();

        // Highlight matching terms
        let mut result = String::new();
        let mut last_end = 0;
        let mut search_start = 0;
        while let Some(match_pos) = window_lower[search_start..].find(&query_lower) {
            let abs_pos = search_start + match_pos;
            result.push_str(&window[last_end..abs_pos]);
            result.push_str("<em>");
            result.push_str(&window[abs_pos..abs_pos + query_text.len()]);
            result.push_str("</em>");
            last_end = abs_pos + query_text.len();
            search_start = last_end;
        }
        result.push_str(&window[last_end..]);

        Some(format!("{}{}{}", prefix, result, suffix))
    }

    /// Get the schema handles.
    pub fn schema(&self) -> &TantivySchema {
        &self.schema
    }

    /// Delete a memory from the Tantivy index by its ID.
    pub fn delete_term_for_memory(&self, memory_id: &str) -> Term {
        Term::from_field_text(self.schema.memory_id, memory_id)
    }
}
