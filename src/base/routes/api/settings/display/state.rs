// BASE/ROUTES/API/SETTINGS/DISPLAY/STATE


// GET /API/SETTINGS/DISPLAY
pub async fn display_state_handler(req: tinyapi::AsyncRequest) -> tinyapi::Response {
    let value = req.param("value").unwrap_or("toggle");
    match value {
        "1" | "on" | "start" | "enable" | "enabled"   => crate::store!(crate::state::DISPLAY_STATE, true),
        "0" | "off" | "stop" | "disable" | "disabled" => crate::store!(crate::state::DISPLAY_STATE, false),
        _ => { }
    }
        
    let new = crate::load!(crate::state::DISPLAY_STATE);
    let msg = match new {
        true => {
             crate::components::co5300::wake_up();
             crate::DISPLAY_CMD.send(crate::DisplayCommand::Start).await;
             crate::store!(crate::state::DISPLAY_TOUCH_ACTIVITY, true);
        }
        false => {
             crate::DISPLAY_CMD.send(crate::DisplayCommand::Stop).await;
        }        
    };
    let state = crate::load!(crate::state::DISPLAY_STATE);
    defmt::info!("Display state is now {}", if state { "ON" } else { "OFF" });
    tinyapi::Response::text(if state { "ON" } else { "OFF" })
}

