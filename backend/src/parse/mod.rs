//! Document parsing engine: trait + factory + parsers for Markdown/Text/PDF.

pub mod markdown;
pub mod pdf;
pub mod text;

use crate::error::{AppError, AppResult};

/// Document parser trait — abstracts file parsing and chunking.
pub trait DocumentParser: Send + Sync {
    /// Parse file content into memory chunks.
    fn parse(&self, content: &[u8]) -> AppResult<Vec<MemoryChunk>>;

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
}

impl FileType {
    /// Detect file type from file extension.
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().trim_start_matches('.') {
            "md" | "markdown" => Some(FileType::Markdown),
            "txt" | "text" => Some(FileType::Text),
            "pdf" => Some(FileType::Pdf),
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
        }
    }
}

/// Get a parser for the given file type.
pub fn get_parser(file_type: FileType) -> Box<dyn DocumentParser> {
    match file_type {
        FileType::Markdown => Box::new(markdown::MarkdownParser),
        FileType::Text => Box::new(text::TextParser),
        FileType::Pdf => Box::new(pdf::PdfParser),
    }
}

/// Get a parser from a filename, returning an error for unsupported types.
pub fn get_parser_for_file(filename: &str) -> AppResult<Box<dyn DocumentParser>> {
    let file_type = FileType::from_filename(filename)
        .ok_or_else(|| AppError::bad_request(format!("unsupported file type: {}", filename)))?;
    Ok(get_parser(file_type))
}
