// BASE/ROUTES/API/SETTINGS/DISPLAY/BRIGHTNESS


// GET /API/SETTINGS/DISPLAY/BRIGHTNESS/{val}
pub fn brightness_handler(req: tinyapi::Request<'_>) -> tinyapi::Response {
    let value = req.param("value").unwrap_or("?");
    defmt::info!("Setting brightness to {}", value);
    if let Ok(percent) = value.parse::<u8>() {
        let percent = percent.clamp(0, 80);
        crate::store!(crate::state::DISPLAY_BRIGHTNESS, percent);
    }
    let msg = alloc::format!("Brightness set to {}", value);
    tinyapi::Response::text(&msg)
}
