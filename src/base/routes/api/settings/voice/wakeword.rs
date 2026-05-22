// SRC/BASE/ROUTES/API/SETTINGS/VOICE/WAKEWORD

// ENABLE/DISABLE WAKE WORD DETECTION
pub fn wake_word_handler(req: tinyapi::Request<'_>) -> tinyapi::Response {
    let value = req.param("value").unwrap_or("?");
    match value {
        "1" | "on" | "enable" | "enabled" => {
            let _ = yo_esp::VOICE_CMD.try_send(yo_esp::VoiceCommand::Enabled);
        }
        "0" | "off" | "disable" | "disabled" => {
            let _ = yo_esp::VOICE_CMD.try_send(yo_esp::VoiceCommand::Disabled);
        }
        _ => {}
    }
    tinyapi::Response::text("OK")
}
