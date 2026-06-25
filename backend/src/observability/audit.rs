//! Audit logging helper.

use std::sync::Arc;

use rusqlite::Connection;
use serde_json::Value;

use crate::db::repository::{AuditLog, AuditRepo};
use crate::error::AppResult;
use crate::generate_id;
use crate::now_ts;

/// Auditor writes audit log entries.
#[derive(Debug, Clone, Default)]
pub struct Auditor;

impl Auditor {
    /// Create a new auditor.
    pub fn new() -> Self {
        Self
    }

    /// Log an audit event.
    #[allow(clippy::too_many_arguments)]
    pub fn log(
        &self,
        db: &Arc<std::sync::Mutex<Connection>>,
        space_id: &str,
        user_id: Option<&str>,
        action: &str,
        entity_type: &str,
        entity_id: Option<&str>,
        details: Option<Value>,
    ) -> AppResult<()> {
        let log = AuditLog {
            id: generate_id("audit"),
            space_id: space_id.to_string(),
            user_id: user_id.map(|s| s.to_string()),
            action: action.to_string(),
            entity_type: entity_type.to_string(),
            entity_id: entity_id.map(|s| s.to_string()),
            details,
            created_at: now_ts(),
        };
        let conn = db.lock().unwrap();
        AuditRepo::insert(&conn, &log)?;
        Ok(())
    }
}
