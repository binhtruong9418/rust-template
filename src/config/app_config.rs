use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub environment: String,
    pub app_name: String,
    pub app_version: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, config::ConfigError> {
        dotenv::dotenv().ok();

        let cfg = config::Config::builder()
            .add_source(config::Environment::default())
            .build()?;

        Ok(Self {
            host: cfg.get_string("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: cfg.get_int("PORT").unwrap_or(3000) as u16,
            environment: cfg.get_string("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()),
            app_name: cfg.get_string("APP_NAME").unwrap_or_else(|_| "rust-backend-template".to_string()),
            app_version: cfg.get_string("APP_VERSION").unwrap_or_else(|_| "0.1.0".to_string()),
        })
    }

    pub fn is_production(&self) -> bool {
        self.environment == "production"
    }

    pub fn is_development(&self) -> bool {
        self.environment == "development"
    }

    pub fn server_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
