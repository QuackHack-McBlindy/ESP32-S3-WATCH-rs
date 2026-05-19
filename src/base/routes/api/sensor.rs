// BASE/ROUTES/API/SENSOR


// GET /API/SENSOR/{val}
pub fn handle_sensor(req: tinyapi::Request<'_>) -> tinyapi::Response {
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
    // let time
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

// GET /API/SENSORS (SHOWS ALL SENSOR VALUES IN ONE CALL FORMATTED AS JSON)
pub fn handle_sensors(_req: tinyapi::Request<'_>) -> tinyapi::Response {
    // LOAD EXISTING SENSORS
    // BATTERY
    let battery_percent = crate::load!(crate::state::BATTERY_PERCENT);
    let battery_voltage = crate::load!(crate::state::BATTERY_VOLTAGE);
    let battery_charging = crate::load!(crate::state::BATTERY_CHARGING);
    let battery_need_charging = crate::load!(crate::state::BATTERY_NEED_CHARGING);
    let battery_full = crate::load!(crate::state::BATTERY_FULL);
    let battery_usb_connected = crate::load!(crate::state::BATTERY_USB_CONNECTED);
        
    let rssi = crate::load!(crate::state::RSSI);
    let mic_vol = crate::load!(crate::state::MIC_VOLUME);
    let spk_vol = crate::load!(crate::state::SPEAKER_VOLUME);
    let display_state = crate::load!(crate::state::DISPLAY_STATE);
    let brightness = crate::load!(crate::state::DISPLAY_BRIGHTNESS);
    let ip_raw = crate::load!(crate::state::CURRENT_IP);
    let ip = embassy_net::Ipv4Address::from(ip_raw);
    let version = crate::state::FW_VERSION;

    // UPTIME (SECONDS SINCE BOOT)
    let uptime_secs = crate::load!(crate::state::UPTIME_SECS);
    let uptime_str = {
        let days = uptime_secs / 86400;
        let hours = (uptime_secs % 86400) / 3600;
        let minutes = (uptime_secs % 3600) / 60;
        let seconds = uptime_secs % 60;
        if days > 0 {
            alloc::format!("{}d {:02}h {:02}m {:02}s", days, hours, minutes, seconds)
        } else if hours > 0 {
            alloc::format!("{:02}h {:02}m {:02}s", hours, minutes, seconds)
        } else if minutes > 0 {
            alloc::format!("{:02}m {:02}s", minutes, seconds)
        } else {
            alloc::format!("{}s", seconds)
        }
    };

    // CONVERT & FORMAT TIME (HH:MM:SS)
    let time_secs = crate::load!(crate::state::CURRENT_TIME_SECS);
    let time_str = if time_secs > 0 {
        let hours = (time_secs % 86400) / 3600;
        let minutes = (time_secs % 3600) / 60;
        let seconds = time_secs % 60;
        alloc::format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    } else { "unknown".into() };

    // FORMAT AS JSON
    let response_str = alloc::format!(
        "{{\"battery_percent\":{},\"battery_voltage\":{},\"battery_charging\":{},\"battery_need_charging\":{},\"battery_full\":{},\"battery_usb_connected\":{},\"display_state\":{},\"rssi\":{},\"mic_volume\":{},\"speaker_volume\":{},\"brightness\":{},\"ip\":\"{}\",\"uptime\":\"{}\",\"time\":\"{}\",\"firmware\":\"{}\",\"media\":\"Nothing playing\"}}",
        battery_percent,
        battery_voltage,
        battery_charging,
        battery_need_charging,
        battery_full,
        battery_usb_connected,
        display_state,
        rssi,
        mic_vol,
        spk_vol,
        brightness,
        ip,
        uptime_str,
        time_str,
        version
    );

    tinyapi::Response::text(&response_str)
}

