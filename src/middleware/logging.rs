use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use tracing_appender::rolling::{RollingFileAppender, Rotation};

/// Setup logging with file and console output
pub fn setup_logging() {
    dotenv::dotenv().ok();

    let log_level = std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
    let log_file = std::env::var("LOG_FILE").unwrap_or_else(|_| "logs/app.log".to_string());

    // Create logs directory if it doesn't exist
    if let Some(parent) = std::path::Path::new(&log_file).parent() {
        std::fs::create_dir_all(parent).ok();
    }

    // File appender with daily rotation
    let file_appender = RollingFileAppender::new(
        Rotation::DAILY,
        std::path::Path::new(&log_file).parent().unwrap_or(std::path::Path::new("logs")),
        std::path::Path::new(&log_file)
            .file_name()
            .unwrap_or(std::ffi::OsStr::new("app.log")),
    );

    // Console layer
    let console_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(false)
        .with_thread_names(false)
        .compact();

    // File layer
    let file_layer = fmt::layer()
        .with_writer(file_appender)
        .with_target(true)
        .with_ansi(false)
        .json();

    // Combine layers
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&log_level)))
        .with(console_layer)
        .with(file_layer)
        .init();

    tracing::info!("Logging initialized with level: {}", log_level);
}
