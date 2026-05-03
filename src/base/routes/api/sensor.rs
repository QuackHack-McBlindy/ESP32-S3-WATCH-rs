
// GET /API/SENSOR/{val}
pub fn handle(req: tinyapi::Request<'_>) -> tinyapi::Response {
    let sensor_name = req.param("value").unwrap_or("unknown");
    defmt::info!("Sensor fetch requested: {}", sensor_name);

    let battery_percent = crate::load!(crate::state::BATTERY_PERCENT);
    let battery_voltage = crate::load!(crate::state::BATTERY_VOLTAGE);
    let rssi = crate::load!(crate::state::RSSI);
    let mic_vol = crate::load!(crate::state::MIC_VOLUME);
    let spk_vol = crate::load!(crate::state::SPEAKER_VOLUME);
    let _mic_muted = crate::load!(crate::state::MIC_MUTED);
    let _spk_muted = crate::load!(crate::state::SPEAKER_MUTED);

    let brightness = crate::load!(crate::state::DISPLAY_BRIGHTNESS);
    let ip_raw = crate::load!(crate::state::CURRENT_IP);
    let ip = embassy_net::Ipv4Address::from(ip_raw);
    // let uptime
    let version = crate::state::FW_VERSION;

    

    let response_str = match sensor_name {
        "battery" | "battery_level" | "battery_percentage" => alloc::format!("{}", battery_percent),
        "battery_voltage" | "voltage" => alloc::format!("{}", battery_voltage),
        "brightness" | "display" => alloc::format!("{}", brightness),
        "rssi" | "wifi_signal" | "wifi" => alloc::format!("{}", rssi),
        "ip" => alloc::format!("{}", ip),
        "media" => alloc::string::String::from("Nothing playing.."),
        "speaker" => alloc::format!("{}", spk_vol),
        "mic" => alloc::format!("{}", mic_vol),
        "uptime" => alloc::format!("19:34"),        
        "time" => alloc::format!("19:34"),        
        "firmware" | "version" => alloc::format!("{}", version),
        _ => alloc::format!("unknown")
    };
    tinyapi::Response::text(&response_str)
}

