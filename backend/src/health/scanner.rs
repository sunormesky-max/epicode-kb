//! Health scanner — aggregates staleness, gaps, orphans into a health snapshot.

use serde::{Deserialize, Serialize};

use crate::db::{DbPool, repository};
use crate::error::AppResult;

/// A computed health snapshot for a space.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthSnapshot {
    pub space_id: String,
    pub snapshot_date: String,
    pub total: usize,
    pub human_ratio: f32,
    pub ai_ratio: f32,
    pub co_ratio: f32,
    pub conflict_count: usize,
    pub avg_trust: f32,
    pub stale_count: usize,
    pub orphan_count: usize,
    pub gap_count: usize,
    pub health_score: f32,
}

/// A knowledge gap entry (query with zero results).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapEntry {
    pub query: String,
    pub count: i64,
}

/// Staleness score for a single memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StalenessScore {
    pub memory_id: String,
    pub score: f32,
    pub days_since_access: i64,
    pub access_count: i64,
}

/// Health scanner for a space.
pub struct HealthScanner {
    db: DbPool,
}

impl HealthScanner {
    pub fn new(db: DbPool) -> Self {
        Self { db }
    }

    /// Full scan: compute ratios, staleness, gaps, orphans, and health score.
    pub fn full_scan(&self, space_id: &str) -> AppResult<HealthSnapshot> {
        let conn = self
            .db
            .lock()
            .map_err(|e| crate::error::AppError::internal(format!("db lock: {}", e)))?;

        let now = crate::now_ts();
        let snapshot_date = chrono::Utc::now().format("%Y-%m-%d").to_string();

        // Total memories and ratios
        let total: i64 = conn.query_row(
            "SELECT COUNT(*) FROM memories WHERE space_id = ?1 AND review_status = 'accepted'",
            rusqlite::params![space_id],
            |row| row.get(0),
        ).unwrap_or(0);

        let human_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM memories WHERE space_id = ?1 AND provenance = 'human' AND review_status = 'accepted'",
            rusqlite::params![space_id], |row| row.get(0),
        ).unwrap_or(0);

        let ai_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM memories WHERE space_id = ?1 AND provenance = 'ai' AND review_status = 'accepted'",
            rusqlite::params![space_id], |row| row.get(0),
        ).unwrap_or(0);

        let co_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM memories WHERE space_id = ?1 AND provenance = 'co' AND review_status = 'accepted'",
            rusqlite::params![space_id], |row| row.get(0),
        ).unwrap_or(0);

        let conflict_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM memories WHERE space_id = ?1 AND provenance = 'conflict'",
            rusqlite::params![space_id], |row| row.get(0),
        ).unwrap_or(0);

        let total_f = total as f32;
        let human_ratio = if total_f > 0.0 { human_count as f32 / total_f } else { 0.0 };
        let ai_ratio = if total_f > 0.0 { ai_count as f32 / total_f } else { 0.0 };
        let co_ratio = if total_f > 0.0 { co_count as f32 / total_f } else { 0.0 };

        // Average trust
        let avg_trust: f32 = conn.query_row(
            "SELECT COALESCE(AVG(trust_level), 0.0) FROM memories WHERE space_id = ?1 AND review_status = 'accepted'",
            rusqlite::params![space_id], |row| row.get(0),
        ).unwrap_or(0.0);

        // Staleness: not accessed in 90 days
        let cutoff = now - 90 * 86400;
        let stale_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM memories WHERE space_id = ?1 AND last_accessed_at IS NOT NULL AND last_accessed_at < ?2 AND review_status = 'accepted'",
            rusqlite::params![space_id, cutoff], |row| row.get(0),
        ).unwrap_or(0);

        // Orphans: zero access count (approximate for "zero inbound links")
        let orphan_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM memories WHERE space_id = ?1 AND access_count = 0 AND review_status = 'accepted'",
            rusqlite::params![space_id], |row| row.get(0),
        ).unwrap_or(0);

        // Gaps: queries with zero results
        let gap_count: i64 = conn.query_row(
            "SELECT COUNT(DISTINCT query) FROM query_logs WHERE space_id = ?1 AND result_count = 0 AND created_at > ?2",
            rusqlite::params![space_id, now - 30 * 86400], |row| row.get(0),
        ).unwrap_or(0);

        // Health score: activity(0.3) + completeness(0.3) + freshness(0.2) + trust(0.2)
        let activity = if total_f > 0.0 { 1.0 - (stale_count as f32 / total_f) } else { 1.0 };
        let completeness = if gap_count > 0 { 1.0 - (gap_count as f32 / (gap_count as f32 + 10.0)) } else { 1.0 };
        let freshness = activity; // same metric for now
        let trust_score = avg_trust;
        let health_score = (activity * 0.3 + completeness * 0.3 + freshness * 0.2 + trust_score * 0.2) * 100.0;

        let snap = HealthSnapshot {
            space_id: space_id.to_string(),
            snapshot_date,
            total: total as usize,
            human_ratio,
            ai_ratio,
            co_ratio,
            conflict_count: conflict_count as usize,
            avg_trust,
            stale_count: stale_count as usize,
            orphan_count: orphan_count as usize,
            gap_count: gap_count as usize,
            health_score,
        };

        tracing::info!(
            "Health scan for space {}: score={:.1}, total={}, stale={}, gaps={}, orphans={}",
            space_id, health_score, total, stale_count, gap_count, orphan_count
        );

        // Persist snapshot
        repository::HealthRepo::insert_snapshot(&conn, &snap)?;

        Ok(snap)
    }

    /// Get stale memory IDs.
    pub fn scan_staleness(&self, space_id: &str) -> AppResult<Vec<StalenessScore>> {
        let conn = self.db.lock().map_err(|e| crate::error::AppError::internal(format!("db lock: {}", e)))?;
        let now = crate::now_ts();
        let cutoff = now - 90 * 86400;

        let mut stmt = conn.prepare(
            "SELECT id, last_accessed_at, access_count FROM memories WHERE space_id = ?1 AND review_status = 'accepted' AND last_accessed_at IS NOT NULL AND last_accessed_at < ?2"
        ).map_err(crate::error::AppError::db)?;

        let mut results = Vec::new();
        let rows = stmt.query_map(rusqlite::params![space_id, cutoff], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<i64>>(1)?, row.get::<_, i64>(2)?))
        }).map_err(crate::error::AppError::db)?;

        for r in rows {
            let (mid, last_access, ac_count) = r.map_err(crate::error::AppError::db)?;
            let days = last_access.map(|t| (now - t) / 86400).unwrap_or(365);
            // Sigmoid staleness: sharpens around 45 days
            let score = 1.0 / (1.0 + (-(days as f32 - 45.0) / 15.0).exp());
            results.push(StalenessScore {
                memory_id: mid,
                score,
                days_since_access: days,
                access_count: ac_count,
            });
        }

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        Ok(results.into_iter().take(20).collect())
    }

    /// Get knowledge gaps.
    pub fn scan_gaps(&self, space_id: &str) -> AppResult<Vec<GapEntry>> {
        let conn = self.db.lock().map_err(|e| crate::error::AppError::internal(format!("db lock: {}", e)))?;
        let gaps = repository::QueryLogRepo::get_zero_result_queries(&conn, space_id, 30)?;
        Ok(gaps.into_iter().map(|(q, c)| GapEntry { query: q, count: c }).collect())
    }
}
