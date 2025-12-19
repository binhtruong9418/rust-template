use deadpool_redis::{Config, Pool, Runtime};
use redis::RedisError;

#[derive(Debug, Clone)]
pub struct RedisConfig {
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
    pub db: i64,
    pub pool_size: usize,
}

impl RedisConfig {
    pub fn from_env() -> Result<Self, config::ConfigError> {
        dotenv::dotenv().ok();

        let cfg = config::Config::builder()
            .add_source(config::Environment::default())
            .build()?;

        let username = cfg.get_string("REDIS_USERNAME").ok();
        let password = cfg.get_string("REDIS_PASSWORD").ok();

        Ok(Self {
            host: cfg.get_string("REDIS_HOST").unwrap_or_else(|_| "localhost".to_string()),
            port: cfg.get_int("REDIS_PORT").unwrap_or(6379) as u16,
            username: if username.as_ref().map_or(false, |u| !u.is_empty()) { username } else { None },
            password: if password.as_ref().map_or(false, |p| !p.is_empty()) { password } else { None },
            db: cfg.get_int("REDIS_DB").unwrap_or(0),
            pool_size: cfg.get_int("REDIS_POOL_SIZE").unwrap_or(10) as usize,
        })
    }

    pub fn create_pool(&self) -> Result<Pool, RedisError> {
        let redis_url = self.build_redis_url();

        let cfg = Config {
            url: Some(redis_url),
            pool: Some(deadpool_redis::PoolConfig {
                max_size: self.pool_size,
                ..Default::default()
            }),
            ..Default::default()
        };

        cfg.create_pool(Some(Runtime::Tokio1))
            .map_err(|e| RedisError::from((redis::ErrorKind::IoError, "Failed to create pool", e.to_string())))
    }

    fn build_redis_url(&self) -> String {
        let auth = match (&self.username, &self.password) {
            (Some(user), Some(pass)) => format!("{}:{}@", user, pass),
            (None, Some(pass)) => format!(":{}@", pass),
            _ => String::new(),
        };

        format!("redis://{}{}:{}/{}", auth, self.host, self.port, self.db)
    }
}
