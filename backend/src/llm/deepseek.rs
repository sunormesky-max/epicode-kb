//! DeepSeek LLM provider — HTTP API client.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::error::{AppError, AppResult};
use crate::llm::{
    ChatMessage, CompletionRequest, CompletionResponse, LLMProvider, StreamChunk, TokenUsage,
};

/// DeepSeek API provider.
pub struct DeepSeekProvider {
    api_key: String,
    base_url: String,
    client: reqwest::Client,
}

impl DeepSeekProvider {
    /// Create a new DeepSeekProvider.
    pub fn new(api_key: String, base_url: String) -> Self {
        Self {
            api_key,
            base_url,
            client: reqwest::Client::new(),
        }
    }
}

/// DeepSeek API request body.
#[derive(Debug, Serialize)]
struct DeepSeekRequestBody {
    model: String,
    messages: Vec<DeepSeekMessage>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    stream: bool,
}

/// DeepSeek API message format.
#[derive(Debug, Serialize, Deserialize)]
struct DeepSeekMessage {
    role: String,
    content: String,
}

impl From<&ChatMessage> for DeepSeekMessage {
    fn from(msg: &ChatMessage) -> Self {
        Self {
            role: msg.role.clone(),
            content: msg.content.clone(),
        }
    }
}

/// DeepSeek API response body.
#[derive(Debug, Deserialize)]
struct DeepSeekResponseBody {
    choices: Vec<DeepSeekChoice>,
    usage: DeepSeekUsage,
    model: String,
}

/// DeepSeek API choice.
#[derive(Debug, Deserialize)]
struct DeepSeekChoice {
    message: DeepSeekMessage,
}

/// DeepSeek API usage.
#[derive(Debug, Deserialize)]
struct DeepSeekUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[async_trait]
impl LLMProvider for DeepSeekProvider {
    async fn complete(&self, req: &CompletionRequest) -> AppResult<CompletionResponse> {
        let url = format!("{}/v1/chat/completions", self.base_url);

        let body = DeepSeekRequestBody {
            model: req.model.clone(),
            messages: req.messages.iter().map(DeepSeekMessage::from).collect(),
            temperature: req.temperature,
            max_tokens: req.max_tokens,
            stream: false,
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?
            .error_for_status()?;

        let resp_body: DeepSeekResponseBody = response.json().await?;

        let content = resp_body
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .unwrap_or_default();

        Ok(CompletionResponse {
            content,
            model: resp_body.model,
            usage: TokenUsage {
                prompt_tokens: resp_body.usage.prompt_tokens,
                completion_tokens: resp_body.usage.completion_tokens,
                total_tokens: resp_body.usage.total_tokens,
            },
        })
    }

    async fn stream(&self, _req: &CompletionRequest) -> AppResult<mpsc::Receiver<StreamChunk>> {
        // TODO: Implement SSE streaming when reqwest stream feature is enabled.
        // For now, return an error.
        Err(AppError::not_implemented(
            "LLM streaming not yet implemented (planned for Sprint 2)",
        ))
    }

    fn model_name(&self) -> &str {
        "deepseek-chat"
    }
}
