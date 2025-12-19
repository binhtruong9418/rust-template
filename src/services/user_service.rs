use tracing::warn;

use crate::config::AppState;
use crate::dto::{CreateUserRequest, LoginRequest, LoginResponse, RegisterResponse, UpdateUserRequest, UserResponse};
use crate::interceptors::AppError;
use crate::middleware::{Claims, generate_token};
use crate::models::User;
use crate::services::EmailService;
use crate::utils::{hash_password, validate_request, verify_password};

#[derive(Clone)]
pub struct UserService {
    state: AppState,
    email_service: Option<EmailService>,
}

impl UserService {
    pub fn new(state: AppState) -> Self {
        Self { state, email_service: None }
    }

    pub fn new_with_email(state: AppState, email_service: EmailService) -> Self {
        Self { state, email_service: Some(email_service) }
    }

    /// Register a new user
    pub async fn register(&self, request: CreateUserRequest) -> Result<RegisterResponse, AppError> {
        // Validate request
        validate_request(&request)?;

        // Check if user already exists
        let existing_user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
            .bind(&request.email)
            .fetch_optional(&self.state.db)
            .await?;

        if existing_user.is_some() {
            return Err(AppError::Conflict("User with this email already exists".to_string()));
        }

        // Hash password
        let password_hash = hash_password(&request.password)?;

        // Create user
        let user = User::new(request.email.clone(), password_hash, request.name);

        // Insert into database
        let inserted_user = sqlx::query_as::<_, User>(
            "INSERT INTO users (id, email, password_hash, name, is_active, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             RETURNING *",
        )
        .bind(&user.id)
        .bind(&user.email)
        .bind(&user.password_hash)
        .bind(&user.name)
        .bind(user.is_active)
        .bind(user.created_at)
        .bind(user.updated_at)
        .fetch_one(&self.state.db)
        .await?;

        let user_response = inserted_user.to_response();

        // Send welcome email via queue (non-blocking)
        if let Some(email_service) = &self.email_service {
            match email_service.send_welcome_email(&user_response).await {
                Ok(job_id) => {
                    tracing::info!("📧 Welcome email job {} queued for user {}", job_id, user_response.id);
                }
                Err(e) => {
                    warn!("⚠️  Failed to queue welcome email for user {}: {}", user_response.id, e);
                    // Don't fail registration if email queueing fails
                }
            }
        }

        // Return user data only (no token for registration)
        Ok(RegisterResponse {
            user: user_response,
        })
    }

    /// Login user
    pub async fn login(&self, request: LoginRequest) -> Result<LoginResponse, AppError> {
        // Validate request
        validate_request(&request)?;

        // Find user by email
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
            .bind(&request.email)
            .fetch_optional(&self.state.db)
            .await?
            .ok_or_else(|| AppError::Unauthorized("Invalid email or password".to_string()))?;

        // Verify password
        let is_valid = verify_password(&request.password, &user.password_hash)?;

        if !is_valid {
            return Err(AppError::Unauthorized("Invalid email or password".to_string()));
        }

        // Check if user is active
        if !user.is_active {
            return Err(AppError::Forbidden("User account is disabled".to_string()));
        }

        // Generate JWT token
        let claims = Claims::with_env_expiration(user.id.clone(), user.email.clone());
        let token = generate_token(&claims)?;

        Ok(LoginResponse {
            token,
            user: user.to_response(),
        })
    }

    /// Get user by ID
    pub async fn get_user_by_id(&self, user_id: &str) -> Result<UserResponse, AppError> {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(&self.state.db)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        Ok(user.to_response())
    }

    /// Update user
    pub async fn update_user(&self, user_id: &str, request: UpdateUserRequest) -> Result<UserResponse, AppError> {
        // Validate request
        validate_request(&request)?;

        // Build dynamic update query
        let mut query = String::from("UPDATE users SET updated_at = NOW()");
        let mut params: Vec<String> = vec![];
        let mut param_count = 1;

        if let Some(email) = &request.email {
            // Check if email already exists for another user
            let existing = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1 AND id != $2")
                .bind(email)
                .bind(user_id)
                .fetch_optional(&self.state.db)
                .await?;

            if existing.is_some() {
                return Err(AppError::Conflict("Email already in use".to_string()));
            }

            query.push_str(&format!(", email = ${}", param_count));
            params.push(email.clone());
            param_count += 1;
        }

        if let Some(name) = &request.name {
            query.push_str(&format!(", name = ${}", param_count));
            params.push(name.clone());
            param_count += 1;
        }

        if let Some(is_active) = request.is_active {
            query.push_str(&format!(", is_active = ${}", param_count));
            params.push(is_active.to_string());
            param_count += 1;
        }

        query.push_str(&format!(" WHERE id = ${} RETURNING *", param_count));

        // Execute update
        let mut query_builder = sqlx::query_as::<_, User>(&query);

        for param in params {
            query_builder = query_builder.bind(param);
        }

        query_builder = query_builder.bind(user_id);

        let updated_user = query_builder.fetch_one(&self.state.db).await?;

        Ok(updated_user.to_response())
    }

    /// Delete user
    pub async fn delete_user(&self, user_id: &str) -> Result<(), AppError> {
        let result = sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(&self.state.db)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("User not found".to_string()));
        }

        Ok(())
    }
}
