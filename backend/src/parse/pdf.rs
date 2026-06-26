//! PDF parser — text extraction with pdf-extract (when available) or ASCII fallback.

use crate::error::AppResult;
use crate::parse::{split_long_text, DocumentParser, FileType, MemoryChunk};

/// PDF document parser.
pub struct PdfParser;

impl DocumentParser for PdfParser {
    fn parse_with_chunk_size(
        &self,
        content: &[u8],
        chunk_size: usize,
        chunk_overlap: usize,
    ) -> AppResult<Vec<MemoryChunk>> {
        #[cfg(feature = "pdf")]
        {
            let text = extract_pdf_text(content)?;
            Ok(self.chunk_text(&text, chunk_size, chunk_overlap))
        }

        #[cfg(not(feature = "pdf"))]
        {
            let text = Self::extract_ascii_text(content);
            if text.trim().is_empty() {
                return Ok(vec![]);
            }
            Ok(self.chunk_text(&text, chunk_size, chunk_overlap))
        }
    }

    fn supported_types(&self) -> Vec<FileType> {
        vec![FileType::Pdf]
    }
}

impl PdfParser {
    fn chunk_text(&self, text: &str, chunk_size: usize, chunk_overlap: usize) -> Vec<MemoryChunk> {
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
                    metadata: Some(serde_json::json!({"source": "pdf"})),
                });
            }
        }

        if chunks.is_empty() && !text.trim().is_empty() {
            chunks.push(MemoryChunk {
                content: text.trim().to_string(),
                chunk_index: 0,
                metadata: Some(serde_json::json!({"source": "pdf"})),
            });
        }

        chunks
    }

    /// Extract readable ASCII text from PDF bytes (basic fallback).
    #[cfg(not(feature = "pdf"))]
    fn extract_ascii_text(content: &[u8]) -> String {
        let text = String::from_utf8_lossy(content);
        let mut result = String::new();
        let mut in_text = false;

        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("BT") {
                in_text = true;
            } else if trimmed.starts_with("ET") {
                in_text = false;
                result.push('\n');
            } else if in_text {
                let mut chars = trimmed.chars().peekable();
                while let Some(c) = chars.next() {
                    if c == '(' {
                        let mut txt = String::new();
                        let mut depth = 1;
                        for c2 in chars.by_ref() {
                            if c2 == '(' {
                                depth += 1;
                                txt.push(c2);
                            } else if c2 == ')' {
                                depth -= 1;
                                if depth == 0 {
                                    break;
                                }
                                txt.push(c2);
                            } else {
                                txt.push(c2);
                            }
                        }
                        if !txt.is_empty() {
                            result.push_str(&txt);
                            result.push(' ');
                        }
                    }
                }
            }
        }

        result
    }
}

#[cfg(feature = "pdf")]
fn extract_pdf_text(content: &[u8]) -> AppResult<String> {
    pdf_extract::extract_text_from_mem(content)
        .map_err(|e| AppError::internal(format!("PDF extract error: {}", e)))
}
