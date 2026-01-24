use crate::cc1101::MeterData;
use crate::config::{HomeAssistantConfig, MeterConfig, MqttConfig};
use log::{debug, error, info};
use rumqttc::{Client, Connection, MqttOptions, QoS};
use serde::Serialize;
use std::time::Duration;

const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug)]
pub enum MqttError {
    PublishError(String),
}

impl std::fmt::Display for MqttError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MqttError::PublishError(msg) => write!(f, "MQTT publish failed: {}", msg),
        }
    }
}

impl std::error::Error for MqttError {}

#[derive(Serialize, Clone)]
struct DeviceInfo<'a> {
    identifiers: Vec<String>,
    name: &'a String,
    manufacturer: &'a String,
    model: &'a String,
    sw_version: &'static str,
}

#[derive(Serialize)]
struct DiscoveryConfig<'a> {
    name: &'static str,
    unique_id: String,
    object_id: &'static str,
    state_topic: &'a String,
    value_template: &'static str,
    icon: &'static str,
    device: &'a DeviceInfo<'a>,
    #[serde(skip_serializing_if = "Option::is_none")]
    unit_of_measurement: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    state_class: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    device_class: Option<&'static str>,
}

pub struct MqttPublisher {
    client: Client,
    connection: Connection,
    mqtt_config: MqttConfig,
    ha_config: HomeAssistantConfig,
}

impl MqttPublisher {
    pub fn new(mqtt_config: MqttConfig, ha_config: HomeAssistantConfig) -> Result<Self, MqttError> {
        let mut mqttoptions =
            MqttOptions::new(&mqtt_config.client_id, &mqtt_config.host, mqtt_config.port);
        mqttoptions.set_keep_alive(Duration::from_secs(20));
        mqttoptions.set_clean_session(true);

        if let (Some(ref username), Some(ref password)) =
            (&mqtt_config.username, &mqtt_config.password)
        {
            mqttoptions.set_credentials(username, password);
        }

        let (client, connection) = Client::new(mqttoptions, 10);

        Ok(Self {
            client,
            connection,
            mqtt_config,
            ha_config,
        })
    }

    pub fn publish_discovery(&self, meter_config: &MeterConfig) -> Result<(), MqttError> {
        let device_info = self.create_device_info(meter_config);
        let state_topic = format!(
            "{}/sensor/{}/state",
            self.ha_config.discovery_prefix, self.ha_config.node_id
        );
        let unique_id =
            |object_id: &str| format!("water_meter_{}_{}", meter_config.serial, object_id);

        let sensors = vec![
            DiscoveryConfig {
                name: "Water Consumption",
                unique_id: unique_id("water_consumption"),
                object_id: "water_consumption",
                state_topic: &state_topic,
                value_template: "{{ value_json.liters }}",
                icon: "mdi:water",
                device: &device_info,
                unit_of_measurement: Some("L"),
                state_class: Some("total_increasing"),
                device_class: Some("water"),
            },
            DiscoveryConfig {
                name: "Battery Life",
                object_id: "battery",
                unique_id: unique_id("battery"),
                state_topic: &state_topic,
                value_template: "{{ value_json.battery_left }}",
                icon: "mdi:battery",
                device: &device_info,
                unit_of_measurement: Some("months"),
                state_class: None,
                device_class: None,
            },
            DiscoveryConfig {
                name: "Read Counter",
                unique_id: unique_id("reads_counter"),
                object_id: "reads_counter",
                state_topic: &state_topic,
                value_template: "{{ value_json.reads_counter }}",
                icon: "mdi:counter",
                device: &device_info,
                unit_of_measurement: Some("reads"),
                state_class: Some("total_increasing"),
                device_class: None,
            },
            DiscoveryConfig {
                name: "Wake Time",
                unique_id: unique_id("wake_time"),
                object_id: "wake_time",
                state_topic: &state_topic,
                value_template: "{{ value_json.time_start }}",
                icon: "mdi:clock-start",
                device: &device_info,
                unit_of_measurement: Some("hour"),
                state_class: None,
                device_class: None,
            },
            DiscoveryConfig {
                name: "Sleep Time",
                unique_id: unique_id("sleep_time"),
                object_id: "sleep_time",
                state_topic: &state_topic,
                value_template: "{{ value_json.time_end }}",
                icon: "mdi:clock-end",
                device: &device_info,
                unit_of_measurement: Some("hour"),
                state_class: None,
                device_class: None,
            },
        ];

        for sensor in sensors {
            let config_topic = format!(
                "{}/sensor/{}/{}/config",
                self.ha_config.discovery_prefix, self.ha_config.node_id, sensor.object_id
            );
            let payload = serde_json::to_string(&sensor)
                .map_err(|e| MqttError::PublishError(format!("JSON serialization error: {}", e)))?;

            self.publish(&config_topic, &payload, true)?;
            info!("Published discovery for: {}", sensor.name);
        }

        Ok(())
    }

    pub fn publish_state(&self, meter_data: &MeterData) -> Result<(), MqttError> {
        let state_topic = format!(
            "{}/sensor/{}/state",
            self.ha_config.discovery_prefix, self.ha_config.node_id
        );

        let payload = serde_json::to_string(&meter_data)
            .map_err(|e| MqttError::PublishError(format!("JSON serialization error: {}", e)))?;

        self.publish(&state_topic, &payload, self.mqtt_config.retain)?;
        info!("Published meter state");

        Ok(())
    }

    // /// Wait for pending MQTT messages to be transmitted before exiting
    // /// This is critical for one-shot programs that exit immediately after publishing
    pub fn disconnect(&mut self) {
        self.client.disconnect().unwrap();
        for notification in self.connection.iter() {
            match notification {
                Ok(notif) => {
                    debug!("mqtt: {notif:?}")
                }
                Err(error) => {
                    error!("mqtt: {error:?}");
                    return;
                }
            }
        }
    }

    fn create_device_info(&self, meter_config: &MeterConfig) -> DeviceInfo<'_> {
        DeviceInfo {
            identifiers: vec![format!("everblu_{}", meter_config.serial)],
            name: &self.ha_config.device_name,
            manufacturer: &self.ha_config.device_manufacturer,
            model: &self.ha_config.device_model,
            sw_version: CARGO_PKG_VERSION,
        }
    }

    fn publish(&self, topic: &str, payload: &str, retain: bool) -> Result<(), MqttError> {
        let qos = match self.mqtt_config.qos {
            0 => QoS::AtMostOnce,
            1 => QoS::AtLeastOnce,
            2 => QoS::ExactlyOnce,
            _ => QoS::AtLeastOnce, // Default to QoS 1
        };

        debug!("Publishing to {}: {}", topic, payload);

        self.client
            .publish(topic, qos, retain, payload.as_bytes())
            .map_err(|e| {
                MqttError::PublishError(format!("Failed to publish to {}: {}", topic, e))
            })?;

        Ok(())
    }
}
