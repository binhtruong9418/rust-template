use axum::{
    response::{IntoResponse, Response},
    Json,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Standard API Response wrapper
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ApiResponse<T> {
    Success(ApiSuccess<T>),
    Error(ApiError),
}

/// Success response structure
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiSuccess<T> {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

/// Error response structure
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiError {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorDetail>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorDetail {
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

impl<T: Serialize> ApiSuccess<T> {
    /// Create a new success response with data
    pub fn new(message: impl Into<String>, data: T) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: Some(data),
        }
    }

    /// Create a new success response without data
    pub fn new_without_data(message: impl Into<String>) -> ApiSuccess<()> {
        ApiSuccess {
            success: true,
            message: message.into(),
            data: None,
        }
    }

    /// Create a success response from data with default message
    pub fn from_data(data: T) -> Self {
        Self {
            success: true,
            message: "Success".to_string(),
            data: Some(data),
        }
    }
}

impl ApiError {
    /// Create a new error response
    pub fn new(message: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            error: Some(ErrorDetail {
                code: code.into(),
                details: None,
            }),
        }
    }

    /// Create a new error response with details
    pub fn with_details(
        message: impl Into<String>,
        code: impl Into<String>,
        details: Value,
    ) -> Self {
        Self {
            success: false,
            message: message.into(),
            error: Some(ErrorDetail {
                code: code.into(),
                details: Some(details),
            }),
        }
    }

    /// Create a simple error without error details
    pub fn simple(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            error: None,
        }
    }
}

// Implement IntoResponse for ApiSuccess
impl<T: Serialize> IntoResponse for ApiSuccess<T> {
    fn into_response(self) -> Response {
        let response = ApiResponse::Success(self);
        (StatusCode::OK, Json(response)).into_response()
    }
}

// Implement IntoResponse for ApiError
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.determine_status_code();
        let response = ApiResponse::<()>::Error(self);
        (status, Json(response)).into_response()
    }
}

impl ApiError {
    fn determine_status_code(&self) -> StatusCode {
        if let Some(ref error) = self.error {
            match error.code.as_str() {
                "UNAUTHORIZED" | "INVALID_TOKEN" | "TOKEN_EXPIRED" => StatusCode::UNAUTHORIZED,
                "FORBIDDEN" => StatusCode::FORBIDDEN,
                "NOT_FOUND" => StatusCode::NOT_FOUND,
                "VALIDATION_ERROR" | "INVALID_INPUT" => StatusCode::BAD_REQUEST,
                "CONFLICT" => StatusCode::CONFLICT,
                "INTERNAL_ERROR" | "DATABASE_ERROR" | "REDIS_ERROR" => StatusCode::INTERNAL_SERVER_ERROR,
                _ => StatusCode::BAD_REQUEST,
            }
        } else {
            StatusCode::BAD_REQUEST
        }
    }
}

// Helper macro for creating success responses
#[macro_export]
macro_rules! success_response {
    ($data:expr) => {
        $crate::interceptors::ApiSuccess::from_data($data)
    };
    ($message:expr, $data:expr) => {
        $crate::interceptors::ApiSuccess::new($message, $data)
    };
}

// Helper macro for creating error responses
#[macro_export]
macro_rules! error_response {
    ($message:expr) => {
        $crate::interceptors::ApiError::simple($message)
    };
    ($message:expr, $code:expr) => {
        $crate::interceptors::ApiError::new($message, $code)
    };
    ($message:expr, $code:expr, $details:expr) => {
        $crate::interceptors::ApiError::with_details($message, $code, $details)
    };
}
