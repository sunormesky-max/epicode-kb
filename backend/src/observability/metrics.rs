//! Prometheus metrics endpoint.

use prometheus::{Counter, Encoder, Gauge, Histogram, HistogramOpts, Registry, TextEncoder};

/// Application metrics.
pub struct Metrics {
    registry: Registry,
    requests_total: Counter,
    request_duration: Histogram,
    embeddings_queue_depth: Gauge,
    search_hits: Counter,
    pending_reviews: Gauge,
}

impl Metrics {
    /// Create and register metrics.
    pub fn new() -> Self {
        let registry = Registry::new();

        let requests_total =
            Counter::new("epicode_kb_requests_total", "Total HTTP requests").unwrap();
        registry.register(Box::new(requests_total.clone())).unwrap();

        let request_duration = Histogram::with_opts(HistogramOpts::new(
            "epicode_kb_request_duration_seconds",
            "HTTP request duration",
        ))
        .unwrap();
        registry
            .register(Box::new(request_duration.clone()))
            .unwrap();

        let embeddings_queue_depth =
            Gauge::new("epicode_kb_embeddings_queue_depth", "Pending embeddings").unwrap();
        registry
            .register(Box::new(embeddings_queue_depth.clone()))
            .unwrap();

        let search_hits =
            Counter::new("epicode_kb_search_hits_total", "Total search hits").unwrap();
        registry.register(Box::new(search_hits.clone())).unwrap();

        let pending_reviews =
            Gauge::new("epicode_kb_pending_reviews", "Pending review count").unwrap();
        registry
            .register(Box::new(pending_reviews.clone()))
            .unwrap();

        Self {
            registry,
            requests_total,
            request_duration,
            embeddings_queue_depth,
            search_hits,
            pending_reviews,
        }
    }

    /// Increment total requests.
    pub fn inc_requests(&self) {
        self.requests_total.inc();
    }

    /// Observe request duration.
    pub fn observe_request_duration(&self, seconds: f64) {
        self.request_duration.observe(seconds);
    }

    /// Set embeddings queue depth.
    pub fn set_embeddings_queue_depth(&self, value: i64) {
        self.embeddings_queue_depth.set(value as f64);
    }

    /// Increment search hits.
    pub fn inc_search_hits(&self) {
        self.search_hits.inc();
    }

    /// Set pending review count.
    pub fn set_pending_reviews(&self, value: i64) {
        self.pending_reviews.set(value as f64);
    }

    /// Render metrics in Prometheus text format.
    pub fn gather(&self) -> AppResult<String> {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder
            .encode(&metric_families, &mut buffer)
            .map_err(|e| AppError::internal(format!("metrics encode error: {}", e)))?;
        String::from_utf8(buffer)
            .map_err(|e| AppError::internal(format!("metrics utf8 error: {}", e)))
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

use crate::error::{AppError, AppResult};
