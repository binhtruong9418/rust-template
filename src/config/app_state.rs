use std::sync::Arc;
use sqlx::PgPool;
use crate::config::AppConfig;
use crate::services::RedisService;

/// Application state shared across all handlers and services
#[derive(Debug, Clone)]
pub struct AppState {
    /// Database connection pool
    pub db: PgPool,
    /// Redis service
    pub redis: RedisService,
    /// Application configuration
    pub config: Arc<AppConfig>,
}

impl AppState {
    /// Create new AppState
    pub fn new(db: PgPool, redis: RedisService, config: AppConfig) -> Self {
        Self {
            db,
            redis,
            config: Arc::new(config),
        }
    }
}