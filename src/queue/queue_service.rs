use anyhow::{Context, Result};
use once_cell::sync::OnceCell;
use redis::{aio::ConnectionManager, AsyncCommands};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::time::{sleep, timeout, Duration};
use uuid::Uuid;

use crate::interceptors::AppError;

// Global queue manager
static QUEUE_MANAGER: OnceCell<QueueManager> = OnceCell::new();

/// Job structure for queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueJob<T>
where
    T: Clone,
{
    pub id: String,
    pub data: T,
    pub attempts: u32,
    pub max_retries: u32,
    pub timeout_ms: u64,
    pub backoff_ms: u64,
    pub created_at: i64,
}

impl<T> QueueJob<T>
where
    T: Serialize + Clone,
{
    pub fn new(data: T, max_retries: u32, timeout_ms: u64) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            data,
            attempts: 0,
            max_retries,
            timeout_ms,
            backoff_ms: 2000, // 2 seconds base
            created_at: chrono::Utc::now().timestamp(),
        }
    }
}

/// Queue configuration
#[derive(Debug, Clone)]
pub struct QueueConfig {
    pub redis_url: String,
    pub environment: String,
    pub remove_on_success: bool,
    pub remove_on_failure: bool,
}

impl QueueConfig {
    pub fn new(redis_url: String, environment: String) -> Self {
        Self {
            redis_url,
            environment,
            remove_on_success: true,
            remove_on_failure: false,
        }
    }
}

/// Queue statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct QueueStats {
    pub waiting: usize,
    pub processing: usize,
    pub succeeded: usize,
    pub failed: usize,
}

/// Global Queue Manager
#[derive(Clone)]
pub struct QueueManager {
    config: Arc<QueueConfig>,
    client: redis::Client,
}

impl QueueManager {
    /// Initialize global queue manager
    pub fn init(config: QueueConfig) -> Result<(), AppError> {
        let client = redis::Client::open(config.redis_url.as_str())
            .map_err(|e| AppError::RedisError(format!("Failed to create Redis client: {}", e)))?;

        let manager = QueueManager {
            config: Arc::new(config),
            client,
        };

        QUEUE_MANAGER
            .set(manager)
            .map_err(|_| AppError::RedisError("Queue manager already initialized".to_string()))?;

        tracing::info!("✅ Queue manager initialized");
        Ok(())
    }

    /// Get global instance
    pub fn global() -> &'static QueueManager {
        QUEUE_MANAGER
            .get()
            .expect("Queue manager not initialized. Call QueueManager::init() first")
    }

    /// Create a queue service instance
    pub fn create_queue(&self, name: &str, max_retries: u32) -> QueueService {
        let queue_name = format!("{}_{}_queue", self.config.environment, name);

        QueueService {
            queue_name,
            max_retries,
            manager: self.clone(),
        }
    }

    /// Create a connection with timeout
    async fn get_connection(&self) -> Result<ConnectionManager, AppError> {
        let connection_future = ConnectionManager::new(self.client.clone());
        
        timeout(Duration::from_secs(3), connection_future)
            .await
            .map_err(|_| AppError::RedisError("Redis connection timeout after 3 seconds".to_string()))?
            .map_err(|e| AppError::RedisError(format!("Failed to get Redis connection: {}", e)))
    }

    /// Health check for Redis connection
    async fn health_check(&self) -> Result<bool, AppError> {
        match timeout(Duration::from_secs(2), async {
            let mut conn = ConnectionManager::new(self.client.clone()).await?;
            let _: Option<String> = conn.get("__health_check_key__").await?;
            Ok::<(), redis::RedisError>(())
        }).await {
            Ok(Ok(_)) => Ok(true),
            Ok(Err(_)) | Err(_) => Ok(false),
        }
    }

    /// Get queue statistics with timeout
    async fn get_stats(&self, queue_name: &str) -> Result<QueueStats, AppError> {
        let result = timeout(Duration::from_secs(3), async {
            let mut conn = self.get_connection().await?;

            let waiting_key = format!("{}:waiting", queue_name);
            let processing_key = format!("{}:processing", queue_name);
            let succeeded_key = format!("{}:succeeded", queue_name);
            let failed_key = format!("{}:failed", queue_name);

            let waiting: usize = conn.llen(&waiting_key).await.unwrap_or(0);
            let processing: usize = conn.llen(&processing_key).await.unwrap_or(0);
            let succeeded: usize = conn.llen(&succeeded_key).await.unwrap_or(0);
            let failed: usize = conn.llen(&failed_key).await.unwrap_or(0);

            Ok::<QueueStats, AppError>(QueueStats {
                waiting,
                processing,
                succeeded,
                failed,
            })
        }).await;

        match result {
            Ok(stats) => stats,
            Err(_) => Err(AppError::RedisError(format!("Timeout getting stats for queue '{}'", queue_name))),
        }
    }
}

