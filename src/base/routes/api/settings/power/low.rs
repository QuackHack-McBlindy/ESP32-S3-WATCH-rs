// BASE/ROUTES/API/SETTINGS/POWER/LOW

pub async fn low_power_on() {
    // BLUETOOTH
    if crate::load!(crate::state::BLUETOOTH_STATE) {
        crate::store!(crate::state::BLUETOOTH_STATE, false);
    }
    // AUDIO STREAMING
    if crate::load!(crate::state::SPEAKER_ALLOW_STREAMING) {
        crate::base::routes::api::settings::speaker::stream::toggle_stream().await;
    }
    // MUTE SPEAKER
    if crate::load!(crate::state::SPEAKER_VOLUME) != 0 {
        crate::set_speaker_volume(0);
    }
    // MUTE MIC
    if crate::load!(crate::state::MIC_VOLUME) != 0 {
        crate::set_mic_gain(0);
    }
    // SSH
    if crate::load!(crate::state::SSH_STATE) {
        crate::base::ssh::SSH_CMD.send(crate::base::ssh::SshCommand::Disable).await;
    }
    // SLEEP
    crate::store!(crate::state::POWERDOWN_TIMEOUT_SECS, 45);
    // UNDERCLOCK CPU TO 80 MHz
    crate::components::frequency::set_cpu_mhz(80);
    crate::store!(crate::state::CPU_FREQ, 80);
    // API
    if crate::load!(crate::state::API_STATE) {
        crate::base::routes::api::settings::api::off::toggle_api().await;
    }
    // WIFI
    if crate::load!(crate::state::WIFI_STATE) {
        crate::base::wifi::toggle_wifi().await;
    }
    crate::store!(crate::state::LOW_POWER_MODE, true);
}

pub async fn low_power_off() {
    // WIFI
    if !crate::load!(crate::state::WIFI_STATE) {
        crate::base::wifi::toggle_wifi().await;
    }
    // API
    if !crate::load!(crate::state::API_STATE) {
        crate::base::routes::api::settings::api::off::toggle_api().await;
    }
    // FULL CPU FREQUENCY
    crate::components::frequency::set_cpu_mhz(240);
    crate::store!(crate::state::CPU_FREQ, 240);
    // SLEEP TIMEOUT
    crate::store!(crate::state::POWERDOWN_TIMEOUT_SECS, 60);
    // SSH
    if !crate::load!(crate::state::SSH_STATE) {
        crate::base::ssh::SSH_CMD.send(crate::base::ssh::SshCommand::Enable).await;
    }
    // MIC
    if crate::load!(crate::state::MIC_VOLUME) == 0 {
        crate::set_mic_gain(70);
    }
    // SPEAKER
    if crate::load!(crate::state::SPEAKER_VOLUME) == 0 {
        crate::set_speaker_volume(65);
    }
    // AUDIO STREAMING
    if !crate::load!(crate::state::SPEAKER_ALLOW_STREAMING) {
        crate::base::routes::api::settings::speaker::stream::toggle_stream().await;
    }
    // BLUETOOTH
    if !crate::load!(crate::state::BLUETOOTH_STATE) {
        crate::store!(crate::state::BLUETOOTH_STATE, true);
    }
    crate::store!(crate::state::LOW_POWER_MODE, false);
}

pub async fn toggle_low_power() {
    if crate::load!(crate::state::LOW_POWER_MODE) {
        low_power_off().await;
    } else {
        low_power_on().await;
    }
}

pub async fn low_power_handler(req: tinyapi::AsyncRequest) -> tinyapi::Response {
    let value = req.param("value").unwrap_or("toggle");
    match value {
        "1" | "on" | "low" | "enable" | "enabled" => {
            if !crate::load!(crate::state::LOW_POWER_MODE) {
                low_power_on().await;
            }
        }
        "0" | "off" | "high" | "disable" | "disabled" => {
            if crate::load!(crate::state::LOW_POWER_MODE) {
                low_power_off().await;
            }
        }
        _ => {
            toggle_low_power().await;
        }
    }
    let state = crate::load!(crate::state::LOW_POWER_MODE);
    let msg = if state {
        "Low‑power mode enabled – all radios, audio, and display turned off"
    } else {
        "Low‑power mode disabled – normal operation restored"
    };
    tinyapi::Response::text(msg)
}
