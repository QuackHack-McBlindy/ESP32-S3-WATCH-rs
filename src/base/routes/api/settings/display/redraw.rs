// BASE/ROUTES/API/SETTINGS/DISPLAY/REDRAW


// GET /API/SETTINGS/DISPLAY/REDRAW
pub fn display_redraw_handler(req: tinyapi::Request<'_>) -> tinyapi::Response {
    crate::dirty!();
    defmt::info!("Redrawed display");
    tinyapi::Response::text("Redrawed the display")
}

