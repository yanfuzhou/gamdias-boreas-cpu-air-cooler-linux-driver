# GAMDIAS BOREAS CPU Air Cooler Linux Driver

A Linux daemon for the GAMDIAS BOREAS CPU Air Cooler with digital display. This device is typically bundled with GAMDIAS CPU coolers and only comes with Windows software (ZEUS CAST).

This project provides a native Linux solution to display CPU temperature and fan speed on the device.

| CPU Air Cooler | USB ID |
|-------|-------------|
| [E2-41D](https://www.gamdias.com/en/component/cooler/BOREAS_E2-41D) |  |
| [M2-51D](https://www.gamdias.com/en/component/cooler/BOREAS_M2-51D) | `1B80:B554` |
| [M2-61L](https://www.gamdias.com/en/component/cooler/BOREAS_M2-61L) |  |
| [P2-62D](https://www.gamdias.com/en/component/cooler/BOREAS_P2-62D) | `1B80:B53A` |

## Features

- Display CPU temperature in Celsius or Fahrenheit
- Display CPU fan speed in RPM
- Rotate between multiple display modes on a configurable timer
- Auto-detection of CPU temperature and fan sensors
- Runs as a systemd daemon
- JSON configuration file

## Hardware

- **Device**: GAMDIAS BOREAS M2-51D Digital Display
- **USB ID**: `1B80:B554`
- **Protocol**: USB HID
- **Display**: 4-digit 7-segment with temperature/fan icons

## Prerequisites

- rustc 1.97.1 (8bab26f4f 2026-07-14)
- cargo 1.97.1 (c980f4866 2026-06-30)
- Linux kernel with hwmon support

### Install Dependencies

```bash
# Arch Linux
sudo pacman -S rustup
rustup default stable
```

## Building

```bash
cargo build --release
```

## Installation

### 1. Build and Install Binary

```bash
sudo mkdir -p /etc/boreas
sudo cp target/release/boreas /etc/boreas/boreas
sudo ln -sf /etc/boreas/boreas /usr/local/bin/boreas
```

### 2. Install udev Rules (for non-root access)

```bash
sudo cp install/99-boreas.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules
sudo udevadm trigger
```

### 3. Install Configuration

```bash
sudo cp config.json /etc/boreas/
```

### 4. Install systemd Service

```bash
sudo cp install/boreas.service /etc/systemd/system/
# run as root user
su - root
systemctl daemon-reload
systemctl enable --now boreas
```

## Configuration

Edit `/etc/boreas/config.json`:

```json
{
  "update_interval_ms": 500,
  "sensors": {
    "cpu_temp_path": null,
    "cpu_fan_path": null
  },
  "display": [
    {
      "mode": "CpuTempCelsius",
      "duration_seconds": 10
    },
    {
      "mode": "CpuFanSpeed",
      "duration_seconds": 6
    }
  ]
}
```

### Options

| Field | Description |
|-------|-------------|
| `update_interval_ms` | How often to update the display (milliseconds) |
| `cpu_temp_path` | Path to temperature sensor, or `null` for auto-detect |
| `cpu_fan_path` | Path to fan sensor, or `null` for auto-detect |
| `display` | List of display modes to rotate through |

### Display Modes

- `CpuTempCelsius` - CPU temperature in °C
- `CpuTempFahrenheit` - CPU temperature in °F
- `CpuFanSpeed` - CPU fan speed in RPM

## Usage

```bash
# Run with default config
boreas
```

## Fan Speed Monitoring

The device itself does not read fan speed - it only displays what you send it. Fan speed is read from Linux hwmon sensors.

### Finding Your Fan Sensor

Many motherboards require a specific kernel module to expose fan sensors:

```bash
# List available sensors
boreas --list-sensors

# Check loaded hwmon drivers
ls /sys/class/hwmon/*/name
```

### Gigabytes Motherboards (IT87)

Gigabytes motherboards often use the IT87 Super I/O chip which requires a third-party driver:

```bash
# This is tested on Gigabytes B550M Gaming X Wifi 6 Motherboard
# Clone and install the driver
git clone https://github.com/frankcrawford/it87.git
cd it87
sudo ./dkms-install.sh

# Load the module
sudo modprobe it87 ignore_resource_conflict=1

# Make it load on boot
echo "it87" | sudo tee /etc/modules-load.d/it87.conf
echo "options it87 ignore_resource_conflict=1" | sudo tee /etc/modprobe.d/it87.conf

# Verify fan sensors appear
cat /sys/class/hwmon/hwmon*/name | grep it8689
```

### Other Motherboards

Run `sudo sensors-detect` from the `lm-sensors` package to identify and load the correct driver for your hardware.

## Protocol Documentation

The USB HID protocol was reverse-engineered from the Windows ZEUS CAST application.

### Packet Structure

| Byte | Field | Description |
|------|-------|-------------|
| 0 | Header | `0x3A` |
| 1 | Header | `0xB5` |
| 2 | Command | `0x01` |
| 3-6 | Digits | Display digits (0-9 or 0x20 for blank) |
| 7 | Decimal | `0x01` = show decimal point |
| 8 | Unit | `0x01` = Celsius, `0x00` = Fahrenheit |
| 9 | CPU Mode | `0x01` = CPU temp mode |
| 10 | Display Mode | `0x00` = temperature, `0x01` = fan |
| 11 | Flashing | `0x01` = flash display |
| 12 | Checksum | Sum of bytes 0-11 & 0xFF |

### Display Modes

**Temperature Mode** (byte 10 = 0x00):
- Digits represent value × 10 (e.g., 457 = 45.7°)
- Decimal point shown between digit 3 and 4
- Shows °C or °F icon based on byte 8

**Fan Mode** (byte 10 = 0x01):
- Digits represent RPM directly (e.g., 1234 = 1234 RPM)
- No decimal point
- Shows fan icon

## Troubleshooting

### Device not found

1. Check the device is connected: `lsusb | grep 1b80`
2. Check udev rules are installed: `ls /etc/udev/rules.d/99-boreas.rules`
3. Reload udev: `sudo udevadm control --reload-rules && sudo udevadm trigger`

### No fan sensor detected

1. Run `boreas --list-sensors` to see available sensors
2. Install appropriate hwmon driver for your motherboard
3. Manually specify the fan path in config.json

## License

This project is licensed under the GNU General Public License v3.0 - see the LICENSE file for details.

## Acknowledgments

- GAMDIAS BOREAS P2-62D Linux Driver by [Riaan Aspeling](https://github.com/RiaanAspeling/gamdias-boreas-P2-62D-linux)
- Protocol reverse-engineered from GAMDIAS ZEUS CAST Windows application
- Uses [HidApi.Net](https://github.com/badcel/HidApi.Net) for USB HID communication
- IT87 driver by [frankcrawford](https://github.com/frankcrawford/it87)
