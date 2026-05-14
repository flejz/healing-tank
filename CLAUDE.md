# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

Rust firmware for ESP32 driving SSD1306 OLED display (128x64) over I2C. Uses `esp-idf-svc` (ESP-IDF framework via Rust), not `no_std` bare-metal.

## Build & Flash

```bash
# Source ESP toolchain env first (required every session)
source ~/export-esp.sh

# Build (cross-compiles to xtensa-esp32-espidf — target set in .cargo/config.toml)
cargo build --release

# Flash to connected ESP32 (use espflash directly, not cargo-espflash)
espflash flash target/xtensa-esp32-espidf/release/healing-tank-screen --port /dev/ttyUSB0

# Monitor serial output (115200 baud, configured in sdkconfig.defaults)
espflash monitor --port /dev/ttyUSB0
```

## Prerequisites

- `espup` must be installed and ESP toolchain initialized (`espup install`)
- `espflash` must be installed (`cargo install espflash`)
- `ldproxy` must be installed (`cargo install ldproxy`)
- User must be in `uucp` group for serial port access: `sudo usermod -aG uucp $USER` (re-login after)
- `source ~/export-esp.sh` sets PATH and LIBCLANG_PATH for the xtensa toolchain

## Architecture

Single-file firmware: `src/main.rs`. All hardware configuration is top-of-file constants (I2C pins, frequency, display address/size).

Runtime flow: init ESP-IDF → take peripherals → configure I2C driver → init SSD1306 in buffered graphics mode → draw with `embedded-graphics` → flush → infinite loop.

Display pipeline: `I2cDriver` → `I2CDisplayInterface` → `Ssd1306` (buffered mode) → `embedded-graphics` draw calls → `display.flush()`.

## Key Dependencies

- `esp-idf-svc` / `esp-idf-hal` — ESP-IDF HAL (I2C, peripherals, delays)
- `ssd1306` — display driver
- `embedded-graphics` — 2D drawing primitives and text

## Configuration

Edit constants at top of `src/main.rs` to change GPIO pins, I2C frequency, display address, or display size. For 128x32 displays change `DisplaySize128x64` → `DisplaySize128x32`.

## Display UI

Cyberpunk biohealing terminal UI. 5 rows on 128x64 OLED:

```
[= BIO-TANK MK.VII =]   ← inverted header bar
SUBJ:ONLINE  00:00:00   ← subject status + live session timer (counts up)
RGN [========░░] 78%    ← animated regen progress bar (0→100%, ~81s cycle)
>>NEURAL SYNC OK  <<    ← blinking brackets, cycles 8 status messages
O2:100%  TEMP:36.6C     ← static vitals
```

Animation loop runs at 100ms ticks. Status messages defined in `STATUS_MSGS` constant array in `src/main.rs`.

## Notes

`platformio.ini` is a leftover Arduino/PlatformIO config — ignore it, the project uses Cargo. No tests exist (embedded firmware, hardware-dependent).
