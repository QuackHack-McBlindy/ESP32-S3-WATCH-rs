// BASE/ROUTES/API/SETTINGS/DISPLAY/PAGE

// GET /API/SETTINGS/DISPLAY/PAGE/{val}
pub fn page_handler(req: tinyapi::Request<'_>) -> tinyapi::Response {
    let page_str = req.param("value").unwrap_or("0");
    let page: u8 = page_str.parse().unwrap_or(0);
    defmt::info!("Switching page to {}", page);

    // SWITCH DISPLAY PAGE  – THIS TRIGGERS A REDRAW IN THE DISPLAY TASK
    crate::store!(crate::gui::pages::CURRENT_PAGE, page);

    let msg = alloc::format!("Page set to {}", page);
    tinyapi::Response::text(&msg)
}