/// Queue Service - Optimized BeeQueue pattern
#[derive(Clone)]
pub struct QueueService {
    queue_name: String,
    max_retries: u32,
    manager: QueueManager,
}

impl QueueService {
    /// Add job to queue with fast fail on Redis error
    pub async fn add_to_queue<T>(&self, data: T) -> Result<String, AppError>
    where
        T: Serialize + Clone,
    {
        // Fast health check before attempting to add job
        if !self.manager.health_check().await? {
            return Err(AppError::RedisError("Redis is not available. Job cannot be added to queue.".to_string()));
        }

        let job = QueueJob::new(data, self.max_retries, 60000); // 60s timeout
        let job_id = job.id.clone();
        let job_json = serde_json::to_string(&job)
            .map_err(|e| AppError::QueueError(format!("Failed to serialize job: {}", e)))?;

        // Wrap Redis operations with timeout
        let result = timeout(Duration::from_secs(5), async {
            let mut conn = self.manager.get_connection().await?;

            // Store job data with TTL (24 hours)
            let job_key = format!("{}:job:{}", self.queue_name, job_id);
            conn.set_ex::<_, _, ()>(&job_key, &job_json, 86400).await?;

            // Push to waiting list
            let waiting_key = format!("{}:waiting", self.queue_name);
            conn.rpush::<_, _, ()>(&waiting_key, &job_json).await?;

            Ok::<(), AppError>(())
        }).await;

        match result {
            Ok(Ok(_)) => {
                tracing::debug!("Job {} added to queue '{}'", job_id, self.queue_name);
                Ok(job_id)
            }
            Ok(Err(e)) => Err(e),
            Err(_) => Err(AppError::RedisError(format!("Timeout adding job {} to queue '{}'", job_id, self.queue_name))),
        }
    }

