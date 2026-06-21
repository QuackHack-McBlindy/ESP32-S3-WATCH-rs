// BASE/ROUTES/API/SETTINGS/DISPLAY/REDRAW


// GET /API/SETTINGS/DISPLAY/REDRAW
pub fn display_redraw_handler(req: tinyapi::Request<'_>) -> tinyapi::Response {
    crate::dirty!();
    defmt::info!("Redrawed display");
    tinyapi::Response::text("Redrawed the display")
}

// GET /API/SETTINGS/DISPLAY/REDRAW/LOOP/
pub async fn redraw_loop_handler(req: tinyapi::AsyncRequest) -> tinyapi::Response {
    let value = req.param("value").unwrap_or("toggle");
    match value {
        "1" | "on" | "start" | "enable" | "enabled" => crate::dirty_loop_on!(),
        "0" | "off" | "stop" | "disable" | "disabled" => crate::dirty_loop_off!(),
        _ => { }
    }
       
    tinyapi::Response::text("Redrawing the display")
}

