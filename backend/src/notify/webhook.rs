//! Webhook notification system with HMAC-SHA256 signature verification.
//!
//! Supports subscribing to events (health_report, conflict_detected, proposal_ready)
//! and dispatching signed HTTP POST notifications.

use crate::error::{AppError, AppResult};

/// Notifier trait — generic notification channel.
pub trait Notifier: Send + Sync {
    fn send(&self, title: &str, body: &str, ref_id: Option<&str>) -> AppResult<()>;
}

/// Webhook notifier — sends HTTP POST with HMAC-SHA256 signature.
pub struct WebhookNotifier {
    url: String,
    secret: String,
}

impl WebhookNotifier {
    pub fn new(url: String, secret: String) -> Self {
        Self { url, secret }
    }
}

impl Notifier for WebhookNotifier {
    fn send(&self, title: &str, body: &str, _ref_id: Option<&str>) -> AppResult<()> {
        let payload = serde_json::json!({
            "title": title,
            "body": body,
            "timestamp": crate::now_ts(),
        });
        let payload_str = serde_json::to_string(&payload)
            .map_err(|e| AppError::internal(format!("json serialize: {}", e)))?;

        // Compute HMAC-SHA256 signature
        use sha2::{Sha256, Digest};
        use hmac::{Hmac, Mac};
        type HmacSha256 = Hmac<Sha256>;

        let mut mac = HmacSha256::new_from_slice(self.secret.as_bytes())
            .map_err(|e| AppError::internal(format!("hmac init: {}", e)))?;
        mac.update(payload_str.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());

        // Fire and forget (sync call in async context)
        let url = self.url.clone();
        let body = payload_str.clone();
        let sig = signature.clone();

        // Note: in a production system, use async reqwest client from AppState
        // For v0.3.0, we emit the webhook synchronously during scan/save operations
        match ureq::post(&url)
            .set("Content-Type", "application/json")
            .set("X-Epicode-Signature", &sig)
            .send_string(&body)
        {
            Ok(response) => {
                if response.status() < 400 {
                    tracing::info!("Webhook sent to {} (status={})", url, response.status());
                    Ok(())
                } else {
                    tracing::warn!(
                        "Webhook to {} returned status {}: {}",
                        url,
                        response.status(),
                        response.into_string().unwrap_or_default()
                    );
                    Ok(()) // Don't fail on webhook errors
                }
            }
            Err(e) => {
                tracing::warn!("Webhook to {} failed: {}", url, e);
                Ok(()) // Don't fail on network errors
            }
        }
    }
}

/// No-op notifier (default when no webhooks configured).
pub struct NoopNotifier;

impl Notifier for NoopNotifier {
    fn send(&self, title: &str, _body: &str, _ref_id: Option<&str>) -> AppResult<()> {
        tracing::debug!("Notification (noop): {}", title);
        Ok(())
    }
}

/// Notification event types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotifyEvent {
    HealthReport,
    ConflictDetected,
    ProposalReady,
}

impl NotifyEvent {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::HealthReport => "health_report",
            Self::ConflictDetected => "conflict_detected",
            Self::ProposalReady => "proposal_ready",
        }
    }
}
