# WiFi + Web Control Interface — Design Spec

## Overview

Add WiFi connectivity and a browser-based control panel to the healing-tank ESP32 firmware. WiFi is non-blocking: the OLED and LED animation run immediately at boot with default state. If WiFi connects, an HTTP server starts and the UI becomes controllable. If WiFi fails, everything runs as-is.

## Architecture

Two concurrent execution contexts share an `Arc<Mutex<AppState>>`:

1. **Main loop** (existing, CPU0) — OLED draw + LED update. Reads `AppState` each iteration.
2. **HTTP server** (new) — `EspHttpServer` (from `esp_idf_svc::http::server`), internal FreeRTOS task. Handlers write `AppState`.

WiFi init is fire-and-forget: spawned before the main loop, does not block it. On successful IP assignment, the HTTP server is started. On failure, the server never starts.

## Shared State

```rust
enum LedMode { Breathing, Glitch, Off }

struct AppState {
    mode: LedMode,
    brightness: u8,         // 0–255, scales LED output
    color: (u8, u8, u8),    // R, G, B
    custom_message: Option<String>, // None = cycle STATUS_MSGS
    wifi_status: WifiStatus,
}

enum WifiStatus { Connecting, Connected(String), Failed }
```

Defaults match current behavior:
- `mode`: Breathing
- `brightness`: 255
- `color`: (255, 0, 0)
- `custom_message`: None
- `wifi_status`: Connecting

## WiFi

- Mode: STA (joins existing network)
- Credentials: compile-time constants in `src/config.rs` (`WIFI_SSID`, `WIFI_PASS`)
- `src/config.rs` is gitignored
- Init via `esp_idf_svc::wifi::EspWifi`

## Display Changes

Header bar shows WiFi status:
- Connecting: `= BIO-TANK MK.VII [W]=` — `[W]` blinks on/off via tick
- Connected:  `= BIO-TANK MK.VII [✓]=`
- Failed:     `= BIO-TANK MK.VII [✗]=`

Custom message: when `AppState.custom_message` is `Some(text)`, that text replaces the cycling STATUS_MSGS in row 4. When `None`, cycling resumes as before.

## LED Changes

Main loop applies `AppState` to LED output:
- `Off`: send all-zero pixels
- `Glitch`: existing random flash behavior, color tinted by `AppState.color`
- `Breathing`: sine-based breathing, color from `AppState.color`, amplitude scaled by `AppState.brightness`

## HTTP Server

HTML page embedded at compile time via `include_str!("../web/index.html")`.

### Endpoints

| Method | Path | Body | Action |
|--------|------|------|--------|
| GET | `/` | — | Serve `index.html` |
| GET | `/api/state` | — | Return full `AppState` as JSON |
| POST | `/api/mode` | `{"mode":"breathing"\|"glitch"\|"off"}` | Set LED mode |
| POST | `/api/brightness` | `{"value":0-255}` | Set brightness |
| POST | `/api/color` | `{"r":0-255,"g":0-255,"b":0-255}` | Set color |
| POST | `/api/message` | `{"text":"..."}` | Set custom message (`""` = resume cycling) |

No polling. UI sends fetch POST on each control change.

## Web UI

Single HTML file at `web/index.html`, embedded in firmware binary. Cyberpunk terminal aesthetic: black background, green/cyan accents, monospace font, no JS frameworks.

Layout:
```
╔══ BIO-TANK MK.VII CONTROL ══╗
│ MODE:  [BREATHING] [GLITCH] [OFF]        │
│ BRIGHTNESS: ████████░░  178              │
│ COLOR:  R[███░] G[░░░░] B[██░░]          │
│ STATUS: [NEURAL SYNC OK__________] [CLR] │
╚══════════════════════════════════════════╝
```

## File Structure

```
src/
  main.rs       — main loop, wiring
  config.rs     — WIFI_SSID, WIFI_PASS (gitignored)
  state.rs      — AppState, LedMode, WifiStatus
  wifi.rs       — WiFi init, fire-and-forget spawn
  server.rs     — EspHttpServer setup, all route handlers
web/
  index.html    — embedded UI
.gitignore      — add src/config.rs, web/ optional
```

## New Dependencies

- No new crates — `esp_idf_svc` already provides `EspWifi` and `EspHttpServer`
- `serde` + `serde_json` for JSON request/response parsing
