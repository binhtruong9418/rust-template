use validator::Validate;

use crate::interceptors::AppError;

/// Validate a request struct using validator
pub fn validate_request<T: Validate>(request: &T) -> Result<(), AppError> {
    request
        .validate()
        .map_err(|e| {
            let errors = e
                .field_errors()
                .iter()
                .map(|(field, errors)| {
                    let messages: Vec<String> = errors
                        .iter()
                        .filter_map(|e| e.message.as_ref().map(|m| m.to_string()))
                        .collect();
                    format!("{}: {}", field, messages.join(", "))
                })
                .collect::<Vec<_>>()
                .join("; ");

            AppError::ValidationError(errors)
        })
}
