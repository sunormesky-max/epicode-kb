//! PDF parser — basic text extraction.
//!
//! TODO: This module is currently a stub because the `pdf-extract` crate is not
//! included in the default build. To enable real PDF parsing:
//!
//! 1. Add to Cargo.toml:
//!    ```toml
//!    pdf-extract = "0.7"
//!    ```
//! 2. Uncomment the `extract_pdf_text` function below.
//! 3. Update `parse()` to use real extraction.

use crate::error::{AppError, AppResult};
use crate::parse::{DocumentParser, FileType, MemoryChunk};

/// Maximum chunk size in characters.
const MAX_CHUNK_SIZE: usize = 512;

/// PDF document parser.
pub struct PdfParser;

impl DocumentParser for PdfParser {
    fn parse(&self, content: &[u8]) -> AppResult<Vec<MemoryChunk>> {
        // TODO: Enable real PDF extraction when `pdf-extract` crate is available.
        //
        // let text = extract_pdf_text(content)?;
        // let mut chunks = Vec::new();
        // for paragraph in text.split("\n\n") {
        //     let trimmed = paragraph.trim();
        //     if trimmed.is_empty() { continue; }
        //     if trimmed.len() > MAX_CHUNK_SIZE {
        //         for part in split_long_text(trimmed, MAX_CHUNK_SIZE) {
        //             chunks.push(MemoryChunk {
        //                 content: part,
        //                 chunk_index: chunks.len(),
        //                 metadata: Some(serde_json::json!({ "source": "pdf" })),
        //             });
        //         }
        //     } else {
        //         chunks.push(MemoryChunk {
        //             content: trimmed.to_string(),
        //             chunk_index: chunks.len(),
        //             metadata: Some(serde_json::json!({ "source": "pdf" })),
        //         });
        //     }
        // }
        // Ok(chunks)

        // Stub: try to extract readable ASCII text as a fallback
        let text = Self::extract_ascii_text(content);
        if text.trim().is_empty() {
            return Err(AppError::not_implemented(
                "PDF parsing requires the `pdf-extract` crate (not compiled in). \
                 Enable the `pdf` feature or add `pdf-extract` to Cargo.toml.",
            ));
        }

        let mut chunks: Vec<MemoryChunk> = Vec::new();
        for paragraph in text.split("\n\n") {
            let trimmed = paragraph.trim();
            if trimmed.is_empty() {
                continue;
            }
            if trimmed.len() > MAX_CHUNK_SIZE {
                for part in Self::split_long_text(trimmed, MAX_CHUNK_SIZE) {
                    chunks.push(MemoryChunk {
                        content: part,
                        chunk_index: chunks.len(),
                        metadata: Some(
                            serde_json::json!({ "source": "pdf", "note": "ascii fallback" }),
                        ),
                    });
                }
            } else {
                chunks.push(MemoryChunk {
                    content: trimmed.to_string(),
                    chunk_index: chunks.len(),
                    metadata: Some(
                        serde_json::json!({ "source": "pdf", "note": "ascii fallback" }),
                    ),
                });
            }
        }

        if chunks.is_empty() {
            return Err(AppError::not_implemented(
                "PDF parsing requires the `pdf-extract` crate (not compiled in)",
            ));
        }

        Ok(chunks)
    }

    fn supported_types(&self) -> Vec<FileType> {
        vec![FileType::Pdf]
    }
}

impl PdfParser {
    /// Extract readable ASCII text from PDF bytes (basic fallback).
    /// This only extracts text between BT and ET markers in PDF streams.
    fn extract_ascii_text(content: &[u8]) -> String {
        // Very basic: extract text from PDF text objects (BT...ET)
        // This is a simplified fallback and won't work for all PDFs.
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
                // Look for text in parentheses (PDF string literals)
                let mut chars = trimmed.chars().peekable();
                while let Some(c) = chars.next() {
                    if c == '(' {
                        let mut text = String::new();
                        let mut depth = 1;
                        for c2 in chars.by_ref() {
                            if c2 == '(' {
                                depth += 1;
                                text.push(c2);
                            } else if c2 == ')' {
                                depth -= 1;
                                if depth == 0 {
                                    break;
                                }
                                text.push(c2);
                            } else {
                                text.push(c2);
                            }
                        }
                        if !text.is_empty() {
                            result.push_str(&text);
                            result.push(' ');
                        }
                    }
                }
            }
        }

        result
    }

    /// Split a long text into chunks.
    fn split_long_text(text: &str, max_size: usize) -> Vec<String> {
        let mut result = Vec::new();
        let mut current = String::new();

        for word in text.split_whitespace() {
            if current.len() + word.len() + 1 > max_size && !current.is_empty() {
                result.push(current.trim().to_string());
                current.clear();
            }
            current.push_str(word);
            current.push(' ');
        }

        if !current.trim().is_empty() {
            result.push(current.trim().to_string());
        }

        if result.is_empty() {
            result.push(text.trim().to_string());
        }

        result
    }
}

// TODO: Uncomment when `pdf-extract` crate is available.
// fn extract_pdf_text(content: &[u8]) -> AppResult<String> {
//     use std::io::Cursor;
//     let cursor = Cursor::new(content);
//     pdf_extract::extract_text_from_mem(cursor)
//         .map_err(|e| AppError::internal(format!("PDF extract error: {}", e)))
// }
