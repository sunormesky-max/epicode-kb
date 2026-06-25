//! Document parsing engine: trait + factory + parsers for Markdown/Text/PDF/DOCX.

pub mod docx;
pub mod markdown;
pub mod pdf;
pub mod text;

use crate::error::{AppError, AppResult};

/// Document parser trait — abstracts file parsing and chunking.
pub trait DocumentParser: Send + Sync {
    /// Parse file content into memory chunks using default chunk size.
    fn parse(&self, content: &[u8]) -> AppResult<Vec<MemoryChunk>> {
        self.parse_with_chunk_size(content, 512, 64)
    }

    /// Parse file content into memory chunks with configurable chunk size and overlap.
    fn parse_with_chunk_size(
        &self,
        content: &[u8],
        chunk_size: usize,
        chunk_overlap: usize,
    ) -> AppResult<Vec<MemoryChunk>>;

    /// Supported file types.
    fn supported_types(&self) -> Vec<FileType>;
}

/// A parsed chunk of content, ready to be converted into a Memory.
#[derive(Debug, Clone)]
pub struct MemoryChunk {
    /// Chunk text content.
    pub content: String,
    /// Chunk index (0-based).
    pub chunk_index: usize,
    /// Optional metadata (e.g., heading, page number).
    pub metadata: Option<serde_json::Value>,
}

/// Supported file types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    Markdown,
    Text,
    Pdf,
    Docx,
}

impl FileType {
    /// Detect file type from file extension.
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().trim_start_matches('.') {
            "md" | "markdown" => Some(FileType::Markdown),
            "txt" | "text" => Some(FileType::Text),
            "pdf" => Some(FileType::Pdf),
            "docx" => Some(FileType::Docx),
            _ => None,
        }
    }

    /// Detect file type from file name.
    pub fn from_filename(filename: &str) -> Option<Self> {
        filename.rsplit('.').next().and_then(Self::from_extension)
    }

    /// String representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            FileType::Markdown => "markdown",
            FileType::Text => "text",
            FileType::Pdf => "pdf",
            FileType::Docx => "docx",
        }
    }
}

/// Get a parser for the given file type.
pub fn get_parser(file_type: FileType) -> Box<dyn DocumentParser> {
    match file_type {
        FileType::Markdown => Box::new(markdown::MarkdownParser),
        FileType::Text => Box::new(text::TextParser),
        FileType::Pdf => Box::new(pdf::PdfParser),
        FileType::Docx => Box::new(docx::DocxParser),
    }
}

/// Get a parser from a filename, returning an error for unsupported types.
pub fn get_parser_for_file(filename: &str) -> AppResult<Box<dyn DocumentParser>> {
    let file_type = FileType::from_filename(filename)
        .ok_or_else(|| AppError::bad_request(format!("unsupported file type: {}", filename)))?;
    Ok(get_parser(file_type))
}

/// Split a long text into chunks, trying to break at sentence boundaries.
pub fn split_long_text(text: &str, chunk_size: usize, overlap: usize) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();

    for sentence in text.split_inclusive(['.', '!', '?', '\n']) {
        if current.len() + sentence.len() > chunk_size && !current.is_empty() {
            result.push(current.trim().to_string());
            let overlap_start = current.len().saturating_sub(overlap);
            current = current[overlap_start..].to_string();
        }
        current.push_str(sentence);
    }

    if !current.trim().is_empty() {
        if current.len() > chunk_size {
            for chunk in current.as_bytes().chunks(chunk_size) {
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
