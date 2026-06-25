//! LLM Provider abstraction layer.

pub mod deepseek;
pub mod ollama;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::config::AppConfig;
use crate::error::AppResult;

/// LLM provider trait.
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Synchronous completion.
    async fn complete(&self, req: &CompletionRequest) -> AppResult<CompletionResponse>;

    /// Streaming completion.
    async fn stream(&self, req: &CompletionRequest) -> AppResult<mpsc::Receiver<StreamChunk>>;

    /// Model name.
    fn model_name(&self) -> &str;
}

/// Completion request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    /// Model name (e.g., "deepseek-chat").
    pub model: String,
    /// Chat messages.
    pub messages: Vec<ChatMessage>,
    /// Temperature (0.0 ~ 2.0).
    pub temperature: Option<f32>,
    /// Maximum output tokens.
    pub max_tokens: Option<u32>,
}

/// A chat message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Role: system, user, or assistant.
    pub role: String,
    /// Message content.
    pub content: String,
}

impl ChatMessage {
    /// Create a system message.
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
        }
    }

    /// Create a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }

    /// Create an assistant message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
        }
    }
}

/// Completion response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    /// Generated content.
    pub content: String,
    /// Model used.
    pub model: String,
    /// Token usage.
    pub usage: TokenUsage,
}

/// Token usage information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Prompt tokens.
    pub prompt_tokens: u32,
    /// Completion tokens.
    pub completion_tokens: u32,
    /// Total tokens.
    pub total_tokens: u32,
}

/// A chunk in a streaming response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    /// Text content delta.
    pub content: String,
    /// Whether this is the final chunk.
    pub done: bool,
}

/// Get an LLM provider based on configuration.
pub fn get_provider(config: &AppConfig) -> Box<dyn LLMProvider> {
    if config.deepseek_api_key.is_some() {
        Box::new(deepseek::DeepSeekProvider::new(
            config.deepseek_api_key.clone().unwrap(),
            config.deepseek_base_url.clone(),
        ))
    } else {
        tracing::warn!("No LLM API key configured, using Ollama stub");
        Box::new(ollama::OllamaProvider::new())
    }
}
