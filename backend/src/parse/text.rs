//! Plain text parser — splits by double newlines, with size-based chunking.

use crate::error::AppResult;
use crate::parse::{DocumentParser, FileType, MemoryChunk};

/// Maximum chunk size in characters.
const MAX_CHUNK_SIZE: usize = 512;

/// Plain text document parser.
pub struct TextParser;

impl DocumentParser for TextParser {
    fn parse(&self, content: &[u8]) -> AppResult<Vec<MemoryChunk>> {
        let text = std::str::from_utf8(content)
            .map_err(|e| crate::error::AppError::bad_request(format!("invalid UTF-8: {}", e)))?;

        let mut chunks: Vec<MemoryChunk> = Vec::new();

        // Split by double newline (paragraph boundaries)
        for paragraph in text.split("\n\n") {
            let trimmed = paragraph.trim();
            if trimmed.is_empty() {
                continue;
            }

            // If paragraph is too long, split further
            if trimmed.len() > MAX_CHUNK_SIZE {
                for part in Self::split_long_text(trimmed, MAX_CHUNK_SIZE) {
                    chunks.push(MemoryChunk {
                        content: part,
                        chunk_index: chunks.len(),
                        metadata: None,
                    });
                }
            } else {
                chunks.push(MemoryChunk {
                    content: trimmed.to_string(),
                    chunk_index: chunks.len(),
                    metadata: None,
                });
            }
        }

        // If no chunks were created, use the full text
        if chunks.is_empty() {
            let full_text = text.trim().to_string();
            if !full_text.is_empty() {
                chunks.push(MemoryChunk {
                    content: full_text,
                    chunk_index: 0,
                    metadata: None,
                });
            }
        }

        Ok(chunks)
    }

    fn supported_types(&self) -> Vec<FileType> {
        vec![FileType::Text]
    }
}

impl TextParser {
    /// Split a long text into chunks, trying to break at sentence boundaries.
    fn split_long_text(text: &str, max_size: usize) -> Vec<String> {
        let mut result = Vec::new();
        let mut current = String::new();

        for sentence in text.split_inclusive(['.', '!', '?', '\n']) {
            if current.len() + sentence.len() > max_size && !current.is_empty() {
                result.push(current.trim().to_string());
                current.clear();
            }
            current.push_str(sentence);
        }

        if !current.trim().is_empty() {
            if current.len() > max_size {
                // Fallback: split by fixed byte size
                for chunk in current.as_bytes().chunks(max_size) {
                    if let Ok(s) = std::str::from_utf8(chunk) {
                        result.push(s.trim().to_string());
                    }
                }
            } else {
                result.push(current.trim().to_string());
            }
        }

        if result.is_empty() {
            result.push(text.trim().to_string());
        }

        result
    }
}
