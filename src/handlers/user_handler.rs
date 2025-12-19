use axum::{
    extract::State,
    Extension,
    Json,
};
use sqlx::PgPool;

use crate::dto::{UpdateUserRequest, UserResponse};
use crate::interceptors::{ApiSuccess, AppError};
use crate::middleware::Claims;
use crate::services::UserService;

/// Get current user (from JWT token)
pub async fn get_user(
    State(pool): State<PgPool>,
    Extension(claims): Extension<Claims>,
) -> Result<ApiSuccess<UserResponse>, AppError> {
    let user_service = UserService::new(pool);
    let user = user_service.get_user_by_id(&claims.id).await?;

    Ok(ApiSuccess::new("User retrieved successfully", user))
}

/// Update user
pub async fn update_user(
    State(pool): State<PgPool>,
    Extension(claims): Extension<Claims>,
    Json(update_request): Json<UpdateUserRequest>,
) -> Result<ApiSuccess<UserResponse>, AppError> {
    let user_service = UserService::new(pool);
    let updated_user = user_service.update_user(&claims.id, update_request).await?;

    Ok(ApiSuccess::new("User updated successfully", updated_user))
}

/// Delete user
pub async fn delete_user(
    State(pool): State<PgPool>,
    Extension(claims): Extension<Claims>,
) -> Result<ApiSuccess<()>, AppError> {
    let user_service = UserService::new(pool);
    user_service.delete_user(&claims.id).await?;

    Ok(ApiSuccess::<()>::new_without_data("User deleted successfully"))
}
