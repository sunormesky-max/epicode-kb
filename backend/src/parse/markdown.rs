//! Markdown parser — uses pulldown-cmark to parse and chunk by headings/paragraphs.

use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};

use crate::error::AppResult;
use crate::parse::{split_long_text, DocumentParser, FileType, MemoryChunk};

/// Markdown document parser.
pub struct MarkdownParser;

impl DocumentParser for MarkdownParser {
    fn parse_with_chunk_size(
        &self,
        content: &[u8],
        chunk_size: usize,
        chunk_overlap: usize,
    ) -> AppResult<Vec<MemoryChunk>> {
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
                    if !current_text.trim().is_empty() {
                        self.push_chunk(
                            &mut chunks,
                            &mut current_text,
                            &current_heading,
                            chunk_size,
                            chunk_overlap,
                        );
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
                | Event::Start(Tag::CodeBlock(_)) => {}
                Event::End(TagEnd::Paragraph)
                | Event::End(TagEnd::List(_))
                | Event::End(TagEnd::CodeBlock) => {
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

            if current_text.len() >= chunk_size {
                self.push_chunk(
                    &mut chunks,
                    &mut current_text,
                    &current_heading,
                    chunk_size,
                    chunk_overlap,
                );
            }
        }

        if !current_text.trim().is_empty() {
            self.push_chunk(
                &mut chunks,
                &mut current_text,
                &current_heading,
                chunk_size,
                chunk_overlap,
            );
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
    fn push_chunk(
        &self,
        chunks: &mut Vec<MemoryChunk>,
        text: &mut String,
        heading: &Option<String>,
        chunk_size: usize,
        chunk_overlap: usize,
    ) {
        let trimmed = text.trim().to_string();
        if trimmed.is_empty() {
            text.clear();
            return;
        }

        let parts = if trimmed.len() > chunk_size {
            split_long_text(&trimmed, chunk_size, chunk_overlap)
        } else {
            vec![trimmed]
        };

        for part in parts {
            let metadata = heading.as_ref().map(|h| {
                serde_json::json!({
                    "source": "markdown",
                    "heading": h,
                })
            });
            chunks.push(MemoryChunk {
                content: part,
                chunk_index: chunks.len(),
                metadata,
            });
        }
        text.clear();
    }
}
