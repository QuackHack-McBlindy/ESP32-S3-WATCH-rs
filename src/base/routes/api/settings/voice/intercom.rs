// BASE/ROUTES/API/SETTINGS/VOICE/PING


// ───────────────────────────────────────────────────────────────────────
// INTERCOM HANDLER
pub async fn intercom_handler(req: tinyapi::AsyncRequest) -> tinyapi::Response {
    let value = req.param("value").unwrap_or("toggle");

    match value {
        "1" | "on" | "enable" | "enabled"    => crate::store!(crate::state::INTERCOM_STATE, true),
        "0" | "off" | "disable" | "disabled" => crate::store!(crate::state::INTERCOM_STATE, false),
        "toggle" => {
            let current = crate::load!(crate::state::INTERCOM_STATE);
            crate::store!(crate::state::INTERCOM_STATE, !current);
        }
        _ => defmt::warn!("Unknown value"),
    }

    let is_on = crate::load!(crate::state::INTERCOM_STATE);

    if is_on {
        yo_esp::VOICE_CMD.send(yo_esp::VoiceCommand::Intercom).await;
    } else {
        yo_esp::VOICE_CMD.send(yo_esp::VoiceCommand::Disabled).await;
    }

    defmt::info!("intercom is now: {}", if is_on { "on" } else { "off" });
    tinyapi::Response::text(if is_on { "on" } else { "off" })
}
