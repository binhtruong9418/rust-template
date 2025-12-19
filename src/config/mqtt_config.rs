use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct MqttConfig {
    pub broker: String,
    pub client_id: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub keep_alive: u64,
}

impl MqttConfig {
    pub fn from_env() -> Result<Self, config::ConfigError> {
        dotenv::dotenv().ok();

        let cfg = config::Config::builder()
            .add_source(config::Environment::default())
            .build()?;

        let username = cfg.get_string("MQTT_USERNAME").ok();
        let password = cfg.get_string("MQTT_PASSWORD").ok();

        Ok(Self {
            broker: cfg.get_string("MQTT_BROKER").unwrap_or_else(|_| "mqtt://localhost:1883".to_string()),
            client_id: cfg.get_string("MQTT_CLIENT_ID").unwrap_or_else(|_| "rust-backend-template".to_string()),
            username: if username.as_ref().map_or(false, |u| !u.is_empty()) { username } else { None },
            password: if password.as_ref().map_or(false, |p| !p.is_empty()) { password } else { None },
            keep_alive: cfg.get_int("MQTT_KEEP_ALIVE").unwrap_or(60) as u64,
        })
    }
}
