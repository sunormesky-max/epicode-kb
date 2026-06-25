//! Plain text parser — splits by double newlines, with size-based chunking.

use crate::error::AppResult;
use crate::parse::{split_long_text, DocumentParser, FileType, MemoryChunk};

/// Plain text document parser.
pub struct TextParser;

impl DocumentParser for TextParser {
    fn parse_with_chunk_size(
        &self,
        content: &[u8],
        chunk_size: usize,
        chunk_overlap: usize,
    ) -> AppResult<Vec<MemoryChunk>> {
        let text = std::str::from_utf8(content)
            .map_err(|e| crate::error::AppError::bad_request(format!("invalid UTF-8: {}", e)))?;

        let mut chunks: Vec<MemoryChunk> = Vec::new();

        for paragraph in text.split("\n\n") {
            let trimmed = paragraph.trim();
            if trimmed.is_empty() {
                continue;
            }

            let parts = if trimmed.len() > chunk_size {
                split_long_text(trimmed, chunk_size, chunk_overlap)
            } else {
                vec![trimmed.to_string()]
            };

            for part in parts {
                chunks.push(MemoryChunk {
                    content: part,
                    chunk_index: chunks.len(),
                    metadata: Some(serde_json::json!({"source": "text"})),
                });
            }
        }

        if chunks.is_empty() {
            let full_text = text.trim().to_string();
            if !full_text.is_empty() {
                chunks.push(MemoryChunk {
                    content: full_text,
                    chunk_index: 0,
                    metadata: Some(serde_json::json!({"source": "text"})),
                });
            }
        }

        Ok(chunks)
    }

    fn supported_types(&self) -> Vec<FileType> {
        vec![FileType::Text]
    }
}
