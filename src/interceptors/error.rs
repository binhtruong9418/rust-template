use axum::{
    response::{IntoResponse, Response},
    http::StatusCode,
};
use thiserror::Error;
use serde_json::json;

use super::response::ApiError;

/// Application error types
#[derive(Debug, Error)]
pub enum AppError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Redis error: {0}")]
    RedisError(String),

    #[error("Authentication error: {0}")]
    AuthError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Internal server error: {0}")]
    InternalError(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("JWT error: {0}")]
    JwtError(#[from] jsonwebtoken::errors::Error),

    #[error("MQTT error: {0}")]
    MqttError(String),

    #[error("Queue error: {0}")]
    QueueError(String),
}

/// Error codes for API responses
#[derive(Debug)]
pub enum ErrorCode {
    DatabaseError,
    RedisError,
    AuthError,
    ValidationError,
    NotFound,
    InternalError,
    BadRequest,
    Unauthorized,
    Forbidden,
    Conflict,
    JwtError,
    MqttError,
    QueueError,
}

impl ErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorCode::DatabaseError => "DATABASE_ERROR",
            ErrorCode::RedisError => "REDIS_ERROR",
            ErrorCode::AuthError => "AUTH_ERROR",
            ErrorCode::ValidationError => "VALIDATION_ERROR",
            ErrorCode::NotFound => "NOT_FOUND",
            ErrorCode::InternalError => "INTERNAL_ERROR",
            ErrorCode::BadRequest => "BAD_REQUEST",
            ErrorCode::Unauthorized => "UNAUTHORIZED",
            ErrorCode::Forbidden => "FORBIDDEN",
            ErrorCode::Conflict => "CONFLICT",
            ErrorCode::JwtError => "JWT_ERROR",
            ErrorCode::MqttError => "MQTT_ERROR",
            ErrorCode::QueueError => "QUEUE_ERROR",
        }
    }
}

impl AppError {
    pub fn error_code(&self) -> ErrorCode {
        match self {
            AppError::DatabaseError(_) => ErrorCode::DatabaseError,
            AppError::RedisError(_) => ErrorCode::RedisError,
            AppError::AuthError(_) => ErrorCode::AuthError,
            AppError::ValidationError(_) => ErrorCode::ValidationError,
            AppError::NotFound(_) => ErrorCode::NotFound,
            AppError::InternalError(_) => ErrorCode::InternalError,
            AppError::BadRequest(_) => ErrorCode::BadRequest,
            AppError::Unauthorized(_) => ErrorCode::Unauthorized,
            AppError::Forbidden(_) => ErrorCode::Forbidden,
            AppError::Conflict(_) => ErrorCode::Conflict,
            AppError::JwtError(_) => ErrorCode::JwtError,
            AppError::MqttError(_) => ErrorCode::MqttError,
            AppError::QueueError(_) => ErrorCode::QueueError,
        }
    }

    pub fn status_code(&self) -> StatusCode {
        match self {
            AppError::DatabaseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::RedisError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::AuthError(_) => StatusCode::UNAUTHORIZED,
            AppError::ValidationError(_) => StatusCode::BAD_REQUEST,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            AppError::Forbidden(_) => StatusCode::FORBIDDEN,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::JwtError(_) => StatusCode::UNAUTHORIZED,
            AppError::MqttError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::QueueError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn to_api_error(&self) -> ApiError {
        let error_code = self.error_code().as_str();
        let message = self.to_string();

        // Add additional details for specific errors
        match self {
            AppError::ValidationError(msg) => {
                ApiError::with_details(
                    message,
                    error_code,
                    json!({ "validation_errors": msg }),
                )
            }
            _ => ApiError::new(message, error_code),
        }
    }
}

// Implement IntoResponse for AppError
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        tracing::error!("Application error: {:?}", self);

        let api_error = self.to_api_error();
        api_error.into_response()
    }
}

// Implement From for redis errors
impl From<redis::RedisError> for AppError {
    fn from(err: redis::RedisError) -> Self {
        AppError::RedisError(err.to_string())
    }
}

// Implement From for deadpool errors
impl From<deadpool_redis::PoolError> for AppError {
    fn from(err: deadpool_redis::PoolError) -> Self {
        AppError::RedisError(err.to_string())
    }
}

// Result type alias
pub type AppResult<T> = Result<T, AppError>;
