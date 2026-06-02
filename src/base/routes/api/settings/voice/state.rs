// BASE/ROUTES/API/SETTINGS/VOICE/STATE
// CHANGE STATE OF THE VOICE ASSISTANT
// THIS CHECKS EVERY DEPENDENCY OF THE VOICE ASSISTANT - TOGGLES ANYTHING IT WILL NEED TO WORK   

// ───────────────────────────────────────────────────────────────────────
// VOICE ASSISTANT ON!
pub async fn voice_on() {
    // IF ALREADY ON - EXIT!
    if crate::load!(crate::state::VOICE_STATE) {
        return;
    }
    // WIFI
    if !crate::load!(crate::state::WIFI_STATE) {
        crate::base::wifi::WIFI_CMD.send(crate::base::wifi::WifiCommand::Enable).await;
        crate::store!(crate::state::WIFI_STATE, true);
        defmt::info!("Wi‑Fi enabled");
    } // API
    if !crate::load!(crate::state::API_STATE) {
        tinyapi::SERVER_CMD.send(tinyapi::ServerCommand::Start).await;
        crate::store!(crate::state::API_STATE, true);
        defmt::info!("API enabled");
    } // MIC
    if crate::load!(crate::state::MIC_VOLUME) == 0 {
        crate::set_mic_gain(72);
        crate::store!(crate::state::MIC_MUTED, false);
    } // SPEAKER
    if crate::load!(crate::state::SPEAKER_VOLUME) == 0 {
        crate::set_speaker_volume(58);
        crate::store!(crate::state::SPEAKER_MUTED, false);
    } // SPEAKER TASK
    if !crate::load!(crate::state::SPEAKER_TASK_STATE) {
        yo_esp::SPEAKER_CMD.send(yo_esp::SpeakerCommand::Start).await;
        crate::store!(crate::state::SPEAKER_TASK_STATE, true);
    } // AUDIO STREAMING
    if !crate::load!(crate::state::SPEAKER_ALLOW_STREAMING) {
        yo_esp::STREAM_CMD.send(yo_esp::StreamCommand::Start).await;
        crate::store!(crate::state::SPEAKER_ALLOW_STREAMING, true);        
    } // AMPLIFIER
    if !crate::load!(crate::state::AMPLIFIER_STATE) {
        crate::amp_on();
    } // SAVE
    crate::store!(crate::state::VOICE_STATE, true);
    defmt::info!("VOICE ASSISTANT: ON");
}


// ───────────────────────────────────────────────────────────────────────
// VOICE ASSISTANT OFF!
pub async fn voice_off() {
    // IF ALREADY OFF - EXIT!
    if !crate::load!(crate::state::VOICE_STATE) {
        return;
    } // AMP
    if crate::load!(crate::state::AMPLIFIER_STATE) {
        crate::amp_off();
    } // AUDIO STREAMING
    if crate::load!(crate::state::SPEAKER_ALLOW_STREAMING) {
        yo_esp::STREAM_CMD.send(yo_esp::StreamCommand::Stop).await;
        crate::store!(crate::state::SPEAKER_ALLOW_STREAMING, false);   
    } // SPEAKER
    if crate::load!(crate::state::SPEAKER_VOLUME) != 0 {
        crate::set_speaker_volume(0);
        crate::store!(crate::state::SPEAKER_MUTED, true);
    } // MIC
    if crate::load!(crate::state::MIC_VOLUME) != 0 {
        crate::set_mic_gain(0);
        crate::store!(crate::state::MIC_MUTED, true);
    } // API
    if crate::load!(crate::state::API_STATE) {
        tinyapi::SERVER_CMD.send(tinyapi::ServerCommand::Stop).await;
        crate::store!(crate::state::API_STATE, false);
        defmt::info!("API disabled");
    } // WIFI
    if crate::load!(crate::state::WIFI_STATE) {
        crate::base::wifi::WIFI_CMD.send(crate::base::wifi::WifiCommand::Disable).await;
        crate::store!(crate::state::WIFI_STATE, false);
        defmt::info!("Wi‑Fi disabled");
    } // SAVE
    crate::store!(crate::state::VOICE_STATE, false);
    defmt::info!("VOICE ASSISTANT: OFF");
}

// ───────────────────────────────────────────────────────────────────────
// TOGGLE ENTIRE VOICE ASSISTANT
pub async fn toggle_voice() {
    if crate::load!(crate::state::VOICE_STATE) {
        voice_off().await;
    } else {
        voice_on().await;
    }    
}


// ───────────────────────────────────────────────────────────────────────
// ENDPOINT
// ENABLE/DISABLE ENTIRE VOICE ASSISTANCE
pub async fn voice_handler(req: tinyapi::AsyncRequest) -> tinyapi::Response {
    let value = req.param("value").unwrap_or("toggle");
    match value {
        "1" | "on" | "enable" | "enabled"    => voice_on().await,
        "0" | "off" | "disable" | "disabled" => voice_off().await,
        "toggle"  => toggle_voice().await,
        _         => defmt::warn!("Unknown voice state: {}", value),
    }

    let is_on = crate::load!(crate::state::VOICE_STATE);
    defmt::info!("Voice Assistant is now: {}", if is_on { "on" } else { "off" });
    tinyapi::Response::text(if is_on { "on" } else { "off" })
}
