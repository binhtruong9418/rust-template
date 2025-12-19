# 🚀 Optimized Queue System - AI Agent Development Guide

This guide provides comprehensive instructions for AI agents working with the **optimized queue system** in this Rust backend project.

## 📋 Table of Contents

-   [Architecture Overview](#-architecture-overview)
-   [Optimized Workflow](#-optimized-workflow)
-   [API Reference](#-api-reference)
-   [Service Pattern Examples](#-service-pattern-examples)
-   [Best Practices](#-best-practices)
-   [Troubleshooting](#-troubleshooting)
-   [Migration from Legacy](#-migration-from-legacy)
-   [Instance vs Static Methods](#instance-vs-static-methods)
-   [Error Handling & Retry Logic](#error-handling--retry-logic)
-   [Testing Queue Systems](#testing-queue-systems)
-   [Common Patterns & Examples](#common-patterns--examples)
-   [Troubleshooting](#troubleshooting)

---

## 🏗️ Queue Architecture Overview

Our queue system follows a **self-contained service pattern** where each service manages its own queue processor:

```
┌─────────────────────────────────────────┐
│              Service Layer              │
│  ┌─────────────┐    ┌─────────────┐    │
│  │EmailService │    │ SmsService  │    │
│  │             │    │             │    │
│  │ ┌─────────┐ │    │ ┌─────────┐ │    │
│  │ │ Queue   │ │    │ │ Queue   │ │    │
│  │ │Processor│ │    │ │Processor│ │    │
│  │ └─────────┘ │    │ └─────────┘ │    │
│  └─────────────┘    └─────────────┘    │
└─────────────────────────────────────────┘
                    │
┌─────────────────────────────────────────┐
│            Queue Manager                │
│         (Global Singleton)              │
└─────────────────────────────────────────┘
                    │
┌─────────────────────────────────────────┐
│              Redis Backend              │
│    (Job Storage & Queue Operations)     │
└─────────────────────────────────────────┘
```

### Key Principles

1. **Self-Contained**: Each service creates and manages its own queue processor
2. **Auto-Initialization**: Queue processors start automatically when services are created
3. **Instance Methods**: Processors can access service state via `&self` (recommended)
4. **Plug-and-Play**: Just create a service to get automatic queue processing

---

## 🧩 Queue System Components

### 1. QueueManager (Global Singleton)

**Location**: `src/queue/queue_service.rs`
**Purpose**: Manages Redis connections and creates queue instances

```rust
// Initialize once in main.rs
QueueManager::init(queue_config)?;

// Use throughout the application
let manager = QueueManager::global();
```

### 2. QueueService

**Location**: `src/queue/queue_service.rs`
**Purpose**: Handles job operations (add, process, retry, fail)

### 3. QueueJob<T>

**Purpose**: Wrapper for job data with metadata (ID, retries, timeout)

```rust
pub struct QueueJob<T> {
    pub id: String,
    pub data: T,
    pub attempts: u32,
    pub max_retries: u32,
    pub timeout_ms: u64,
    pub backoff_ms: u64,
    pub created_at: i64,
}
```

---

## 🛠️ Creating New Queue Services

### Step 1: Define Job Data Structure

Create a struct that represents your job payload in the service file:

```rust
// Example: src/services/sms_service.rs

use serde::{Deserialize, Serialize};

/// SMS job data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmsJobData {
    pub to: String,
    pub message: String,
    pub sms_type: String, // "verification", "notification", etc.
    pub metadata: Option<serde_json::Value>,
}
```

### Step 2: Create Service with Auto-Processing

**Pattern**: Service creates queue with processor in constructor using **instance methods**

```rust
use crate::queue::{QueueManager, QueueJob, QueueService};

#[derive(Clone)]
pub struct SmsService {
    state: AppState,
    sms_queue: QueueService,
}

impl SmsService {
    /// Create new SmsService with automatic processor setup
    pub fn new(state: AppState) -> Self {
        let manager = QueueManager::global();

        // First create temporary service instance
        let service = Self {
            state,
            sms_queue: manager.create_queue("sms", 3), // Temporary queue
        };

        // Create service clone for the closure
        let service_clone = service.clone();

        // Create queue with processor function using instance method
        let sms_queue = manager.create_queue_with_processor::<SmsJobData, _, _>(
            "sms",           // Queue name
            3,               // Max retries
            move |job: QueueJob<SmsJobData>| {
                let service = service_clone.clone();
                async move {
                    service.process_sms_job(job).await // Instance method!
                }
            }
        );

        info!("🚀 SMS service initialized with automatic queue processing");

        Self {
            state: service.state,
            sms_queue,
        }
    }

    /// Instance method for processing SMS jobs (can access self and state)
    async fn process_sms_job(&self, job: QueueJob<SmsJobData>) -> Result<(), AppError> {
        let data = &job.data;
        info!("📱 Processing SMS job: {} - Type: {:?}", job.id, data.sms_type);

        // Can now access self.state for database operations, Redis, etc.
        // Example: Check user exists in database
        // let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE phone = $1")
        //     .bind(&data.to)
        //     .fetch_optional(&self.state.db)
        //     .await?;

        match data.sms_type.as_str() {
            "verification" => {
                info!("🔐 Sending verification SMS to: {}", data.to);

                // Access Redis to store verification code
                // self.state.redis.set_ex(&format!("sms_code:{}", data.to), "123456", 300).await?;

                // Simulate SMS sending
                tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
            }
            "notification" => {
                info!("🔔 Sending notification SMS to: {}", data.to);

                // Can access database for user preferences
                // let preferences = self.get_user_notification_preferences(&data.to).await?;

                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            }
            _ => {
                error!("❌ Unknown SMS type: {}", data.sms_type);
                return Err(AppError::ValidationError(format!("Unknown SMS type: {}", data.sms_type)));
            }
        }

        info!("✅ SMS sent successfully to: {} (Environment: {})", data.to, self.state.config.environment);
        Ok(())
    }

    /// Public method to send SMS (adds to queue)
    pub async fn send_verification_sms(&self, phone: &str, code: &str) -> Result<String, AppError> {
        let sms_data = SmsJobData {
            to: phone.to_string(),
            message: format!("Your verification code is: {}", code),
            sms_type: "verification".to_string(),
            metadata: Some(serde_json::json!({
                "code": code,
                "phone": phone
            })),
        };

        let job_id = self.sms_queue.add_to_queue(sms_data).await?;
        info!("📱 Verification SMS queued for {} (Job ID: {})", phone, job_id);

        Ok(job_id)
    }

    /// Helper method that can be used in processor (since we have access to self)
    async fn get_user_notification_preferences(&self, phone: &str) -> Result<UserPreferences, AppError> {
        // Access database through self.state
        let preferences = sqlx::query_as::<_, UserPreferences>(
            "SELECT * FROM user_preferences WHERE phone = $1"
        )
        .bind(phone)
        .fetch_optional(&self.state.db)
        .await?;

        preferences.ok_or_else(|| AppError::NotFound("User preferences not found".into()))
    }
}
```

### Step 3: Register Service in main.rs

**Add to main.rs after AppState creation**:

```rust
// Initialize services (they auto-start their queue processors)
let _email_service = EmailService::new(app_state.clone());
let _sms_service = SmsService::new(app_state.clone());        // Add this line
tracing::info!("Services initialized with automatic queue processing");
```

### Step 4: Export Service in services/mod.rs

```rust
pub mod sms_service;

pub use sms_service::SmsService;
```

---

## ⚙️ Queue Job Processing Rules

### 1. Processor Function Requirements

**✅ Instance Methods (Recommended)**: Can access `&self` and service state

```rust
// ✅ Recommended: Instance method with access to self
async fn process_job(&self, job: QueueJob<JobData>) -> Result<(), AppError> {
    // Can access self.state.db, self.state.redis, etc.
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(&job.data.user_id)
        .fetch_optional(&self.state.db)
        .await?;

    Ok(())
}
```

**✅ Static Methods (Alternative)**: Cannot access service state

```rust
// ✅ Alternative: Static method (no access to self)
async fn process_job_static(job: QueueJob<JobData>) -> Result<(), AppError> {
    // Cannot access database or Redis - only job data
    let data = &job.data;
    // Process using only job data
    Ok(())
}
```

**✅ Must return `Result<(), AppError>`**:

```rust
// ✅ Correct
async fn process_job(&self, job: QueueJob<JobData>) -> Result<(), AppError> {
    // Processing logic
    Ok(())
}

// ❌ Wrong return type
async fn process_job(&self, job: QueueJob<JobData>) -> bool
```

### 2. Job Data Access Pattern

**Access job data through `job.data`**:

```rust
async fn process_email_job(&self, job: QueueJob<EmailJobData>) -> Result<(), AppError> {
    let data = &job.data;  // ✅ Access the payload

    info!("Processing job: {} for {}", job.id, data.to);

    // Can now access service state for complex operations
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
        .bind(&data.to)
        .fetch_optional(&self.state.db)
        .await?;

    match data.email_type.as_str() {
        "welcome" => {
            // Access Redis for caching
            self.state.redis.set(&format!("email_sent:{}", data.to), "true", 3600).await?;
        }
        "reset" => { /* handle password reset */ }
        _ => return Err(AppError::ValidationError("Unknown email type".into()))
    }

    Ok(())
}
```

---

## 🔄 Instance vs Static Methods

### When to Use Instance Methods (Recommended)

**✅ Use Instance Methods When**:

-   Need database access
-   Need Redis operations
-   Need configuration access
-   Need to call other service methods
-   Want to share common logic between processor and public methods

```rust
impl EmailService {
    async fn process_email_job(&self, job: QueueJob<EmailJobData>) -> Result<(), AppError> {
        // ✅ Can access database
        let user = self.get_user_by_email(&job.data.to).await?;

        // ✅ Can access Redis
        self.state.redis.incr(&format!("emails_sent:{}", job.data.email_type)).await?;

        // ✅ Can call helper methods
        self.send_via_smtp(&job.data).await?;

        Ok(())
    }

    // Helper method available to processor
    async fn get_user_by_email(&self, email: &str) -> Result<User, AppError> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
            .bind(email)
            .fetch_optional(&self.state.db)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".into()))
    }

    // Another helper method
    async fn send_via_smtp(&self, email_data: &EmailJobData) -> Result<(), AppError> {
        // SMTP implementation using self.state configuration
        Ok(())
    }
}
```

### When to Use Static Methods

**✅ Use Static Methods When**:

-   Only need job data (no external dependencies)
-   Simple processing logic
-   Stateless operations

```rust
impl NotificationService {
    async fn process_simple_notification_static(job: QueueJob<NotificationData>) -> Result<(), AppError> {
        let data = &job.data;

        // Simple processing with no external dependencies
        info!("Processing notification: {}", data.message);

        // Call external API directly (stateless)
        send_push_notification_api(&data.user_id, &data.message).await?;

        Ok(())
    }
}
```

---

## 🧪 Testing Queue Systems

### 1. Test Instance Methods

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppState;

    async fn create_test_service() -> EmailService {
        let app_state = create_test_app_state().await;
        EmailService::new(app_state)
    }

    #[tokio::test]
    async fn test_email_job_processing() {
        let service = create_test_service().await;

        let job_data = EmailJobData {
            to: "test@example.com".to_string(),
            subject: "Test".to_string(),
            body: "Test body".to_string(),
            email_type: "welcome".to_string(),
            template_data: None,
        };

        let job = QueueJob::new(job_data, 3, 60000);

        // Test instance method
        let result = service.process_email_job(job).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_database_access_in_processor() {
        let service = create_test_service().await;

        // Insert test user first
        sqlx::query("INSERT INTO users (email, name) VALUES ($1, $2)")
            .bind("test@example.com")
            .bind("Test User")
            .execute(&service.state.db)
            .await
            .unwrap();

        let job_data = EmailJobData {
            to: "test@example.com".to_string(),
            email_type: "welcome".to_string(),
            // ... other fields
        };

        let job = QueueJob::new(job_data, 3, 60000);

        // This should work because processor can access database
        let result = service.process_email_job(job).await;
        assert!(result.is_ok());
    }
}
```

---

## 💡 Common Patterns & Examples

### Pattern 1: Database-Dependent Processing

```rust
impl UserService {
    async fn process_user_verification(&self, job: QueueJob<UserVerificationData>) -> Result<(), AppError> {
        let data = &job.data;

        // Get user from database
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(&data.user_id)
            .fetch_optional(&self.state.db)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".into()))?;

        // Update verification status
        sqlx::query("UPDATE users SET verified = true WHERE id = $1")
            .bind(&data.user_id)
            .execute(&self.state.db)
            .await?;

        // Send confirmation email
        self.send_verification_complete_email(&user.email).await?;

        Ok(())
    }

    async fn send_verification_complete_email(&self, email: &str) -> Result<(), AppError> {
        // Implementation using self.state
        Ok(())
    }
}
```

### Pattern 2: Redis-Dependent Processing

```rust
impl SessionService {
    async fn process_session_cleanup(&self, job: QueueJob<SessionCleanupData>) -> Result<(), AppError> {
        let data = &job.data;

        // Check if session still exists in Redis
        let session_exists = self.state.redis
            .exists(&format!("session:{}", data.session_id))
            .await?;

        if !session_exists {
            info!("Session {} already expired", data.session_id);
            return Ok(());
        }

        // Remove from Redis
        self.state.redis
            .del(&format!("session:{}", data.session_id))
            .await?;

        // Update database
        sqlx::query("DELETE FROM sessions WHERE id = $1")
            .bind(&data.session_id)
            .execute(&self.state.db)
            .await?;

        info!("✅ Session {} cleaned up successfully", data.session_id);
        Ok(())
    }
}
```

### Pattern 3: Configuration-Dependent Processing

```rust
impl PaymentService {
    async fn process_payment(&self, job: QueueJob<PaymentData>) -> Result<(), AppError> {
        let data = &job.data;

        // Use configuration from self.state
        let api_key = &self.state.config.payment_api_key;
        let webhook_url = &self.state.config.payment_webhook_url;

        // Different behavior based on environment
        match self.state.config.environment.as_str() {
            "development" => {
                info!("🧪 Processing payment in development mode: ${}", data.amount);
                // Use sandbox API
                self.process_sandbox_payment(data, api_key).await?;
            }
            "production" => {
                info!("💳 Processing live payment: ${}", data.amount);
                // Use live API
                self.process_live_payment(data, api_key).await?;
            }
            _ => {
                return Err(AppError::ConfigError("Invalid environment".into()));
            }
        }

        Ok(())
    }
}
```

---

## 🚨 Troubleshooting

### Common Issues & Solutions

#### 1. Cannot Access Database in Processor

**Problem**: `self.state.db` not available in static method

```rust
// ❌ Wrong: Static method cannot access state
async fn process_job_static(job: QueueJob<JobData>) -> Result<(), AppError> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(&job.data.user_id)
        .fetch_one(&self.state.db)  // ❌ Error: no self in static method
        .await?;
}

// ✅ Fix: Use instance method
async fn process_job(&self, job: QueueJob<JobData>) -> Result<(), AppError> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(&job.data.user_id)
        .fetch_one(&self.state.db)  // ✅ Works: can access self.state.db
        .await?;
}
```

#### 2. Service Constructor Issues

**Problem**: Circular dependency when creating service in constructor

```rust
// ❌ Wrong: Trying to use self before it's created
pub fn new(state: AppState) -> Self {
    let service = Self { state, sms_queue: ??? }; // sms_queue not ready yet

    let sms_queue = manager.create_queue_with_processor(
        "sms", 3,
        |job| async move { service.process_sms_job(job).await } // ❌ service not complete
    );
}

// ✅ Fix: Create temporary service first, then replace queue
pub fn new(state: AppState) -> Self {
    let manager = QueueManager::global();

    let service = Self {
        state,
        sms_queue: manager.create_queue("sms", 3), // Temporary queue
    };

    let service_clone = service.clone();
    let sms_queue = manager.create_queue_with_processor(/* ... with service_clone */);

    Self {
        state: service.state,
        sms_queue,  // Replace with real queue
    }
}
```

---

## 📝 Queue Development Checklist

When creating a new queue service with instance methods:

-   [ ] Job data struct has `#[derive(Debug, Clone, Serialize, Deserialize)]`
-   [ ] Service implements `Clone` trait
-   [ ] Service constructor creates temporary instance first, then real queue
-   [ ] Processor method is instance method: `async fn process_job(&self, job: QueueJob<T>)`
-   [ ] Processor method returns `Result<(), AppError>`
-   [ ] Service clone is captured in closure properly
-   [ ] Service is initialized in `main.rs`
-   [ ] Service is exported in `services/mod.rs`
-   [ ] Tests cover both processor logic and state access
-   [ ] Database/Redis operations are properly handled with error checking

---

## 🎯 Summary

**Golden Rules for Instance Method Queue Services**:

1. **Instance Methods Preferred**: Use `&self` to access service state and dependencies
2. **Two-Phase Construction**: Create temporary service, then create queue with clone
3. **Clone Capture**: Properly clone service instance for async closure
4. **State Access**: Leverage `self.state` for database, Redis, and configuration access
5. **Error Handling**: Return appropriate `AppError` types for retry logic
6. **Testing**: Test both processor logic and state-dependent operations
7. **Helper Methods**: Create reusable helper methods that processors can call

This approach gives you full access to service state while maintaining clean separation of concerns! 🦀✨
