//! Common test helpers.

use std::path::PathBuf;
use std::sync::Arc;

use axum::{body::Body, Router};
use epicode_kb::{config::AppConfig, state::AppState};

/// Create a fresh `AppState` and router backed by temporary directories.
pub async fn create_test_app() -> (Router, Arc<AppState>, PathBuf) {
    let temp = std::env::temp_dir().join(format!("epicode_kb_test_{}", uuid::Uuid::new_v4().simple()));
    std::fs::create_dir_all(&temp).unwrap();

    let config = AppConfig {
        listen_addr: "127.0.0.1:0".to_string(),
        database_url: temp.join("test.db").to_string_lossy().to_string(),
        tantivy_path: temp.join("tantivy").to_string_lossy().to_string(),
        embed_model_path: temp.join("model.onnx").to_string_lossy().to_string(),
        embed_dimensions: 8,
        upload_dir: temp.join("uploads").to_string_lossy().to_string(),
        max_upload_size: 1024 * 1024,
        jwt_secret: "qa-test-secret-32bytes-long-key!!".to_string(),
        jwt_access_ttl: 3600,
        jwt_refresh_ttl: 7 * 24 * 3600,
        api_key_salt: Some("salt".to_string()),
        agent_write_enabled: true,
        deepseek_api_key: None,
        deepseek_base_url: "https://api.deepseek.com".to_string(),
        oidc_issuer_url: None,
        oidc_client_id: None,
        oidc_client_secret: None,
        chunk_size: 64,
        chunk_overlap: 8,
        conflict_threshold: None,
        conflict_llm_confidence: None,
    };

    let state = AppState::new(Arc::new(config)).await.unwrap();
    let state = Arc::new(state);
    let app = epicode_kb::api::routes::create_router(state.clone());
    (app, state, temp)
}

/// Build a JSON request body.
pub fn json_body(json: &str) -> Body {
    Body::from(json.to_string())
}
