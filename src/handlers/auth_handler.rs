use axum::{extract::State, Json};
use sqlx::PgPool;

use crate::dto::{CreateUserRequest, LoginRequest, LoginResponse, RegisterResponse};
use crate::interceptors::{ApiSuccess, AppError};
use crate::services::UserService;

/// Register a new user
pub async fn register(
    State(pool): State<PgPool>,
    Json(request): Json<CreateUserRequest>,
) -> Result<ApiSuccess<RegisterResponse>, AppError> {
    let user_service = UserService::new(pool);
    let response = user_service.register(request).await?;

    Ok(ApiSuccess::new("User registered successfully", response))
}

/// Login user
pub async fn login(
    State(pool): State<PgPool>,
    Json(request): Json<LoginRequest>,
) -> Result<ApiSuccess<LoginResponse>, AppError> {
    let user_service = UserService::new(pool);
    let response = user_service.login(request).await?;

    Ok(ApiSuccess::new("Login successful", response))
}
