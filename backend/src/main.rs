//! epicode-kb server entry point.

use epicode_kb::{api, config::AppConfig, state::AppState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env file
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "epicode_kb=info,tower_http=info".into()),
        )
        .init();

    // Load configuration
    let config = AppConfig::from_env().into_arc();
    tracing::info!(
        "epicode-kb starting (version {})",
        env!("CARGO_PKG_VERSION")
    );
    tracing::info!("Listen address: {}", config.listen_addr);
    tracing::info!("Database: {}", config.database_url);
    tracing::info!("Tantivy path: {}", config.tantivy_path);
    tracing::info!("Embed dimensions: {}", config.embed_dimensions);

    // Initialize application state
    let state = AppState::new(config.clone()).await?;

    // Build router
    let app = api::routes::create_router(std::sync::Arc::new(state));

    // Start server
    let listener = tokio::net::TcpListener::bind(&config.listen_addr).await?;
    tracing::info!("Server listening on {}", config.listen_addr);

    axum::serve(listener, app).await?;

    Ok(())
}
