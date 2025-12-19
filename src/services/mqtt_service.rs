use rumqttc::{AsyncClient, Event, EventLoop, MqttOptions, Packet, QoS};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use crate::config::MqttConfig;
use crate::interceptors::AppError;

#[derive(Clone)]
pub struct MqttService {
    client: AsyncClient,
    config: MqttConfig,
}

impl MqttService {
    /// Create a new MqttService instance
    pub async fn new() -> Result<Self, AppError> {
        let config = MqttConfig::from_env()
            .map_err(|e| AppError::MqttError(format!("Failed to load MQTT config: {}", e)))?;

        // Parse broker URL
        let broker_url = config.broker.clone();
        let (host, port) = Self::parse_broker_url(&broker_url)?;

        // Create MQTT options
        let mut mqtt_options = MqttOptions::new(&config.client_id, host, port);
        mqtt_options.set_keep_alive(Duration::from_secs(config.keep_alive));

        // Set credentials if provided
        if let (Some(username), Some(password)) = (&config.username, &config.password) {
            mqtt_options.set_credentials(username, password);
        }

        // Create client and event loop
        let (client, mut event_loop) = AsyncClient::new(mqtt_options, 10);

        // Spawn event loop handler
        tokio::spawn(async move {
            loop {
                match event_loop.poll().await {
                    Ok(notification) => {
                        if let Event::Incoming(Packet::ConnAck(_)) = notification {
                            tracing::info!("MQTT connected successfully");
                        }
                    }
                    Err(e) => {
                        tracing::error!("MQTT connection error: {}", e);
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }
            }
        });

        tracing::info!("MQTT service initialized");

        Ok(Self { client, config })
    }

    /// Parse broker URL to extract host and port
    fn parse_broker_url(url: &str) -> Result<(String, u16), AppError> {
        let url = url.trim_start_matches("mqtt://").trim_start_matches("mqtts://");

        if let Some((host, port_str)) = url.split_once(':') {
            let port = port_str.parse::<u16>()
                .map_err(|_| AppError::MqttError(format!("Invalid port in broker URL: {}", port_str)))?;
            Ok((host.to_string(), port))
        } else {
            Ok((url.to_string(), 1883)) // Default MQTT port
        }
    }

    /// Subscribe to a topic
    pub async fn subscribe(&self, topic: &str) -> Result<(), AppError> {
        self.client
            .subscribe(topic, QoS::AtLeastOnce)
            .await
            .map_err(|e| AppError::MqttError(format!("Failed to subscribe to topic '{}': {}", topic, e)))?;

        tracing::info!("Subscribed to MQTT topic: {}", topic);
        Ok(())
    }

    /// Unsubscribe from a topic
    pub async fn unsubscribe(&self, topic: &str) -> Result<(), AppError> {
        self.client
            .unsubscribe(topic)
            .await
            .map_err(|e| AppError::MqttError(format!("Failed to unsubscribe from topic '{}': {}", topic, e)))?;

        tracing::info!("Unsubscribed from MQTT topic: {}", topic);
        Ok(())
    }

    /// Publish a message to a topic
    pub async fn publish(&self, topic: &str, payload: &str, retain: bool) -> Result<(), AppError> {
        self.client
            .publish(topic, QoS::AtLeastOnce, retain, payload)
            .await
            .map_err(|e| AppError::MqttError(format!("Failed to publish to topic '{}': {}", topic, e)))?;

        tracing::debug!("Published message to MQTT topic: {}", topic);
        Ok(())
    }

    /// Publish JSON message to a topic
    pub async fn publish_json<T: serde::Serialize>(&self, topic: &str, payload: &T, retain: bool) -> Result<(), AppError> {
        let json = serde_json::to_string(payload)
            .map_err(|e| AppError::MqttError(format!("Failed to serialize JSON: {}", e)))?;

        self.publish(topic, &json, retain).await
    }

    /// Publish bytes to a topic
    pub async fn publish_bytes(&self, topic: &str, payload: &[u8], retain: bool) -> Result<(), AppError> {
        self.client
            .publish(topic, QoS::AtLeastOnce, retain, payload)
            .await
            .map_err(|e| AppError::MqttError(format!("Failed to publish bytes to topic '{}': {}", topic, e)))?;

        tracing::debug!("Published bytes to MQTT topic: {}", topic);
        Ok(())
    }

    /// Disconnect from MQTT broker
    pub async fn disconnect(&self) -> Result<(), AppError> {
        self.client
            .disconnect()
            .await
            .map_err(|e| AppError::MqttError(format!("Failed to disconnect: {}", e)))?;

        tracing::info!("Disconnected from MQTT broker");
        Ok(())
    }

    /// Create a message handler (subscribe and listen for messages)
    pub async fn listen<F>(&self, topic: &str, handler: F) -> Result<(), AppError>
    where
        F: Fn(String, Vec<u8>) + Send + Sync + 'static,
    {
        // Subscribe to topic
        self.subscribe(topic).await?;

        // Create new event loop for listening
        let broker_url = self.config.broker.clone();
        let (host, port) = Self::parse_broker_url(&broker_url)?;

        let mut mqtt_options = MqttOptions::new(
            &format!("{}_listener", self.config.client_id),
            host,
            port,
        );
        mqtt_options.set_keep_alive(Duration::from_secs(self.config.keep_alive));

        if let (Some(username), Some(password)) = (&self.config.username, &self.config.password) {
            mqtt_options.set_credentials(username, password);
        }

        let (client, mut event_loop) = AsyncClient::new(mqtt_options, 10);

        // Subscribe with the new client
        client
            .subscribe(topic, QoS::AtLeastOnce)
            .await
            .map_err(|e| AppError::MqttError(format!("Failed to subscribe listener: {}", e)))?;

        // Spawn listener task
        let topic_name = topic.to_string();
        tokio::spawn(async move {
            tracing::info!("MQTT listener started for topic: {}", topic_name);

            loop {
                match event_loop.poll().await {
                    Ok(notification) => {
                        if let Event::Incoming(Packet::Publish(publish)) = notification {
                            let topic = publish.topic.clone();
                            let payload = publish.payload.to_vec();

                            tracing::debug!("Received MQTT message on topic: {}", topic);
                            handler(topic, payload);
                        }
                    }
                    Err(e) => {
                        tracing::error!("MQTT listener error: {}", e);
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }
            }
        });

        Ok(())
    }
}
