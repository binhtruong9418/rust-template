use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
    http::{header, StatusCode},
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use chrono::{Duration, Utc};

use crate::interceptors::AppError;

/// JWT Claims structure - contains user id and email
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub id: String,
    pub email: String,
    pub exp: i64,
    pub iat: i64,
}

impl Claims {
    /// Create new claims with id and email
    pub fn new(id: String, email: String, expiration_hours: i64) -> Self {
        let iat = Utc::now();
        let exp = iat + Duration::hours(expiration_hours);

        Self {
            id,
            email,
            exp: exp.timestamp(),
            iat: iat.timestamp(),
        }
    }

    /// Create claims from environment expiration (in seconds)
    pub fn with_env_expiration(id: String, email: String) -> Self {
        let expiration_seconds = std::env::var("JWT_EXPIRATION")
            .unwrap_or_else(|_| "86400".to_string())
            .parse::<i64>()
            .unwrap_or(86400);

        let iat = Utc::now();
        let exp = iat + Duration::seconds(expiration_seconds);

        Self {
            id,
            email,
            exp: exp.timestamp(),
            iat: iat.timestamp(),
        }
    }
}

#[derive(Clone)]
pub struct JwtConfig {
    pub secret: String,
}

impl JwtConfig {
    pub fn from_env() -> Result<Self, AppError> {
        dotenv::dotenv().ok();

        let secret = std::env::var("JWT_SECRET")
            .map_err(|_| AppError::InternalError("JWT_SECRET not found in environment".to_string()))?;

        if secret.is_empty() {
            return Err(AppError::InternalError("JWT_SECRET cannot be empty".to_string()));
        }

        Ok(Self { secret })
    }
}

/// Generate JWT token from claims
pub fn generate_token(claims: &Claims) -> Result<String, AppError> {
    let jwt_config = JwtConfig::from_env()?;

    encode(
        &Header::default(),
        claims,
        &EncodingKey::from_secret(jwt_config.secret.as_bytes()),
    )
    .map_err(|e| AppError::JwtError(e))
}

/// Verify and decode JWT token
pub fn verify_token(token: &str) -> Result<Claims, AppError> {
    let jwt_config = JwtConfig::from_env()?;

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_config.secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| {
        tracing::warn!("JWT verification failed: {}", e);
        match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                AppError::Unauthorized("Token expired".to_string())
            }
            _ => AppError::Unauthorized("Invalid token".to_string()),
        }
    })?;

    Ok(token_data.claims)
}

/// JWT Authentication Middleware
pub struct JwtMiddleware;

impl JwtMiddleware {
    pub async fn auth(
        mut request: Request,
        next: Next,
    ) -> Result<Response, AppError> {
        // Extract token from Authorization header
        let auth_header = request
            .headers()
            .get(header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| AppError::Unauthorized("Missing authorization header".to_string()))?;

        // Check if it starts with "Bearer "
        if !auth_header.starts_with("Bearer ") {
            return Err(AppError::Unauthorized("Invalid authorization header format".to_string()));
        }

        // Extract the token
        let token = auth_header.trim_start_matches("Bearer ");

        // Verify token
        let claims = verify_token(token)?;

        // Add claims to request extensions for handlers to use
        request.extensions_mut().insert(claims);

        // Continue to next middleware/handler
        Ok(next.run(request).await)
    }
}

// Helper to extract claims from request extensions in handlers
pub trait ClaimsExtractor {
    fn get_claims(&self) -> Result<Claims, AppError>;
}

impl ClaimsExtractor for Request {
    fn get_claims(&self) -> Result<Claims, AppError> {
        self.extensions()
            .get::<Claims>()
            .cloned()
            .ok_or_else(|| AppError::Unauthorized("Claims not found in request".to_string()))
    }
}
