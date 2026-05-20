use core::convert::TryInto;

use esp_idf_svc::wifi::{AuthMethod, ClientConfiguration, Configuration};
use esp_idf_hal::modem::Modem;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    handle::RawHandle,
    nvs::EspDefaultNvsPartition,
    wifi::{BlockingWifi, EspWifi},
};

use crate::config::{WIFI_PASS, WIFI_SSID};
use crate::server::start_server;
use crate::state::{SharedState, WifiStatus};

pub fn spawn_wifi(
    modem: Modem<'static>,
    sys_loop: EspSystemEventLoop,
    nvs: EspDefaultNvsPartition,
    state: SharedState,
) {
    std::thread::Builder::new()
        .stack_size(8192)
        .spawn(move || match connect(modem, sys_loop, nvs) {
            Ok((ip, wifi)) => {
                log::info!("WiFi connected: {}", ip);
                state.lock().unwrap().wifi_status = WifiStatus::Connected(ip);
                match start_server(state) {
                    Ok(server) => {
                        core::mem::forget(wifi);
                        core::mem::forget(server);
                    }
                    Err(e) => log::error!("HTTP server error: {:?}", e),
                }
            }
            Err(e) => {
                log::error!("WiFi failed: {:?}", e);
                state.lock().unwrap().wifi_status = WifiStatus::Failed;
            }
        })
        .expect("WiFi thread spawn");
}

fn connect(
    modem: Modem<'static>,
    sys_loop: EspSystemEventLoop,
    nvs: EspDefaultNvsPartition,
) -> anyhow::Result<(String, BlockingWifi<EspWifi<'static>>)> {
    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(modem, sys_loop.clone(), Some(nvs))?,
        sys_loop,
    )?;

    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: WIFI_SSID.try_into().unwrap(),
        password: WIFI_PASS.try_into().unwrap(),
        auth_method: AuthMethod::WPA2Personal,
        ..Default::default()
    }))?;

    wifi.start()?;
    wifi.connect()?;
    wifi.wait_netif_up()?;

    // Set DHCP hostname so the device announces itself as "healing-tank" on the network
    let hostname = c"healing-tank";
    unsafe { esp_idf_svc::sys::esp_netif_set_hostname(wifi.wifi().sta_netif().handle(), hostname.as_ptr()) };

    let ip = wifi.wifi().sta_netif().get_ip_info()?.ip.to_string();
    Ok((ip, wifi))
}
