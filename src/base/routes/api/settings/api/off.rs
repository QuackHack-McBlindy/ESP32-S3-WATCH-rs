// BASE/ROUTES/API/SETTINGS/API/OFF


// TOGGLE THE API/WEB SERVER ON OR OFF
pub async fn toggle_api() {
    let current = crate::load!(crate::state::API_STATE);
    if !current {
        // TURN API ON
        tinyapi::SERVER_CMD.send(tinyapi::ServerCommand::Start).await;
        crate::store!(crate::state::API_STATE, true);
    } else {
        // TURN API OFF
        tinyapi::SERVER_CMD.send(tinyapi::ServerCommand::Stop).await;
        crate::store!(crate::state::API_STATE, false);
    }
}

// /API/SETTINGS/API/OFF
pub async fn disable_api(req: tinyapi::AsyncRequest) -> tinyapi::Response {
    tinyapi::SERVER_CMD.send(tinyapi::ServerCommand::Stop).await;
    crate::store!(crate::state::API_STATE, false);
    defmt::info!("API & WEB SERVER DISABLED");
    tinyapi::Response::text("API & webserver disabled")
}
