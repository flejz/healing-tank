mod config;
mod server;
mod state;
mod wifi;

use anyhow::Result;
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::i2c::{I2cConfig, I2cDriver};
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::rmt::{
    config::{TransmitConfig, TxChannelConfig},
    encoder::{BytesEncoder, BytesEncoderConfig},
    PinState, Pulse, PulseTicks, Symbol, TxChannelDriver,
};
use esp_idf_hal::units::FromValueType;
use esp_idf_svc::{eventloop::EspSystemEventLoop, log::EspLogger, nvs::EspDefaultNvsPartition};

use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::Text,
};
use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};

use state::{AppState, LedMode, SharedState, WifiStatus};
use std::sync::{Arc, Mutex};

const I2C_FREQ_HZ: u32 = 400_000;
const LOOP_MS: u32 = 100;
const LED_COUNT: usize = 22;

const GLITCH_MSGS: &[&str] = &[
    "##ERR:0xDEAD####",
    ">>>CORRUPTION<<<",
    "!!SIGNAL_LOST!!!",
    "??REALIGN_BIO???",
    "###GLITCH_MEM###",
];

const STATUS_MSGS: &[&str] = &[
    "NEURAL SYNC OK  ",
    "CELL REGEN ACTV ",
    "BIO-FLUID STABLE",
    "TISSUE REPAIR...",
    "SYS NOMINAL     ",
    "O2 SAT OPTIMAL  ",
    "STASIS LOCKED   ",
    "VITALS NOMINAL  ",
];

fn prng(seed: u32) -> u32 {
    seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223)
}

fn hsv_to_rgb(h: u16, s: u8, v: u8) -> (u8, u8, u8) {
    if s == 0 {
        return (v, v, v);
    }
    let region = (h / 60) % 6;
    let rem = (h % 60) as u32 * 255 / 60;
    let sv = s as u32;
    let vv = v as u32;
    let p = (vv * (255 - sv) / 255) as u8;
    let q = (vv * (255 - sv * rem / 255) / 255) as u8;
    let t = (vv * (255 - sv * (255 - rem) / 255) / 255) as u8;
    match region {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    }
}

