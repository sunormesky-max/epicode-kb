//! Webhook notification (stub — Sprint 3).
//!
//! TODO: Implement webhook notifications in Sprint 3.

use crate::error::{AppError, AppResult};

/// Notifier trait.
pub trait Notifier: Send + Sync {
    /// Send a notification.
    fn send(&self, title: &str, body: &str, ref_id: Option<&str>) -> AppResult<()>;
}

/// Webhook notifier — sends HTTP POST to a configured URL.
/// TODO: Implement in Sprint 3.
pub struct WebhookNotifier {
    url: String,
}

impl WebhookNotifier {
    /// Create a new WebhookNotifier.
    pub fn new(url: String) -> Self {
        Self { url }
    }
}

impl Notifier for WebhookNotifier {
    fn send(&self, _title: &str, _body: &str, _ref_id: Option<&str>) -> AppResult<()> {
        // TODO: Implement HTTP POST webhook in Sprint 3.
        tracing::debug!("Webhook notification (stub): url={}", self.url);
        Err(AppError::not_implemented(
            "webhook notifications (planned for Sprint 3)",
        ))
    }
}

/// No-op notifier (used as default when no webhook is configured).
pub struct NoopNotifier;

impl Notifier for NoopNotifier {
    fn send(&self, title: &str, _body: &str, _ref_id: Option<&str>) -> AppResult<()> {
        tracing::debug!("Notification (noop): {}", title);
        Ok(())
    }
}
