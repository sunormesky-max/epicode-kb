//! ONNX embedding provider — uses ONNX Runtime for real inference.
//!
//! TODO: This module is currently a stub because the `ort`, `tokenizers`, and `ndarray`
//! crates are not included in the default build. To enable real ONNX embedding:
//!
//! 1. Add to Cargo.toml:
//!    ```toml
//!    ort = { version = "2.0.0-rc.10", features = ["download-binaries"] }
//!    tokenizers = "0.19"
//!    ndarray = "0.16"
//!    ```
//! 2. Uncomment the `OnnxEmbedder` implementation below.
//! 3. Update `embed::create_embedder()` to use `OnnxEmbedder` when the model file exists.

use crate::embed::EmbeddingProvider;
use crate::error::{AppError, AppResult};

// ============================================================
// RandomEmbedder — fallback when ONNX is not available
// ============================================================

/// Random embedding provider — generates deterministic pseudo-random vectors
/// based on a hash of the input text. This ensures the same text always
/// produces the same embedding (important for consistency in search).
pub struct RandomEmbedder {
    dim: usize,
}

impl RandomEmbedder {
    /// Create a new RandomEmbedder with the given dimensions.
    pub fn new(dim: usize) -> Self {
        Self { dim }
    }

    /// Generate a deterministic pseudo-random vector from text.
    fn hash_embedding(&self, text: &str) -> Vec<f32> {
        let mut result = vec![0.0f32; self.dim];
        let bytes = text.as_bytes();
        for (i, slot) in result.iter_mut().enumerate() {
            let byte_idx = i % bytes.len().max(1);
            let seed = (i as u32).wrapping_mul(2654435761)
                ^ (bytes[byte_idx.min(bytes.len().saturating_sub(1))] as u32)
                ^ (text.len() as u32).wrapping_mul(40503);
            // Simple LCG to generate a pseudo-random float in [-1, 1]
            let val = (seed % 2000) as f32 / 1000.0 - 1.0;
            *slot = val;
        }
        // L2 normalize
        let norm: f32 = result.iter().map(|v| v * v).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in &mut result {
                *v /= norm;
            }
        }
        result
    }
}

impl EmbeddingProvider for RandomEmbedder {
    fn embed(&self, text: &str) -> AppResult<Vec<f32>> {
        Ok(self.hash_embedding(text))
    }

    fn embed_batch(&self, texts: &[&str]) -> AppResult<Vec<Vec<f32>>> {
        Ok(texts.iter().map(|t| self.hash_embedding(t)).collect())
    }

    fn model_name(&self) -> &str {
        "random-embedding"
    }

    fn dimensions(&self) -> usize {
        self.dim
    }
}

// ============================================================
// OnnxEmbedder — real ONNX Runtime inference (TODO: enable when ort crate available)
// ============================================================

/// ONNX embedding provider using the `ort` crate.
///
/// TODO: Uncomment and implement when `ort`, `tokenizers`, and `ndarray` are added to Cargo.toml.
///
/// ```ignore
/// use ort::session::Session;
/// use ort::inputs;
/// use ndarray::Array2;
///
/// pub struct OnnxEmbedder {
///     session: Session,
///     tokenizer: tokenizers::Tokenizer,
///     dim: usize,
/// }
///
/// impl OnnxEmbedder {
///     pub fn new(model_path: &str, dim: usize) -> AppResult<Self> {
///         if !std::path::Path::new(model_path).exists() {
///             return Err(AppError::internal(format!(
///                 "ONNX model not found: {}", model_path
///             )));
///         }
///         let session = Session::builder()?
///             .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level1)?
///             .with_intra_threads(2)?
///             .commit_from_file(model_path)?;
///         let tokenizer_path = std::path::Path::new(model_path)
///             .parent()
///             .unwrap_or(std::path::Path::new("."))
///             .join("tokenizer.json");
///         let tokenizer = tokenizers::Tokenizer::from_file(&tokenizer_path)
///             .map_err(|e| AppError::internal(format!("tokenizer load error: {}", e)))?;
///         Ok(Self { session, tokenizer, dim })
///     }
/// }
///
/// impl EmbeddingProvider for OnnxEmbedder {
///     fn embed(&self, text: &str) -> AppResult<Vec<f32>> {
///         let encoding = self.tokenizer.encode(text, true)
///             .map_err(|e| AppError::internal(format!("tokenize error: {}", e)))?;
///         let input_ids = encoding.get_ids();
///         let attention_mask = encoding.get_attention_mask();
///         let token_type_ids = encoding.get_type_ids();
///         // ... ONNX inference + mean pooling ...
///         todo!("implement when ort crate is available")
///     }
///
///     fn embed_batch(&self, texts: &[&str]) -> AppResult<Vec<Vec<f32>>> {
///         texts.iter().map(|t| self.embed(t)).collect()
///     }
///
///     fn model_name(&self) -> &str {
///         "all-MiniLM-L6-v2"
///     }
///
///     fn dimensions(&self) -> usize {
///         self.dim
///     }
/// }
/// ```
///
/// Placeholder type for documentation purposes.
pub struct OnnxEmbedder;

impl OnnxEmbedder {
    /// TODO: Implement when `ort` crate is available.
    pub fn new(_model_path: &str, _dim: usize) -> AppResult<Self> {
        Err(AppError::not_implemented(
            "ONNX embedding requires the `ort` crate (not compiled in). \
             Using RandomEmbedder as fallback.",
        ))
    }
}
