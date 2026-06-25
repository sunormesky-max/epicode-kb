//! Unified error type implementing `IntoResponse` for Axum.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

/// Application error type.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("unauthorized: {0}")]
    Unauthorized(String),

    #[error("forbidden: {0}")]
    Forbidden(String),

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("internal error: {0}")]
    Internal(String),

    #[error("not implemented: {0}")]
    NotImplemented(String),

    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("tantivy error: {0}")]
    Tantivy(#[from] tantivy::TantivyError),

    #[error("tantivy query parse error: {0}")]
    TantivyQueryParse(String),

    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("jwt error: {0}")]
    Jwt(String),

    #[error("password hash error: {0}")]
    PasswordHash(String),

    #[error("websocket error: {0}")]
    WebSocket(String),
}

impl AppError {
    /// Create a bad request error.
    pub fn bad_request(msg: impl Into<String>) -> Self {
        AppError::BadRequest(msg.into())
    }

    /// Create an unauthorized error.
    pub fn unauthorized(msg: impl Into<String>) -> Self {
        AppError::Unauthorized(msg.into())
    }

    /// Create a forbidden error.
    pub fn forbidden(msg: impl Into<String>) -> Self {
        AppError::Forbidden(msg.into())
    }

    /// Create a conflict error.
    pub fn conflict(msg: impl Into<String>) -> Self {
        AppError::Conflict(msg.into())
    }

    /// Create a not found error.
    pub fn not_found(msg: impl Into<String>) -> Self {
        AppError::NotFound(msg.into())
    }

    /// Create an internal error.
    pub fn internal(msg: impl Into<String>) -> Self {
        AppError::Internal(msg.into())
    }

    /// Create a not implemented error.
    pub fn not_implemented(msg: impl Into<String>) -> Self {
        AppError::NotImplemented(msg.into())
    }

    /// Create a JWT error.
    pub fn jwt(msg: impl Into<String>) -> Self {
        AppError::Jwt(msg.into())
    }

    /// Create a password hash error.
    pub fn password_hash(msg: impl Into<String>) -> Self {
        AppError::PasswordHash(msg.into())
    }

    /// Create a database error from rusqlite.
    pub fn db(e: rusqlite::Error) -> Self {
        AppError::Database(e)
    }
}

/// Error code mapping.
impl AppError {
    fn error_code(&self) -> i32 {
        match self {
            AppError::BadRequest(_) => 40000,
            AppError::Unauthorized(_) => 40100,
            AppError::Forbidden(_) => 40300,
            AppError::Conflict(_) => 40900,
            AppError::NotFound(_) => 40400,
            AppError::Internal(_) => 50000,
            AppError::NotImplemented(_) => 50100,
            AppError::Database(_) => 50001,
            AppError::Serde(_) => 50002,
            AppError::Io(_) => 50003,
            AppError::Tantivy(_) => 50004,
            AppError::TantivyQueryParse(_) => 50006,
            AppError::Http(_) => 50005,
            AppError::Jwt(_) => 40101,
            AppError::PasswordHash(_) => 50007,
            AppError::WebSocket(_) => 50008,
        }
    }

    fn status_code(&self) -> StatusCode {
        match self {
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::Unauthorized(_) | AppError::Jwt(_) => StatusCode::UNAUTHORIZED,
            AppError::Forbidden(_) => StatusCode::FORBIDDEN,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::NotImplemented(_) => StatusCode::NOT_IMPLEMENTED,
            AppError::Internal(_)
            | AppError::Database(_)
            | AppError::Serde(_)
            | AppError::Io(_)
            | AppError::Tantivy(_)
            | AppError::TantivyQueryParse(_)
            | AppError::Http(_)
            | AppError::PasswordHash(_)
            | AppError::WebSocket(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

/// Error response body.
#[derive(Debug, Serialize)]
struct ErrorBody {
    code: i32,
    data: Option<()>,
    message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let body = ErrorBody {
            code: self.error_code(),
            data: None,
            message: self.to_string(),
        };
        (self.status_code(), Json(body)).into_response()
    }
}

/// Type alias for results using `AppError`.
pub type AppResult<T> = Result<T, AppError>;
