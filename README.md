# Itron-EverBlu-Cyble - Water Meter Reader

A Rust implementation for reading water consumption data from Itron EverBlu Cyble Enhanced meters using the RADIAN protocol over 433MHz RF.

This is a Rust port of the original C code from https://github.com/neutrinus/everblu-meters, featuring:
- Full MQTT integration with Home Assistant auto-discovery
- Configuration via TOML file (no recompilation needed)
- Modern Rust implementation using `rppal` for GPIO/SPI
- Automatic sensor creation in Home Assistant

Meters supported:
- Itron EverBlu Cyble Enhanced


## Hardware
![Raspberry Pi Zero with CC1101](board.jpg)
The project runs on Raspberry Pi with an RF transreciver (CC1101). 

### Connections (rpi to CC1101):
- pin 1 (3V3) to pin 2 (VCC)
- pin 6 (GND) to pin 1 (GND)
- pin 11 (GPIO17) to pin 3 (GDO0)
- pin 24 (CE0) to pin 4 (CSN)
- pin 23 (SCLK) to pin 5 (SCK)
- pin 19 (MOSI) to pin 6 (MOSI)
- pin 21 (MISO) to pin 7 (MISO)
- pin 13 (GPIO27) to pin 8 (GD02)


## Installation and Setup

### 1. Enable SPI Interface
```bash
sudo raspi-config
# Select Interfacing Options > SPI
# Select Yes to enable SPI interface
# Select Yes to load SPI kernel module automatically
# Reboot when prompted
```

### 2. Install Rust
If you don't have Rust installed:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### 3. Configure Your Meter
Copy the example configuration and edit it with your meter details:
```bash
cp config.toml.example config.toml
nano config.toml
```

Set your meter serial number and production year (found on the meter label):

![Cyble Meter Label](meter_label.png)

```toml
[meter]
serial = 1234567    # Your meter's serial number
year = 16           # Production year code
```

Configure your MQTT broker:
```toml
[mqtt]
host = "localhost"
port = 1883
client_id = "everblu-meter-reader"
username = "your_username"
password = "your_password"
```

### 4. Build and Run
```bash
# Build in release mode (optimized)
cargo build --release

# Run the program
cargo run --release

# Or run the compiled binary directly
./target/release/everblu-meters
```

## Development

### Running Tests
```bash
cargo test
cargo test -- --nocapture  # With output visible
```

### Code Quality
```bash
cargo check              # Quick syntax check
cargo clippy             # Linter
cargo fmt                # Format code
```

### CI/CD
The project uses GitHub Actions for continuous integration:
- Runs tests and linting on all pushes and pull requests
- Builds release binaries for Raspberry Pi (ARM targets)
- Creates artifacts for both 32-bit (armv7) and 64-bit (aarch64) architectures
- Automatically creates releases with binaries when tags are pushed

## Home Assistant Integration

The program automatically publishes MQTT discovery messages for Home Assistant. After the first successful run, five sensor entities will appear:

1. Water Consumption (liters) - Total water usage
2. Battery Life (months) - Remaining battery life
3. Read Counter (reads) - Number of successful meter reads
4. Wake Time (hour) - When meter starts listening (e.g., 6 for 6am)
5. Sleep Time (hour) - When meter stops listening (e.g., 18 for 6pm)

All sensors are grouped under a single device in Home Assistant under Settings → Devices & Services → MQTT.

### Automated Periodic Reading

For hourly automated readings, create a systemd timer:

**/etc/systemd/system/everblu-meter.service**:
```ini
[Unit]
Description=EverBlu Water Meter Reader
After=network.target

[Service]
Type=oneshot
ExecStart=/usr/local/bin/everblu-meters /etc/everblu/config.toml
User=pi
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
```

**/etc/systemd/system/everblu-meter.timer**:
```ini
[Unit]
Description=Read water meter every hour

[Timer]
OnCalendar=hourly
Persistent=true

[Install]
WantedBy=timers.target
```

Enable and start the timer:
```bash
sudo systemctl enable everblu-meter.timer
sudo systemctl start everblu-meter.timer
```


## Troubleshooting

### Debugging
Enable debug logging to see detailed RF communication:
```bash
RUST_LOG=debug cargo run --release
RUST_LOG=trace cargo run --release  # Even more verbose
```

Monitor MQTT messages:
```bash
mosquitto_sub -h <broker> -t 'homeassistant/#' -v
```

### Frequency Adjustment
Your CC1101 transceiver module may not be calibrated correctly. You may need to modify the frequency slightly in `cc1101.rs` (lines 200-205). Use an RTL-SDR to measure the offset needed. The default is 433.8MHz.

### Business Hours
Your meter may be configured to listen for requests only during business hours (typically 6am-6pm) to conserve battery. If you cannot communicate with the meter, try again during these hours. The wake/sleep times are reported in the meter data.

### Serial Number Starting with 0
If your meter serial number starts with 0, ignore the leading zero when entering it in `config.toml`.

### MQTT Connection Issues
- Verify broker URL is correct in `config.toml`
- Check username/password credentials
- Ensure MQTT broker is running and accessible
- Check firewall settings


## Origin and license

This code is based on code from http://www.lamaisonsimon.fr/wiki/doku.php?id=maison2:compteur_d_eau:compteur_d_eau 


The license is unknown, citing one of the authors (fred):

> I didn't put a license on this code maybe I should, I didn't know much about it in terms of licensing.
> this code was made by "looking" at the radian protocol which is said to be open source earlier in the page, I don't know if that helps?


