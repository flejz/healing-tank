use anyhow::Result;
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::i2c::{I2cConfig, I2cDriver};
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::units::FromValueType;
use esp_idf_svc::log::EspLogger;

use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::Text,
};
use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};

const I2C_FREQ_HZ: u32 = 400_000;
const LOOP_MS: u32 = 100;

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

fn main() -> Result<()> {
    esp_idf_svc::sys::link_patches();
    EspLogger::initialize_default();

    let peripherals = Peripherals::take()?;
    let i2c_driver = I2cDriver::new(
        peripherals.i2c0,
        peripherals.pins.gpio21,
        peripherals.pins.gpio22,
        &I2cConfig::new().baudrate(I2C_FREQ_HZ.Hz()),
    )?;

    let interface = I2CDisplayInterface::new(i2c_driver);
    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    display.init().unwrap();

    let on = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
    let off = MonoTextStyle::new(&FONT_6X10, BinaryColor::Off);
    let fill_on = PrimitiveStyle::with_fill(BinaryColor::On);
    let stroke_on = PrimitiveStyle::with_stroke(BinaryColor::On, 1);

    let mut tick: u32 = 0;
    let mut secs: u32 = 0;
    let mut ms_acc: u32 = 0;

    loop {
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
        // pct cycles 0→100 every ~808 ticks (~81s), then resets
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

        // ── Row 4: cycling status with blinking brackets ────────────────
        let msg_idx = (tick / 25) as usize % STATUS_MSGS.len();
        let msg = STATUS_MSGS[msg_idx];
        // brackets blink: on for 20 ticks, off for 5
        let brackets = (tick % 25) < 20;
        let status_str = if brackets {
            format!(">>{}<< ", msg)
        } else {
            format!("  {}   ", msg)
        };
        Text::new(&status_str, Point::new(0, 42), on)
            .draw(&mut display)
            .unwrap();

        // ── Row 5: vitals ───────────────────────────────────────────────
        Text::new("O2:100%  TEMP:36.6C", Point::new(0, 53), on)
            .draw(&mut display)
            .unwrap();

        display.flush().unwrap();

        FreeRtos::delay_ms(LOOP_MS);
        tick += 1;
        ms_acc += LOOP_MS;
        if ms_acc >= 1000 {
            secs += 1;
            ms_acc = 0;
        }
    }
}
