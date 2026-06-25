//! Embedding engine: trait + ONNX (stub) + random fallback.

pub mod onnx;

use std::sync::Arc;

use crate::config::AppConfig;
use crate::error::AppResult;

/// Embedding provider trait — abstracts embedding generation.
pub trait EmbeddingProvider: Send + Sync {
    /// Embed a single text string.
    fn embed(&self, text: &str) -> AppResult<Vec<f32>>;

    /// Embed a batch of text strings.
    fn embed_batch(&self, texts: &[&str]) -> AppResult<Vec<Vec<f32>>>;

    /// Model name.
    fn model_name(&self) -> &str;

    /// Embedding vector dimensions.
    fn dimensions(&self) -> usize;
}

/// Create an embedder based on configuration.
/// Tries to load the ONNX model; falls back to random embeddings if unavailable.
pub fn create_embedder(config: &AppConfig) -> Arc<dyn EmbeddingProvider> {
    // Try ONNX embedder first
    // TODO: When `ort` and `tokenizers` crates are available, use OnnxEmbedder::new()
    // and check if the model file exists. For now, use RandomEmbedder as fallback.
    //
    // if std::path::Path::new(&config.embed_model_path).exists() {
    //     match onnx::OnnxEmbedder::new(&config.embed_model_path, config.embed_dimensions) {
    //         Ok(embedder) => {
    //             tracing::info!("Loaded ONNX embedding model: {}", config.embed_model_path);
    //             return Arc::new(embedder);
    //         }
    //         Err(e) => {
    //             tracing::warn!("Failed to load ONNX model: {}, falling back to random embeddings", e);
    //         }
    //     }
    // } else {
    //     tracing::warn!("ONNX model not found at {}, using random embeddings", config.embed_model_path);
    // }

    tracing::warn!(
        "ONNX embedding not available (ort crate not compiled in), using random embeddings. \
         Model path was: {}",
        config.embed_model_path
    );
    Arc::new(onnx::RandomEmbedder::new(config.embed_dimensions))
}
