//! Application configuration loaded from environment variables.

use std::sync::Arc;

/// Application configuration.
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Server listen address.
    pub listen_addr: String,
    /// SQLite database file path.
    pub database_url: String,
    /// Tantivy index directory path.
    pub tantivy_path: String,
    /// ONNX embedding model file path.
    pub embed_model_path: String,
    /// Embedding vector dimensions.
    pub embed_dimensions: usize,
    /// File upload directory.
    pub upload_dir: String,
    /// Maximum upload size in bytes.
    pub max_upload_size: usize,
    /// DeepSeek API key (optional).
    pub deepseek_api_key: Option<String>,
    /// DeepSeek API base URL.
    pub deepseek_base_url: String,
}

impl AppConfig {
    /// Load configuration from environment variables with defaults.
    pub fn from_env() -> Self {
        Self {
            listen_addr: std::env::var("EPICODE_KB_LISTEN_ADDR")
                .unwrap_or_else(|_| "0.0.0.0:3000".to_string()),
            database_url: std::env::var("EPICODE_KB_DATABASE_URL")
                .unwrap_or_else(|_| "data/epicode_kb.db".to_string()),
            tantivy_path: std::env::var("EPICODE_KB_TANTIVY_PATH")
                .unwrap_or_else(|_| "data/tantivy".to_string()),
            embed_model_path: std::env::var("EPICODE_KB_EMBED_MODEL")
                .unwrap_or_else(|_| "models/all-MiniLM-L6-v2.onnx".to_string()),
            embed_dimensions: std::env::var("EPICODE_KB_EMBED_DIMENSIONS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(384),
            upload_dir: std::env::var("EPICODE_KB_UPLOAD_DIR")
                .unwrap_or_else(|_| "data/uploads".to_string()),
            max_upload_size: std::env::var("EPICODE_KB_MAX_UPLOAD_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10 * 1024 * 1024),
            deepseek_api_key: std::env::var("DEEPSEEK_API_KEY").ok(),
            deepseek_base_url: std::env::var("DEEPSEEK_BASE_URL")
                .unwrap_or_else(|_| "https://api.deepseek.com".to_string()),
        }
    }

    /// Wrap in Arc for shared ownership.
    pub fn into_arc(self) -> Arc<Self> {
        Arc::new(self)
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            listen_addr: "0.0.0.0:3000".to_string(),
            database_url: "data/epicode_kb.db".to_string(),
            tantivy_path: "data/tantivy".to_string(),
            embed_model_path: "models/all-MiniLM-L6-v2.onnx".to_string(),
            embed_dimensions: 384,
            upload_dir: "data/uploads".to_string(),
            max_upload_size: 10 * 1024 * 1024,
            deepseek_api_key: None,
            deepseek_base_url: "https://api.deepseek.com".to_string(),
        }
    }
}