fn main() -> Result<()> {
    esp_idf_svc::sys::link_patches();
    EspLogger::initialize_default();

    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    let state: SharedState = Arc::new(Mutex::new(AppState::default()));
    wifi::spawn_wifi(peripherals.modem, sys_loop, nvs, state.clone());

    let i2c_driver = I2cDriver::new(
        peripherals.i2c0,
        peripherals.pins.gpio21,
        peripherals.pins.gpio22,
        &I2cConfig::new().baudrate(I2C_FREQ_HZ.Hz()),
    )?;

    // WS2812B at 10 MHz (100 ns/tick): bit0 = 400 ns high + 800 ns low, bit1 = 800 ns high + 400 ns low
    let bit0 = Symbol::new(
        Pulse::new(PinState::High, PulseTicks::new(4).unwrap()),
        Pulse::new(PinState::Low, PulseTicks::new(8).unwrap()),
    );
    let bit1 = Symbol::new(
        Pulse::new(PinState::High, PulseTicks::new(8).unwrap()),
        Pulse::new(PinState::Low, PulseTicks::new(4).unwrap()),
    );
    let mut encoder = BytesEncoder::with_config(&BytesEncoderConfig {
        bit0,
        bit1,
        msb_first: true,
        ..Default::default()
    })?;
    let mut rmt_tx = TxChannelDriver::new(
        peripherals.pins.gpio2,
        &TxChannelConfig {
            resolution: 10_000_000u32.Hz(),
            ..Default::default()
        },
    )?;
    let transmit_cfg = TransmitConfig::default();

    let interface = I2CDisplayInterface::new(i2c_driver);
    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    display.init().unwrap();

    let on = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
    let off = MonoTextStyle::new(&FONT_6X10, BinaryColor::Off);
    let fill_on = PrimitiveStyle::with_fill(BinaryColor::On);
    let stroke_on = PrimitiveStyle::with_stroke(BinaryColor::On, 1);

    // Pre-build ticker tape: all status messages joined, scrolls continuously
    let ticker_tape: String = STATUS_MSGS.iter()
        .map(|s| format!("{} /// ", s))
        .collect();
    let ticker_px = (ticker_tape.len() * 6) as i32;

    let mut tick: u32 = 0;
    let mut secs: u32 = 0;
    let mut ms_acc: u32 = 0;

    loop {
        let glitch_phase = tick % 83;

        // Read shared state once per tick
        let (led_mode, brightness, color, custom_msg, wifi_row) = {
            let s = state.lock().unwrap();
            let dots = ".".repeat(((tick / 3) % 4) as usize);
            let wr = match &s.wifi_status {
                WifiStatus::Connecting => format!("WIFI: CONNECTING{}", dots),
                WifiStatus::Connected(ip) => format!("WIFI: {}", ip),
                WifiStatus::Failed => "WIFI: FAILED".to_string(),
            };
            (s.mode.clone(), s.brightness, s.color, s.custom_message.clone(), wr)
        };

        display.clear(BinaryColor::Off).unwrap();

        // ── Row 1: inverted header ──────────────────────────────────────
        Rectangle::new(Point::new(0, 0), Size::new(128, 11))
            .into_styled(fill_on)
            .draw(&mut display)
            .unwrap();
        Text::new("= BIO-TANK MK.VII =", Point::new(4, 9), off)
            .draw(&mut display)
            .unwrap();

        // ── Row 2: subject status + session timer ───────────────────────
        Text::new("SUBJ:ONLINE", Point::new(0, 20), on)
            .draw(&mut display)
            .unwrap();
        let time_str = format!(
            "{:02}:{:02}:{:02}",
            secs / 3600,
            (secs % 3600) / 60,
            secs % 60
        );
        Text::new(&time_str, Point::new(80, 20), on)
            .draw(&mut display)
            .unwrap();

        // ── Row 3: regen progress bar ───────────────────────────────────
        let pct = (tick / 8) % 101;
        let bar_fill = pct * 76 / 100;
        Text::new("RGN", Point::new(0, 31), on)
            .draw(&mut display)
            .unwrap();
        Rectangle::new(Point::new(22, 23), Size::new(78, 8))
            .into_styled(stroke_on)
            .draw(&mut display)
            .unwrap();
        if bar_fill > 0 {
            Rectangle::new(Point::new(23, 24), Size::new(bar_fill, 6))
                .into_styled(fill_on)
                .draw(&mut display)
                .unwrap();
        }
        let pct_str = format!("{:3}%", pct);
        Text::new(&pct_str, Point::new(103, 31), on)
            .draw(&mut display)
            .unwrap();

        // ── Row 4: scrolling ticker (or static custom message) ──────────
        match custom_msg {
            Some(ref text) => {
                // Custom message: static, centered
                let clamped = &text[..text.len().min(21)];
                let x = ((128 - clamped.len() as i32 * 6) / 2).max(0);
                Text::new(clamped, Point::new(x, 42), on)
                    .draw(&mut display)
                    .unwrap();
            }
            None => {
                // Marquee: 1px/tick (100ms) = 10px/s leftward scroll
                let scroll = (tick % ticker_px as u32) as i32;
                let x0 = -scroll;
                Text::new(&ticker_tape, Point::new(x0, 42), on)
                    .draw(&mut display)
                    .unwrap();
                // Wrap-around second copy when tape end approaches
                if x0 + ticker_px < 128 {
                    Text::new(&ticker_tape, Point::new(x0 + ticker_px, 42), on)
                        .draw(&mut display)
                        .unwrap();
                }
            }
        }

        // ── Row 5: vitals ───────────────────────────────────────────────
        Text::new("O2:100%  TEMP:36.6C", Point::new(0, 53), on)
            .draw(&mut display)
            .unwrap();

        // ── Row 6: WiFi status (left-aligned) ───────────────────────────
        Text::new(&wifi_row, Point::new(0, 63), on)
            .draw(&mut display)
            .unwrap();

        // ── Glitch overlay ──────────────────────────────────────────────
        if glitch_phase < 3 {
            let mut rng = prng(tick ^ 0xCAFE_BABE);
            let strip_count = 2 + (glitch_phase as usize);
            for _ in 0..strip_count {
                rng = prng(rng);
                let y = (rng >> 16) as i32 % 64;
                rng = prng(rng);
                let h = 1 + ((rng >> 24) % 3) as u32;
                Rectangle::new(Point::new(0, y), Size::new(128, h))
                    .into_styled(fill_on)
                    .draw(&mut display)
                    .unwrap();
            }
            if glitch_phase == 1 {
                Rectangle::new(Point::new(0, 33), Size::new(128, 11))
                    .into_styled(PrimitiveStyle::with_fill(BinaryColor::Off))
                    .draw(&mut display)
                    .unwrap();
                rng = prng(rng);
                let gmsg = GLITCH_MSGS[(rng as usize) % GLITCH_MSGS.len()];
                rng = prng(rng);
                let x_off = (rng % 8) as i32;
                Text::new(gmsg, Point::new(x_off, 42), on)
                    .draw(&mut display)
                    .unwrap();
            }
        }

        display.flush().unwrap();

        // ── LED update ──────────────────────────────────────────────────
        let mut pixels = [0u8; LED_COUNT * 3];
        match led_mode {
            LedMode::Off => {}
            LedMode::Solid => {
                let v = brightness as u16;
                for i in 0..LED_COUNT {
                    pixels[i * 3] = (v * color.1 as u16 / 255) as u8; // G
                    pixels[i * 3 + 1] = (v * color.0 as u16 / 255) as u8; // R
                    pixels[i * 3 + 2] = (v * color.2 as u16 / 255) as u8; // B
                }
            }
            LedMode::Glitch => {
                let mut rng = prng(tick ^ 0xBEEF_FADE);
                for i in 0..LED_COUNT {
                    rng = prng(rng);
                    let v = (rng >> 24) as u16 * brightness as u16 / 255;
                    pixels[i * 3] = (v * color.1 as u16 / 255) as u8; // G
                    pixels[i * 3 + 1] = (v * color.0 as u16 / 255) as u8; // R
                    pixels[i * 3 + 2] = (v * color.2 as u16 / 255) as u8; // B
                }
            }
            LedMode::Breathing => {
                let phase = (tick % 60) as f32;
                let t = phase / 60.0;
                let sine = libm::sinf(t * core::f32::consts::TAU);
                let base = (137.5 + 117.5 * sine) as u16;
                let scaled = base * brightness as u16 / 255;
                for i in 0..LED_COUNT {
                    pixels[i * 3] = (scaled * color.1 as u16 / 255) as u8; // G
                    pixels[i * 3 + 1] = (scaled * color.0 as u16 / 255) as u8; // R
                    pixels[i * 3 + 2] = (scaled * color.2 as u16 / 255) as u8; // B
                }
            }
            LedMode::Rainbow => {
                for i in 0..LED_COUNT {
                    let hue = ((tick as u32 * 5 + i as u32 * (360 / LED_COUNT as u32)) % 360) as u16;
                    let (r, g, b) = hsv_to_rgb(hue, 255, brightness);
                    pixels[i * 3] = g; // GRB
                    pixels[i * 3 + 1] = r;
                    pixels[i * 3 + 2] = b;
                }
            }
        }
        unsafe { rmt_tx.start_send(&mut encoder, &pixels, &transmit_cfg).unwrap() };
        rmt_tx.wait_all_done(None).unwrap();

        FreeRtos::delay_ms(LOOP_MS);
        tick += 1;
        ms_acc += LOOP_MS;
        if ms_acc >= 1000 {
            secs += 1;
            ms_acc = 0;
        }
    }
}
