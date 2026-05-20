use std::sync::{Arc, Mutex};

#[derive(Clone, PartialEq)]
pub enum LedMode {
    Breathing,
    Glitch,
    Rainbow,
    Off,
}

pub enum WifiStatus {
    Connecting,
    Connected(String),
    Failed,
}

pub struct AppState {
    pub mode: LedMode,
    pub brightness: u8,
    pub color: (u8, u8, u8),
    pub custom_message: Option<String>,
    pub wifi_status: WifiStatus,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            mode: LedMode::Breathing,
            brightness: 255,
            color: (255, 0, 0),
            custom_message: None,
            wifi_status: WifiStatus::Connecting,
        }
    }
}

pub type SharedState = Arc<Mutex<AppState>>;
