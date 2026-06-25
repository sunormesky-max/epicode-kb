//! Application state — shared resources for all handlers.

use std::sync::Arc;

use rusqlite::Connection;
use tantivy::{Index, IndexWriter};

use crate::auth::service::AuthService;
use crate::collab::room::RoomManager;
use crate::config::AppConfig;
use crate::db;
use crate::embed::{self, EmbeddingProvider};
use crate::error::AppResult;
use crate::observability::metrics::Metrics;
use crate::search::fulltext::{self, TantivySchema};
use crate::search::hybrid::HybridSearcher;
use crate::search::semantic::SemanticSearcher;

/// Application state shared across all Axum handlers.
#[derive(Clone)]
pub struct AppState {
    /// Application configuration.
    pub config: Arc<AppConfig>,
    /// SQLite database connection (Mutex for thread safety).
    pub db: Arc<std::sync::Mutex<Connection>>,
    /// Tantivy index for full-text search.
    pub tantivy_index: Arc<Index>,
    /// Tantivy index writer (Mutex for thread safety).
    pub tantivy_writer: Arc<std::sync::Mutex<IndexWriter>>,
    /// Tantivy schema field handles.
    pub tantivy_schema: TantivySchema,
    /// Embedding provider.
    pub embedder: Arc<dyn EmbeddingProvider>,
    /// Hybrid search engine.
    pub search_engine: Arc<HybridSearcher>,
    /// Authentication & API key service.
    pub auth_service: Arc<AuthService>,
    /// Collaboration room manager.
    pub room_manager: Arc<RoomManager>,
    /// Prometheus metrics.
    pub metrics: Arc<Metrics>,
}

impl AppState {
    /// Initialize all application state: database, Tantivy index, embedder, auth, metrics.
    pub async fn new(config: Arc<AppConfig>) -> AppResult<Self> {
        // Ensure directories exist
        std::fs::create_dir_all("data")?;
        std::fs::create_dir_all(&config.upload_dir)?;
        std::fs::create_dir_all(&config.tantivy_path)?;

        // Initialize database
        let conn = db::init_database(&config.database_url)?;
        db::ensure_default_space(&conn)?;
        let db_pool = Arc::new(std::sync::Mutex::new(conn));

        // Initialize Tantivy index
        let schema = fulltext::create_schema();
        let tantivy_index = Arc::new(fulltext::create_or_open_index(
            &config.tantivy_path,
            schema,
        )?);
        let tantivy_writer = Arc::new(std::sync::Mutex::new(tantivy_index.writer(50_000_000)?));
        let tantivy_schema = TantivySchema::from_index(&tantivy_index);

        // Initialize embedder (returns Arc<dyn EmbeddingProvider>)
        let embedder = embed::create_embedder(&config);

        // Initialize search engine
        let semantic = SemanticSearcher::new(db_pool.clone(), embedder.clone());
        let fulltext_searcher =
            fulltext::FulltextSearcher::new(tantivy_index.clone(), tantivy_schema.clone())?;
        let search_engine = Arc::new(HybridSearcher::new(
            db_pool.clone(),
            semantic,
            fulltext_searcher,
        ));

        // Initialize authentication service
        let auth_service = Arc::new(AuthService::new(db_pool.clone(), config.clone()));

        // Initialize collaboration room manager
        let room_manager = Arc::new(RoomManager::new(db_pool.clone()));

        // Initialize metrics
        let metrics = Arc::new(Metrics::new());

        Ok(Self {
            config,
            db: db_pool,
            tantivy_index,
            tantivy_writer,
            tantivy_schema,
            embedder,
            search_engine,
            auth_service,
            room_manager,
            metrics,
        })
    }
}
