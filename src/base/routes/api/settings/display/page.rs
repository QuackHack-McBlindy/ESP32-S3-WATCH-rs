// BASE/ROUTES/API/SETTINGS/DISPLAY/PAGE

// GET /API/SETTINGS/DISPLAY/PAGE/{val}
pub fn page_handler(req: tinyapi::Request<'_>) -> tinyapi::Response {
    let value = req.param("value").unwrap_or("?");
    defmt::info!("Switching page to {}", value);

    // CONVERT PARAM TO A VALID PAGE NUMBER
    let page = match value.parse::<u8>() {
        Ok(p) if p <= 4 => p,
        _ => {
            return tinyapi::Response::text("Invalid page number. Use 0–4.");
        }
    };

    // SWITCH DISPLAY PAGE  – THIS TRIGGERS A REDRAW IN THE DISPLAY TASK
    crate::store!(crate::gui::pages::CURRENT_PAGE, page);

    let msg = alloc::format!("Page set to {}", page);
    tinyapi::Response::text(&msg)
}
