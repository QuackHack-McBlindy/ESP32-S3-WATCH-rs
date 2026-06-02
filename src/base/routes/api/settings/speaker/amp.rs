// SRC/BASE/ROUTES/API/SETTINGS/SPEAKER/AMP

pub fn set_amp(state: bool) {
    if state {
        crate::amp_on();
    } else {
        crate::amp_off();
    }
    crate::store!(crate::state::AMPLIFIER_STATE, state);
}

pub fn toggle_amp() {
    let current = crate::load!(crate::state::AMPLIFIER_STATE);
    set_amp(!current);
}


pub fn amp_handler(req: tinyapi::Request<'_>) -> tinyapi::Response {
    let value = req.param("value").unwrap_or("toggle");

    match value {
        "1" | "on" | "enable" | "enabled" => set_amp(true),
        "0" | "off" | "disable" | "disabled" => set_amp(false),
        "toggle" => toggle_amp(),
        _ => defmt::warn!("Unknown amplifier command: {}", value),
    }

    let is_on = crate::load!(crate::state::AMPLIFIER_STATE);
    defmt::info!("Amplifier is now: {}", if is_on { "on" } else { "off" });
    tinyapi::Response::text(if is_on { "on" } else { "off" })
}
