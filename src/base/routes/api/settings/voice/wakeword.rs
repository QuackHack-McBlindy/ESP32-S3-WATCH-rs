// BASE/ROUTES/API/SETTINGS/VOICE/WAKEWORD
// ENABLE WAKEWORD DETECTION: ```yo_esp::VOICE_CMD.send(yo_esp::VoiceCommand::Enabled).await;```
// DISABLE WAKE-WORD DETECTION: ```yo_esp::VOICE_CMD.send(yo_esp::VoiceCommand::Disabled).await;```



// ───────────────────────────────────────────────────────────────────────
// WAKE WORD DETECTION ON
pub async fn wake_word_on() {
    yo_esp::VOICE_CMD.send(yo_esp::VoiceCommand::Enabled).await;
    crate::store!(crate::state::WAKE_WORD_ENABLED, true);
}

// ───────────────────────────────────────────────────────────────────────
// WAKE WORD DETECTION OFF
pub async fn wake_word_off() {
    yo_esp::VOICE_CMD.send(yo_esp::VoiceCommand::Disabled).await;
    crate::store!(crate::state::WAKE_WORD_ENABLED, false);
}

// ───────────────────────────────────────────────────────────────────────
// TOGGLE WAKE WORD DETECTION
pub async fn toggle_wake_word() {
    let current = crate::load!(crate::state::SPEAKER_ALLOW_STREAMING);
    if !current {
        yo_esp::VOICE_CMD.send(yo_esp::VoiceCommand::Enabled).await;
        crate::store!(crate::state::WAKE_WORD_ENABLED, true);
    } else {
        yo_esp::VOICE_CMD.send(yo_esp::VoiceCommand::Disabled).await;
        crate::store!(crate::state::WAKE_WORD_ENABLED, false);
    }
}


// ───────────────────────────────────────────────────────────────────────
// ENABLE/DISABLE WAKE WORD DETECTION
pub async fn wake_word_handler(req: tinyapi::AsyncRequest) -> tinyapi::Response {
    let value = req.param("value").unwrap_or("?");
    match value {
        "1" | "on" | "enable" | "enabled" => { 
            yo_esp::VOICE_CMD.send(yo_esp::VoiceCommand::Enabled).await;
            crate::store!(crate::state::WAKE_WORD_ENABLED, true);
        }
        "0" | "off" | "disable" | "disabled" => {
            yo_esp::VOICE_CMD.send(yo_esp::VoiceCommand::Disabled).await;
            crate::store!(crate::state::WAKE_WORD_ENABLED, false);
        }
        _ => {}
    }
    tinyapi::Response::text("OK")
}
