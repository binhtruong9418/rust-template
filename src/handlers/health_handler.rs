use axum::Json;
use serde_json::{json, Value};

use crate::interceptors::{ApiSuccess, AppError};

/// Health check endpoint
pub async fn health_check() -> Result<ApiSuccess<Value>, AppError> {
    let data = json!({
        "status": "ok",
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });

    Ok(ApiSuccess::new("Service is healthy", data))
}
