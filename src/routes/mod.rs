use axum::{
    middleware,
    routing::{delete, get, post, put},
    Router,
};
use sqlx::PgPool;

use crate::handlers::{delete_user, get_user, health_check, login, register, update_user};
use crate::middleware::JwtMiddleware;

/// Create API router
pub fn create_router(pool: PgPool) -> Router {
    // Health check route (outside /api)
    let health_routes = Router::new()
        .route("/health", get(health_check));

    // Public API routes (no authentication required)
    let public_routes = Router::new()
        .route("/auth/register", post(register))
        .route("/auth/login", post(login));

    // Protected API routes (authentication required)
    let protected_routes = Router::new()
        .route("/user", get(get_user))
        .route("/user", put(update_user))
        .route("/user", delete(delete_user))
        .route_layer(middleware::from_fn(JwtMiddleware::auth));

    // Combine routes
    Router::new()
        .merge(health_routes)  // Health check at /health
        .nest("/api", Router::new()
            .merge(public_routes)
            .merge(protected_routes)
        )
        .with_state(pool)
}
