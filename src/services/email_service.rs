use serde::{Deserialize, Serialize};
use tracing::{info, error};

use crate::config::AppState;
use crate::dto::UserResponse;
use crate::interceptors::AppError;
use crate::queue::{QueueManager, QueueJob, QueueService};

/// Email job data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailJobData {
    pub to: String,
    pub subject: String,
    pub body: String,
    pub email_type: String, // "welcome", "password_reset", etc.
    pub template_data: Option<serde_json::Value>,
}

/// Optimized Email Service with automatic queue processing
#[derive(Clone)]
pub struct EmailService {
    state: AppState,
    email_queue: QueueService,
}

impl EmailService {
    /// Create new EmailService with automatic processor setup (optimized - single queue creation)
    pub fn new(state: AppState) -> Self {
        let manager = QueueManager::global();
        
        // Create queue only once
        let email_queue = manager.create_queue("email", 3);

        let service = Self {
            state,
            email_queue: email_queue.clone(),
        };

        // Attach processor to existing queue
        let service_clone = service.clone();
        email_queue.attach_processor::<EmailJobData, _, _>(
            move |job: QueueJob<EmailJobData>| {
                let service = service_clone.clone();
                async move {
                    service.process_email_job(job).await
                }
            }
        );

        service
    }

    /// Instance method for processing email jobs (can access self and state)
    async fn process_email_job(&self, job: QueueJob<EmailJobData>) -> Result<(), AppError> {
        let data = &job.data;
        info!("📧 Processing email job: {} - Type: {:?}", job.id, data.email_type);

        // Simulate processing time based on email type
        match data.email_type.as_str() {
            "welcome" => {
                info!("📬 Sending welcome email to: {}", data.to);
                info!("   Subject: {}", data.subject);
                // Simulate welcome email processing
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
            "password_reset" => {
                info!("🔐 Sending password reset email to: {}", data.to);
                info!("   Subject: {}", data.subject);
                // Simulate password reset email processing
                tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
            }
            "notification" => {
                info!("🔔 Sending notification email to: {}", data.to);
                info!("   Subject: {}", data.subject);
                // Simulate notification email processing
                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            }
            _ => {
                error!("❌ Unknown email type: {}", data.email_type);
                return Err(AppError::ValidationError(format!("Unknown email type: {}", data.email_type)));
            }
        }

        info!("✅ Email sent successfully to: {}", data.to);
        Ok(())
    }

    /// Send welcome email (adds to queue)
    pub async fn send_welcome_email(&self, user: &UserResponse) -> Result<String, AppError> {
        let email_data = EmailJobData {
            to: user.email.clone(),
            subject: "Welcome to our platform!".to_string(),
            body: format!("Hello {}, welcome to our platform!", user.name.as_deref().unwrap_or("User")),
            email_type: "welcome".to_string(),
            template_data: Some(serde_json::json!({
                "user_name": user.name,
                "user_id": user.id
            })),
        };

        let job_id = self.email_queue.add_to_queue(email_data).await?;
        info!("📧 Welcome email queued for {} (Job ID: {})", user.email, job_id);
        
        Ok(job_id)
    }

    /// Send password reset email (adds to queue)
    pub async fn send_password_reset_email(&self, email: &str, reset_token: &str) -> Result<String, AppError> {
        let email_data = EmailJobData {
            to: email.to_string(),
            subject: "Password Reset Request".to_string(),
            body: format!("Your password reset token is: {}", reset_token),
            email_type: "password_reset".to_string(),
            template_data: Some(serde_json::json!({
                "reset_token": reset_token,
                "email": email
            })),
        };

        let job_id = self.email_queue.add_to_queue(email_data).await?;
        info!("🔐 Password reset email queued for {} (Job ID: {})", email, job_id);
        
        Ok(job_id)
    }

    /// Send notification email (adds to queue)
    pub async fn send_notification_email(&self, email: &str, subject: &str, message: &str) -> Result<String, AppError> {
        let email_data = EmailJobData {
            to: email.to_string(),
            subject: subject.to_string(),
            body: message.to_string(),
            email_type: "notification".to_string(),
            template_data: Some(serde_json::json!({
                "message": message
            })),
        };

        let job_id = self.email_queue.add_to_queue(email_data).await?;
        info!("🔔 Notification email queued for {} (Job ID: {})", email, job_id);
        
        Ok(job_id)
    }
}