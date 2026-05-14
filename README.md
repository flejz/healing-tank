# Healing Tank Screen - ESP32 I2C Display Driver

Rust application for ESP32 displaying content on SSD1306 I2C OLED display.

## Configuration

Edit `src/main.rs` constants (top of file):

```rust
const I2C_SDA_PIN: i32 = 21;           // GPIO pin for I2C SDA
const I2C_SCL_PIN: i32 = 22;           // GPIO pin for I2C SCL
const I2C_FREQ_HZ: u32 = 400_000;      // I2C frequency Hz
const I2C_ADDRESS: u8 = 0x3C;          // SSD1306 I2C address
const DISPLAY_WIDTH: u32 = 128;        // Display width pixels
const DISPLAY_HEIGHT: u32 = 64;        // Display height pixels
```

## Prerequisites

- Rust toolchain: `rustup`
- ESP32 tools: `cargo install espup && espup install`
- USB serial driver for ESP32

## Build

```bash
cargo build --release
```

## Flash to ESP32

```bash
cargo espflash flash --release
```

Or specify port:
```bash
cargo espflash flash --release --port /dev/ttyUSB0
```

## Monitor Serial Output

```bash
cargo espflash monitor
```

## Supported Displays

- SSD1306 128x64 OLED (default)
- SSD1306 128x32 OLED (modify DisplaySize128x64 to DisplaySize128x32)
- Other displays: update `ssd1306` driver as needed

## Troubleshooting

**Display not detected**: Check I2C address (0x3C vs 0x3D) and GPIO pins

**I2C errors**: Verify SDA/SCL connections and pull-up resistors (usually 4.7k Ohm to 3.3V)

**Build errors**: Run `espup update` to update ESP-IDF
