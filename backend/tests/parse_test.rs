//! Tests for document parsing and chunking.

use epicode_kb::parse::{
    docx::DocxParser, markdown::MarkdownParser, pdf::PdfParser, split_long_text, text::TextParser,
    DocumentParser, FileType,
};

#[test]
fn test_file_type_from_extension_variants() {
    assert_eq!(FileType::from_extension("md"), Some(FileType::Markdown));
    assert_eq!(FileType::from_extension("markdown"), Some(FileType::Markdown));
    assert_eq!(FileType::from_extension("txt"), Some(FileType::Text));
    assert_eq!(FileType::from_extension("pdf"), Some(FileType::Pdf));
    assert_eq!(FileType::from_extension("docx"), Some(FileType::Docx));
    assert_eq!(FileType::from_extension("exe"), None);
}

#[test]
fn test_file_type_from_filename_case_insensitive() {
    assert_eq!(FileType::from_filename("README.MD"), Some(FileType::Markdown));
    assert_eq!(FileType::from_filename("Report.PDF"), Some(FileType::Pdf));
    assert_eq!(FileType::from_filename("no_extension"), None);
}

#[test]
fn test_text_parser_produces_nonempty_chunks() {
    let content = b"First paragraph.\n\nSecond paragraph.\n\nThird paragraph with more words.";
    let chunks = TextParser.parse(content).unwrap();
    assert!(!chunks.is_empty());
    for chunk in &chunks {
        assert!(!chunk.content.is_empty());
        assert!(chunk.metadata.is_some());
    }
}

#[test]
fn test_markdown_parser_strips_markers() {
    let content = b"# Title\n\nFirst paragraph.\n\n## Section\n\nSecond paragraph.";
    let chunks = MarkdownParser.parse(content).unwrap();
    assert!(!chunks.is_empty());
    for chunk in &chunks {
        assert!(!chunk.content.is_empty());
        assert!(!chunk.content.starts_with('#'));
    }
}

#[test]
fn test_pdf_parser_ascii_fallback_without_feature() {
    // A minimal PDF header without real text content.
    let content = b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog >>\nendobj";
    let result = PdfParser.parse(content);
    // Without the `pdf` feature the ASCII fallback may yield empty text; the parser should
    // return an empty chunk set rather than panic.
    assert!(result.is_ok());
}

#[test]
fn test_docx_xml_fallback_without_feature() {
    // Minimal DOCX XML fragment; without the `docx` feature it falls back to XML text extraction.
    let content = b"<?xml version=\"1.0\"?><w:document><w:p><w:t>Hello world</w:t></w:p></w:document>";
    let result = DocxParser.parse(content);
    assert!(result.is_ok());
    let chunks = result.unwrap();
    assert!(!chunks.is_empty());
    assert!(chunks.iter().any(|c| c.content.contains("Hello world")));
}

#[test]
fn test_split_long_text_respects_chunk_size() {
    let text = "A. ".repeat(100);
    let chunks = split_long_text(&text, 50, 8);
    assert!(!chunks.is_empty());
    for chunk in &chunks {
        assert!(chunk.len() <= 50, "chunk length {} exceeds limit 50", chunk.len());
    }
}

#[test]
fn test_split_long_text_returns_at_least_one_chunk_for_empty() {
    let chunks = split_long_text("", 50, 8);
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0], "");
}
