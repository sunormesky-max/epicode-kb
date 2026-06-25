//! Ollama LLM provider — local deployment (stub for Sprint 2).

use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::error::{AppError, AppResult};
use crate::llm::{CompletionRequest, CompletionResponse, LLMProvider, StreamChunk};

/// Ollama local LLM provider.
/// TODO: Implement in Sprint 2 when Ollama integration is needed.
#[allow(dead_code)]
pub struct OllamaProvider {
    base_url: String,
}

impl OllamaProvider {
    /// Create a new OllamaProvider with default URL.
    pub fn new() -> Self {
        Self {
            base_url: "http://localhost:11434".to_string(),
        }
    }

    /// Create with custom base URL.
    pub fn with_base_url(base_url: String) -> Self {
        Self { base_url }
    }
}

impl Default for OllamaProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LLMProvider for OllamaProvider {
    async fn complete(&self, _req: &CompletionRequest) -> AppResult<CompletionResponse> {
        // TODO: Implement Ollama API call in Sprint 2.
        // POST {base_url}/api/chat with Ollama format
        Err(AppError::not_implemented(
            "Ollama provider not yet implemented (planned for Sprint 2)",
        ))
    }

    async fn stream(&self, _req: &CompletionRequest) -> AppResult<mpsc::Receiver<StreamChunk>> {
        // TODO: Implement SSE streaming in Sprint 2.
        Err(AppError::not_implemented(
            "Ollama streaming not yet implemented (planned for Sprint 2)",
        ))
    }

    fn model_name(&self) -> &str {
        "ollama"
    }
}
