// BASE/ROUTES/API/SETTINGS/DISPLAY/TEXT

// GET /API/SETTINGS/DISPLAY/TEXT/{val}
pub fn display_string_handler(req: tinyapi::Request<'_>) -> tinyapi::Response {
    let value = req.param("value").unwrap_or("?");
    defmt::info!("{} wants to be displayed! Switching page!", value);

    critical_section::with(|cs| {
        let mut cell = crate::state::DISPLAY_STRING.borrow(cs).borrow_mut();
        let mut string = heapless::String::<32>::new();
        let _ = string.push_str(value);
        *cell = Some(string);
    });

    // SWITCH DISPLAY PAGE  – THIS TRIGGERS A REDRAW IN THE DISPLAY TASK
    crate::store!(crate::gui::pages::CURRENT_PAGE, 101);

    let msg = alloc::format!("String is set");
    tinyapi::Response::text(&msg)
}
