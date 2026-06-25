//! Authentication service: login, registration, API key verification.

use std::sync::Arc;

use rusqlite::Connection;

use crate::auth::jwt::{JwtIssuer, TokenPair};
use crate::auth::model::{
    AgentContext, CreateLocalUserRequest, LoginRequest, RefreshRequest, User,
};
use crate::auth::rbac::RbacEngine;
use crate::config::AppConfig;
use crate::db::repository::{ApiKeyRepo, SpaceMemberRepo, UserRepo};
use crate::error::{AppError, AppResult};

/// Authentication service.
pub struct AuthService {
    db: Arc<std::sync::Mutex<Connection>>,
    jwt: Arc<JwtIssuer>,
    rbac: RbacEngine,
    api_key_salt: Option<String>,
    agent_write_enabled: bool,
}

impl AuthService {
    /// Create a new authentication service.
    pub fn new(db: Arc<std::sync::Mutex<Connection>>, config: Arc<AppConfig>) -> Self {
        Self {
            db,
            jwt: Arc::new(JwtIssuer::new(&config)),
            rbac: RbacEngine::new(),
            api_key_salt: config.api_key_salt.clone(),
            agent_write_enabled: config.agent_write_enabled,
        }
    }

    /// Register a local user.
    pub fn register(&self, req: CreateLocalUserRequest) -> AppResult<User> {
        req.validate()?;
        let conn = self.db.lock().unwrap();
        if UserRepo::find_by_email(&conn, &req.email)?.is_some() {
            return Err(AppError::conflict(format!(
                "user with email {} already exists",
                req.email
            )));
        }
        let password_hash = hash_password(&req.password)?;
        let user = User::new_local(req.email, req.name, req.global_role, password_hash);
        UserRepo::insert(&conn, &user)?;
        Ok(user)
    }

    /// Login with email and password.
    pub fn login(&self, req: LoginRequest) -> AppResult<(TokenPair, User)> {
        req.validate()?;
        let conn = self.db.lock().unwrap();
        let user = UserRepo::find_by_email(&conn, &req.email)?
            .ok_or_else(|| AppError::unauthorized("invalid email or password"))?;

        if !user.is_active {
            return Err(AppError::forbidden("user account is disabled"));
        }

        let Some(ref hash) = user.password_hash else {
            return Err(AppError::unauthorized("invalid email or password"));
        };

        if !verify_password(&req.password, hash)? {
            return Err(AppError::unauthorized("invalid email or password"));
        }

        let tokens = self.jwt.issue(&user)?;
        Ok((tokens, user))
    }

    /// Refresh access token using a refresh token.
    pub fn refresh(&self, req: RefreshRequest) -> AppResult<TokenPair> {
        let user_id = self.jwt.verify_refresh(&req.refresh_token)?;
        let conn = self.db.lock().unwrap();
        let user = UserRepo::get_by_id(&conn, &user_id)?;
        if !user.is_active {
            return Err(AppError::forbidden("user account is disabled"));
        }
        let tokens = self.jwt.issue(&user)?;
        Ok(tokens)
    }

    /// Verify a JWT access token and return the user.
    pub fn verify_access_token(&self, token: &str) -> AppResult<User> {
        let claims = self.jwt.verify_access(token)?;
        let conn = self.db.lock().unwrap();
        let user = UserRepo::get_by_id(&conn, &claims.sub)?;
        if !user.is_active {
            return Err(AppError::forbidden("user account is disabled"));
        }
        Ok(user)
    }

    /// Verify an Agent API key and return agent context.
    pub fn verify_api_key(&self, key: &str, space_id: &str) -> AppResult<AgentContext> {
        let key_hash = hash_api_key(key, self.api_key_salt.as_deref());
        let conn = self.db.lock().unwrap();
        let api_key = ApiKeyRepo::find_by_hash_and_space(&conn, &key_hash, space_id)?
            .ok_or_else(|| AppError::unauthorized("invalid api key"))?;

        let now = crate::now_ts();
        if let Some(exp) = api_key.expires_at {
            if exp < now {
                return Err(AppError::unauthorized("api key expired"));
            }
        }

        if !self.agent_write_enabled {
            return Err(AppError::forbidden("agent writes are disabled"));
        }

        ApiKeyRepo::update_last_used(&conn, &api_key.id, now)?;

        let user = UserRepo::get_by_id(&conn, &api_key.user_id)?;
        let space_role = SpaceMemberRepo::find_role(&conn, space_id, &api_key.user_id)?;

        Ok(AgentContext {
            api_key_id: api_key.id,
            space_id: api_key.space_id,
            user_id: api_key.user_id,
            user_role: user.global_role,
            space_role,
            scope: api_key.scope,
        })
    }

    /// Build authentication context for a user in a space.
    pub fn build_context(
        &self,
        user: &User,
        space_id: &str,
    ) -> AppResult<crate::auth::rbac::AuthContext> {
        let conn = self.db.lock().unwrap();
        let space_role = SpaceMemberRepo::find_role(&conn, space_id, &user.id)?;
        Ok(crate::auth::rbac::AuthContext {
            user_id: user.id.clone(),
            global_role: user.global_role,
            space_id: space_id.to_string(),
            space_role,
        })
    }

    /// Get the RBAC engine.
    pub fn rbac(&self) -> &RbacEngine {
        &self.rbac
    }

    /// Get the JWT issuer.
    pub fn jwt(&self) -> &JwtIssuer {
        &self.jwt
    }
}

/// Hash a password using bcrypt.
pub fn hash_password(password: &str) -> AppResult<String> {
    bcrypt::hash(password, bcrypt::DEFAULT_COST)
        .map_err(|e| AppError::password_hash(format!("failed to hash password: {}", e)))
}

/// Verify a password against a bcrypt hash.
pub fn verify_password(password: &str, hash: &str) -> AppResult<bool> {
    bcrypt::verify(password, hash)
        .map_err(|e| AppError::password_hash(format!("failed to verify password: {}", e)))
}

/// Hash an API key using SHA256 + optional salt.
pub fn hash_api_key(key: &str, salt: Option<&str>) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    if let Some(s) = salt {
        hasher.update(s.as_bytes());
    }
    hex::encode(hasher.finalize())
}

// Hex encoding helper used for API key hashes.
mod hex {
    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        bytes
            .as_ref()
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect()
    }
}
