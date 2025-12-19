use axum::{extract::State, Json};

use crate::config::AppState;
use crate::dto::{CreateUserRequest, LoginRequest, LoginResponse, RegisterResponse};
use crate::interceptors::{ApiSuccess, AppError};
use crate::services::{UserService, EmailService};

/// Register a new user
pub async fn register(
    State(state): State<AppState>,
    Json(request): Json<CreateUserRequest>,
) -> Result<ApiSuccess<RegisterResponse>, AppError> {
    let email_service = EmailService::new(state.clone());
    let user_service = UserService::new_with_email(state.clone(), email_service);
    let response = user_service.register(request).await?;

    Ok(ApiSuccess::new("User registered successfully", response))
}

/// Login a user
pub async fn login(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> Result<ApiSuccess<LoginResponse>, AppError> {
    let user_service = UserService::new(state.clone());
    let response = user_service.login(request).await?;

    Ok(ApiSuccess::new("Login successful", response))
}
