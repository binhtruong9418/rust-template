use deadpool_redis::{Connection, Pool};
use deadpool_redis::redis::AsyncCommands;
use serde::{Deserialize, Serialize};

use crate::config::RedisConfig;
use crate::interceptors::AppError;

#[derive(Clone)]
pub struct RedisService {
    pool: Pool,
}

impl RedisService {
    /// Create a new RedisService instance
    pub async fn new() -> Result<Self, AppError> {
        let config = RedisConfig::from_env()
            .map_err(|e| AppError::RedisError(format!("Failed to load Redis config: {}", e)))?;

        let pool = config.create_pool()
            .map_err(|e| AppError::RedisError(format!("Failed to create Redis pool: {}", e)))?;

        // Test connection
        let mut conn = pool.get().await
            .map_err(|e| AppError::RedisError(format!("Failed to get Redis connection: {}", e)))?;

        let _: String = redis::cmd("PING")
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::RedisError(format!("Redis ping failed: {}", e)))?;

        tracing::info!("Redis service initialized successfully");

        Ok(Self { pool })
    }

    /// Get a connection from the pool
    pub async fn get_connection(&self) -> Result<Connection, AppError> {
        self.pool.get().await
            .map_err(|e| AppError::RedisError(format!("Failed to get connection: {}", e)))
    }

    /// Set a key-value pair
    pub async fn set(&self, key: &str, value: &str) -> Result<(), AppError> {
        let mut conn = self.get_connection().await?;
        conn.set(key, value)
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))
    }

    /// Set a key-value pair with expiration (in seconds)
    pub async fn set_ex(&self, key: &str, value: &str, seconds: i64) -> Result<(), AppError> {
        let mut conn = self.get_connection().await?;
        conn.set_ex(key, value, seconds as u64)
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))
    }

    /// Get a value by key
    pub async fn get(&self, key: &str) -> Result<Option<String>, AppError> {
        let mut conn = self.get_connection().await?;
        conn.get(key)
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))
    }

    /// Delete a key
    pub async fn del(&self, key: &str) -> Result<(), AppError> {
        let mut conn = self.get_connection().await?;
        conn.del(key)
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))
    }

    /// Check if a key exists
    pub async fn exists(&self, key: &str) -> Result<bool, AppError> {
        let mut conn = self.get_connection().await?;
        conn.exists(key)
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))
    }

    /// Set expiration on a key (in seconds)
    pub async fn expire(&self, key: &str, seconds: i64) -> Result<(), AppError> {
        let mut conn = self.get_connection().await?;
        conn.expire(key, seconds)
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))
    }

    /// Increment a key
    pub async fn incr(&self, key: &str) -> Result<i64, AppError> {
        let mut conn = self.get_connection().await?;
        conn.incr(key, 1)
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))
    }

    /// Decrement a key
    pub async fn decr(&self, key: &str) -> Result<i64, AppError> {
        let mut conn = self.get_connection().await?;
        conn.decr(key, 1)
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))
    }

    // JSON helpers

    /// Set a JSON value
    pub async fn set_json<T: Serialize>(&self, key: &str, value: &T) -> Result<(), AppError> {
        let json = serde_json::to_string(value)
            .map_err(|e| AppError::RedisError(format!("Failed to serialize JSON: {}", e)))?;

        self.set(key, &json).await
    }

    /// Set a JSON value with expiration
    pub async fn set_json_ex<T: Serialize>(&self, key: &str, value: &T, seconds: i64) -> Result<(), AppError> {
        let json = serde_json::to_string(value)
            .map_err(|e| AppError::RedisError(format!("Failed to serialize JSON: {}", e)))?;

        self.set_ex(key, &json, seconds).await
    }

    /// Get a JSON value
    pub async fn get_json<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<T>, AppError> {
        let value = self.get(key).await?;

        match value {
            Some(json) => {
                let data = serde_json::from_str(&json)
                    .map_err(|e| AppError::RedisError(format!("Failed to deserialize JSON: {}", e)))?;
                Ok(Some(data))
            }
            None => Ok(None),
        }
    }

    // List operations

    /// Push to the right of a list
    pub async fn rpush(&self, key: &str, value: &str) -> Result<(), AppError> {
        let mut conn = self.get_connection().await?;
        conn.rpush(key, value)
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))
    }

    /// Push to the left of a list
    pub async fn lpush(&self, key: &str, value: &str) -> Result<(), AppError> {
        let mut conn = self.get_connection().await?;
        conn.lpush(key, value)
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))
    }

    /// Pop from the right of a list
    pub async fn rpop(&self, key: &str) -> Result<Option<String>, AppError> {
        let mut conn = self.get_connection().await?;
        conn.rpop(key, None)
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))
    }

    /// Pop from the left of a list
    pub async fn lpop(&self, key: &str) -> Result<Option<String>, AppError> {
        let mut conn = self.get_connection().await?;
        conn.lpop(key, None)
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))
    }

    /// Get list length
    pub async fn llen(&self, key: &str) -> Result<i64, AppError> {
        let mut conn = self.get_connection().await?;
        conn.llen(key)
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))
    }

    // Hash operations

    /// Set a hash field
    pub async fn hset(&self, key: &str, field: &str, value: &str) -> Result<(), AppError> {
        let mut conn = self.get_connection().await?;
        conn.hset(key, field, value)
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))
    }

    /// Get a hash field
    pub async fn hget(&self, key: &str, field: &str) -> Result<Option<String>, AppError> {
        let mut conn = self.get_connection().await?;
        conn.hget(key, field)
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))
    }

    /// Get all hash fields
    pub async fn hgetall(&self, key: &str) -> Result<std::collections::HashMap<String, String>, AppError> {
        let mut conn = self.get_connection().await?;
        conn.hgetall(key)
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))
    }

    /// Delete a hash field
    pub async fn hdel(&self, key: &str, field: &str) -> Result<(), AppError> {
        let mut conn = self.get_connection().await?;
        conn.hdel(key, field)
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))
    }

    // Cache helpers with prefix

    /// Generate a cache key with prefix
    pub fn cache_key(&self, prefix: &str, key: &str) -> String {
        format!("cache:{}:{}", prefix, key)
    }

    /// Set cache with TTL
    pub async fn cache_set(&self, prefix: &str, key: &str, value: &str, ttl_seconds: i64) -> Result<(), AppError> {
        let cache_key = self.cache_key(prefix, key);
        self.set_ex(&cache_key, value, ttl_seconds).await
    }

    /// Get from cache
    pub async fn cache_get(&self, prefix: &str, key: &str) -> Result<Option<String>, AppError> {
        let cache_key = self.cache_key(prefix, key);
        self.get(&cache_key).await
    }

    /// Delete from cache
    pub async fn cache_del(&self, prefix: &str, key: &str) -> Result<(), AppError> {
        let cache_key = self.cache_key(prefix, key);
        self.del(&cache_key).await
    }

    /// Set JSON cache with TTL
    pub async fn cache_set_json<T: Serialize>(&self, prefix: &str, key: &str, value: &T, ttl_seconds: i64) -> Result<(), AppError> {
        let cache_key = self.cache_key(prefix, key);
        self.set_json_ex(&cache_key, value, ttl_seconds).await
    }

    /// Get JSON from cache
    pub async fn cache_get_json<T: for<'de> Deserialize<'de>>(&self, prefix: &str, key: &str) -> Result<Option<T>, AppError> {
        let cache_key = self.cache_key(prefix, key);
        self.get_json(&cache_key).await
    }
}
