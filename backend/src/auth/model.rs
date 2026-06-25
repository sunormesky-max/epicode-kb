//! Auth domain models (stub — Sprint 2).
//!
//! TODO: Implement full RBAC models in Sprint 2.

use serde::{Deserialize, Serialize};

/// User global role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GlobalRole {
    Admin,
    Owner,
    Editor,
    Viewer,
}

impl GlobalRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            GlobalRole::Admin => "admin",
            GlobalRole::Owner => "owner",
            GlobalRole::Editor => "editor",
            GlobalRole::Viewer => "viewer",
        }
    }
}

/// Space-level role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SpaceRole {
    Owner,
    Editor,
    Viewer,
}

impl SpaceRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            SpaceRole::Owner => "owner",
            SpaceRole::Editor => "editor",
            SpaceRole::Viewer => "viewer",
        }
    }
}

/// User model.
/// TODO: Implement full User model with auth in Sprint 2.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub email: String,
    pub name: String,
    pub global_role: GlobalRole,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Space model.
/// TODO: Implement full Space model in Sprint 2.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Space {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub ai_write_enabled: bool,
    pub default_ai_trust_level: f32,
    pub retention_days: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Permission check result.
/// TODO: Implement permission system in Sprint 2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Permission {
    Read,
    Write,
    Delete,
    Admin,
}
