//! DOCX parser — text extraction with docx-rs (when available) or plain text fallback.

use crate::error::{AppError, AppResult};
use crate::parse::{split_long_text, DocumentParser, FileType, MemoryChunk};

/// DOCX document parser.
pub struct DocxParser;

impl DocumentParser for DocxParser {
    fn parse_with_chunk_size(
        &self,
        content: &[u8],
        chunk_size: usize,
        chunk_overlap: usize,
    ) -> AppResult<Vec<MemoryChunk>> {
        #[cfg(feature = "docx")]
        {
            let text = extract_docx_text(content)?;
            Ok(self.chunk_text(&text, chunk_size, chunk_overlap))
        }

        #[cfg(not(feature = "docx"))]
        {
            // Without docx feature, attempt a simple XML text extraction fallback.
            let text = Self::extract_xml_text(content);
            if text.trim().is_empty() {
                return Err(AppError::not_implemented(
                    "DOCX parsing requires the `docx` feature. Enable it or add `docx-rs`.",
                ));
            }
            Ok(self.chunk_text(&text, chunk_size, chunk_overlap))
        }
    }

    fn supported_types(&self) -> Vec<FileType> {
        vec![FileType::Docx]
    }
}

impl DocxParser {
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
                    metadata: Some(serde_json::json!({"source": "docx"})),
                });
            }
        }

        if chunks.is_empty() && !text.trim().is_empty() {
            chunks.push(MemoryChunk {
                content: text.trim().to_string(),
                chunk_index: 0,
                metadata: Some(serde_json::json!({"source": "docx"})),
            });
        }

        chunks
    }

    /// Very basic XML text extraction fallback for DOCX.
    #[cfg(not(feature = "docx"))]
    fn extract_xml_text(content: &[u8]) -> String {
        let text = String::from_utf8_lossy(content);
        let mut result = String::new();
        let mut in_tag = false;
        let mut current = String::new();

        for c in text.chars() {
            match c {
                '<' => {
                    if !current.trim().is_empty() {
                        result.push_str(current.trim());
                        result.push(' ');
                    }
                    current.clear();
                    in_tag = true;
                }
                '>' => {
                    in_tag = false;
                }
                _ if !in_tag => current.push(c),
                _ => {}
            }
        }

        if !current.trim().is_empty() {
            result.push_str(current.trim());
        }

        result.split_whitespace().collect::<Vec<_>>().join(" ")
    }
}

#[cfg(feature = "docx")]
fn extract_docx_text(content: &[u8]) -> AppResult<String> {
    let docx = docx_rs::read_docx(content)
        .map_err(|e| AppError::internal(format!("DOCX read error: {}", e)))?;
    let mut text = String::new();
    for child in docx.document.children {
        if let docx_rs::DocumentChild::Paragraph(paragraph) = child {
            for run in paragraph.children {
                if let docx_rs::ParagraphChild::Run(r) = run {
                    for run_child in r.children {
                        if let docx_rs::RunChild::Text(t) = run_child {
                            text.push_str(&t.text);
                        }
                    }
                }
            }
            text.push('\n');
        }
    }
    Ok(text)
}
