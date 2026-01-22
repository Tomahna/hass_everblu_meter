use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub mqtt: MqttConfig,
    pub homeassistant: HomeAssistantConfig,
    pub meter: MeterConfig,
    #[serde(default)]
    pub advanced: AdvancedConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MqttConfig {
    pub host: String,
    pub port: u16,
    pub client_id: String,
    pub username: Option<String>,
    pub password: Option<String>,
    #[serde(default = "default_qos")]
    pub qos: i32,
    #[serde(default = "default_retain")]
    pub retain: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HomeAssistantConfig {
    #[serde(default = "default_discovery_prefix")]
    pub discovery_prefix: String,
    pub node_id: String,
    pub device_name: String,
    #[serde(default = "default_manufacturer")]
    pub device_manufacturer: String,
    #[serde(default = "default_model")]
    pub device_model: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MeterConfig {
    pub serial: u32,
    pub year: u8,
    pub location: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AdvancedConfig {
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    #[serde(default = "default_read_timeout_ms")]
    pub read_timeout_ms: u64,
}

impl Default for AdvancedConfig {
    fn default() -> Self {
        Self {
            timeout_ms: default_timeout_ms(),
            max_retries: default_max_retries(),
            read_timeout_ms: default_read_timeout_ms(),
        }
    }
}

fn default_qos() -> i32 {
    1
}

fn default_retain() -> bool {
    true
}

fn default_discovery_prefix() -> String {
    "homeassistant".to_string()
}

fn default_manufacturer() -> String {
    "Itron".to_string()
}

fn default_model() -> String {
    "EverBlu Cyble Enhanced".to_string()
}

fn default_timeout_ms() -> u64 {
    2000
}

fn default_max_retries() -> u32 {
    3
}

fn default_read_timeout_ms() -> u64 {
    5000
}

#[derive(Debug)]
pub enum ConfigError {
    FileNotFound(String),
    ParseError(toml::de::Error),
    ValidationError(String),
    IoError(std::io::Error),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::FileNotFound(path) => {
                write!(f, "Config file not found: {}\n\nCreate a config.toml file with MQTT and meter settings. See config.toml.example for reference.", path)
            }
            ConfigError::ParseError(e) => {
                write!(f, "Failed to parse config file: {}", e)
            }
            ConfigError::ValidationError(msg) => {
                write!(f, "Config validation failed: {}", msg)
            }
            ConfigError::IoError(e) => {
                write!(f, "Failed to read config file: {}", e)
            }
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<std::io::Error> for ConfigError {
    fn from(err: std::io::Error) -> Self {
        ConfigError::IoError(err)
    }
}

impl From<toml::de::Error> for ConfigError {
    fn from(err: toml::de::Error) -> Self {
        ConfigError::ParseError(err)
    }
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let path = path.as_ref();

        if !path.exists() {
            return Err(ConfigError::FileNotFound(path.display().to_string()));
        }

        let content = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;

        config.validate()?;

        Ok(config)
    }

    fn validate(&self) -> Result<(), ConfigError> {
        if self.meter.serial == 0 {
            return Err(ConfigError::ValidationError(
                "Meter serial number cannot be 0".to_string(),
            ));
        }

        if self.mqtt.host.is_empty() {
            return Err(ConfigError::ValidationError(
                "MQTT broker host cannot be empty".to_string(),
            ));
        }

        if self.mqtt.qos < 0 || self.mqtt.qos > 2 {
            return Err(ConfigError::ValidationError(
                "MQTT QoS must be 0, 1, or 2".to_string(),
            ));
        }

        if self.homeassistant.node_id.is_empty() {
            return Err(ConfigError::ValidationError(
                "Home Assistant node_id cannot be empty".to_string(),
            ));
        }

        if self.homeassistant.device_name.is_empty() {
            return Err(ConfigError::ValidationError(
                "Home Assistant device_name cannot be empty".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_validation() {
        let mut config = Config {
            mqtt: MqttConfig {
                host: "localhost".to_string(),
                port: 1883,
                client_id: "test".to_string(),
                username: None,
                password: None,
                qos: 1,
                retain: true,
            },
            homeassistant: HomeAssistantConfig {
                discovery_prefix: "homeassistant".to_string(),
                node_id: "test_meter".to_string(),
                device_name: "Test Meter".to_string(),
                device_manufacturer: "Itron".to_string(),
                device_model: "EverBlu".to_string(),
            },
            meter: MeterConfig {
                serial: 123456,
                year: 14,
                location: None,
            },
            advanced: AdvancedConfig::default(),
        };

        assert!(config.validate().is_ok());

        config.meter.serial = 0;
        assert!(config.validate().is_err());

        config.meter.serial = 123456;
        config.mqtt.qos = 3;
        assert!(config.validate().is_err());
    }
}
