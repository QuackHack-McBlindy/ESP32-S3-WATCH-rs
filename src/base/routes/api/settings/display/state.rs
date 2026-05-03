// GET /API/SETTINGS/DISPLAY
pub fn display_state_handler(req: tinyapi::Request<'_>) -> tinyapi::Response {
    let value = req.param("value").unwrap_or("toggle");
    match value {
        "on" => crate::store!(crate::state::DISPLAY_STATE, true),
        "off" => crate::store!(crate::state::DISPLAY_STATE, false),
        _ => {
            let new = crate::load!(crate::state::DISPLAY_STATE);
            crate::store!(crate::state::DISPLAY_STATE, new);
        }
    }
    let state = crate::load!(crate::state::DISPLAY_STATE);
    defmt::info!("Display state -> {}", if state { "ON" } else { "OFF" });
    tinyapi::Response::text(if state { "ON" } else { "OFF" })
}

