//! JWT token issuance and verification.

use std::sync::Arc;

use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::auth::model::{GlobalRole, User};
use crate::config::AppConfig;
use crate::error::{AppError, AppResult};

/// JWT claims carried in access tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: String,
    pub email: String,
    pub global_role: GlobalRole,
    pub exp: usize,
    pub iat: usize,
    pub token_type: String,
}

/// JWT claims carried in refresh tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshClaims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
    pub token_type: String,
}

/// Issued token pair.
#[derive(Debug, Clone, Serialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
}

/// JWT issuer / verifier.
pub struct JwtIssuer {
    secret: String,
    access_ttl: i64,
    refresh_ttl: i64,
}

impl JwtIssuer {
    /// Create a new JWT issuer from configuration.
    pub fn new(config: &AppConfig) -> Self {
        Self {
            secret: config.jwt_secret.clone(),
            access_ttl: config.jwt_access_ttl,
            refresh_ttl: config.jwt_refresh_ttl,
        }
    }

    /// Issue a new token pair for a user.
    pub fn issue(&self, user: &User) -> AppResult<TokenPair> {
        let now = crate::now_ts();
        let access_exp = now + self.access_ttl;
        let refresh_exp = now + self.refresh_ttl;

        let access_claims = JwtClaims {
            sub: user.id.clone(),
            email: user.email.clone(),
            global_role: user.global_role,
            iat: now as usize,
            exp: access_exp as usize,
            token_type: "access".to_string(),
        };

        let refresh_claims = RefreshClaims {
            sub: user.id.clone(),
            iat: now as usize,
            exp: refresh_exp as usize,
            token_type: "refresh".to_string(),
        };

        let header = Header::new(Algorithm::HS256);
        let encoding_key = EncodingKey::from_secret(self.secret.as_bytes());

        let access_token = encode(&header, &access_claims, &encoding_key)
            .map_err(|e| AppError::jwt(format!("failed to encode access token: {}", e)))?;
        let refresh_token = encode(&header, &refresh_claims, &encoding_key)
            .map_err(|e| AppError::jwt(format!("failed to encode refresh token: {}", e)))?;

        Ok(TokenPair {
            access_token,
            refresh_token,
            expires_in: self.access_ttl,
        })
    }

    /// Verify an access token and return its claims.
    pub fn verify_access(&self, token: &str) -> AppResult<JwtClaims> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_required_spec_claims(&["exp", "sub"]);
        let decoding_key = DecodingKey::from_secret(self.secret.as_bytes());

        let token_data = decode::<JwtClaims>(token, &decoding_key, &validation)
            .map_err(|e| AppError::jwt(format!("invalid access token: {}", e)))?;

        if token_data.claims.token_type != "access" {
            return Err(AppError::jwt("token is not an access token"));
        }

        Ok(token_data.claims)
    }

    /// Verify a refresh token and return its subject.
    pub fn verify_refresh(&self, token: &str) -> AppResult<String> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_required_spec_claims(&["exp", "sub"]);
        let decoding_key = DecodingKey::from_secret(self.secret.as_bytes());

        let token_data = decode::<RefreshClaims>(token, &decoding_key, &validation)
            .map_err(|e| AppError::jwt(format!("invalid refresh token: {}", e)))?;

        if token_data.claims.token_type != "refresh" {
            return Err(AppError::jwt("token is not a refresh token"));
        }

        Ok(token_data.claims.sub)
    }
}

/// Build a JWT issuer wrapped in Arc for shared use.
pub fn build_jwt_issuer(config: Arc<AppConfig>) -> Arc<JwtIssuer> {
    Arc::new(JwtIssuer::new(&config))
}
