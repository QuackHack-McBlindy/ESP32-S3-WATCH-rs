// BASE/API
// CONFIGURES `GET` ENDPOINTS VIA `tinyapi`
// FOR CONTROLLING/CONFIGURING THE DEVICE EXTERNALLY
// ++ SERVE WEBSERVER AT `http://0.0.0.0:80`
// EXAMPLE USAGE: (SET DISPLAY BRIGHTNESS TO `70%` USING `curl`) 
// `curl 192.168.1.11:80/api/settings/display/brightness/70`
use tinyapi::{log, register_route, Request, Response};
use defmt::info;
use alloc::format;
use alloc::string::String;
use alloc::vec;
use embassy_net::Ipv4Address;

use crate::components::aht20::HUMIDITY;
use crate::components::aht20::TEMPERATURE;
use crate::components::presence::PRESENCE;
use crate::{BATTERY_PERCENT, BATTERY_VOLTAGE, RSSI, CURRENT_IP, MIC_VOLUME, SPEAKER_VOLUME, MIC_MUTED, SPEAKER_MUTED, BACKLIGHT_PERCENT, DISPLAY_STATE, FW_VERSION};
use crate::{init_bool, store, load};

// INIT ATOMIC DEFAULT VALUES
init_bool!(POWER_STATE, true);
init_bool!(MIC_ACTIVE, true);
init_bool!(PAUSE_FLAG, true);

// GET /API - RETURN LIST OF ENDPOINTS 
fn api_list_handler(_req: Request<'_>) -> Response {
    let endpoints = vec![
        "/",
        "/api/settings/power/state/{value}",
        "/api/settings/display/state/{value}",
        // ...
    ];
    Response::text(&endpoints.join("\n"))
}

// GET /INDEX.HTML - SERVES WEB FRONTEND 
fn index_handler(_req: Request<'_>) -> Response {
    Response::html(include_str!("./../../assets/index.html"))
}

// GET /FAVICON.ICO - SERVES FRONTEND FAVICON
fn favicon_handler(_req: Request<'_>) -> Response {
    //Response::file(include_bytes!("./../assets/favicon.ico"));
    Response::not_found()    
}

// GET /SCRIPT.JS - SERVES FRONTEND JAVASCRIPT
fn js_handler(_req: Request<'_>) -> Response {
    Response::script(include_str!("./../../assets/script.js"))
}

// GET /OTA - OVER THE AIR UPDATES 
fn ota_handler(_req: Request<'_>) -> Response {
    info!("OTA update requested");
    Response::text("update started")
}

// GET /API/SETTINGS/DISPLAY/BRIGHTNESS/{val}
pub fn brightness_handler(req: Request<'_>) -> Response {
    let value = req.param("value").unwrap_or("?");
    info!("Setting brightness to {}", value);
    if let Ok(percent) = value.parse::<u8>() {
        let percent = percent.clamp(0, 80);
        store!(BACKLIGHT_PERCENT, percent);
    }
    let msg = format!("Brightness set to {}", value);
    Response::text(&msg)
}

// GET /API/SETTINGS
fn power_state_handler(req: Request<'_>) -> Response {
    let value = req.param("value").unwrap_or("toggle");
    match value {
        "on" => store!(POWER_STATE, true),
        "off" => store!(POWER_STATE, false),
        _ => {
            let new = !load!(POWER_STATE);
            store!(POWER_STATE, new);
        }
    }
    let state = load!(POWER_STATE);
    info!("Power state -> {}", if state { "ON" } else { "OFF" });
    Response::text(if state { "ON" } else { "OFF" })
}

// GET /API/SETTINGS/DISPLAY
fn display_state_handler(req: Request<'_>) -> Response {
    let value = req.param("value").unwrap_or("toggle");
    match value {
        "on" => store!(DISPLAY_STATE, true),
        "off" => store!(DISPLAY_STATE, false),
        _ => {
            let new = !load!(DISPLAY_STATE);
            store!(DISPLAY_STATE, new);
        }
    }
    let state = load!(DISPLAY_STATE);
    info!("Display state -> {}", if state { "ON" } else { "OFF" });
    Response::text(if state { "ON" } else { "OFF" })
}

// GET /API/SETTINGS/MIC/VOLUME/{val}
fn mic_volume_handler(req: Request<'_>) -> Response {
    let value = req.param("value").unwrap_or("?");
    if let Ok(vol) = value.parse::<u8>() {
        let vol = vol.clamp(0, 100);
        store!(MIC_VOLUME, vol);
        info!("Mic volume set to {}%", vol);
    }
    Response::text(&format!("Mic volume {}", value))
}

fn mic_mute_handler(req: Request<'_>) -> Response {
    let value = req.param("value").unwrap_or("toggle");
    match value {
        "1" | "on" | "mute" => store!(MIC_MUTED, true),
        "0" | "off" | "unmute" => store!(MIC_MUTED, false),
        _ => {
            let new = !load!(MIC_MUTED);
            store!(MIC_MUTED, new);
        }
    }
    let muted = load!(MIC_MUTED);
    if muted {
        store!(MIC_VOLUME, 0);
    } else {
        store!(MIC_VOLUME, 72);
    }
    info!("Mic muted: {}", muted);
    Response::text(if muted { "muted" } else { "unmuted" })
}

