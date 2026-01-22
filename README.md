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

### Method 1: Debian Package Installation (Recommended)

The easiest way to install on Raspberry Pi OS is using the pre-built .deb package:

#### 1. Download and Install Package

**For 32-bit Raspberry Pi OS (Raspberry Pi 2/3/4):**
```bash
# Download the latest release
wget https://github.com/tomahna/hass-everblu-meter/releases/latest/download/hass-everblu-meter_armhf.deb

# Install the package
sudo dpkg -i hass-everblu-meter_armhf.deb
sudo apt-get install -f  # Install any missing dependencies
```

**For 64-bit Raspberry Pi OS (Raspberry Pi 3/4/5):**
```bash
# Download the latest release
wget https://github.com/tomahna/hass-everblu-meter/releases/latest/download/hass-everblu-meter_arm64.deb

# Install the package
sudo dpkg -i hass-everblu-meter_arm64.deb
sudo apt-get install -f  # Install any missing dependencies
```

#### 2. Enable SPI Interface
```bash
sudo raspi-config
# Select Interfacing Options > SPI
# Select Yes to enable SPI interface
# Select Yes to load SPI kernel module automatically
# Reboot when prompted
```

#### 3. Configure Your Meter

Copy the example configuration and edit it:
```bash
sudo cp /etc/hass-everblu-meter/config.toml.example /etc/hass-everblu-meter/config.toml
sudo nano /etc/hass-everblu-meter/config.toml
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

#### 4. Test the Configuration

```bash
# Test reading the meter
hass-everblu-meter /etc/hass-everblu-meter/config.toml

# Or with debug logging
RUST_LOG=debug hass-everblu-meter /etc/hass-everblu-meter/config.toml
```

#### 5. Enable Automatic Daily Readings

**⚠️ IMPORTANT - Battery Conservation:**

The water meter runs on a battery that can last 10-15 years under normal utility reading schedules (typically once per month). Reading the meter too frequently will significantly reduce battery life. **The default timer is set to run once per day**, which is already more frequent than recommended by the manufacturer.

Only increase reading frequency if:
- You need real-time leak detection
- You're actively monitoring water consumption patterns
- You understand and accept reduced battery life

```bash
# Enable and start the systemd timer (runs once daily by default)
sudo systemctl enable hass-everblu-meter.timer
sudo systemctl start hass-everblu-meter.timer

# Check timer status and next run time
sudo systemctl status hass-everblu-meter.timer

# View recent readings in the journal
sudo journalctl -u hass-everblu-meter.service -f
```

**To change reading frequency:**
```bash
# Edit the timer file
sudo systemctl edit --full hass-everblu-meter.timer

# Change OnCalendar= to one of:
#   daily        - Once per day (default, recommended)
#   weekly       - Once per week (better for battery life)
#   *:0/30       - Every 30 minutes (will drain battery quickly)
#   hourly       - Every hour (not recommended, reduces battery life)

# After editing, reload and restart
sudo systemctl daemon-reload
sudo systemctl restart hass-everblu-meter.timer
```

---

### Method 2: Manual Installation from Source

If you prefer to build from source or the .deb package doesn't work for your setup:

#### 1. Enable SPI Interface
```bash
sudo raspi-config
# Select Interfacing Options > SPI
# Select Yes to enable SPI interface
# Select Yes to load SPI kernel module automatically
# Reboot when prompted
```

#### 2. Install Rust
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

#### 3. Build and Install
```bash
# Clone the repository
git clone https://github.com/tomahna/hass-everblu-meter.git
cd hass-everblu-meter

# Build in release mode
cargo build --release

# Install the binary
sudo cp target/release/hass-everblu-meter /usr/local/bin/
sudo chmod +x /usr/local/bin/hass-everblu-meter
```

#### 4. Configure Your Meter
```bash
# Create config directory
sudo mkdir -p /etc/hass-everblu-meter

# Copy example config
sudo cp config.toml.example /etc/hass-everblu-meter/config.toml

# Edit configuration
sudo nano /etc/hass-everblu-meter/config.toml
```

Follow the same configuration steps as Method 1 above.

#### 5. Install Systemd Service (Optional)

```bash
# Copy systemd files
sudo cp debian/hass-everblu-meter.service /etc/systemd/system/
sudo cp debian/hass-everblu-meter.timer /etc/systemd/system/

# Reload systemd and enable
sudo systemctl daemon-reload
sudo systemctl enable hass-everblu-meter.timer
sudo systemctl start hass-everblu-meter.timer
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

**⚠️ Battery Warning:** Reading the meter too frequently will drain its battery. The default is once per day, which is already more frequent than the manufacturer's recommended monthly reading schedule. See the installation section above for details on battery conservation.

If you installed via the Debian package, systemd service and timer files are already installed. Simply enable them:

```bash
sudo systemctl enable hass-everblu-meter.timer
sudo systemctl start hass-everblu-meter.timer
```

If you installed manually from source, the systemd files are located in the `debian/` directory:

```bash
sudo cp debian/hass-everblu-meter.service /etc/systemd/system/
sudo cp debian/hass-everblu-meter.timer /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable hass-everblu-meter.timer
sudo systemctl start hass-everblu-meter.timer
```

#### Checking Status

```bash
# Check timer status
systemctl status hass-everblu-meter.timer

# Check service logs
journalctl -u hass-everblu-meter.service -f

# List next scheduled runs
systemctl list-timers hass-everblu-meter.timer
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


