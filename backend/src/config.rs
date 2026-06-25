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

    /// JWT secret for signing tokens.
    pub jwt_secret: String,
    /// JWT access token expiration in seconds.
    pub jwt_access_ttl: i64,
    /// JWT refresh token expiration in seconds.
    pub jwt_refresh_ttl: i64,

    /// API key hashing salt (hex encoded, optional).
    pub api_key_salt: Option<String>,
    /// Whether agent writes are globally enabled.
    pub agent_write_enabled: bool,

    /// DeepSeek API key (optional).
    pub deepseek_api_key: Option<String>,
    /// DeepSeek API base URL.
    pub deepseek_base_url: String,

    /// OIDC issuer URL (optional).
    pub oidc_issuer_url: Option<String>,
    /// OIDC client ID (optional).
    pub oidc_client_id: Option<String>,
    /// OIDC client secret (optional).
    pub oidc_client_secret: Option<String>,

    /// Default chunk size for document parsing.
    pub chunk_size: usize,
    /// Default chunk overlap for document parsing.
    pub chunk_overlap: usize,

    /// Conflict detection: semantic distance threshold.
    pub conflict_threshold: Option<f32>,
    /// Conflict detection: LLM confidence threshold.
    pub conflict_llm_confidence: Option<f32>,
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

            jwt_secret: std::env::var("EPICODE_KB_JWT_SECRET")
                .unwrap_or_else(|_| "change-me-in-production".to_string()),
            jwt_access_ttl: std::env::var("EPICODE_KB_JWT_ACCESS_TTL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3600),
            jwt_refresh_ttl: std::env::var("EPICODE_KB_JWT_REFRESH_TTL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(7 * 24 * 3600),

            api_key_salt: std::env::var("EPICODE_KB_API_KEY_SALT").ok(),
            agent_write_enabled: std::env::var("EPICODE_KB_AGENT_WRITE_ENABLED")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(true),

            deepseek_api_key: std::env::var("DEEPSEEK_API_KEY").ok(),
            deepseek_base_url: std::env::var("DEEPSEEK_BASE_URL")
                .unwrap_or_else(|_| "https://api.deepseek.com".to_string()),

            oidc_issuer_url: std::env::var("EPICODE_KB_OIDC_ISSUER_URL").ok(),
            oidc_client_id: std::env::var("EPICODE_KB_OIDC_CLIENT_ID").ok(),
            oidc_client_secret: std::env::var("EPICODE_KB_OIDC_CLIENT_SECRET").ok(),

            chunk_size: std::env::var("EPICODE_KB_CHUNK_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(512),
            chunk_overlap: std::env::var("EPICODE_KB_CHUNK_OVERLAP")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(64),
            conflict_threshold: std::env::var("EPICODE_KB_CONFLICT_THRESHOLD")
                .ok()
                .and_then(|v| v.parse().ok()),
            conflict_llm_confidence: std::env::var("EPICODE_KB_CONFLICT_LLM_CONFIDENCE")
                .ok()
                .and_then(|v| v.parse().ok()),
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
            jwt_secret: "change-me-in-production".to_string(),
            jwt_access_ttl: 3600,
            jwt_refresh_ttl: 7 * 24 * 3600,
            api_key_salt: None,
            agent_write_enabled: true,
            deepseek_api_key: None,
            deepseek_base_url: "https://api.deepseek.com".to_string(),
            oidc_issuer_url: None,
            oidc_client_id: None,
            oidc_client_secret: None,
            chunk_size: 512,
            chunk_overlap: 64,
            conflict_threshold: None,
            conflict_llm_confidence: None,
        }
    }
}
