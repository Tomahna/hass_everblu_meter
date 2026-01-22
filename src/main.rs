mod cc1101;
mod cc1101_const;
mod config;
mod mqtt;
mod radian;

use cc1101::{MeterData, CC1101};
use config::Config;
use log::{error, info};
use mqtt::MqttPublisher;
use std::process::exit;

fn main() {
    simple_logger::init_with_env().unwrap();

    match run() {
        Ok(_) => exit(0),
        Err(e) => {
            error!("Process failed with: {}", e);
            exit(1)
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config = load_config()?;

    info!("Initializing cc1101 device");
    let cc1101 = CC1101::new();
    info!(
        "Reading meter serial={} year={}",
        config.meter.serial, config.meter.year
    );
    let meter_data = cc1101.get_meter_data(config.meter.year, config.meter.serial)?;
    info!("Meter data read successfully:\n{:?}", meter_data);
    info!(
        "Publishing sensor to mqtt broker {}:{}",
        config.mqtt.host, config.mqtt.port
    );
    publish_to_mqtt(&config, &meter_data)
}

fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
    let config_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "config.toml".to_string());

    info!("Loading configuration from: {}", config_path);
    Config::load(&config_path).map_err(Into::into)
}

fn publish_to_mqtt(
    config: &Config,
    meter_data: &MeterData,
) -> Result<(), Box<dyn std::error::Error>> {
    let publisher = MqttPublisher::new(config.mqtt.clone(), config.homeassistant.clone())?;

    info!("Publishing Home Assistant discovery messages");
    publisher.publish_discovery(&config.meter)?;

    info!("Publishing meter state");
    publisher.publish_state(meter_data)?;

    Ok(())
}
