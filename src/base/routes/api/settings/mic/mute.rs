// SRC/BASE/ROUTES/API/SETTINGS/MIC/MUTE

pub fn mic_mute_handler(req: tinyapi::Request<'_>) -> tinyapi::Response {
    let value = req.param("value").unwrap_or("toggle");
    match value {
        "1" | "on" | "mute" => crate::store!(crate::state::MIC_MUTED, true),
        "0" | "off" | "unmute" => crate::store!(crate::state::MIC_MUTED, false),
        _ => {
            let new = !crate::load!(crate::state::MIC_MUTED);
            crate::store!(crate::state::MIC_MUTED, new);
        }
    }
    let muted = crate::load!(crate::state::MIC_MUTED);
    if muted {
        crate::store!(crate::state::MIC_VOLUME, 0);
    } else {
        crate::store!(crate::state::MIC_VOLUME, 72);
    }
    defmt::info!("Mic muted: {}", muted);
    tinyapi::Response::text(if muted { "muted" } else { "unmuted" })
}