    /// Process queue with handler
    pub async fn handle_process_queue<T, F, Fut>(&self, handler: F) -> Result<(), AppError>
    where
        T: for<'de> Deserialize<'de> + Serialize + Clone + Send + Sync + 'static,
        F: Fn(QueueJob<T>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<(), AppError>> + Send + 'static,
    {
        let handler = Arc::new(handler);
        let queue_name = self.queue_name.clone();
        let manager = self.manager.clone();
        let waiting_key = format!("{}:waiting", queue_name);
        let processing_key = format!("{}:processing", queue_name);

        tokio::spawn(async move {
            tracing::info!("🚀 Worker started for queue: {}", queue_name);

            loop {
                // Check Redis health before attempting connection
                if !manager.health_check().await.unwrap_or(false) {
                    tracing::warn!("Queue {} Redis health check failed, waiting 10 seconds...", queue_name);
                    sleep(Duration::from_secs(10)).await;
                    continue;
                }

                let mut conn = match manager.get_connection().await {
                    Ok(c) => c,
                    Err(_) => {
                        sleep(Duration::from_secs(5)).await;
                        continue;
                    }
                };

                // Move job from waiting to processing (BRPOPLPUSH with 5s timeout)
                let result: Result<Option<String>, _> =
                    conn.brpoplpush(&waiting_key, &processing_key, 5.0).await;

                match result {
                    Ok(Some(job_json)) => {
                        let mut job: QueueJob<T> = match serde_json::from_str(&job_json) {
                            Ok(j) => j,
                            Err(_) => continue,
                        };

                        tracing::debug!("Processing job: {} in queue '{}'", job.id, queue_name);
                        job.attempts += 1;

                        let handler_clone = Arc::clone(&handler);
                        let job_clone = job.clone();

                        // Execute handler with timeout
                        let timeout_duration = Duration::from_millis(job.timeout_ms);
                        let result = tokio::time::timeout(
                            timeout_duration,
                            handler_clone(job_clone),
                        )
                        .await;

                        match result {
                            Ok(Ok(_)) => {
                                if let Err(e) = Self::handle_success(&manager, &queue_name, &job, &processing_key).await {
                                    tracing::error!("Error handling success: {}", e);
                                }
                            }
                            Ok(Err(e)) => {
                                tracing::debug!("Job {} failed: {}", job.id, e);
                                if let Err(err) = Self::handle_failure(&manager, &queue_name, job, &processing_key, &waiting_key).await {
                                    tracing::error!("Error handling failure: {}", err);
                                }
                            }
                            Err(_) => {
                                tracing::debug!("Job {} timed out", job.id);
                                if let Err(err) = Self::handle_failure(&manager, &queue_name, job, &processing_key, &waiting_key).await {
                                    tracing::error!("Error handling timeout: {}", err);
                                }
                            }
                        }
                    }
                    Ok(None) => {
                        // No job available, small sleep
                        sleep(Duration::from_millis(100)).await;
                    }
                    Err(_) => {
                        sleep(Duration::from_secs(5)).await;
                    }
                }
            }
        });

        Ok(())
    }

    async fn handle_success<T>(
        manager: &QueueManager,
        queue_name: &str,
        job: &QueueJob<T>,
        processing_key: &str,
    ) -> Result<(), AppError>
    where
        T: Serialize + Clone,
    {
        let job_json = serde_json::to_string(job)
            .map_err(|e| AppError::QueueError(format!("Failed to serialize job: {}", e)))?;
        
        let result = timeout(Duration::from_secs(3), async {
            let mut conn = manager.get_connection().await?;

            // Remove from processing
            conn.lrem::<_, _, ()>(processing_key, 1, &job_json).await?;

            if manager.config.remove_on_success {
                // Remove job data
                let job_key = format!("{}:job:{}", queue_name, job.id);
                conn.del::<_, ()>(&job_key).await?;
            } else {
                // Move to succeeded list
                let succeeded_key = format!("{}:succeeded", queue_name);
                conn.lpush::<_, _, ()>(&succeeded_key, &job_json).await?;
            }

            Ok::<(), AppError>(())
        }).await;

        match result {
            Ok(res) => res,
            Err(_) => Err(AppError::RedisError(format!("Timeout handling success for job {}", job.id))),
        }
    }

    async fn handle_failure<T>(
        manager: &QueueManager,
        queue_name: &str,
        job: QueueJob<T>,
        processing_key: &str,
        waiting_key: &str,
    ) -> Result<(), AppError>
    where
        T: Serialize + Clone,
    {
        let job_json = serde_json::to_string(&job)
            .map_err(|e| AppError::QueueError(format!("Failed to serialize job: {}", e)))?;
        
        let result = timeout(Duration::from_secs(3), async {
            let mut conn = manager.get_connection().await?;

            // Remove from processing
            conn.lrem::<_, _, ()>(processing_key, 1, &job_json).await?;

            if job.attempts < job.max_retries {
                // Calculate exponential backoff
                let backoff = job.backoff_ms * (2_u64.pow(job.attempts - 1));
                tracing::debug!("Retrying job {} (attempt {}/{}) after {} ms", job.id, job.attempts, job.max_retries, backoff);

                sleep(Duration::from_millis(backoff)).await;

                // Re-queue job
                let updated_job_json = serde_json::to_string(&job)?;
                conn.lpush::<_, _, ()>(waiting_key, &updated_job_json).await?;
            } else {
                tracing::debug!("Job {} failed permanently after {} attempts", job.id, job.attempts);

                if manager.config.remove_on_failure {
                    // Remove job data
                    let job_key = format!("{}:job:{}", queue_name, job.id);
                    conn.del::<_, ()>(&job_key).await?;
                } else {
                    // Move to failed list
                    let failed_key = format!("{}:failed", queue_name);
                    conn.lpush::<_, _, ()>(&failed_key, &job_json).await?;
                }
            }

            Ok::<(), AppError>(())
        }).await;

        match result {
            Ok(res) => res,
            Err(_) => Err(AppError::RedisError(format!("Timeout handling failure for job {}", job.id))),
        }
    }

    /// Get queue stats with fast fail
    pub async fn get_stats(&self) -> Result<QueueStats, AppError> {
        if !self.manager.health_check().await? {
            return Err(AppError::RedisError("Redis is not available. Cannot get queue stats.".to_string()));
        }
        
        self.manager.get_stats(&self.queue_name).await
    }

    /// Get queue name
    pub fn get_name(&self) -> &str {
        &self.queue_name
    }
}