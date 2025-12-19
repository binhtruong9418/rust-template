use async_trait::async_trait;
use deadpool_redis::{Connection, Pool};
use deadpool_redis::redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};

use crate::config::RedisConfig;
use crate::interceptors::AppError;
use super::job::{Job, JobId, JobStatus, JobResult};

type JobHandler<T, R> = Arc<dyn Fn(Job<T>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<R, AppError>> + Send>> + Send + Sync>;

/// Queue Service - similar to BeeQueue in Node.js
pub struct QueueService<T, R>
where
    T: Serialize + for<'de> Deserialize<'de> + Clone + Send + Sync + 'static,
    R: Serialize + for<'de> Deserialize<'de> + Clone + Send + Sync + 'static,
{
    queue_name: String,
    redis_pool: Pool,
    max_retries: u32,
    is_worker: bool,
    remove_on_success: bool,
    remove_on_failure: bool,
    delayed_debounce: u64,
    processing: Arc<RwLock<bool>>,
    _phantom_t: std::marker::PhantomData<T>,
    _phantom_r: std::marker::PhantomData<R>,
}

impl<T, R> QueueService<T, R>
where
    T: Serialize + for<'de> Deserialize<'de> + Clone + Send + Sync + 'static,
    R: Serialize + for<'de> Deserialize<'de> + Clone + Send + Sync + 'static,
{
    /// Create a new QueueService instance
    pub async fn new(name: &str, retries: u32) -> Result<Self, AppError> {
        dotenv::dotenv().ok();

        let environment = std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string());
        let queue_name = format!("rust_template_{}_queue_{}", name, environment);

        // Create Redis pool
        let redis_config = RedisConfig::from_env()
            .map_err(|e| AppError::RedisError(format!("Failed to load Redis config: {}", e)))?;

        let redis_pool = redis_config.create_pool()
            .map_err(|e| AppError::RedisError(format!("Failed to create Redis pool: {}", e)))?;

        let service = Self {
            queue_name,
            redis_pool,
            max_retries: retries,
            is_worker: true,
            remove_on_success: true,
            remove_on_failure: false,
            delayed_debounce: 1000, // 1 second
            processing: Arc::new(RwLock::new(false)),
            _phantom_t: std::marker::PhantomData,
            _phantom_r: std::marker::PhantomData,
        };

        service.initialize().await?;

        Ok(service)
    }

    /// Initialize the queue
    async fn initialize(&self) -> Result<(), AppError> {
        // Test Redis connection
        let mut conn = self.get_connection().await?;
        let _: String = redis::cmd("PING")
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::RedisError(format!("Redis connection failed: {}", e)))?;

        tracing::info!("Queue '{}' initialized successfully", self.queue_name);
        Ok(())
    }

    /// Add a job to the queue
    pub async fn add_to_queue(&self, data: T) -> Result<Job<T>, AppError> {
        let job = Job::new(data, self.max_retries)
            .with_id(chrono::Utc::now().timestamp_millis().to_string())
            .with_timeout(60000)
            .with_backoff_delay(2000);

        let mut conn = self.get_connection().await?;

        // Serialize job
        let job_json = serde_json::to_string(&job)
            .map_err(|e| AppError::QueueError(format!("Failed to serialize job: {}", e)))?;

        // Add to pending queue (RPUSH to maintain order)
        let pending_key = format!("{}:pending", self.queue_name);
        conn.rpush::<_, _, ()>(&pending_key, &job_json)
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))?;

        // Store job data for tracking
        let job_key = format!("{}:job:{}", self.queue_name, job.id);
        conn.set_ex::<_, _, ()>(&job_key, &job_json, 86400) // 24 hours expiration
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))?;

        tracing::debug!("Job {} added to queue '{}'", job.id, self.queue_name);

        Ok(job)
    }

    /// Process jobs from the queue
    pub async fn handle_process_queue<F, Fut>(&self, handler: F) -> Result<(), AppError>
    where
        F: Fn(Job<T>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<R, AppError>> + Send + 'static,
    {
        if !self.is_worker {
            return Err(AppError::QueueError("Queue is not configured as worker".to_string()));
        }

        let processing = self.processing.clone();
        let queue_name = self.queue_name.clone();
        let redis_pool = self.redis_pool.clone();
        let remove_on_success = self.remove_on_success;
        let remove_on_failure = self.remove_on_failure;
        let delayed_debounce = self.delayed_debounce;

        // Spawn worker task
        tokio::spawn(async move {
            tracing::info!("Worker started for queue '{}'", queue_name);

            loop {
                // Check if already processing
                {
                    let is_processing = processing.read().await;
                    if *is_processing {
                        sleep(Duration::from_millis(100)).await;
                        continue;
                    }
                }

                // Try to get a job
                let job_result = Self::fetch_job(&redis_pool, &queue_name).await;

                match job_result {
                    Ok(Some(mut job)) => {
                        // Mark as processing
                        {
                            let mut is_processing = processing.write().await;
                            *is_processing = true;
                        }

                        tracing::debug!("Processing job {} from queue '{}'", job.id, queue_name);
                        job.mark_processing();

                        // Process the job
                        let result = handler(job.clone()).await;

                        match result {
                            Ok(res) => {
                                job.mark_completed();
                                tracing::info!("Job {} completed successfully", job.id);

                                // Remove from queue if configured
                                if remove_on_success {
                                    let _ = Self::remove_job(&redis_pool, &queue_name, &job.id).await;
                                }
                            }
                            Err(e) => {
                                let error_msg = e.to_string();
                                tracing::error!("Job {} failed: {}", job.id, error_msg);

                                // Check if can retry
                                if job.can_retry() {
                                    job.increment_retry();
                                    tracing::info!("Retrying job {} (attempt {}/{})", job.id, job.retries, job.max_retries);

                                    // Calculate backoff delay
                                    let backoff_delay = job.calculate_backoff();
                                    sleep(Duration::from_millis(backoff_delay)).await;

                                    // Re-add to queue
                                    let _ = Self::requeue_job(&redis_pool, &queue_name, &job).await;
                                } else {
                                    job.mark_failed(error_msg);
                                    tracing::error!("Job {} failed permanently after {} retries", job.id, job.retries);

                                    // Remove from queue if configured
                                    if remove_on_failure {
                                        let _ = Self::remove_job(&redis_pool, &queue_name, &job.id).await;
                                    }
                                }
                            }
                        }

                        // Mark as not processing
                        {
                            let mut is_processing = processing.write().await;
                            *is_processing = false;
                        }

                        // Debounce delay
                        sleep(Duration::from_millis(delayed_debounce)).await;
                    }
                    Ok(None) => {
                        // No jobs available, wait before checking again
                        sleep(Duration::from_millis(delayed_debounce)).await;
                    }
                    Err(e) => {
                        tracing::error!("Error fetching job from queue '{}': {}", queue_name, e);
                        sleep(Duration::from_millis(delayed_debounce * 2)).await;
                    }
                }
            }
        });

        Ok(())
    }

    /// Get job result (wait for completion)
    pub async fn get_job_result(&self, job_id: &str, timeout_secs: u64) -> Result<JobResult<R>, AppError> {
        let start = std::time::Instant::now();
        let timeout = Duration::from_secs(timeout_secs);

        loop {
            if start.elapsed() > timeout {
                return Err(AppError::QueueError(format!("Job {} timed out", job_id)));
            }

            let job_key = format!("{}:job:{}", self.queue_name, job_id);
            let mut conn = self.get_connection().await?;

            let job_data: Option<String> = conn.get(&job_key)
                .await
                .map_err(|e| AppError::RedisError(e.to_string()))?;

            if let Some(job_json) = job_data {
                let job: Job<T> = serde_json::from_str(&job_json)
                    .map_err(|e| AppError::QueueError(format!("Failed to deserialize job: {}", e)))?;

                match job.status {
                    JobStatus::Completed => {
                        // Try to get result from results key
                        let result_key = format!("{}:result:{}", self.queue_name, job_id);
                        let result_data: Option<String> = conn.get(&result_key)
                            .await
                            .map_err(|e| AppError::RedisError(e.to_string()))?;

                        let result = result_data.and_then(|r| serde_json::from_str(&r).ok());

                        return Ok(JobResult {
                            job_id: job.id,
                            status: JobStatus::Completed,
                            result,
                            error: None,
                        });
                    }
                    JobStatus::Failed => {
                        return Ok(JobResult {
                            job_id: job.id,
                            status: JobStatus::Failed,
                            result: None,
                            error: job.error,
                        });
                    }
                    _ => {
                        // Still processing, wait and check again
                        sleep(Duration::from_millis(500)).await;
                    }
                }
            } else {
                return Err(AppError::QueueError(format!("Job {} not found", job_id)));
            }
        }
    }

    // Helper methods

    async fn get_connection(&self) -> Result<Connection, AppError> {
        self.redis_pool.get().await.map_err(|e| AppError::RedisError(e.to_string()))
    }

    async fn fetch_job(pool: &Pool, queue_name: &str) -> Result<Option<Job<T>>, AppError> {
        let mut conn = pool.get().await.map_err(|e| AppError::RedisError(e.to_string()))?;

        let pending_key = format!("{}:pending", queue_name);

        // LPOP to get oldest job (FIFO)
        let job_data: Option<String> = conn.lpop(&pending_key, None)
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))?;

        if let Some(job_json) = job_data {
            let job: Job<T> = serde_json::from_str(&job_json)
                .map_err(|e| AppError::QueueError(format!("Failed to deserialize job: {}", e)))?;

            Ok(Some(job))
        } else {
            Ok(None)
        }
    }

    async fn requeue_job(pool: &Pool, queue_name: &str, job: &Job<T>) -> Result<(), AppError> {
        let mut conn = pool.get().await.map_err(|e| AppError::RedisError(e.to_string()))?;

        let job_json = serde_json::to_string(job)
            .map_err(|e| AppError::QueueError(format!("Failed to serialize job: {}", e)))?;

        let pending_key = format!("{}:pending", queue_name);
        conn.rpush::<_, _, ()>(&pending_key, &job_json)
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))?;

        // Update job data
        let job_key = format!("{}:job:{}", queue_name, job.id);
        conn.set_ex::<_, _, ()>(&job_key, &job_json, 86400)
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))?;

        Ok(())
    }

    async fn remove_job(pool: &Pool, queue_name: &str, job_id: &str) -> Result<(), AppError> {
        let mut conn = pool.get().await.map_err(|e| AppError::RedisError(e.to_string()))?;

        let job_key = format!("{}:job:{}", queue_name, job_id);
        conn.del::<_, ()>(&job_key)
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))?;

        Ok(())
    }
}

// Implement Clone manually since derive doesn't work well with PhantomData
impl<T, R> Clone for QueueService<T, R>
where
    T: Serialize + for<'de> Deserialize<'de> + Clone + Send + Sync + 'static,
    R: Serialize + for<'de> Deserialize<'de> + Clone + Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Self {
            queue_name: self.queue_name.clone(),
            redis_pool: self.redis_pool.clone(),
            max_retries: self.max_retries,
            is_worker: self.is_worker,
            remove_on_success: self.remove_on_success,
            remove_on_failure: self.remove_on_failure,
            delayed_debounce: self.delayed_debounce,
            processing: self.processing.clone(),
            _phantom_t: std::marker::PhantomData,
            _phantom_r: std::marker::PhantomData,
        }
    }
}
