//! Auth domain models: User, roles, permissions, spaces, API keys.

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::generate_id;
use crate::now_ts;

// ============================================================
// GlobalRole
// ============================================================

/// User global role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum GlobalRole {
    Admin,
    Owner,
    Editor,
    #[default]
    Viewer,
}

impl GlobalRole {
    /// String representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            GlobalRole::Admin => "admin",
            GlobalRole::Owner => "owner",
            GlobalRole::Editor => "editor",
            GlobalRole::Viewer => "viewer",
        }
    }

    /// Parse from string.
    pub fn parse_str(s: &str) -> Result<Self, String> {
        match s {
            "admin" => Ok(GlobalRole::Admin),
            "owner" => Ok(GlobalRole::Owner),
            "editor" => Ok(GlobalRole::Editor),
            "viewer" => Ok(GlobalRole::Viewer),
            _ => Err(format!("invalid global_role: {}", s)),
        }
    }
}

// ============================================================
// SpaceRole
// ============================================================

/// Space-level role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SpaceRole {
    Owner,
    Editor,
    #[default]
    Viewer,
}

impl SpaceRole {
    /// String representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            SpaceRole::Owner => "owner",
            SpaceRole::Editor => "editor",
            SpaceRole::Viewer => "viewer",
        }
    }

    /// Parse from string.
    pub fn parse_str(s: &str) -> Result<Self, String> {
        match s {
            "owner" => Ok(SpaceRole::Owner),
            "editor" => Ok(SpaceRole::Editor),
            "viewer" => Ok(SpaceRole::Viewer),
            _ => Err(format!("invalid space_role: {}", s)),
        }
    }
}

// ============================================================
// Permission
// ============================================================

/// Permission enum used by the RBAC engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    SpaceRead,
    SpaceWrite,
    SpaceAdmin,
    MemoryRead,
    MemoryWrite,
    MemoryAdmin,
    AgentWrite,
    ApiKeyManage,
    UserManage,
}

// ============================================================
// SpaceVisibility
// ============================================================

/// Space visibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SpaceVisibility {
    #[default]
    Team,
    Public,
    Private,
}

impl SpaceVisibility {
    /// String representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            SpaceVisibility::Public => "public",
            SpaceVisibility::Team => "team",
            SpaceVisibility::Private => "private",
        }
    }

    /// Parse from string.
    pub fn parse_str(s: &str) -> Result<Self, String> {
        match s {
            "public" => Ok(SpaceVisibility::Public),
            "team" => Ok(SpaceVisibility::Team),
            "private" => Ok(SpaceVisibility::Private),
            _ => Err(format!("invalid space visibility: {}", s)),
        }
    }
}

// ============================================================
// User
// ============================================================

/// User model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub email: String,
    pub name: String,
    #[serde(skip_serializing)]
    pub password_hash: Option<String>,
    pub sso_subject: Option<String>,
    pub global_role: GlobalRole,
    pub is_active: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

impl User {
    /// Create a new local user.
    pub fn new_local(
        email: String,
        name: String,
        global_role: GlobalRole,
        password_hash: String,
    ) -> Self {
        let now = now_ts();
        Self {
            id: generate_id("usr"),
            email,
            name,
            password_hash: Some(password_hash),
            sso_subject: None,
            global_role,
            is_active: true,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Actor context used for authorization and audit.
#[derive(Debug, Clone)]
pub struct Actor {
    pub user_id: String,
    pub global_role: GlobalRole,
    pub space_role: Option<SpaceRole>,
}

impl Actor {
    /// Create an actor from a user and optional space role.
    pub fn new(user: &User, space_role: Option<SpaceRole>) -> Self {
        Self {
            user_id: user.id.clone(),
            global_role: user.global_role,
            space_role,
        }
    }
}

/// Request to create a local user.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateLocalUserRequest {
    pub email: String,
    pub name: String,
    pub password: String,
    #[serde(default)]
    pub global_role: GlobalRole,
}

impl CreateLocalUserRequest {
    /// Validate the request.
    pub fn validate(&self) -> Result<(), AppError> {
        if self.email.trim().is_empty() || !self.email.contains('@') {
            return Err(AppError::bad_request("email is invalid"));
        }
        if self.name.trim().is_empty() {
            return Err(AppError::bad_request("name must not be empty"));
        }
        if self.password.len() < 12 {
            return Err(AppError::bad_request(
                "password must be at least 12 characters",
            ));
        }
        if self.password.len() > 128 {
            return Err(AppError::bad_request(
                "password must not exceed 128 characters",
            ));
        }
        let has_upper = self.password.chars().any(|c| c.is_uppercase());
        let has_lower = self.password.chars().any(|c| c.is_lowercase());
        let has_digit = self.password.chars().any(|c| c.is_ascii_digit());
        let has_special = self.password.chars().any(|c| !c.is_alphanumeric());
        if !has_upper || !has_lower || !has_digit || !has_special {
            return Err(AppError::bad_request(
                "password must contain at least one uppercase letter, one lowercase letter, one digit, and one special character",
            ));
        }
        Ok(())
    }
}

/// Login request.
#[derive(Debug, Clone, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

impl LoginRequest {
    /// Validate the request.
    pub fn validate(&self) -> Result<(), AppError> {
        if self.email.trim().is_empty() {
            return Err(AppError::bad_request("email must not be empty"));
        }
        if self.password.is_empty() {
            return Err(AppError::bad_request("password must not be empty"));
        }
        Ok(())
    }
}

/// Refresh token request.
#[derive(Debug, Clone, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

// ============================================================
// Space
// ============================================================

/// Space model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Space {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub visibility: SpaceVisibility,
    pub owner_id: String,
    pub ai_write_enabled: bool,
    pub default_ai_trust_level: f32,
    pub retention_days: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl Space {
    /// Create a new space.
    pub fn new(name: String, slug: String, owner_id: String) -> Self {
        let now = now_ts();
        Self {
            id: generate_id("sp"),
            name,
            slug,
            description: None,
            visibility: SpaceVisibility::Team,
            owner_id,
            ai_write_enabled: true,
            default_ai_trust_level: 0.5,
            retention_days: None,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Space member model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaceMember {
    pub id: String,
    pub space_id: String,
    pub user_id: String,
    pub role: SpaceRole,
    pub created_at: i64,
}

// ============================================================
// ApiKey
// ============================================================

/// API key model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: String,
    pub space_id: String,
    pub user_id: String,
    pub key_hash: String,
    pub scope: String,
    pub name: String,
    pub expires_at: Option<i64>,
    pub last_used_at: Option<i64>,
    pub created_at: i64,
}

/// Agent context extracted from a valid API key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentContext {
    pub api_key_id: String,
    pub space_id: String,
    pub user_id: String,
    pub user_role: GlobalRole,
    pub space_role: Option<SpaceRole>,
    pub scope: String,
}

/// Memory permission (ACL) model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPermission {
    pub id: String,
    pub memory_id: String,
    pub user_id: String,
    pub permission: String,
    pub created_at: i64,
}

// ============================================================
// OIDC stubs (optional feature)
// ============================================================

/// OIDC callback request.
#[derive(Debug, Clone, Deserialize)]
pub struct OidcCallbackRequest {
    pub code: String,
    pub state: Option<String>,
}
