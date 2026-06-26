//! Application state — shared resources for all handlers.

use std::sync::Arc;

use rusqlite::Connection;
use tantivy::{Index, IndexWriter};

use crate::auth::service::AuthService;
use crate::collab::room::RoomManager;
use crate::config::AppConfig;
use crate::conflict::detect::ConflictDetector;
use crate::conflict::model::ConflictConfig;
use crate::db;
use crate::dream::proposal::ProposalEngine;
use crate::embed::{self, EmbeddingProvider};
use crate::error::AppResult;
use crate::health::scanner::HealthScanner;
use crate::notify::subscriptions::SubscriptionManager;
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
    /// AI proposal engine.
    pub proposal_engine: Arc<ProposalEngine>,
    /// Conflict detector (optional, requires embedding).
    pub conflict_detector: Option<Arc<ConflictDetector>>,
    /// Health scanner.
    pub health_scanner: Option<Arc<HealthScanner>>,
    /// Notification subscription manager.
    pub subscription_manager: Arc<SubscriptionManager>,
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

        // Initialize conflict detector
        let conflict_detector = {
            let cc = ConflictConfig {
                semantic_threshold: config.conflict_threshold.unwrap_or(0.3),
                llm_confidence_threshold: config.conflict_llm_confidence.unwrap_or(0.6),
                max_neighbors: 10,
            };
            Some(Arc::new(ConflictDetector::new(
                db_pool.clone(),
                embedder.clone(),
                cc,
            )))
        };

        // Initialize proposal engine (wired to the conflict detector so dream
        // cycle scans also surface knowledge contradictions as proposals).
        let proposal_engine = Arc::new(ProposalEngine::new_with_conflict(
            db_pool.clone(),
            conflict_detector.clone().expect("conflict detector is always Some after init"),
        ));

        // Initialize health scanner
        let health_scanner = Some(Arc::new(HealthScanner::new(db_pool.clone())));

        // Initialize subscription manager
        let subscription_manager = Arc::new(SubscriptionManager::new(db_pool.clone()));

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
            proposal_engine,
            conflict_detector,
            health_scanner,
            subscription_manager,
        })
    }
}
