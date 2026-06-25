//! Notification subscription manager.

use crate::db::{DbPool, repository::NotifySubRepo};
use crate::error::AppResult;

pub struct SubscriptionManager {
    db: DbPool,
}

impl SubscriptionManager {
    pub fn new(db: DbPool) -> Self {
        Self { db }
    }

    pub fn subscribe(
        &self,
        space_id: &str,
        event_type: &str,
        webhook_url: &str,
        webhook_secret: &str,
    ) -> AppResult<String> {
        let conn = self
            .db
            .lock()
            .map_err(|e| crate::error::AppError::internal(format!("db lock: {}", e)))?;
        NotifySubRepo::subscribe(&conn, space_id, event_type, webhook_url, webhook_secret)
    }

    pub fn get_subscribers(
        &self,
        space_id: &str,
        event_type: &str,
    ) -> AppResult<Vec<(String, String, String)>> {
        let conn = self
            .db
            .lock()
            .map_err(|e| crate::error::AppError::internal(format!("db lock: {}", e)))?;
        NotifySubRepo::list_active(&conn, space_id, event_type)
    }
}
