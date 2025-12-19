use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

pub type JobId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Retrying,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job<T> {
    pub id: JobId,
    pub data: T,
    pub status: JobStatus,
    pub retries: u32,
    pub max_retries: u32,
    pub timeout: u64,
    pub backoff_delay: u64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub error: Option<String>,
}

impl<T> Job<T>
where
    T: Serialize + for<'de> Deserialize<'de> + Clone,
{
    pub fn new(data: T, max_retries: u32) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            data,
            status: JobStatus::Pending,
            retries: 0,
            max_retries,
            timeout: 60000, // 60 seconds default
            backoff_delay: 2000, // 2 seconds default
            created_at: now,
            updated_at: now,
            error: None,
        }
    }

    pub fn with_id(mut self, id: String) -> Self {
        self.id = id;
        self
    }

    pub fn with_timeout(mut self, timeout: u64) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_backoff_delay(mut self, delay: u64) -> Self {
        self.backoff_delay = delay;
        self
    }

    pub fn can_retry(&self) -> bool {
        self.retries < self.max_retries
    }

    pub fn increment_retry(&mut self) {
        self.retries += 1;
        self.status = JobStatus::Retrying;
        self.updated_at = Utc::now();
    }

    pub fn mark_processing(&mut self) {
        self.status = JobStatus::Processing;
        self.updated_at = Utc::now();
    }

    pub fn mark_completed(&mut self) {
        self.status = JobStatus::Completed;
        self.updated_at = Utc::now();
    }

    pub fn mark_failed(&mut self, error: String) {
        self.status = JobStatus::Failed;
        self.error = Some(error);
        self.updated_at = Utc::now();
    }

    /// Calculate exponential backoff delay
    pub fn calculate_backoff(&self) -> u64 {
        self.backoff_delay * 2_u64.pow(self.retries)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobResult<T> {
    pub job_id: JobId,
    pub status: JobStatus,
    pub result: Option<T>,
    pub error: Option<String>,
}

impl<T> JobResult<T> {
    pub fn success(job_id: JobId, result: T) -> Self {
        Self {
            job_id,
            status: JobStatus::Completed,
            result: Some(result),
            error: None,
        }
    }

    pub fn failed(job_id: JobId, error: String) -> Self {
        Self {
            job_id,
            status: JobStatus::Failed,
            result: None,
            error: Some(error),
        }
    }
}
