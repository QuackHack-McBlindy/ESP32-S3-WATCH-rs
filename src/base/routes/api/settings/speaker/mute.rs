// SRC/BASE/ROUTES/API/SETTINGS/SPEAKER/MUTE
pub fn speaker_mute_handler(req: tinyapi::Request<'_>) -> tinyapi::Response {
    let value = req.param("value").unwrap_or("toggle");
    match value {
        "1" | "on" | "mute" => crate::store!(crate::state::SPEAKER_MUTED, true),
        "0" | "off" | "unmute" => crate::store!(crate::state::SPEAKER_MUTED, false),
        _ => {
            let new = crate::load!(crate::state::SPEAKER_MUTED);
            crate::store!(crate::state::SPEAKER_MUTED, new);
        }
    }
    let muted = crate::load!(crate::state::SPEAKER_MUTED);
    if muted {
        crate::store!(crate::state::SPEAKER_VOLUME, 0);
    } else {
        crate::store!(crate::state::SPEAKER_VOLUME, 58);
    }
    defmt::info!("Speaker muted: {}", muted);
    tinyapi::Response::text(if muted { "muted" } else { "unmuted" })
}