fn speaker_volume_handler(req: Request<'_>) -> Response {
    let value = req.param("value").unwrap_or("?");
    if let Ok(vol) = value.parse::<u8>() {
        let vol = vol.clamp(0, 100);
        store!(SPEAKER_VOLUME, vol);
        info!("Speaker volume set to {}%", vol);
    }
    Response::text(&format!("Speaker volume {}", value))
}

fn speaker_mute_handler(req: Request<'_>) -> Response {
    let value = req.param("value").unwrap_or("toggle");
    match value {
        "1" | "on" | "mute" => store!(SPEAKER_MUTED, true),
        "0" | "off" | "unmute" => store!(SPEAKER_MUTED, false),
        _ => {
            let new = !load!(SPEAKER_MUTED);
            store!(SPEAKER_MUTED, new);
        }
    }
    let muted = load!(SPEAKER_MUTED);
    if muted {
        store!(SPEAKER_VOLUME, 0);
    } else {
        store!(SPEAKER_VOLUME, 58);
    }
    info!("Speaker muted: {}", muted);
    Response::text(if muted { "muted" } else { "unmuted" })
}

fn media_handler(req: Request<'_>) -> Response {
    let action = req.param("action").unwrap_or("none");
    info!("Media action: {}", action);
    let status = crate::apps::media::handle_action(action);
    Response::text(status)
}

// GET /API/SENSOR/{val}
fn sensor_fetcher(req: Request<'_>) -> Response {
    let sensor_name = req.param("value").unwrap_or("unknown");
    info!("Sensor fetch requested: {}", sensor_name);

    let battery_percent = load!(BATTERY_PERCENT);
    let battery_voltage = load!(BATTERY_VOLTAGE);
    let rssi = load!(RSSI);
    let mic_vol = load!(MIC_VOLUME);
    let spk_vol = load!(SPEAKER_VOLUME);
    let _mic_muted = load!(MIC_MUTED);
    let _spk_muted = load!(SPEAKER_MUTED);
    let temp = load!(TEMPERATURE);
    let hum = load!(HUMIDITY);
    let presence = load!(PRESENCE);
    let brightness = load!(BACKLIGHT_PERCENT);
    let ip_raw = load!(CURRENT_IP);
    let ip = Ipv4Address::from(ip_raw);
    // let uptime
    let version = FW_VERSION;
    // let media
    

    let response_str = match sensor_name {
        "temp" | "temperature" => format!("{}", temp),
        "hum" | "humidity" => format!("{}", hum),
        "battery" | "battery_level" | "battery_percentage" => format!("{}", battery_percent),
        "battery_voltage" | "voltage" => format!("{}", battery_voltage),
        "brightness" | "display" => format!("{}", brightness),
        "occupancy" | "motion" | "presence" => format!("{}", presence),
        "rssi" | "wifi_signal" | "wifi" => format!("{}", rssi),
        "ip" => format!("{}", ip),
        "ir" => String::from("11111"),
        "media" => String::from("Nothing playing.."),
        "speaker" => format!("{}", spk_vol),
        "mic" => format!("{}", mic_vol),
        "uptime" => format!("19:34"),        
        "time" => format!("19:34"),        
        "firmware" | "version" => format!("{}", version),
        _ => format!("unknown")
    };
    Response::text(&response_str)
}

fn voice_state_handler(req: Request<'_>) -> Response {
    let value = req.param("value").unwrap_or("toggle");
    match value {
        "start" => {
            info!("Voice recording started");
            store!(MIC_ACTIVE, true);
        }
        "stop" => {
            info!("Voice recording stopped");
            store!(MIC_ACTIVE, false);
        }
        _ => {
            info!("Invalid voice state: {}", value);
            return Response::text("invalid state (use start/stop)");
        }
    }
    Response::text("ok")
}

// FUNCTION TO INIT EDPOINTS
pub async fn init_routes() {
    // SERVE THE WEB FRONTEND
    register_route("/", index_handler).await;
    register_route("/favicon.ico", favicon_handler).await;
    register_route("/script.js", js_handler).await;
    // OTA
    register_route("/api/update", ota_handler).await;        
    // CONTROLLER ENDPOINTS
    register_route("/api/settings/power/state/{value}", power_state_handler).await;
    register_route("/api/settings/display/state/{value}", display_state_handler).await;
    register_route("/api/settings/display/brightness/{value}", brightness_handler).await;
    register_route("/api/settings/mic/volume/{value}", mic_volume_handler).await;
    register_route("/api/settings/mic/mute/{value}", mic_mute_handler).await;
    register_route("/api/settings/speaker/volume/{value}", speaker_volume_handler).await;
    register_route("/api/settings/speaker/mute/{value}", speaker_mute_handler).await;
    register_route("/api/settings/voice/state/{value}", voice_state_handler).await;     
    register_route("/api/media/{action}", media_handler).await;
    
    // DATA ENDPOINTS
    // HANDLE ALL SENSOR VALUES ON `ESP32-S3-BOX-3`
    register_route("/api", api_list_handler).await;
    register_route("/api/sensor/{value}", sensor_fetcher).await;

    tinyapi::log!("API routes registered");
    log!("API routes registered!")
}
