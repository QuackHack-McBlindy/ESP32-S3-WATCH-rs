// BASE/ROUTES/API/SETTINGS/DISPLAY/CALL

// GET /API/SETTINGS/DISPLAY/CALL/{val}
pub fn call_handler(req: tinyapi::Request<'_>) -> tinyapi::Response {
    let caller = req.param("value").unwrap_or("?");
    defmt::info!("{} is calling! Switching page to {}", caller, 100);

    critical_section::with(|cs| {
        let mut cell = crate::state::CALLER_NAME.borrow(cs).borrow_mut();
        let mut name = heapless::String::<32>::new();
        let _ = name.push_str(caller);
        *cell = Some(name);
    });

    // SWITCH DISPLAY PAGE  – THIS TRIGGERS A REDRAW IN THE DISPLAY TASK
    crate::store!(crate::gui::pages::CURRENT_PAGE, 100);

    let msg = alloc::format!("Caller is set");
    tinyapi::Response::text(&msg)
}
