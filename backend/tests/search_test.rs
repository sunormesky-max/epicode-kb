//! Integration tests for search engine.

use epicode_kb::search::semantic::cosine_similarity;

#[test]
fn test_cosine_similarity_identical_vectors() {
    let a = vec![1.0, 0.0, 0.0];
    let b = vec![1.0, 0.0, 0.0];
    let sim = cosine_similarity(&a, &b);
    assert!((sim - 1.0).abs() < 1e-6);
}

#[test]
fn test_cosine_similarity_orthogonal_vectors() {
    let a = vec![1.0, 0.0];
    let b = vec![0.0, 1.0];
    let sim = cosine_similarity(&a, &b);
    assert!(sim.abs() < 1e-6);
}

#[test]
fn test_cosine_similarity_opposite_vectors() {
    let a = vec![1.0, 0.0];
    let b = vec![-1.0, 0.0];
    let sim = cosine_similarity(&a, &b);
    assert!((sim + 1.0).abs() < 1e-6);
}

#[test]
fn test_cosine_similarity_different_lengths() {
    let a = vec![1.0, 0.0, 0.0];
    let b = vec![1.0, 0.0];
    let sim = cosine_similarity(&a, &b);
    assert!(sim.abs() < 1e-6);
}

#[test]
fn test_cosine_similarity_empty_vectors() {
    let a: Vec<f32> = vec![];
    let b: Vec<f32> = vec![];
    let sim = cosine_similarity(&a, &b);
    assert!(sim.abs() < 1e-6);
}

#[test]
fn test_search_mode_from_str() {
    use epicode_kb::search::SearchMode;

    assert!(matches!(
        SearchMode::parse_str("semantic"),
        SearchMode::Semantic
    ));
    assert!(matches!(
        SearchMode::parse_str("fulltext"),
        SearchMode::Fulltext
    ));
    assert!(matches!(
        SearchMode::parse_str("hybrid"),
        SearchMode::Hybrid
    ));
    assert!(matches!(
        SearchMode::parse_str("invalid"),
        SearchMode::Hybrid
    )); // default
}

#[test]
fn test_file_type_detection() {
    use epicode_kb::parse::FileType;

    assert_eq!(FileType::from_extension("md"), Some(FileType::Markdown));
    assert_eq!(
        FileType::from_extension("markdown"),
        Some(FileType::Markdown)
    );
    assert_eq!(FileType::from_extension("txt"), Some(FileType::Text));
    assert_eq!(FileType::from_extension("pdf"), Some(FileType::Pdf));
    assert_eq!(FileType::from_extension("unknown"), None);

    assert_eq!(FileType::from_filename("test.md"), Some(FileType::Markdown));
    assert_eq!(FileType::from_filename("doc.PDF"), Some(FileType::Pdf));
    assert_eq!(FileType::from_filename("noext"), None);
}

#[test]
fn test_markdown_parser() {
    use epicode_kb::parse::{markdown::MarkdownParser, DocumentParser};

    let parser = MarkdownParser;
    let content = b"# Title\n\nFirst paragraph.\n\n## Subtitle\n\nSecond paragraph with more text.";

    let chunks = parser.parse(content).unwrap();
    assert!(chunks.len() >= 2);

    for chunk in &chunks {
        assert!(!chunk.content.is_empty());
        assert!(!chunk.content.contains("#")); // No markdown markers
    }
}

#[test]
fn test_text_parser() {
    use epicode_kb::parse::{text::TextParser, DocumentParser};

    let parser = TextParser;
    let content = b"First paragraph.\n\nSecond paragraph.\n\nThird paragraph with some content.";

    let chunks = parser.parse(content).unwrap();
    assert!(chunks.len() >= 3);

    for chunk in &chunks {
        assert!(!chunk.content.is_empty());
    }
}

#[test]
fn test_random_embedder() {
    use epicode_kb::embed::{onnx::RandomEmbedder, EmbeddingProvider};

    let embedder = RandomEmbedder::new(384);

    let vec1 = embedder.embed("hello world").unwrap();
    assert_eq!(vec1.len(), 384);

    // Same text should produce same embedding
    let vec2 = embedder.embed("hello world").unwrap();
    assert_eq!(vec1, vec2);

    // Different text should produce different embedding
    let vec3 = embedder.embed("goodbye world").unwrap();
    assert_ne!(vec1, vec3);

    // Batch
    let batch = embedder.embed_batch(&["hello", "world"]).unwrap();
    assert_eq!(batch.len(), 2);
    assert_eq!(batch[0].len(), 384);

    assert_eq!(embedder.dimensions(), 384);
    assert_eq!(embedder.model_name(), "random-embedding");
}

#[test]
fn test_generate_id() {
    let id1 = epicode_kb::generate_id("mem");
    let id2 = epicode_kb::generate_id("mem");

    assert!(id1.starts_with("mem_"));
    assert!(id2.starts_with("mem_"));
    assert_ne!(id1, id2); // UUIDs should be unique
}
