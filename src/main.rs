use anyhow::Result;
use esp_idf_hal::i2c::I2cDriver;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::units::FromValueType;
use esp_idf_svc::log::EspLogger;
use log::info;
use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    text::Text,
};

// ============================================================================
// CONFIGURATION - Change these to customize for your setup
// ============================================================================
const I2C_SDA_PIN: i32 = 21;           // GPIO pin for I2C SDA
const I2C_SCL_PIN: i32 = 22;           // GPIO pin for I2C SCL
const I2C_FREQ_HZ: u32 = 400_000;      // I2C frequency in Hz (400kHz standard)
const I2C_ADDRESS: u8 = 0x3C;          // I2C address of SSD1306 (0x3C or 0x3D)
const DISPLAY_WIDTH: u32 = 128;        // Display width in pixels
const DISPLAY_HEIGHT: u32 = 64;        // Display height in pixels
// ============================================================================

fn main() -> Result<()> {
    esp_idf_svc::sys::link_patches();
    EspLogger::initialize_default();

    info!("Starting Healing Tank Screen...");

    let peripherals = Peripherals::take()?;
    let i2c = peripherals.i2c0;
    let sda = peripherals.pins.gpio21;
    let scl = peripherals.pins.gpio22;

    // Configure I2C
    let i2c_driver = I2cDriver::new(
        i2c,
        sda,
        scl,
        &esp_idf_hal::i2c::I2cConfig::new().baudrate(I2C_FREQ_HZ.Hz()),
    )?;

    info!("I2C configured. Attempting to communicate with display at address 0x{:02X}", I2C_ADDRESS);

    // Initialize SSD1306 display
    let interface = I2CDisplayInterface::new(i2c_driver);
    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();

    display.init().unwrap();

    info!("Display initialized successfully!");

    // Clear display
    display.clear(BinaryColor::Off).unwrap();

    // Draw text on display
    let text_style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
    Text::new("Healing Tank", Point::new(10, 15), text_style)
        .draw(&mut display)
        .unwrap();

    Text::new("Screen Ready", Point::new(10, 35), text_style)
        .draw(&mut display)
        .unwrap();

    // Flush display buffer to screen
    display.flush().unwrap();

    info!("Text rendered on display");

    loop {
        esp_idf_hal::delay::FreeRtos::delay_ms(1000);
        info!("Running...");
    }
}
