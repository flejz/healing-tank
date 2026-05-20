use esp_idf_svc::http::server::{Configuration, EspHttpConnection, EspHttpServer};
use esp_idf_svc::http::Method;
use esp_idf_svc::io::Write;
use serde::Deserialize;

use crate::state::{AppState, LedMode, SharedState, WifiStatus};

const MAX_BODY: usize = 256;

static INDEX_HTML: &str = include_str!("../web/index.html");

#[derive(Deserialize)]
struct ModeBody {
    mode: String,
}

#[derive(Deserialize)]
struct BrightnessBody {
    value: u8,
}

#[derive(Deserialize)]
struct ColorBody {
    r: u8,
    g: u8,
    b: u8,
}

#[derive(Deserialize)]
struct MessageBody {
    text: String,
}

pub fn start_server(state: SharedState) -> anyhow::Result<EspHttpServer<'static>> {
    let cfg = Configuration {
        stack_size: 10240,
        ..Default::default()
    };
    let mut server = EspHttpServer::new(&cfg)?;

    server.fn_handler::<anyhow::Error, _>("/", Method::Get, |req| {
        Ok(req
            .into_response(200, Some("OK"), &[("Content-Type", "text/html")])?
            .write_all(INDEX_HTML.as_bytes())?)
    })?;

    let s1 = state.clone();
    server.fn_handler::<anyhow::Error, _>("/api/state", Method::Get, move |req| {
        let json = state_json(&s1.lock().unwrap());
        Ok(req
            .into_response(200, Some("OK"), &[("Content-Type", "application/json")])?
            .write_all(json.as_bytes())?)
    })?;

    let s2 = state.clone();
    server.fn_handler::<anyhow::Error, _>("/api/mode", Method::Post, move |mut req| {
        let buf = body_bytes(&mut req);
        if let Ok(body) = serde_json::from_slice::<ModeBody>(&buf) {
            let mode = match body.mode.as_str() {
                "breathing" => Some(LedMode::Breathing),
                "glitch" => Some(LedMode::Glitch),
                "rainbow" => Some(LedMode::Rainbow),
                "off" => Some(LedMode::Off),
                _ => None,
            };
            if let Some(m) = mode {
                s2.lock().unwrap().mode = m;
            }
        }
        Ok(req.into_ok_response()?.write_all(b"OK")?)
    })?;

    let s3 = state.clone();
    server.fn_handler::<anyhow::Error, _>("/api/brightness", Method::Post, move |mut req| {
        let buf = body_bytes(&mut req);
        if let Ok(body) = serde_json::from_slice::<BrightnessBody>(&buf) {
            s3.lock().unwrap().brightness = body.value;
        }
        Ok(req.into_ok_response()?.write_all(b"OK")?)
    })?;

    let s4 = state.clone();
    server.fn_handler::<anyhow::Error, _>("/api/color", Method::Post, move |mut req| {
        let buf = body_bytes(&mut req);
        if let Ok(body) = serde_json::from_slice::<ColorBody>(&buf) {
            s4.lock().unwrap().color = (body.r, body.g, body.b);
        }
        Ok(req.into_ok_response()?.write_all(b"OK")?)
    })?;

    let s5 = state.clone();
    server.fn_handler::<anyhow::Error, _>("/api/message", Method::Post, move |mut req| {
        let buf = body_bytes(&mut req);
        if let Ok(body) = serde_json::from_slice::<MessageBody>(&buf) {
            let mut s = s5.lock().unwrap();
            s.custom_message = if body.text.is_empty() {
                None
            } else {
                Some(body.text)
            };
        }
        Ok(req.into_ok_response()?.write_all(b"OK")?)
    })?;

    Ok(server)
}

fn body_bytes(req: &mut esp_idf_svc::http::server::Request<&mut EspHttpConnection<'_>>) -> Vec<u8> {
    let mut buf = [0u8; MAX_BODY];
    let mut total = 0;
    loop {
        match req.read(&mut buf[total..]) {
            Ok(0) | Err(_) => break,
            Ok(n) => total += n,
        }
    }
    buf[..total].to_vec()
}

fn state_json(s: &AppState) -> String {
    let mode = match s.mode {
        LedMode::Breathing => "breathing",
        LedMode::Glitch => "glitch",
        LedMode::Rainbow => "rainbow",
        LedMode::Off => "off",
    };
    let wifi = match &s.wifi_status {
        WifiStatus::Connecting => r#""connecting""#.to_string(),
        WifiStatus::Connected(ip) => format!(r#""{}""#, ip),
        WifiStatus::Failed => r#""failed""#.to_string(),
    };
    let msg = match &s.custom_message {
        Some(m) => format!(r#""{}""#, m.replace('"', "\\\"")),
        None => "null".to_string(),
    };
    format!(
        r#"{{"mode":"{}","brightness":{},"color":{{"r":{},"g":{},"b":{}}},"message":{},"wifi":{}}}"#,
        mode, s.brightness, s.color.0, s.color.1, s.color.2, msg, wifi
    )
}
