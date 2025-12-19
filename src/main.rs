mod config;
mod dto;
mod handlers;
mod interceptors;
mod middleware;
mod models;
mod queue;
mod routes;
mod services;
mod utils;

use config::{AppConfig, DatabaseConfig};
use middleware::setup_logging;
use routes::create_router;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup logging
    setup_logging();

    tracing::info!("Starting application...");

    // Load configurations
    let app_config = AppConfig::from_env()?;
    let db_config = DatabaseConfig::from_env()?;

    tracing::info!("Loaded configuration for environment: {}", app_config.environment);

    // Create database connection pool
    let db_pool = db_config.create_pool().await?;
    tracing::info!("Database connection pool created");

    // Create router
    let app = create_router(db_pool)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TraceLayer::new_for_http());

    // Create server address
    let addr = app_config.server_address();
    tracing::info!("Server starting on {}", addr);

    // Start server
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!(
        "{} v{} is running on {}",
        app_config.app_name,
        app_config.app_version,
        addr
    );

    axum::serve(listener, app).await?;

    Ok(())
}
