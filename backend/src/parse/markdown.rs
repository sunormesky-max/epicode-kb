//! Markdown parser — uses pulldown-cmark to parse and chunk by headings/paragraphs.

use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};

use crate::error::AppResult;
use crate::parse::{DocumentParser, FileType, MemoryChunk};

/// Maximum chunk size in characters.
const MAX_CHUNK_SIZE: usize = 512;

/// Markdown document parser.
pub struct MarkdownParser;

impl DocumentParser for MarkdownParser {
    fn parse(&self, content: &[u8]) -> AppResult<Vec<MemoryChunk>> {
        let text = std::str::from_utf8(content)
            .map_err(|e| crate::error::AppError::bad_request(format!("invalid UTF-8: {}", e)))?;

        let parser = Parser::new(text);
        let mut chunks: Vec<MemoryChunk> = Vec::new();
        let mut current_text = String::new();
        let mut current_heading: Option<String> = None;
        let mut in_heading = false;

        for event in parser {
            match event {
                Event::Start(Tag::Heading { level, .. }) => {
                    // Flush current chunk if non-empty
                    if !current_text.trim().is_empty() {
                        self.push_chunk(&mut chunks, &mut current_text, &current_heading);
                    }
                    in_heading = true;
                    current_heading = Some(match level {
                        HeadingLevel::H1 => "# ".to_string(),
                        HeadingLevel::H2 => "## ".to_string(),
                        HeadingLevel::H3 => "### ".to_string(),
                        HeadingLevel::H4 => "#### ".to_string(),
                        HeadingLevel::H5 => "##### ".to_string(),
                        HeadingLevel::H6 => "###### ".to_string(),
                    });
                }
                Event::End(TagEnd::Heading(_)) => {
                    in_heading = false;
                }
                Event::Start(Tag::Paragraph)
                | Event::Start(Tag::List(_))
                | Event::Start(Tag::CodeBlock(_)) => {
                    // Just continue accumulating text
                }
                Event::End(TagEnd::Paragraph)
                | Event::End(TagEnd::List(_))
                | Event::End(TagEnd::CodeBlock) => {
                    // Add a paragraph break
                    if !current_text.is_empty() && !current_text.ends_with('\n') {
                        current_text.push('\n');
                    }
                }
                Event::Text(t) => {
                    if in_heading {
                        if let Some(ref mut h) = current_heading {
                            h.push_str(&t);
                        }
                    }
                    current_text.push_str(&t);
                }
                Event::Code(t) => {
                    current_text.push_str(&t);
                }
                Event::SoftBreak | Event::HardBreak => {
                    current_text.push('\n');
                }
                _ => {}
            }

            // Split if chunk gets too large
            if current_text.len() >= MAX_CHUNK_SIZE {
                self.push_chunk(&mut chunks, &mut current_text, &current_heading);
            }
        }

        // Flush remaining text
        if !current_text.trim().is_empty() {
            self.push_chunk(&mut chunks, &mut current_text, &current_heading);
        }

        if chunks.is_empty() {
            chunks.push(MemoryChunk {
                content: text.trim().to_string(),
                chunk_index: 0,
                metadata: None,
            });
        }

        Ok(chunks)
    }

    fn supported_types(&self) -> Vec<FileType> {
        vec![FileType::Markdown]
    }
}

impl MarkdownParser {
    /// Push the current text as a chunk, splitting if necessary.
    fn push_chunk(
        &self,
        chunks: &mut Vec<MemoryChunk>,
        text: &mut String,
        heading: &Option<String>,
    ) {
        let trimmed = text.trim().to_string();
        if trimmed.is_empty() {
            text.clear();
            return;
        }

        // If the text is too long, split by sentences or fixed size
        if trimmed.len() > MAX_CHUNK_SIZE {
            let parts = Self::split_long_text(&trimmed, MAX_CHUNK_SIZE);
            for part in parts {
                let metadata = heading
                    .as_ref()
                    .map(|h| serde_json::json!({ "heading": h }));
                chunks.push(MemoryChunk {
                    content: part,
                    chunk_index: chunks.len(),
                    metadata,
                });
            }
        } else {
            let metadata = heading
                .as_ref()
                .map(|h| serde_json::json!({ "heading": h }));
            chunks.push(MemoryChunk {
                content: trimmed,
                chunk_index: chunks.len(),
                metadata,
            });
        }
        text.clear();
    }

    /// Split a long text into chunks, trying to break at sentence boundaries.
    fn split_long_text(text: &str, max_size: usize) -> Vec<String> {
        let mut result = Vec::new();
        let mut current = String::new();

        for sentence in text.split_inclusive(['.', '!', '?']) {
            if current.len() + sentence.len() > max_size && !current.is_empty() {
                result.push(current.trim().to_string());
                current.clear();
            }
            current.push_str(sentence);
        }

        if !current.trim().is_empty() {
            // If still too long, split by fixed size
            if current.len() > max_size {
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
